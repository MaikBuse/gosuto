use anyhow::{Context as _, Result};
use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions};
use livekit::e2ee::{E2eeOptions, EncryptionType};
use livekit::options::TrackPublishOptions;
use livekit::prelude::*;
use livekit::webrtc::audio_source::RtcAudioSource;
use livekit::webrtc::audio_source::native::NativeAudioSource;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info};

#[derive(Debug)]
pub enum LiveKitEvent {
    Connected,
    ParticipantJoined {
        identity: String,
    },
    ParticipantLeft {
        identity: String,
    },
    TrackSubscribed {
        track: RemoteAudioTrack,
        participant_identity: String,
    },
    TrackUnsubscribed {
        participant_identity: String,
    },
    Disconnected {
        reason: String,
    },
    Reconnecting,
    Reconnected,
    E2eeStateChanged {
        participant_identity: String,
        state: String,
    },
}

pub struct LiveKitSession {
    room: Room,
    key_provider: KeyProvider,
    event_rx: mpsc::UnboundedReceiver<LiveKitEvent>,
    _event_task: JoinHandle<()>,
}

impl LiveKitSession {
    pub async fn connect(
        server_url: &str,
        token: &str,
        encryption_key: Vec<u8>,
        use_e2ee: bool,
    ) -> Result<Self> {
        // Append access_token as query param — the Rust SDK only sends it
        // as an Authorization header, which reverse proxies may strip during
        // WebSocket upgrade. The query param ensures the token reaches LiveKit.
        let mut url = url::Url::parse(server_url).context("Invalid LiveKit server URL")?;
        url.query_pairs_mut().append_pair("access_token", token);

        let key_provider = KeyProvider::new(KeyProviderOptions {
            failure_tolerance: 10,
            ..KeyProviderOptions::default()
        });

        let mut options = RoomOptions::default();
        if use_e2ee {
            options.encryption = Some(E2eeOptions {
                encryption_type: EncryptionType::Gcm,
                key_provider: key_provider.clone(),
            });
        }

        let (room, mut room_events) = Room::connect(url.as_str(), token, options).await?;

        // Set our own encryption key using our actual LiveKit identity
        let local_identity = room.local_participant().identity();
        if use_e2ee {
            debug!(
                "Set local E2EE key: {}...",
                encryption_key
                    .iter()
                    .take(4)
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>()
            );
            key_provider.set_key(&local_identity, 0, encryption_key);
            info!(
                "Connected to LiveKit room: {} (E2EE enabled, GCM, identity: {})",
                room.name(),
                local_identity
            );
        } else {
            info!(
                "Connected to LiveKit room: {} (E2EE DISABLED for testing, identity: {})",
                room.name(),
                local_identity
            );
        }

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let event_task = tokio::spawn(async move {
            while let Some(event) = room_events.recv().await {
                let lk_event = match event {
                    RoomEvent::Connected { .. } => Some(LiveKitEvent::Connected),
                    RoomEvent::ParticipantConnected(participant) => {
                        Some(LiveKitEvent::ParticipantJoined {
                            identity: participant.identity().to_string(),
                        })
                    }
                    RoomEvent::ParticipantDisconnected(participant) => {
                        Some(LiveKitEvent::ParticipantLeft {
                            identity: participant.identity().to_string(),
                        })
                    }
                    RoomEvent::TrackSubscribed {
                        track: RemoteTrack::Audio(audio_track),
                        publication: _,
                        participant,
                    } => Some(LiveKitEvent::TrackSubscribed {
                        track: audio_track,
                        participant_identity: participant.identity().to_string(),
                    }),
                    RoomEvent::TrackSubscribed { .. } => None,
                    RoomEvent::TrackUnsubscribed {
                        publication,
                        participant,
                        ..
                    } => {
                        if publication.kind() == TrackKind::Audio {
                            Some(LiveKitEvent::TrackUnsubscribed {
                                participant_identity: participant.identity().to_string(),
                            })
                        } else {
                            None
                        }
                    }
                    RoomEvent::Disconnected { reason } => Some(LiveKitEvent::Disconnected {
                        reason: format!("{:?}", reason),
                    }),
                    RoomEvent::Reconnecting => Some(LiveKitEvent::Reconnecting),
                    RoomEvent::Reconnected => Some(LiveKitEvent::Reconnected),
                    RoomEvent::E2eeStateChanged { participant, state } => {
                        Some(LiveKitEvent::E2eeStateChanged {
                            participant_identity: participant.identity().to_string(),
                            state: format!("{:?}", state),
                        })
                    }
                    _ => None,
                };

                if let Some(ev) = lk_event
                    && event_tx.send(ev).is_err()
                {
                    break;
                }
            }
        });

        Ok(Self {
            room,
            key_provider,
            event_rx,
            _event_task: event_task,
        })
    }

    pub async fn publish_audio(&self, source: NativeAudioSource) -> Result<()> {
        let track =
            LocalAudioTrack::create_audio_track("microphone", RtcAudioSource::Native(source));
        let options = TrackPublishOptions {
            source: TrackSource::Microphone,
            ..Default::default()
        };
        self.room
            .local_participant()
            .publish_track(LocalTrack::Audio(track), options)
            .await?;
        info!("Published local audio track");
        Ok(())
    }

    pub async fn recv_event(&mut self) -> Option<LiveKitEvent> {
        self.event_rx.recv().await
    }

    pub fn local_identity(&self) -> String {
        self.room.local_participant().identity().to_string()
    }

    pub fn remote_participants(&self) -> Vec<String> {
        self.room
            .remote_participants()
            .values()
            .map(|p| p.identity().to_string())
            .collect()
    }

    /// Set the encryption key for a specific remote participant.
    pub fn set_participant_key(&self, identity: &str, key_index: i32, key: Vec<u8>) {
        self.key_provider
            .set_key(&identity.to_owned().into(), key_index, key);
    }

    pub async fn close(self) -> Result<()> {
        info!("Disconnecting from LiveKit room");
        self.room.close().await?;
        Ok(())
    }
}
