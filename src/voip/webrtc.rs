use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{error, info};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_remote::TrackRemote;

/// Events emitted by the WebRTC session back to the CallManager
#[derive(Debug)]
pub enum WebRtcEvent {
    /// Local ICE candidate to send to remote peer
    IceCandidate(String),
    /// ICE gathering complete
    IceGatheringComplete,
    /// Peer connection state changed
    ConnectionStateChanged(RTCPeerConnectionState),
    /// Remote audio track received
    RemoteTrack(Arc<TrackRemote>),
}

pub struct WebRtcSession {
    peer_connection: Arc<RTCPeerConnection>,
    pub local_track: Arc<TrackLocalStaticSample>,
    event_rx: mpsc::UnboundedReceiver<WebRtcEvent>,
}

impl WebRtcSession {
    pub async fn new(
        ice_servers: Vec<RTCIceServer>,
    ) -> Result<Self> {
        // Create media engine with Opus support
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;

        // Create interceptor registry
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)?;

        // Build API
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        // Create peer connection
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        // Create local audio track for sending
        let local_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_string(),
                clock_rate: 48000,
                channels: 1,
                ..Default::default()
            },
            "audio".to_string(),
            "walrust-audio".to_string(),
        ));

        // Add track to peer connection
        peer_connection
            .add_track(local_track.clone() as Arc<dyn webrtc::track::track_local::TrackLocal + Send + Sync>)
            .await?;

        // Set up event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // ICE candidate callback
        let ice_tx = event_tx.clone();
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let tx = ice_tx.clone();
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    match candidate.to_json() {
                        Ok(json) => {
                            let json_str = serde_json::to_string(&json).unwrap_or_default();
                            let _ = tx.send(WebRtcEvent::IceCandidate(json_str));
                        }
                        Err(e) => error!("Failed to serialize ICE candidate: {}", e),
                    }
                } else {
                    let _ = tx.send(WebRtcEvent::IceGatheringComplete);
                }
            })
        }));

        // Connection state change callback
        let state_tx = event_tx.clone();
        peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            let tx = state_tx.clone();
            info!("WebRTC connection state changed: {:?}", state);
            Box::pin(async move {
                let _ = tx.send(WebRtcEvent::ConnectionStateChanged(state));
            })
        }));

        // Remote track callback (incoming audio)
        let track_tx = event_tx.clone();
        peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
            let tx = track_tx.clone();
            Box::pin(async move {
                info!(
                    "Got remote track: kind={}, codec={}",
                    track.kind(),
                    track.codec().capability.mime_type
                );
                let _ = tx.send(WebRtcEvent::RemoteTrack(track));
            })
        }));

        Ok(Self {
            peer_connection,
            local_track,
            event_rx,
        })
    }

    /// Create an SDP offer (caller side)
    pub async fn create_offer(&self) -> Result<String> {
        let offer = self.peer_connection.create_offer(None).await?;
        self.peer_connection.set_local_description(offer.clone()).await?;
        Ok(offer.sdp)
    }

    /// Accept an SDP offer and create an answer (callee side)
    pub async fn accept_offer(&self, sdp: &str) -> Result<String> {
        let offer = RTCSessionDescription::offer(sdp.to_string())?;
        self.peer_connection.set_remote_description(offer).await?;

        let answer = self.peer_connection.create_answer(None).await?;
        self.peer_connection.set_local_description(answer.clone()).await?;
        Ok(answer.sdp)
    }

    /// Set the remote answer SDP (caller side, after receiving callee's answer)
    pub async fn set_remote_answer(&self, sdp: &str) -> Result<()> {
        let answer = RTCSessionDescription::answer(sdp.to_string())?;
        self.peer_connection.set_remote_description(answer).await?;
        Ok(())
    }

    /// Add a remote ICE candidate
    pub async fn add_ice_candidate(&self, candidate_json: &str) -> Result<()> {
        let init: RTCIceCandidateInit = serde_json::from_str(candidate_json)?;
        self.peer_connection.add_ice_candidate(init).await?;
        Ok(())
    }

    /// Receive the next WebRTC event
    pub async fn recv_event(&mut self) -> Option<WebRtcEvent> {
        self.event_rx.recv().await
    }

    /// Close the peer connection
    pub async fn close(&self) -> Result<()> {
        self.peer_connection.close().await?;
        Ok(())
    }
}
