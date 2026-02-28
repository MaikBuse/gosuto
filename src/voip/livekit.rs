use anyhow::Result;
use livekit::options::TrackPublishOptions;
use livekit::prelude::*;
use livekit::webrtc::audio_source::RtcAudioSource;
use livekit::webrtc::audio_source::native::NativeAudioSource;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::info;

#[derive(Debug)]
#[allow(dead_code)]
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
    Error(String),
}

pub struct LiveKitSession {
    room: Room,
    event_rx: mpsc::UnboundedReceiver<LiveKitEvent>,
    _event_task: JoinHandle<()>,
}

impl LiveKitSession {
    pub async fn connect(server_url: &str, token: &str) -> Result<Self> {
        let (room, mut room_events) =
            Room::connect(server_url, token, RoomOptions::default()).await?;
        info!("Connected to LiveKit room: {}", room.name());

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

    #[allow(dead_code)]
    pub fn remote_participants(&self) -> Vec<String> {
        self.room
            .remote_participants()
            .values()
            .map(|p| p.identity().to_string())
            .collect()
    }

    pub async fn close(self) -> Result<()> {
        info!("Disconnecting from LiveKit room");
        self.room.close().await?;
        Ok(())
    }
}
