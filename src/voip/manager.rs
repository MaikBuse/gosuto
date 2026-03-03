use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use base64::Engine;
use matrix_sdk::Client;
use matrix_sdk::ruma::{OwnedDeviceId, OwnedRoomId};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, warn};

use crate::config::AudioConfig;
use crate::event::{AppEvent, EventSender};
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
}

pub type CallCommandSender = mpsc::UnboundedSender<CallCommand>;
pub type CallCommandReceiver = mpsc::UnboundedReceiver<CallCommand>;

/// Creates a new command channel for the CallManager
pub fn command_channel() -> (CallCommandSender, CallCommandReceiver) {
    mpsc::unbounded_channel()
}

struct ActiveCall {
    room_id: OwnedRoomId,
    livekit_session: LiveKitSession,
    audio: AudioPipeline,
    participants: Vec<String>,
}

pub struct CallManager {
    cmd_rx: CallCommandReceiver,
    event_tx: EventSender,
    client: Arc<Mutex<Option<Client>>>,
    active_call: Option<ActiveCall>,
    audio_config: Arc<std::sync::Mutex<AudioConfig>>,
    transmitting: Arc<AtomicBool>,
}

impl CallManager {
    pub fn new(
        cmd_rx: CallCommandReceiver,
        event_tx: EventSender,
        client: Arc<Mutex<Option<Client>>>,
        audio_config: Arc<std::sync::Mutex<AudioConfig>>,
        transmitting: Arc<AtomicBool>,
    ) -> Self {
        Self {
            cmd_rx,
            event_tx,
            client,
            active_call: None,
            audio_config,
            transmitting,
        }
    }

    pub async fn run(mut self) {
        info!("CallManager started");

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
                    let _ = self
                        .event_tx
                        .send(AppEvent::CallError("Not logged in".to_string()));
                    return;
                }
            }
        };

        let room_id: OwnedRoomId = match room_id.as_str().try_into() {
            Ok(id) => id,
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::CallError(format!("Invalid room ID: {}", e)));
                return;
            }
        };

        let device_id: OwnedDeviceId = match client.device_id() {
            Some(id) => id.to_owned(),
            None => {
                let _ = self
                    .event_tx
                    .send(AppEvent::CallError("No device ID".to_string()));
                return;
            }
        };

        // 1. Discover LiveKit focus
        let focus = match matrixrtc::discover_livekit_focus(&client).await {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to discover LiveKit focus: {:#}", e);
                let _ = self.event_tx.send(AppEvent::CallError(
                    "No VoIP service configured on this homeserver".to_string(),
                ));
                return;
            }
        };

        // 2. Ensure user has permission to send m.call.member, auto-fix if admin
        if let Err(e) = matrixrtc::ensure_call_member_permissions(&client, &room_id).await {
            error!("Call permission check failed: {:#}", e);
            let _ = self.event_tx.send(AppEvent::CallError(format!("{}", e)));
            return;
        }

        // 3. Publish m.call.member state event (before requesting JWT —
        //    some SFU implementations expect the state event to exist)
        let call_member_event_id: Option<String> =
            match matrixrtc::publish_call_member(&client, &room_id, &device_id, &focus).await {
                Ok(event_id) => Some(event_id),
                Err(e) => {
                    warn!("Failed to publish m.call.member (will attempt call anyway): {:#}", e);
                    None
                }
            };

        // 3b. Send rtc.notification so other clients ring (MSC4075)
        if let Some(ref event_id) = call_member_event_id {
            if let Err(e) = matrixrtc::send_call_notify(&client, &room_id, event_id).await {
                warn!(
                    "Failed to send call notification (ringing may not work on other clients): {:#}",
                    e
                );
            }
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
                if call_member_event_id.is_some() {
                    let _ = matrixrtc::remove_call_member(&client, &room_id, &device_id).await;
                }
                let _ = self.event_tx.send(AppEvent::CallError(
                    "Failed to get call credentials from the SFU service".to_string(),
                ));
                return;
            }
        };

        // 5. Generate E2EE encryption key
        let mut encryption_key = vec![0u8; 32];
        if let Err(e) = getrandom::getrandom(&mut encryption_key) {
            error!("Failed to generate encryption key: {}", e);
            if call_member_event_id.is_some() {
                let _ = matrixrtc::remove_call_member(&client, &room_id, &device_id).await;
            }
            let _ = self.event_tx.send(AppEvent::CallError(
                "Failed to generate encryption key".to_string(),
            ));
            return;
        }

        // 5b. Publish our encryption key to Matrix
        let published_encryption_keys = matrixrtc::publish_encryption_keys(
            &client, &room_id, &device_id, &encryption_key,
        )
        .await
        .is_ok();
        if !published_encryption_keys {
            warn!("Failed to publish encryption keys (call will proceed without E2EE signaling)");
        }

        // 5c. Connect to LiveKit with E2EE
        warn!("Connecting to LiveKit server: {}", creds.server_url);
        log_jwt_claims(&creds.token);
        let session =
            match LiveKitSession::connect(&creds.server_url, &creds.token, encryption_key).await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to connect to LiveKit: {:#}", e);
                    log_jwt_claims(&creds.token);
                    // Try to clean up state events
                    if call_member_event_id.is_some() {
                        let _ = matrixrtc::remove_call_member(&client, &room_id, &device_id).await;
                    }
                    if published_encryption_keys {
                        let _ = matrixrtc::remove_encryption_keys(&client, &room_id, &device_id).await;
                    }
                    let err_str = format!("{:#}", e);
                    let msg = if err_str.contains("401") || err_str.contains("nauthorized") {
                        "LiveKit rejected credentials — check SFU server configuration".to_string()
                    } else {
                        format!("LiveKit connection failed: {}", err_str)
                    };
                    let _ = self.event_tx.send(AppEvent::CallError(msg));
                    return;
                }
            };

        // 5d. Read other participants' encryption keys directly from server
        info!("Our LiveKit identity: {}", session.local_identity());
        let remote_identities = session.remote_participants();
        for identity in &remote_identities {
            if let Some((user_id, device_id)) = matrixrtc::parse_livekit_identity(identity) {
                match matrixrtc::fetch_participant_key(&client, &room_id, user_id, device_id).await
                {
                    Ok(Some(pk)) => {
                        info!(
                            "Setting key for remote participant: identity={}, user_id={}, device_id={}",
                            identity, pk.user_id, pk.device_id
                        );
                        session.set_participant_key(identity, pk.key_index, pk.key_bytes);
                    }
                    Ok(None) => debug!("No encryption key yet for {}", identity),
                    Err(e) => warn!("Failed to fetch key for {}: {:#}", identity, e),
                }
            }
        }

        // 6. Start audio capture
        let mut audio = AudioPipeline::new();
        let audio_cfg = self.audio_config.lock().unwrap().clone();
        let source = match audio.start_capture(&audio_cfg, self.transmitting.clone()) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to start audio capture: {:#}", e);
                let _ = session.close().await;
                if call_member_event_id.is_some() {
                    let _ = matrixrtc::remove_call_member(&client, &room_id, &device_id).await;
                }
                let _ = self.event_tx.send(AppEvent::CallError(
                    "Microphone error: could not start audio capture".to_string(),
                ));
                return;
            }
        };

        // 7. Publish local audio track
        if let Err(e) = session.publish_audio(source).await {
            error!("Failed to publish audio track: {:#}", e);
            audio.stop();
            let _ = session.close().await;
            if call_member_event_id.is_some() {
                let _ = matrixrtc::remove_call_member(&client, &room_id, &device_id).await;
            }
            let _ = self.event_tx.send(AppEvent::CallError(
                "Failed to publish audio to the call".to_string(),
            ));
            return;
        }

        let _ = self.event_tx.send(AppEvent::CallStateChanged {
            room_id: room_id.to_string(),
            state: CallState::Connecting,
        });

        self.active_call = Some(ActiveCall {
            room_id,
            livekit_session: session,
            audio,
            participants: Vec::new(),
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
                let _ = matrixrtc::remove_call_member(&client, &room_id, &device_id).await;
                let _ = matrixrtc::remove_encryption_keys(&client, &room_id, &device_id).await;
            }
        }

        self.cleanup().await;
        let _ = self.event_tx.send(AppEvent::CallEnded);
    }

    async fn handle_livekit_event(&mut self, event: LiveKitEvent) {
        match event {
            LiveKitEvent::Connected => {
                info!("LiveKit connected");
                if let Some(ref call) = self.active_call {
                    let _ = self.event_tx.send(AppEvent::CallStateChanged {
                        room_id: call.room_id.to_string(),
                        state: CallState::Active,
                    });
                }
            }
            LiveKitEvent::ParticipantJoined { identity } => {
                info!("Participant joined: {}", identity);
                if let Some(ref mut call) = self.active_call {
                    if !call.participants.contains(&identity) {
                        call.participants.push(identity.clone());
                    }
                    // Proactively fetch their encryption key
                    let client = self.client.lock().await.clone();
                    if let Some(ref client) = client
                        && let Some((user_id, device_id)) =
                            matrixrtc::parse_livekit_identity(&identity)
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
                                call.livekit_session.set_participant_key(
                                    &identity,
                                    pk.key_index,
                                    pk.key_bytes,
                                );
                            }
                            Ok(None) => debug!(
                                "No encryption key yet for {} (will retry on track subscribe)",
                                identity
                            ),
                            Err(e) => {
                                warn!("Failed to fetch key for {}: {:#}", identity, e)
                            }
                        }
                    }
                    let _ = self.event_tx.send(AppEvent::CallParticipantUpdate {
                        participants: call.participants.clone(),
                    });
                }
            }
            LiveKitEvent::ParticipantLeft { identity } => {
                info!("Participant left: {}", identity);
                if let Some(ref mut call) = self.active_call {
                    call.participants.retain(|p| p != &identity);
                    let _ = self.event_tx.send(AppEvent::CallParticipantUpdate {
                        participants: call.participants.clone(),
                    });
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
                                call.livekit_session.set_participant_key(
                                    &participant_identity,
                                    pk.key_index,
                                    pk.key_bytes,
                                );
                            }
                            Ok(None) => {
                                warn!("No encryption key for {}", participant_identity)
                            }
                            Err(e) => {
                                warn!("Failed to fetch key for {}: {:#}", participant_identity, e)
                            }
                        }
                    }

                    // Start playback (audio will decrypt once key is set)
                    let audio_cfg = self.audio_config.lock().unwrap().clone();
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
                let _ = self
                    .event_tx
                    .send(AppEvent::CallError(format!("Disconnected: {}", reason)));
            }
            LiveKitEvent::Reconnecting => {
                info!("LiveKit reconnecting...");
            }
            LiveKitEvent::Reconnected => {
                info!("LiveKit reconnected");
            }
            LiveKitEvent::Error(err) => {
                error!("LiveKit error: {}", err);
                let _ = self.event_tx.send(AppEvent::CallError(err));
            }
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
