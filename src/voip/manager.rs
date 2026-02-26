use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use matrix_sdk::Client;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

use crate::config::AudioConfig;
use crate::event::{AppEvent, EventSender};
use crate::voip::audio::AudioPipeline;
use crate::voip::signaling;
use crate::voip::state::CallState;
use crate::voip::turn;
use crate::voip::webrtc::{WebRtcEvent, WebRtcSession};

/// Commands sent from the App to the CallManager
#[derive(Debug)]
pub enum CallCommand {
    /// Initiate an outgoing call
    Initiate {
        call_id: String,
        room_id: String,
    },
    /// Answer an incoming call
    Answer {
        call_id: String,
    },
    /// Reject an incoming call
    Reject {
        call_id: String,
        room_id: String,
    },
    /// Reject an incoming call automatically (busy)
    RejectIncoming {
        call_id: String,
        room_id: String,
    },
    /// Hang up an active call
    Hangup {
        call_id: String,
        room_id: String,
    },
    /// Remote peer sent an SDP answer
    RemoteAnswer {
        call_id: String,
        sdp: String,
    },
    /// Remote peer sent ICE candidates
    RemoteCandidates {
        call_id: String,
        candidates: Vec<String>,
    },
    /// Remote peer sent an invite (forwarded from App)
    RemoteInvite {
        call_id: String,
        room_id: String,
        sdp: String,
    },
    /// Remote peer hung up
    RemoteHangup {
        call_id: String,
    },
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
    call_id: String,
    room_id: String,
    webrtc: WebRtcSession,
    audio: AudioPipeline,
    invite_time: Instant,
    remote_sdp: Option<String>,
    pending_candidates: Vec<String>,
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

        let mut timeout_interval = tokio::time::interval(Duration::from_secs(1));

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
                // Check for WebRTC events from active call
                event = async {
                    if let Some(ref mut call) = self.active_call {
                        call.webrtc.recv_event().await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    if let Some(event) = event {
                        self.handle_webrtc_event(event).await;
                    }
                }
                // Timeout check
                _ = timeout_interval.tick() => {
                    self.check_timeout().await;
                }
            }
        }

        info!("CallManager stopped");
    }

    async fn handle_command(&mut self, cmd: CallCommand) {
        match cmd {
            CallCommand::Initiate {
                call_id,
                room_id,
            } => {
                self.initiate_call(call_id, room_id).await;
            }
            CallCommand::Answer { call_id } => {
                self.answer_call(call_id).await;
            }
            CallCommand::Reject { call_id, room_id } | CallCommand::RejectIncoming { call_id, room_id } => {
                self.reject_call(&call_id, &room_id).await;
            }
            CallCommand::Hangup { call_id, room_id } => {
                self.hangup_call(&call_id, &room_id, "user_hangup").await;
            }
            CallCommand::RemoteInvite {
                call_id,
                room_id,
                sdp,
            } => {
                self.handle_remote_invite(call_id, room_id, sdp).await;
            }
            CallCommand::RemoteAnswer { call_id, sdp } => {
                self.handle_remote_answer(&call_id, &sdp).await;
            }
            CallCommand::RemoteCandidates {
                call_id,
                candidates,
            } => {
                self.handle_remote_candidates(&call_id, candidates).await;
            }
            CallCommand::RemoteHangup { call_id } => {
                self.handle_remote_hangup(&call_id).await;
            }
            CallCommand::Shutdown => {} // handled in run()
        }
    }

    async fn initiate_call(&mut self, call_id: String, room_id: String) {
        info!("Initiating call {} in room {}", call_id, room_id);

        // Get ICE servers
        let ice_servers = {
            let guard = self.client.lock().await;
            if let Some(ref client) = *guard {
                turn::get_ice_servers(client).await
            } else {
                let _ = self.event_tx.send(AppEvent::CallError("Not logged in".to_string()));
                return;
            }
        };

        // Create WebRTC session
        let webrtc = match WebRtcSession::new(ice_servers).await {
            Ok(session) => session,
            Err(e) => {
                error!("Failed to create WebRTC session: {}", e);
                let _ = self.event_tx.send(AppEvent::CallError(format!("WebRTC error: {}", e)));
                return;
            }
        };

        // Start audio capture
        let mut audio = AudioPipeline::new();
        let audio_cfg = self.audio_config.lock().unwrap().clone();
        if let Err(e) = audio.start(webrtc.local_track.clone(), &audio_cfg, self.transmitting.clone()) {
            error!("Failed to start audio capture: {}", e);
            let _ = webrtc.close().await;
            let _ = self.event_tx.send(AppEvent::CallError(format!("Audio error: {}", e)));
            return;
        }

        // Create SDP offer
        let sdp = match webrtc.create_offer().await {
            Ok(sdp) => sdp,
            Err(e) => {
                error!("Failed to create SDP offer: {}", e);
                audio.stop();
                let _ = webrtc.close().await;
                let _ = self.event_tx.send(AppEvent::CallError(format!("SDP error: {}", e)));
                return;
            }
        };

        // Send m.call.invite
        let send_result = {
            let guard = self.client.lock().await;
            if let Some(ref client) = *guard {
                signaling::send_call_invite(client, &room_id, &call_id, &sdp).await
            } else {
                Err(anyhow::anyhow!("Not logged in"))
            }
        };

        if let Err(e) = send_result {
            error!("Failed to send call invite: {}", e);
            audio.stop();
            let _ = webrtc.close().await;
            let _ = self.event_tx.send(AppEvent::CallError(format!("Signaling error: {}", e)));
            return;
        }

        self.active_call = Some(ActiveCall {
            call_id: call_id.clone(),
            room_id,
            webrtc,
            audio,
            invite_time: Instant::now(),
            remote_sdp: None,
            pending_candidates: Vec::new(),
        });

        let _ = self.event_tx.send(AppEvent::CallStateChanged {
            call_id,
            state: CallState::Inviting,
        });
    }

    async fn handle_remote_invite(&mut self, call_id: String, room_id: String, sdp: String) {
        info!("Handling remote invite for call {}", call_id);

        // Get ICE servers
        let ice_servers = {
            let guard = self.client.lock().await;
            if let Some(ref client) = *guard {
                turn::get_ice_servers(client).await
            } else {
                let _ = self.event_tx.send(AppEvent::CallError("Not logged in".to_string()));
                return;
            }
        };

        // Create WebRTC session (but don't answer yet — wait for user)
        let webrtc = match WebRtcSession::new(ice_servers).await {
            Ok(session) => session,
            Err(e) => {
                error!("Failed to create WebRTC session for incoming call: {}", e);
                let _ = self.event_tx.send(AppEvent::CallError(format!("WebRTC error: {}", e)));
                return;
            }
        };

        let audio = AudioPipeline::new();

        self.active_call = Some(ActiveCall {
            call_id: call_id.clone(),
            room_id,
            webrtc,
            audio,
            invite_time: Instant::now(),
            remote_sdp: Some(sdp),
            pending_candidates: Vec::new(),
        });

        let _ = self.event_tx.send(AppEvent::CallStateChanged {
            call_id,
            state: CallState::Ringing,
        });
    }

    async fn answer_call(&mut self, call_id: String) {
        let call = match self.active_call.as_mut() {
            Some(c) if c.call_id == call_id => c,
            _ => {
                warn!("No matching call to answer: {}", call_id);
                return;
            }
        };

        let remote_sdp = match call.remote_sdp.take() {
            Some(sdp) => sdp,
            None => {
                error!("No remote SDP to answer");
                return;
            }
        };

        // Start audio capture
        let audio_cfg = self.audio_config.lock().unwrap().clone();
        if let Err(e) = call.audio.start(call.webrtc.local_track.clone(), &audio_cfg, self.transmitting.clone()) {
            error!("Failed to start audio for answer: {}", e);
            let _ = self.event_tx.send(AppEvent::CallError(format!("Audio error: {}", e)));
            self.cleanup().await;
            return;
        }

        // Accept offer and create answer
        let answer_sdp = match call.webrtc.accept_offer(&remote_sdp).await {
            Ok(sdp) => sdp,
            Err(e) => {
                error!("Failed to accept SDP offer: {}", e);
                let _ = self.event_tx.send(AppEvent::CallError(format!("SDP error: {}", e)));
                self.cleanup().await;
                return;
            }
        };

        // Process any pending ICE candidates
        let call = self.active_call.as_mut().unwrap();
        let pending = std::mem::take(&mut call.pending_candidates);
        for candidate in pending {
            if let Err(e) = call.webrtc.add_ice_candidate(&candidate).await {
                warn!("Failed to add pending ICE candidate: {}", e);
            }
        }

        let room_id = call.room_id.clone();
        let cid = call.call_id.clone();

        // Send m.call.answer
        let send_result = {
            let guard = self.client.lock().await;
            if let Some(ref client) = *guard {
                signaling::send_call_answer(client, &room_id, &cid, &answer_sdp).await
            } else {
                Err(anyhow::anyhow!("Not logged in"))
            }
        };

        if let Err(e) = send_result {
            error!("Failed to send call answer: {}", e);
            let _ = self.event_tx.send(AppEvent::CallError(format!("Signaling error: {}", e)));
            self.cleanup().await;
            return;
        }

        let _ = self.event_tx.send(AppEvent::CallStateChanged {
            call_id,
            state: CallState::Connecting,
        });
    }

    async fn reject_call(&mut self, call_id: &str, room_id: &str) {
        info!("Rejecting call {}", call_id);

        // Send hangup then cleanup — avoid holding &ActiveCall across await
        {
            let guard = self.client.lock().await;
            if let Some(ref client) = *guard {
                if let Err(e) = signaling::send_call_hangup(client, room_id, call_id, "user_hangup").await {
                    error!("Failed to send hangup for rejection: {}", e);
                }
            }
        }

        let should_cleanup = self.active_call.as_ref().is_some_and(|c| c.call_id == call_id);
        if should_cleanup {
            self.cleanup().await;
        }
    }

    async fn hangup_call(&mut self, call_id: &str, room_id: &str, reason: &str) {
        info!("Hanging up call {} (reason: {})", call_id, reason);
        {
            let guard = self.client.lock().await;
            if let Some(ref client) = *guard {
                if let Err(e) = signaling::send_call_hangup(client, room_id, call_id, reason).await {
                    error!("Failed to send hangup: {}", e);
                }
            }
        }
        self.cleanup().await;
    }

    async fn handle_remote_answer(&mut self, call_id: &str, sdp: &str) {
        let call = match self.active_call.as_mut() {
            Some(c) if c.call_id == call_id => c,
            _ => {
                warn!("No matching call for remote answer: {}", call_id);
                return;
            }
        };

        if let Err(e) = call.webrtc.set_remote_answer(sdp).await {
            error!("Failed to set remote answer: {}", e);
            let room_id = call.room_id.clone();
            let cid = call.call_id.clone();
            let _ = self.event_tx.send(AppEvent::CallError(format!("SDP error: {}", e)));
            self.hangup_call(&cid, &room_id, "unknown_error").await;
            return;
        }

        // Process pending candidates
        let call = self.active_call.as_mut().unwrap();
        let pending = std::mem::take(&mut call.pending_candidates);
        for candidate in pending {
            if let Err(e) = call.webrtc.add_ice_candidate(&candidate).await {
                warn!("Failed to add pending ICE candidate: {}", e);
            }
        }

        let _ = self.event_tx.send(AppEvent::CallStateChanged {
            call_id: call_id.to_string(),
            state: CallState::Connecting,
        });
    }

    async fn handle_remote_candidates(&mut self, call_id: &str, candidates: Vec<String>) {
        let call = match self.active_call.as_mut() {
            Some(c) if c.call_id == call_id => c,
            _ => {
                warn!("No matching call for remote candidates: {}", call_id);
                return;
            }
        };

        // If we haven't set remote description yet, buffer the candidates
        if call.remote_sdp.is_some() {
            call.pending_candidates.extend(candidates);
            return;
        }

        for candidate in candidates {
            if let Err(e) = call.webrtc.add_ice_candidate(&candidate).await {
                warn!("Failed to add ICE candidate: {}", e);
            }
        }
    }

    async fn handle_remote_hangup(&mut self, call_id: &str) {
        let matches = self.active_call.as_ref().is_some_and(|c| c.call_id == call_id);
        if matches {
            info!("Remote peer hung up call {}", call_id);
            self.cleanup().await;
            let _ = self.event_tx.send(AppEvent::CallEnded);
        }
    }

    async fn handle_webrtc_event(&mut self, event: WebRtcEvent) {
        match event {
            WebRtcEvent::IceCandidate(candidate_json) => {
                // Extract data before awaiting
                let (room_id, call_id) = match self.active_call.as_ref() {
                    Some(call) => (call.room_id.clone(), call.call_id.clone()),
                    None => return,
                };
                let candidate: serde_json::Value =
                    serde_json::from_str(&candidate_json).unwrap_or_default();
                let guard = self.client.lock().await;
                if let Some(ref client) = *guard {
                    if let Err(e) = signaling::send_call_candidates(
                        client,
                        &room_id,
                        &call_id,
                        &[candidate],
                    )
                    .await
                    {
                        warn!("Failed to send ICE candidate: {}", e);
                    }
                }
            }
            WebRtcEvent::IceGatheringComplete => {
                info!("ICE gathering complete");
            }
            WebRtcEvent::ConnectionStateChanged(state) => {
                info!("WebRTC connection state: {:?}", state);
                match state {
                    RTCPeerConnectionState::Connected => {
                        if let Some(ref call) = self.active_call {
                            let _ = self.event_tx.send(AppEvent::CallStateChanged {
                                call_id: call.call_id.clone(),
                                state: CallState::Active,
                            });
                        }
                    }
                    RTCPeerConnectionState::Failed => {
                        error!("WebRTC connection failed");
                        let ids = self.active_call.as_ref().map(|c| (c.call_id.clone(), c.room_id.clone()));
                        if let Some((call_id, room_id)) = ids {
                            self.hangup_call(&call_id, &room_id, "ice_failed").await;
                            let _ = self.event_tx.send(AppEvent::CallError("Connection failed".to_string()));
                        }
                    }
                    RTCPeerConnectionState::Disconnected | RTCPeerConnectionState::Closed => {
                        if self.active_call.is_some() {
                            self.cleanup().await;
                            let _ = self.event_tx.send(AppEvent::CallEnded);
                        }
                    }
                    _ => {}
                }
            }
            WebRtcEvent::RemoteTrack(track) => {
                info!("Got remote audio track");
                if let Some(ref mut call) = self.active_call {
                    let audio_cfg = self.audio_config.lock().unwrap().clone();
                    if let Err(e) = call.audio.add_playback(track, &audio_cfg) {
                        error!("Failed to start playback: {}", e);
                    }
                }
            }
        }
    }

    async fn check_timeout(&mut self) {
        let timed_out = self.active_call.as_ref().is_some_and(|call| {
            call.invite_time.elapsed() > Duration::from_secs(60)
        });

        if timed_out {
            let (call_id, room_id) = {
                let call = self.active_call.as_ref().unwrap();
                (call.call_id.clone(), call.room_id.clone())
            };
            warn!("Call {} timed out", call_id);
            self.hangup_call(&call_id, &room_id, "invite_timeout").await;
            let _ = self.event_tx.send(AppEvent::CallError("Call timed out".to_string()));
        }
    }

    async fn cleanup(&mut self) {
        if let Some(mut call) = self.active_call.take() {
            call.audio.stop();
            if let Err(e) = call.webrtc.close().await {
                error!("Error closing WebRTC session: {}", e);
            }
            info!("Call {} cleaned up", call.call_id);
        }
    }
}
