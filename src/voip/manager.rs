use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use base64::Engine;
use matrix_sdk::Client;
use matrix_sdk::ruma::{OwnedDeviceId, OwnedRoomId};
use tokio::sync::{Mutex, mpsc};
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

use crate::config::AudioConfig;
use crate::event::{AppEvent, EventSender, WarnClosed};
use crate::voip::audio::AudioPipeline;
use crate::voip::livekit::{LiveKitEvent, LiveKitSession};
use crate::voip::matrixrtc;
use crate::voip::state::CallState;

/// Commands sent from the App to the CallManager
#[derive(Debug)]
pub enum CallCommand {
    /// Start/join a call in a room
    Initiate { room_id: String },
    /// Leave current call
    Leave,
    /// Shutdown the CallManager
    Shutdown,
    /// Encryption key received via sync for a participant
    EncryptionKeyReceived {
        room_id: String,
        user_id: String,
        device_id: String,
        key_index: i32,
        key_bytes: Vec<u8>,
    },
}

pub type CallCommandSender = mpsc::UnboundedSender<CallCommand>;
pub type CallCommandReceiver = mpsc::UnboundedReceiver<CallCommand>;

/// Creates a new command channel for the CallManager
pub fn command_channel() -> (CallCommandSender, CallCommandReceiver) {
    mpsc::unbounded_channel()
}

struct ActiveCall {
    room_id: OwnedRoomId,
    device_id: OwnedDeviceId,
    livekit_session: LiveKitSession,
    audio: AudioPipeline,
    participants: Vec<String>,
    /// Our encryption key (kept so we can send it to new participants via to-device)
    encryption_key: Vec<u8>,
    /// Participants whose encryption key fetch returned 404 — retry periodically
    pending_keys: HashMap<String, Instant>,
    /// Identities whose encryption keys have been received (skip state event polling for these)
    received_keys: HashSet<String>,
}

pub struct CallManager {
    cmd_rx: CallCommandReceiver,
    event_tx: EventSender,
    client: Arc<Mutex<Option<Client>>>,
    active_call: Option<ActiveCall>,
    audio_config: Arc<parking_lot::RwLock<AudioConfig>>,
    transmitting: Arc<AtomicBool>,
    mic_active: Arc<AtomicBool>,
}

impl CallManager {
    pub fn new(
        cmd_rx: CallCommandReceiver,
        event_tx: EventSender,
        client: Arc<Mutex<Option<Client>>>,
        audio_config: Arc<parking_lot::RwLock<AudioConfig>>,
        transmitting: Arc<AtomicBool>,
        mic_active: Arc<AtomicBool>,
    ) -> Self {
        Self {
            cmd_rx,
            event_tx,
            client,
            active_call: None,
            audio_config,
            transmitting,
            mic_active,
        }
    }

    pub async fn run(mut self) {
        info!("CallManager started");

        let mut key_retry_interval = tokio::time::interval(Duration::from_secs(2));
        key_retry_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(CallCommand::Shutdown) | None => {
                            info!("CallManager shutting down");
                            self.cleanup().await;
                            break;
                        }
                        Some(cmd) => self.handle_command(cmd).await,
                    }
                }
                // Check for LiveKit events from active call
                event = async {
                    if let Some(ref mut call) = self.active_call {
                        call.livekit_session.recv_event().await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    if let Some(event) = event {
                        self.handle_livekit_event(event).await;
                    }
                }
                // Retry fetching encryption keys for participants that returned 404
                _ = key_retry_interval.tick() => {
                    self.retry_pending_keys().await;
                }
            }
        }

        info!("CallManager stopped");
    }

    async fn handle_command(&mut self, cmd: CallCommand) {
        match cmd {
            CallCommand::Initiate { room_id } => {
                self.initiate_call(room_id).await;
            }
            CallCommand::Leave => {
                self.leave_call().await;
            }
            CallCommand::Shutdown => {} // handled in run()
            CallCommand::EncryptionKeyReceived {
                room_id,
                user_id,
                device_id,
                key_index,
                key_bytes,
            } => {
                if let Some(ref mut call) = self.active_call {
                    if call.room_id.as_str() != room_id {
                        return;
                    }
                    // Find the matching participant identity
                    let identity = format!("{}:{}", user_id, device_id);
                    debug!(
                        "Encryption key received via sync for {}, index={}, key={}...",
                        identity,
                        key_index,
                        key_bytes
                            .iter()
                            .take(4)
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>()
                    );
                    call.livekit_session
                        .set_participant_key(&identity, key_index, key_bytes);
                    call.received_keys.insert(identity.clone());
                    call.pending_keys.remove(&identity);
                }
            }
        }
    }

    async fn initiate_call(&mut self, room_id: String) {
        info!("Initiating call in room {}", room_id);

        // Clean up any existing call first
        if self.active_call.is_some() {
            self.cleanup().await;
        }

        let client = {
            let guard = self.client.lock().await;
            match guard.as_ref() {
                Some(client) => client.clone(),
                None => {
                    self.event_tx
                        .send(AppEvent::CallError("Not logged in".to_string()))
                        .warn_closed("CallError");
                    return;
                }
            }
        };

        let room_id: OwnedRoomId = match room_id.as_str().try_into() {
            Ok(id) => id,
            Err(e) => {
                self.event_tx
                    .send(AppEvent::CallError(format!("Invalid room ID: {}", e)))
                    .warn_closed("CallError");
                return;
            }
        };

        let device_id: OwnedDeviceId = match client.device_id() {
            Some(id) => id.to_owned(),
            None => {
                self.event_tx
                    .send(AppEvent::CallError("No device ID".to_string()))
                    .warn_closed("CallError");
                return;
            }
        };

        // 1. Discover LiveKit focus
        let focus = match matrixrtc::discover_livekit_focus(&client).await {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to discover LiveKit focus: {:#}", e);
                self.event_tx
                    .send(AppEvent::CallError(
                        "No VoIP service configured on this homeserver".to_string(),
                    ))
                    .warn_closed("CallError");
                return;
            }
        };

        // 2. Ensure user has permission to send m.call.member, auto-fix if admin
        if let Err(e) = matrixrtc::ensure_call_member_permissions(&client, &room_id).await {
            error!("Call permission check failed: {:#}", e);
            self.event_tx
                .send(AppEvent::CallError(format!("{}", e)))
                .warn_closed("CallError");
            return;
        }

        // 3. Publish m.call.member state event (before requesting JWT —
        //    some SFU implementations expect the state event to exist)
        let call_member_event_id: Option<String> =
            match matrixrtc::publish_call_member(&client, &room_id, &device_id, &focus).await {
                Ok(event_id) => Some(event_id),
                Err(e) => {
                    warn!(
                        "Failed to publish m.call.member (will attempt call anyway): {:#}",
                        e
                    );
                    None
                }
            };

        // 3b. Send rtc.notification so other clients ring (MSC4075)
        if let Some(ref event_id) = call_member_event_id
            && let Err(e) = matrixrtc::send_call_notify(&client, &room_id, event_id).await
        {
            warn!(
                "Failed to send call notification (ringing may not work on other clients): {:#}",
                e
            );
        }

        // 4. Get LiveKit credentials (JWT) — SFU can now see the call member event
        let creds = match matrixrtc::get_livekit_credentials(
            &client,
            &focus.livekit_service_url,
            &room_id,
            &device_id,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get LiveKit credentials: {:#}", e);
                // Clean up the state event we just published
                if call_member_event_id.is_some()
                    && let Err(e) =
                        matrixrtc::remove_call_member(&client, &room_id, &device_id).await
                {
                    warn!("remove_call_member failed: {e}");
                }
                self.event_tx
                    .send(AppEvent::CallError(
                        "Failed to get call credentials from the SFU service".to_string(),
                    ))
                    .warn_closed("CallError");
                return;
            }
        };

        // 5. Generate E2EE encryption key
        let mut encryption_key = vec![0u8; 16];
        if let Err(e) = getrandom::getrandom(&mut encryption_key) {
            error!("Failed to generate encryption key: {}", e);
            if call_member_event_id.is_some()
                && let Err(e) = matrixrtc::remove_call_member(&client, &room_id, &device_id).await
            {
                warn!("remove_call_member failed: {e}");
            }
            self.event_tx
                .send(AppEvent::CallError(
                    "Failed to generate encryption key".to_string(),
                ))
                .warn_closed("CallError");
            return;
        }

        // 5b. Publish our encryption key to Matrix
        let published_encryption_keys =
            matrixrtc::publish_encryption_keys(&client, &room_id, &device_id, &encryption_key)
                .await
                .is_ok();
        if !published_encryption_keys {
            warn!("Failed to publish encryption keys (call will proceed without E2EE signaling)");
        }

        // 5c. Connect to LiveKit with E2EE
        warn!("Connecting to LiveKit server: {}", creds.server_url);
        log_jwt_claims(&creds.token);
        let encryption_key_copy = encryption_key.clone();
        let use_e2ee = self.audio_config.read().e2ee;
        let session = match LiveKitSession::connect(
            &creds.server_url,
            &creds.token,
            encryption_key,
            use_e2ee,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to connect to LiveKit: {:#}", e);
                log_jwt_claims(&creds.token);
                // Try to clean up state events
                if call_member_event_id.is_some()
                    && let Err(e) =
                        matrixrtc::remove_call_member(&client, &room_id, &device_id).await
                {
                    warn!("remove_call_member failed: {e}");
                }
                if published_encryption_keys
                    && let Err(e) =
                        matrixrtc::remove_encryption_keys(&client, &room_id, &device_id).await
                {
                    warn!("remove_encryption_keys failed: {e}");
                }
                let err_str = format!("{:#}", e);
                let msg = if err_str.contains("401") || err_str.contains("nauthorized") {
                    "LiveKit rejected credentials — check SFU server configuration".to_string()
                } else {
                    format!("LiveKit connection failed: {}", err_str)
                };
                self.event_tx
                    .send(AppEvent::CallError(msg))
                    .warn_closed("CallError");
                return;
            }
        };

        // 5d. Read other participants' encryption keys directly from server
        info!("Our LiveKit identity: {}", session.local_identity());
        let remote_identities = session.remote_participants();
        let mut pending_keys = HashMap::new();
        let mut received_keys = HashSet::new();
        let mut participant_pairs: Vec<(String, String)> = Vec::new();
        for identity in &remote_identities {
            if let Some((user_id, dev_id)) = matrixrtc::parse_livekit_identity(identity) {
                participant_pairs.push((user_id.to_string(), dev_id.to_string()));
                if received_keys.contains(identity) {
                    continue;
                }
                match matrixrtc::fetch_participant_key(&client, &room_id, user_id, dev_id).await {
                    Ok(Some(pk)) => {
                        info!(
                            "Setting key for remote participant: identity={}, user_id={}, device_id={}",
                            identity, pk.user_id, pk.device_id
                        );
                        session.set_participant_key(identity, pk.key_index, pk.key_bytes);
                        received_keys.insert(identity.clone());
                    }
                    Ok(None) => {
                        debug!("No encryption key yet for {} (will retry)", identity);
                        pending_keys.insert(identity.clone(), Instant::now());
                    }
                    Err(e) => warn!("Failed to fetch key for {}: {:#}", identity, e),
                }
            }
        }

        // 5e. Send our encryption key to existing participants via to-device messages
        if !participant_pairs.is_empty()
            && let Err(e) = matrixrtc::send_encryption_keys_to_device(
                &client,
                &room_id,
                &device_id,
                &encryption_key_copy,
                &participant_pairs,
            )
            .await
        {
            warn!("Failed to send encryption keys via to-device: {:#}", e);
        }

        // 6. Start audio capture
        let mut audio = AudioPipeline::new();
        let audio_cfg = self.audio_config.read().clone();
        let source = match audio.start_capture(
            &audio_cfg,
            self.transmitting.clone(),
            self.mic_active.clone(),
        ) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to start audio capture: {:#}", e);
                if let Err(e) = session.close().await {
                    warn!("session.close() failed: {e}");
                }
                if call_member_event_id.is_some()
                    && let Err(e) =
                        matrixrtc::remove_call_member(&client, &room_id, &device_id).await
                {
                    warn!("remove_call_member failed: {e}");
                }
                self.event_tx
                    .send(AppEvent::CallError(
                        "Microphone error: could not start audio capture".to_string(),
                    ))
                    .warn_closed("CallError");
                return;
            }
        };

        // 7. Publish local audio track
        if let Err(e) = session.publish_audio(source).await {
            error!("Failed to publish audio track: {:#}", e);
            audio.stop();
            if let Err(e) = session.close().await {
                warn!("session.close() failed: {e}");
            }
            if call_member_event_id.is_some()
                && let Err(e) = matrixrtc::remove_call_member(&client, &room_id, &device_id).await
            {
                warn!("remove_call_member failed: {e}");
            }
            self.event_tx
                .send(AppEvent::CallError(
                    "Failed to publish audio to the call".to_string(),
                ))
                .warn_closed("CallError");
            return;
        }

        self.event_tx
            .send(AppEvent::CallStateChanged {
                room_id: room_id.to_string(),
                state: CallState::Connecting,
            })
            .warn_closed("CallStateChanged");

        self.active_call = Some(ActiveCall {
            room_id,
            device_id,
            livekit_session: session,
            audio,
            participants: Vec::new(),
            encryption_key: encryption_key_copy,
            pending_keys,
            received_keys,
        });
    }

    async fn leave_call(&mut self) {
        if let Some(call) = self.active_call.as_ref() {
            let room_id = call.room_id.clone();
            info!("Leaving call in room {}", room_id);

            // Remove m.call.member and encryption key state events
            let client = self.client.lock().await.clone();
            if let Some(client) = client
                && let Some(device_id) = client.device_id()
            {
                let device_id = device_id.to_owned();
                if let Err(e) = matrixrtc::remove_call_member(&client, &room_id, &device_id).await {
                    warn!("remove_call_member failed: {e}");
                }
                if let Err(e) =
                    matrixrtc::remove_encryption_keys(&client, &room_id, &device_id).await
                {
                    warn!("remove_encryption_keys failed: {e}");
                }
            }
        }

        self.cleanup().await;
        self.event_tx
            .send(AppEvent::CallEnded)
            .warn_closed("CallEnded");
    }

    async fn handle_livekit_event(&mut self, event: LiveKitEvent) {
        match event {
            LiveKitEvent::Connected => {
                info!("LiveKit connected");
                if let Some(ref call) = self.active_call {
                    self.event_tx
                        .send(AppEvent::CallStateChanged {
                            room_id: call.room_id.to_string(),
                            state: CallState::Active,
                        })
                        .warn_closed("CallStateChanged");
                }
            }
            LiveKitEvent::ParticipantJoined { identity } => {
                info!("Participant joined: {}", identity);
                if let Some(ref mut call) = self.active_call {
                    if !call.participants.contains(&identity) {
                        call.participants.push(identity.clone());
                    }
                    // Proactively fetch their encryption key and send ours via to-device
                    let client = self.client.lock().await.clone();
                    if let Some(ref client) = client
                        && let Some((user_id, dev_id)) =
                            matrixrtc::parse_livekit_identity(&identity)
                    {
                        if !call.received_keys.contains(&identity) {
                            match matrixrtc::fetch_participant_key(
                                client,
                                &call.room_id,
                                user_id,
                                dev_id,
                            )
                            .await
                            {
                                Ok(Some(pk)) => {
                                    call.livekit_session.set_participant_key(
                                        &identity,
                                        pk.key_index,
                                        pk.key_bytes,
                                    );
                                    call.received_keys.insert(identity.clone());
                                }
                                Ok(None) => {
                                    debug!(
                                        "No encryption key yet for {} (will retry periodically)",
                                        identity
                                    );
                                    call.pending_keys.insert(identity.clone(), Instant::now());
                                }
                                Err(e) => {
                                    warn!("Failed to fetch key for {}: {:#}", identity, e)
                                }
                            }
                        }

                        // Send our encryption key to the new participant via to-device
                        let pairs = vec![(user_id.to_string(), dev_id.to_string())];
                        if let Err(e) = matrixrtc::send_encryption_keys_to_device(
                            client,
                            &call.room_id,
                            &call.device_id,
                            &call.encryption_key,
                            &pairs,
                        )
                        .await
                        {
                            warn!(
                                "Failed to send encryption key via to-device to {}: {:#}",
                                identity, e
                            );
                        }
                    }
                    self.event_tx
                        .send(AppEvent::CallParticipantUpdate {
                            participants: call.participants.clone(),
                        })
                        .warn_closed("CallParticipantUpdate");
                }
            }
            LiveKitEvent::ParticipantLeft { identity } => {
                info!("Participant left: {}", identity);
                if let Some(ref mut call) = self.active_call {
                    call.participants.retain(|p| p != &identity);
                    self.event_tx
                        .send(AppEvent::CallParticipantUpdate {
                            participants: call.participants.clone(),
                        })
                        .warn_closed("CallParticipantUpdate");
                }
            }
            LiveKitEvent::TrackSubscribed {
                track,
                participant_identity,
            } => {
                info!("Track subscribed from: {}", participant_identity);
                if let Some(ref mut call) = self.active_call {
                    // Fetch encryption key for this specific participant directly from server
                    let client = self.client.lock().await.clone();
                    if let Some(ref client) = client
                        && let Some((user_id, device_id)) =
                            matrixrtc::parse_livekit_identity(&participant_identity)
                        && !call.received_keys.contains(&participant_identity)
                    {
                        match matrixrtc::fetch_participant_key(
                            client,
                            &call.room_id,
                            user_id,
                            device_id,
                        )
                        .await
                        {
                            Ok(Some(pk)) => {
                                debug!(
                                    "Setting remote E2EE key for {}: {}...",
                                    participant_identity,
                                    pk.key_bytes
                                        .iter()
                                        .take(4)
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<String>()
                                );
                                call.livekit_session.set_participant_key(
                                    &participant_identity,
                                    pk.key_index,
                                    pk.key_bytes,
                                );
                                call.received_keys.insert(participant_identity.clone());
                            }
                            Ok(None) => {
                                warn!(
                                    "No encryption key for {} (will retry periodically)",
                                    participant_identity
                                );
                                call.pending_keys
                                    .insert(participant_identity.clone(), Instant::now());
                            }
                            Err(e) => {
                                warn!("Failed to fetch key for {}: {:#}", participant_identity, e)
                            }
                        }
                    }

                    // Start playback (audio will decrypt once key is set)
                    info!(
                        "Starting playback for {} (encryption key: {})",
                        participant_identity,
                        if call.received_keys.contains(&participant_identity) {
                            "available"
                        } else {
                            "NOT available"
                        }
                    );
                    let audio_cfg = self.audio_config.read().clone();
                    if let Err(e) = call.audio.add_playback(track, &audio_cfg) {
                        error!(
                            "Failed to start playback for {}: {:#}",
                            participant_identity, e
                        );
                    }
                }
            }
            LiveKitEvent::TrackUnsubscribed {
                participant_identity,
            } => {
                info!("Track unsubscribed from: {}", participant_identity);
            }
            LiveKitEvent::Disconnected { reason } => {
                warn!("LiveKit disconnected: {}", reason);
                self.cleanup().await;
                self.event_tx
                    .send(AppEvent::CallError(format!("Disconnected: {}", reason)))
                    .warn_closed("CallError");
            }
            LiveKitEvent::Reconnecting => {
                info!("LiveKit reconnecting...");
            }
            LiveKitEvent::Reconnected => {
                info!("LiveKit reconnected");
            }
            LiveKitEvent::E2eeStateChanged {
                participant_identity,
                state,
            } => {
                info!("E2EE state: {} → {}", participant_identity, state);
            }
        }
    }

    async fn retry_pending_keys(&mut self) {
        let call = match self.active_call.as_mut() {
            Some(c) if !c.pending_keys.is_empty() => c,
            _ => return,
        };

        let client = self.client.lock().await.clone();
        let client = match client.as_ref() {
            Some(c) => c,
            None => return,
        };

        // Collect identities to retry (remove entries older than 30s)
        let now = Instant::now();
        let mut resolved = Vec::new();
        let mut expired = Vec::new();

        for (identity, first_seen) in &call.pending_keys {
            if call.received_keys.contains(identity) {
                resolved.push(identity.clone());
                continue;
            }
            if now.duration_since(*first_seen) > Duration::from_secs(30) {
                warn!("Giving up on encryption key for {} after 30s", identity);
                expired.push(identity.clone());
                continue;
            }

            if let Some((user_id, device_id)) = matrixrtc::parse_livekit_identity(identity) {
                match matrixrtc::fetch_participant_key(client, &call.room_id, user_id, device_id)
                    .await
                {
                    Ok(Some(pk)) => {
                        info!(
                            "Retry succeeded: got encryption key for {} (index={})",
                            identity, pk.key_index
                        );
                        call.livekit_session.set_participant_key(
                            identity,
                            pk.key_index,
                            pk.key_bytes,
                        );
                        resolved.push(identity.clone());
                    }
                    Ok(None) => {
                        debug!("Still no encryption key for {} (retrying...)", identity);
                    }
                    Err(e) => {
                        debug!("Retry fetch failed for {}: {:#}", identity, e);
                    }
                }
            }
        }

        for id in resolved.iter().chain(expired.iter()) {
            call.pending_keys.remove(id);
        }
    }

    async fn cleanup(&mut self) {
        if let Some(mut call) = self.active_call.take() {
            call.audio.stop();
            if let Err(e) = call.livekit_session.close().await {
                error!("Error closing LiveKit session: {:#}", e);
            }
            info!("Call in room {} cleaned up", call.room_id);
        }
    }
}

/// Decode and log JWT claims for diagnostics (without verification).
fn log_jwt_claims(token: &str) {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        debug!("JWT: malformed token ({} parts)", parts.len());
        return;
    }

    let payload = match base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1]) {
        Ok(bytes) => bytes,
        Err(_) => {
            // Try URL-safe variant
            match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1]) {
                Ok(bytes) => bytes,
                Err(e) => {
                    debug!("JWT: failed to decode payload: {}", e);
                    return;
                }
            }
        }
    };

    let claims: serde_json::Value = match serde_json::from_slice(&payload) {
        Ok(v) => v,
        Err(e) => {
            debug!("JWT: failed to parse payload JSON: {}", e);
            return;
        }
    };

    let video = claims.get("video");

    let room = video
        .and_then(|v| v.get("room"))
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");
    let sub = claims
        .get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");
    let iss = claims
        .get("iss")
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");
    let exp = claims.get("exp").and_then(|v| v.as_i64());

    let room_join = video
        .and_then(|v| v.get("roomJoin"))
        .map_or("<missing>".to_string(), |v| v.to_string());
    let can_publish = video
        .and_then(|v| v.get("canPublish"))
        .map_or("<missing>".to_string(), |v| v.to_string());
    let can_subscribe = video
        .and_then(|v| v.get("canSubscribe"))
        .map_or("<missing>".to_string(), |v| v.to_string());

    let expired = exp.map_or("unknown".to_string(), |exp_ts| {
        let now = chrono::Utc::now().timestamp();
        if now > exp_ts {
            format!("YES (expired {}s ago)", now - exp_ts)
        } else {
            format!("no (valid for {}s)", exp_ts - now)
        }
    });

    debug!(
        "JWT grant: iss={}, roomJoin={}, canPublish={}, canSubscribe={}",
        iss, room_join, can_publish, can_subscribe
    );
    debug!(
        "JWT claims: video.room={}, sub={}, expired={}",
        room, sub, expired
    );
    if let Some(v) = video {
        debug!("JWT full video grant: {}", v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- log_jwt_claims smoke tests (no panics) ---

    #[test]
    fn log_jwt_valid_three_parts() {
        // header.payload.signature — payload = {"sub":"test","iss":"lk","video":{"room":"r"}}
        let payload =
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(br#"{"sub":"test","iss":"lk","video":{"room":"r","roomJoin":true,"canPublish":true,"canSubscribe":true},"exp":9999999999}"#);
        let token = format!("eyJhbGciOiJIUzI1NiJ9.{}.signature", payload);
        log_jwt_claims(&token); // should not panic
    }

    #[test]
    fn log_jwt_malformed_two_parts() {
        log_jwt_claims("only.two"); // should not panic
    }

    #[test]
    fn log_jwt_empty() {
        log_jwt_claims(""); // should not panic
    }

    #[test]
    fn log_jwt_invalid_base64_payload() {
        log_jwt_claims("header.!!!invalid!!!.sig"); // should not panic
    }
}
