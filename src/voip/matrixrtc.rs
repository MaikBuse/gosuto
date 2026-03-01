use anyhow::{Context, Result};
use matrix_sdk::Client;
use matrix_sdk::ruma::{OwnedDeviceId, OwnedRoomId};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

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
        Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
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
    let legacy_ok = legacy_result
        .as_ref()
        .is_ok_and(|r| r.status().is_success());

    let (response, endpoint_used) = if legacy_ok {
        debug!("LiveKit /sfu/get succeeded");
        (legacy_result.unwrap(), "/sfu/get")
    } else {
        match legacy_result {
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
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "LiveKit JWT service (/get_token) returned {}: {}",
                status,
                body
            );
        }
        (resp, "/get_token")
    };

    let raw_body = response
        .text()
        .await
        .context("Failed to read JWT service response body")?;
    let sfu_preview: serde_json::Value = serde_json::from_str(&raw_body).unwrap_or_default();
    debug!(
        "SFU response (via {}): url present={}, jwt length={}",
        endpoint_used,
        sfu_preview.get("url").is_some(),
        sfu_preview.get("jwt").and_then(|v| v.as_str()).map_or(0, |s| s.len()),
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

    if user_pl >= required_pl {
        debug!(
            "User PL {} >= required PL {} for m.call.member, permission OK",
            user_pl, required_pl
        );
        return Ok(());
    }

    // User doesn't have permission — can we fix it?
    let pl_change_pl = events
        .and_then(|e| e.get("m.room.power_levels").and_then(|v| v.as_i64()))
        .unwrap_or(state_default);

    if user_pl < pl_change_pl {
        anyhow::bail!(
            "Cannot start call: insufficient permissions. Ask a room admin to allow call events."
        );
    }

    // We can modify power levels — add call member event overrides
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

    room.send_state_event_raw("", "m.room.power_levels", updated)
        .await
        .context("Failed to update power levels for call events")?;

    info!("Updated room power levels to allow m.call.member events");
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
    let user_id = client.user_id().context("Not logged in")?;
    let room = client.get_room(room_id).context("Room not found")?;

    let state_key = format!("_{}_{}_{}", user_id, device_id, "m.call");

    let content = serde_json::json!({
        "application": "m.call",
        "call_id": "",
        "scope": "m.room",
        "device_id": device_id.to_string(),
        "expires": 7_200_000,
        "focus_active": {
            "type": "livekit",
            "focus_selection": "oldest_membership",
        },
        "foci_preferred": [{
            "type": "livekit",
            "livekit_alias": room_id.to_string(),
            "livekit_service_url": focus.livekit_service_url,
        }],
    });

    let pretty_json = serde_json::to_string_pretty(&content).unwrap_or_default();
    debug!("Publishing m.call.member: state_key={}, content={}", state_key, pretty_json);

    // Try unstable event type first (Element X watches for this), fall back to stable
    let result = room
        .send_state_event_raw(&state_key, "org.matrix.msc3401.call.member", content.clone())
        .await;

    let event_id = match result {
        Ok(resp) => resp.event_id.to_string(),
        Err(unstable_err) => {
            debug!("Unstable org.matrix.msc3401.call.member failed ({unstable_err:#}), trying stable type");
            let resp = room
                .send_state_event_raw(&state_key, "m.call.member", content)
                .await
                .context("Failed to publish m.call.member state event")?;
            resp.event_id.to_string()
        }
    };

    info!(
        "Published m.call.member for room {} (state_key: {}, event_id: {})",
        room_id, state_key, event_id
    );
    Ok(event_id)
}

/// Remove the m.call.member state event (leave the call).
pub async fn remove_call_member(
    client: &Client,
    room_id: &OwnedRoomId,
    device_id: &OwnedDeviceId,
) -> Result<()> {
    let user_id = client.user_id().context("Not logged in")?;
    let room = client.get_room(room_id).context("Room not found")?;

    let state_key = format!("_{}_{}_{}", user_id, device_id, "m.call");

    let content = serde_json::json!({});

    // Try unstable event type first (Element X watches for this), fall back to stable
    let result = room
        .send_state_event_raw(&state_key, "org.matrix.msc3401.call.member", content.clone())
        .await;

    if let Err(unstable_err) = result {
        debug!("Unstable org.matrix.msc3401.call.member failed ({unstable_err:#}), trying stable type");
        room.send_state_event_raw(&state_key, "m.call.member", content)
            .await
            .context("Failed to remove m.call.member state event")?;
    }

    info!(
        "Removed m.call.member for room {} (state_key: {})",
        room_id, state_key
    );
    Ok(())
}
