use std::collections::HashMap;

use anyhow::{Context, Result};
use base64::Engine;
use matrix_sdk::Client;
use matrix_sdk::ruma::{OwnedDeviceId, OwnedRoomId};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

pub struct LiveKitCredentials {
    pub server_url: String,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WellKnownResponse {
    #[serde(rename = "org.matrix.msc4143.rtc_foci")]
    rtc_foci: Option<Vec<RtcFocus>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RtcFocus {
    #[serde(rename = "type")]
    focus_type: String,
    livekit_service_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LivekitFocus {
    pub livekit_service_url: String,
}

/// Discover the LiveKit focus from the homeserver's well-known endpoint.
pub async fn discover_livekit_focus(client: &Client) -> Result<LivekitFocus> {
    let homeserver = client.homeserver().to_string();
    let well_known_url = format!(
        "{}/_matrix/client/unstable/org.matrix.msc4143/rtc_foci",
        homeserver.trim_end_matches('/')
    );

    let http_client = reqwest::Client::new();

    // Try MSC4143 endpoint first
    let response = http_client.get(&well_known_url).send().await;

    let foci: Vec<RtcFocus> = match response {
        Ok(resp) if resp.status().is_success() => match resp.json().await {
            Ok(foci) => foci,
            Err(e) => {
                warn!("Failed to parse MSC4143 RTC foci response: {e}");
                vec![]
            }
        },
        _ => {
            // Fallback: try .well-known/matrix/client
            let base = homeserver.trim_end_matches('/');
            let wk_url = format!("{}/.well-known/matrix/client", base);
            let wk_resp = http_client.get(&wk_url).send().await?;
            let wk: WellKnownResponse = wk_resp.json().await?;
            wk.rtc_foci.unwrap_or_default()
        }
    };

    foci.iter()
        .find(|f| f.focus_type == "livekit")
        .and_then(|f| f.livekit_service_url.clone())
        .map(|url| LivekitFocus {
            livekit_service_url: url,
        })
        .context("No LiveKit focus found in homeserver configuration")
}

/// Select the LiveKit focus to connect to based on existing participants' m.call.member events.
///
/// In federated rooms, each homeserver has its own SFU. To join an existing call,
/// we must connect to the same SFU that other participants are using. This function
/// reads the room state to find the most commonly advertised LiveKit focus URL.
///
/// Returns `Some(focus)` if active participants advertise a LiveKit focus,
/// or `None` if no existing participants have one (i.e. we are starting a new call).
pub async fn select_focus_from_room_state(
    client: &Client,
    room_id: &OwnedRoomId,
) -> Result<Option<LivekitFocus>> {
    let request =
        matrix_sdk::ruma::api::client::state::get_state_events::v3::Request::new(room_id.clone());
    let response = client
        .send(request)
        .await
        .context("Failed to fetch room state for focus selection")?;

    let mut focus_counts: HashMap<String, usize> = HashMap::new();

    for raw_event in &response.room_state {
        let json: serde_json::Value = match serde_json::from_str(raw_event.json().get()) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if event_type != "m.call.member" && event_type != "org.matrix.msc3401.call.member" {
            continue;
        }

        let content = match json.get("content").and_then(|c| c.as_object()) {
            Some(c) if !c.is_empty() => c,
            _ => continue, // empty content = member left
        };

        // Session-format: foci_preferred at top level
        if let Some(foci) = content.get("foci_preferred").and_then(|v| v.as_array()) {
            for focus in foci {
                if focus.get("type").and_then(|v| v.as_str()) == Some("livekit")
                    && let Some(url) = focus.get("livekit_service_url").and_then(|v| v.as_str())
                {
                    *focus_counts.entry(url.to_string()).or_default() += 1;
                }
            }
        }

        // Legacy format: memberships[].foci_active
        if let Some(memberships) = content.get("memberships").and_then(|v| v.as_array()) {
            for membership in memberships {
                if let Some(foci) = membership.get("foci_active").and_then(|v| v.as_array()) {
                    for focus in foci {
                        if focus.get("type").and_then(|v| v.as_str()) == Some("livekit")
                            && let Some(url) =
                                focus.get("livekit_service_url").and_then(|v| v.as_str())
                        {
                            *focus_counts.entry(url.to_string()).or_default() += 1;
                        }
                    }
                }
            }
        }
    }

    if focus_counts.is_empty() {
        debug!("No existing LiveKit foci found in room state");
        return Ok(None);
    }

    let (best_url, count) = focus_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .expect("focus_counts is non-empty");

    info!(
        "Selected focus from room state: {} ({} participant(s) using it)",
        best_url, count
    );

    Ok(Some(LivekitFocus {
        livekit_service_url: best_url,
    }))
}

/// Get LiveKit credentials (server URL + JWT) from the LiveKit JWT service.
pub async fn get_livekit_credentials(
    client: &Client,
    service_url: &str,
    room_id: &OwnedRoomId,
    device_id: &OwnedDeviceId,
) -> Result<LiveKitCredentials> {
    // Get OpenID token from homeserver
    let user_id = client.user_id().context("Not logged in")?.to_owned();
    let openid_request =
        matrix_sdk::ruma::api::client::account::request_openid_token::v3::Request::new(
            user_id.clone(),
        );
    let openid = client.send(openid_request).await?;
    let access_token = openid.access_token;
    let expires_in = openid.expires_in.as_secs();

    let http_client = reqwest::Client::new();

    #[derive(Serialize)]
    struct OpenIdToken {
        access_token: String,
        token_type: String,
        matrix_server_name: String,
        expires_in: u64,
    }

    #[derive(Serialize)]
    struct SfuRequest {
        room_id: String,
        slot_id: String,
        openid_token: OpenIdToken,
        member: MatrixRTCMember,
    }

    #[derive(Serialize)]
    struct MatrixRTCMember {
        id: String,
        claimed_user_id: String,
        claimed_device_id: String,
    }

    #[derive(Serialize)]
    struct LegacySfuRequest {
        room: String,
        openid_token: OpenIdToken,
        device_id: String,
    }

    #[derive(Deserialize)]
    struct SfuResponse {
        url: String,
        jwt: String,
    }

    let server_name = client
        .user_id()
        .context("Not logged in")?
        .server_name()
        .to_string();

    let base = service_url.trim_end_matches('/');

    let make_openid_token = |access_token: String| OpenIdToken {
        access_token,
        token_type: "Bearer".to_string(),
        matrix_server_name: server_name.clone(),
        expires_in,
    };

    // Try legacy endpoint first: /sfu/get (compatible with Element X)
    let legacy_url = format!("{}/sfu/get", base);
    let legacy_body = LegacySfuRequest {
        room: room_id.to_string(),
        openid_token: make_openid_token(access_token.clone()),
        device_id: device_id.to_string(),
    };

    let legacy_result = http_client
        .post(&legacy_url)
        .json(&legacy_body)
        .send()
        .await;
    let (response, endpoint_used) = match legacy_result {
        Ok(resp) if resp.status().is_success() => {
            debug!("LiveKit /sfu/get succeeded");
            (resp, "/sfu/get")
        }
        other => {
            match other {
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<unreadable>".to_string());
                    let truncated = &body[..body.floor_char_boundary(500)];
                    error!(
                        "LiveKit /sfu/get returned {} (body: {}), falling back to /get_token",
                        status, truncated
                    );
                }
                Err(e) => error!(
                    "LiveKit /sfu/get network error ({}), falling back to /get_token",
                    e
                ),
            }

            // Fallback to new endpoint: /get_token (MSC4195)
            let new_url = format!("{}/get_token", base);
            let new_body = SfuRequest {
                room_id: room_id.to_string(),
                slot_id: "0".to_string(),
                openid_token: make_openid_token(access_token),
                member: MatrixRTCMember {
                    id: format!("{}_{}", user_id, device_id),
                    claimed_user_id: user_id.to_string(),
                    claimed_device_id: device_id.to_string(),
                },
            };
            let resp = http_client
                .post(&new_url)
                .json(&new_body)
                .send()
                .await
                .context("Failed to contact LiveKit JWT service (/get_token)")?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|_| "<unreadable>".to_string());
                anyhow::bail!(
                    "LiveKit JWT service (/get_token) returned {}: {}",
                    status,
                    body
                );
            }
            (resp, "/get_token")
        }
    };

    let raw_body = response
        .text()
        .await
        .context("Failed to read JWT service response body")?;
    let sfu_preview: serde_json::Value = match serde_json::from_str(&raw_body) {
        Ok(v) => v,
        Err(e) => {
            debug!("Failed to parse SFU response for debug preview: {e}");
            serde_json::Value::Null
        }
    };
    debug!(
        "SFU response (via {}): url present={}, jwt length={}",
        endpoint_used,
        sfu_preview.get("url").is_some(),
        sfu_preview
            .get("jwt")
            .and_then(|v| v.as_str())
            .map_or(0, |s| s.len()),
    );
    let sfu_resp: SfuResponse =
        serde_json::from_str(&raw_body).context("Invalid JWT service response JSON")?;

    info!(
        "Got LiveKit credentials for room {} (via {})",
        room_id, endpoint_used
    );
    Ok(LiveKitCredentials {
        server_url: sfu_resp.url,
        token: sfu_resp.jwt,
    })
}

/// Ensure the user has permission to send `m.call.member` state events.
/// If the user is an admin (can modify power levels), auto-fix by lowering the
/// required PL for call member events to 0.
pub async fn ensure_call_member_permissions(client: &Client, room_id: &OwnedRoomId) -> Result<()> {
    let user_id = client.user_id().context("Not logged in")?;
    let room = client.get_room(room_id).context("Room not found")?;

    // Fetch raw m.room.power_levels state event via the Client API
    let request = matrix_sdk::ruma::api::client::state::get_state_event_for_key::v3::Request::new(
        room_id.clone(),
        "m.room.power_levels".into(),
        "".to_owned(),
    );

    let pl_json: serde_json::Value = match client.send(request).await {
        Ok(resp) => serde_json::from_str(resp.event_or_content.get())?,
        Err(_) => {
            // No power levels event — default rules apply, state_default=50
            anyhow::bail!(
                "Cannot start call: insufficient permissions. Ask a room admin to allow call events."
            );
        }
    };

    let user_id_str = user_id.to_string();
    let users = pl_json.get("users").and_then(|u| u.as_object());
    let users_default = pl_json
        .get("users_default")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let state_default = pl_json
        .get("state_default")
        .and_then(|v| v.as_i64())
        .unwrap_or(50);

    let user_pl = users
        .and_then(|u| u.get(&user_id_str))
        .and_then(|v| v.as_i64())
        .unwrap_or(users_default);

    let events = pl_json.get("events").and_then(|e| e.as_object());

    let call_member_pl = events.and_then(|e| {
        e.get("m.call.member")
            .or_else(|| e.get("org.matrix.msc3401.call.member"))
            .and_then(|v| v.as_i64())
    });

    let required_pl = call_member_pl.unwrap_or(state_default);

    // Check if m.call.member is already explicitly set to PL <= 0
    let call_member_open = call_member_pl.is_some_and(|pl| pl <= 0);

    // Check if io.element.call.encryption_keys is already explicitly set to PL <= 0
    let encryption_keys_pl = events.and_then(|e| {
        e.get("io.element.call.encryption_keys")
            .and_then(|v| v.as_i64())
    });
    let encryption_keys_open = encryption_keys_pl.is_some_and(|pl| pl <= 0);

    let all_open = call_member_open && encryption_keys_open;

    if all_open && user_pl >= required_pl {
        debug!(
            "m.call.member and io.element.call.encryption_keys already at PL <= 0, user PL {} is sufficient, permission OK",
            user_pl
        );
        return Ok(());
    }

    // Determine if user can modify power levels
    let pl_change_pl = events
        .and_then(|e| e.get("m.room.power_levels").and_then(|v| v.as_i64()))
        .unwrap_or(state_default);
    let can_modify_pls = user_pl >= pl_change_pl;

    if !all_open && can_modify_pls {
        // Proactively fix power levels for all participants
        info!(
            "Auto-fixing power levels: setting m.call.member PL to 0 (user PL={}, state_default={})",
            user_pl, state_default
        );

        let mut updated = pl_json.clone();
        let events_map = updated
            .as_object_mut()
            .context("Power levels not an object")?
            .entry("events")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .context("events field not an object")?;

        events_map.insert("m.call.member".to_string(), serde_json::json!(0));
        events_map.insert(
            "org.matrix.msc3401.call.member".to_string(),
            serde_json::json!(0),
        );
        events_map.insert(
            "io.element.call.encryption_keys".to_string(),
            serde_json::json!(0),
        );

        room.send_state_event_raw("m.room.power_levels", "", updated)
            .await
            .context("Failed to update power levels for call events")?;

        info!("Updated room power levels to allow m.call.member events");
        return Ok(());
    }

    if user_pl >= required_pl {
        // User has enough PL to send call events but can't modify power levels — proceed anyway
        let encryption_keys_required_pl = encryption_keys_pl.unwrap_or(state_default);
        if user_pl < encryption_keys_required_pl {
            warn!(
                "User PL {} is sufficient for m.call.member (PL {}) but NOT for \
                 io.element.call.encryption_keys (PL {}). \
                 Encryption key state events will fail; falling back to to-device delivery.",
                user_pl, required_pl, encryption_keys_required_pl
            );
        } else {
            debug!(
                "User PL {} >= required PL {} for m.call.member, permission OK (cannot fix for others)",
                user_pl, required_pl
            );
        }
        return Ok(());
    }

    warn!(
        "User PL {} < required PL {} for m.call.member and cannot modify power levels. \
         Will attempt to join call anyway.",
        user_pl, required_pl
    );
    Ok(())
}

/// Send an rtc.notification room message event to make other clients ring.
/// Per MSC4075, this notifies room members that a call has started.
pub async fn send_call_notify(
    client: &Client,
    room_id: &OwnedRoomId,
    call_member_event_id: &str,
) -> Result<()> {
    let room = client.get_room(room_id).context("Room not found")?;

    let sender_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let content = serde_json::json!({
        "notification_type": "ring",
        "sender_ts": sender_ts,
        "lifetime": 30000,
        "m.mentions": {
            "user_ids": [],
            "room": true
        },
        "m.relates_to": {
            "event_id": call_member_event_id,
            "rel_type": "m.reference"
        },
        "m.call.intent": "audio"
    });

    let pretty_json = serde_json::to_string_pretty(&content).unwrap_or_default();
    debug!("Sending rtc.notification: content={}", pretty_json);

    // Try unstable prefix first (Element X only recognizes this while MSC4075 is unstable),
    // fall back to stable event type
    let result = room
        .send_raw("org.matrix.msc4075.rtc.notification", content.clone())
        .await;

    if let Err(unstable_err) = result {
        debug!("org.matrix.msc4075.rtc.notification failed ({unstable_err:#}), trying stable type");
        room.send_raw("m.rtc.notification", content)
            .await
            .context("Failed to send call notification event")?;
    }

    info!(
        "Sent call notification (rtc.notification) for room {} (ref: {})",
        room_id, call_member_event_id
    );
    Ok(())
}

/// Publish an m.call.member state event to join the call in a room.
/// Returns the event ID of the published state event (needed for call notifications).
pub async fn publish_call_member(
    client: &Client,
    room_id: &OwnedRoomId,
    device_id: &OwnedDeviceId,
    focus: &LivekitFocus,
) -> Result<String> {
    use matrix_sdk::ruma::events::call::member::{
        ActiveFocus, ActiveLivekitFocus, Application, CallApplicationContent, CallMemberStateKey,
        CallScope, Focus, LivekitFocus as RumaLivekitFocus,
    };

    let user_id = client.user_id().context("Not logged in")?.to_owned();
    let room = client.get_room(room_id).context("Room not found")?;

    let state_key = CallMemberStateKey::new(user_id, Some(format!("{device_id}_m.call")), true);

    let content = matrix_sdk::ruma::events::call::member::CallMemberEventContent::new(
        Application::Call(CallApplicationContent::new(String::new(), CallScope::Room)),
        device_id.to_owned(),
        ActiveFocus::Livekit(ActiveLivekitFocus::new()),
        vec![Focus::Livekit(RumaLivekitFocus::new(
            room_id.to_string(),
            focus.livekit_service_url.clone(),
        ))],
        None,
        Some(std::time::Duration::from_secs(7200)),
    );

    debug!("Publishing m.call.member: state_key={}", state_key.as_ref(),);

    let resp = room
        .send_state_event_for_key(&state_key, content)
        .await
        .context("Failed to publish m.call.member state event")?;
    let event_id = resp.event_id.to_string();

    info!(
        "Published m.call.member for room {} (state_key: {}, event_id: {})",
        room_id,
        state_key.as_ref(),
        event_id
    );
    Ok(event_id)
}

/// Decode base64 leniently, accepting both padded and unpadded input.
/// Element X uses unpadded base64, but we should handle both.
pub fn lenient_base64_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::alphabet::STANDARD as STANDARD_ALPHABET;
    use base64::engine::{GeneralPurpose, GeneralPurposeConfig};
    let engine = GeneralPurpose::new(
        &STANDARD_ALPHABET,
        GeneralPurposeConfig::new()
            .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
    );
    engine.decode(input)
}

// --- Encryption key exchange (SFrame E2EE for MatrixRTC) ---

#[derive(Debug, Clone)]
pub struct ParticipantKey {
    pub user_id: String,
    pub device_id: String,
    pub key_index: i32,
    pub key_bytes: Vec<u8>,
}

/// Parse a LiveKit identity string into (user_id, device_id).
/// LiveKit identity format: "@user:server:device_id"
pub fn parse_livekit_identity(identity: &str) -> Option<(&str, &str)> {
    identity.rsplit_once(':') // ("@marie:buse.io", "7L3ctM9tHg")
}

/// Fetch encryption key for a specific participant directly from the server.
/// This bypasses the local sync store which never populates custom event types.
pub async fn fetch_participant_key(
    client: &Client,
    room_id: &OwnedRoomId,
    user_id: &str,
    device_id: &str,
) -> Result<Option<ParticipantKey>> {
    let state_key = format!("_{}_{}", user_id, device_id);

    let request = matrix_sdk::ruma::api::client::state::get_state_event_for_key::v3::Request::new(
        room_id.clone(),
        "io.element.call.encryption_keys".into(),
        state_key,
    );

    let resp = match client.send(request).await {
        Ok(r) => r,
        Err(e) => {
            debug!(
                "No encryption key state event for {}:{}: {}",
                user_id, device_id, e
            );
            return Ok(None);
        }
    };

    let content: serde_json::Value = serde_json::from_str(resp.event_or_content.get())?;

    let key_entries = match content.get("keys").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            debug!(
                "No keys array in encryption key event for {}:{}",
                user_id, device_id
            );
            return Ok(None);
        }
    };

    // Use the first valid key entry
    for entry in key_entries {
        let index = entry.get("index").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let encoded = match entry.get("key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => continue,
        };
        let key_bytes = match lenient_base64_decode(encoded) {
            Ok(b) => b,
            Err(e) => {
                warn!(
                    "Failed to decode encryption key from {}:{}: {}",
                    user_id, device_id, e
                );
                continue;
            }
        };

        info!(
            "Fetched encryption key: user_id={}, device_id={}, index={}, key_len={}",
            user_id,
            device_id,
            index,
            key_bytes.len()
        );

        return Ok(Some(ParticipantKey {
            user_id: user_id.to_string(),
            device_id: device_id.to_string(),
            key_index: index,
            key_bytes,
        }));
    }

    Ok(None)
}

/// Publish our SFrame encryption key as an `io.element.call.encryption_keys` state event.
/// Element X reads these to decrypt audio from other participants.
pub async fn publish_encryption_keys(
    client: &Client,
    room_id: &OwnedRoomId,
    device_id: &OwnedDeviceId,
    key: &[u8],
) -> Result<()> {
    let user_id = client.user_id().context("Not logged in")?;
    let room = client.get_room(room_id).context("Room not found")?;

    let encoded_key = base64::engine::general_purpose::STANDARD_NO_PAD.encode(key);

    let state_key = format!("_{}_{}", user_id, device_id);

    let content = serde_json::json!({
        "keys": [{
            "index": 0,
            "key": encoded_key,
        }],
        "device_id": device_id.to_string(),
        "call_id": "",
    });

    debug!(
        "Publishing encryption key: state_key={}, key_len={}",
        state_key,
        key.len()
    );

    room.send_state_event_raw("io.element.call.encryption_keys", &state_key, content)
        .await
        .context("Failed to publish encryption keys")?;

    info!("Published encryption key for room {}", room_id);
    Ok(())
}

/// Send our SFrame encryption key to specific call participants as to-device messages.
/// Element X expects to receive encryption keys this way (not just via state events).
/// `participants` is a list of (user_id, device_id) pairs (e.g. from LiveKit identities).
pub async fn send_encryption_keys_to_device(
    client: &Client,
    room_id: &OwnedRoomId,
    device_id: &OwnedDeviceId,
    key: &[u8],
    participants: &[(String, String)],
) -> Result<()> {
    use matrix_sdk::encryption::identities::Device;
    use matrix_sdk::ruma::OwnedUserId;
    use matrix_sdk_base::crypto::CollectStrategy;

    let our_user_id = client.user_id().context("Not logged in")?;
    let our_device_id = device_id.to_string();

    let encoded_key = base64::engine::general_purpose::STANDARD_NO_PAD.encode(key);

    let content = serde_json::json!({
        "keys": {
            "index": 0,
            "key": encoded_key,
        },
        "member": {
            "claimed_device_id": our_device_id,
        },
        "room_id": room_id.to_string(),
        "sent_ts": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        "session": {
            "application": "m.call",
            "call_id": "",
            "scope": "m.room",
        },
    });

    let raw_content = matrix_sdk::ruma::serde::Raw::from_json(
        serde_json::value::to_raw_value(&content)
            .context("Failed to serialize encryption key content")?,
    );

    let encryption = client.encryption();
    let mut devices: Vec<Device> = Vec::new();

    for (user_id, participant_device_id) in participants {
        if user_id.as_str() == our_user_id.as_str() && participant_device_id == &our_device_id {
            continue;
        }

        let uid: OwnedUserId = match user_id.as_str().try_into() {
            Ok(id) => id,
            Err(_) => continue,
        };
        let did: OwnedDeviceId = participant_device_id.as_str().into();

        // Ensure we have fresh device keys for this user (critical for federated rooms
        // where the device may not yet be in the local crypto store)
        if let Err(e) = encryption.get_user_devices(&uid).await {
            warn!("Failed to query devices for {}: {}", uid, e);
        }

        match encryption.get_device(&uid, &did).await {
            Ok(Some(device)) => devices.push(device),
            Ok(None) => {
                warn!(
                    "Device not found for {}:{}, cannot send encryption key",
                    user_id, participant_device_id
                );
            }
            Err(e) => {
                warn!(
                    "Failed to get device {}:{}: {}",
                    user_id, participant_device_id, e
                );
            }
        }
    }

    if devices.is_empty() {
        debug!("No devices to send encryption keys to via to-device");
        return Ok(());
    }

    let device_count = devices.len();
    let failures = encryption
        .encrypt_and_send_raw_to_device(
            devices.iter().collect(),
            "io.element.call.encryption_keys",
            raw_content,
            CollectStrategy::AllDevices,
        )
        .await
        .context("Failed to send encrypted to-device encryption keys")?;

    for (uid, did) in &failures {
        warn!("Failed to encrypt encryption key for {}:{}", uid, did);
    }

    info!(
        "Sent encryption keys via to-device to {} recipient(s) for room {}",
        device_count - failures.len(),
        room_id
    );
    Ok(())
}

/// Remove our encryption key state event (called when leaving a call).
pub async fn remove_encryption_keys(
    client: &Client,
    room_id: &OwnedRoomId,
    device_id: &OwnedDeviceId,
) -> Result<()> {
    let user_id = client.user_id().context("Not logged in")?;
    let room = client.get_room(room_id).context("Room not found")?;

    let state_key = format!("_{}_{}", user_id, device_id);

    room.send_state_event_raw(
        "io.element.call.encryption_keys",
        &state_key,
        serde_json::json!({}),
    )
    .await
    .context("Failed to remove encryption keys")?;

    info!("Removed encryption keys for room {}", room_id);
    Ok(())
}

/// Remove the m.call.member state event (leave the call).
pub async fn remove_call_member(
    client: &Client,
    room_id: &OwnedRoomId,
    device_id: &OwnedDeviceId,
) -> Result<()> {
    use matrix_sdk::ruma::events::call::member::CallMemberStateKey;

    let user_id = client.user_id().context("Not logged in")?.to_owned();
    let room = client.get_room(room_id).context("Room not found")?;

    let state_key = CallMemberStateKey::new(user_id, Some(format!("{device_id}_m.call")), true);

    let content = matrix_sdk::ruma::events::call::member::CallMemberEventContent::new_empty(None);

    room.send_state_event_for_key(&state_key, content)
        .await
        .context("Failed to remove m.call.member state event")?;

    info!(
        "Removed m.call.member for room {} (state_key: {})",
        room_id,
        state_key.as_ref()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_livekit_identity ---

    #[test]
    fn parse_standard_identity() {
        assert_eq!(
            parse_livekit_identity("@marie:buse.io:7L3ctM9tHg"),
            Some(("@marie:buse.io", "7L3ctM9tHg"))
        );
    }

    #[test]
    fn parse_multiple_colons_in_server() {
        assert_eq!(
            parse_livekit_identity("@user:sub.domain.com:DEV"),
            Some(("@user:sub.domain.com", "DEV"))
        );
    }

    #[test]
    fn parse_no_colon() {
        assert_eq!(parse_livekit_identity("nocolon"), None);
    }

    #[test]
    fn parse_empty_string() {
        assert_eq!(parse_livekit_identity(""), None);
    }

    #[test]
    fn parse_single_colon() {
        assert_eq!(
            parse_livekit_identity("@user:device"),
            Some(("@user", "device"))
        );
    }

    #[test]
    fn parse_trailing_colon() {
        assert_eq!(
            parse_livekit_identity("@user:server:"),
            Some(("@user:server", ""))
        );
    }

    // --- lenient_base64_decode ---

    #[test]
    fn decode_padded_base64() {
        assert_eq!(
            lenient_base64_decode("aGVsbG8=").unwrap(),
            b"hello".to_vec()
        );
    }

    #[test]
    fn decode_unpadded_base64() {
        assert_eq!(lenient_base64_decode("aGVsbG8").unwrap(), b"hello".to_vec());
    }

    #[test]
    fn decode_empty() {
        assert_eq!(lenient_base64_decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn decode_invalid_chars() {
        assert!(lenient_base64_decode("!!!invalid!!!").is_err());
    }

    #[test]
    fn decode_roundtrip_standard_no_pad() {
        let original = b"MatrixRTC E2EE key material";
        let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(original);
        assert_eq!(lenient_base64_decode(&encoded).unwrap(), original);
    }

    #[test]
    fn decode_roundtrip_standard_padded() {
        let original = b"test";
        let encoded = base64::engine::general_purpose::STANDARD.encode(original);
        assert_eq!(lenient_base64_decode(&encoded).unwrap(), original);
    }
}
