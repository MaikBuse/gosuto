use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::encryption::verification::VerificationRequest;
use matrix_sdk::ruma::events::room::message::{MessageType, OriginalSyncRoomMessageEvent};
use matrix_sdk::ruma::events::typing::SyncTypingEvent;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::config;
use crate::event::{AppEvent, EventSender};
use crate::matrix::{rooms, session};

pub type IncomingVerification = Arc<Mutex<Option<VerificationRequest>>>;

pub async fn start_sync(
    client: Client,
    tx: EventSender,
    incoming_verification: IncomingVerification,
) -> Result<()> {
    // Register event handler for messages
    let msg_tx = tx.clone();
    client.add_event_handler(
        move |event: OriginalSyncRoomMessageEvent, room: matrix_sdk::Room| {
            let tx = msg_tx.clone();
            async move {
                let room_id = room.room_id().to_string();
                let sender = event.sender.to_string();
                let millis: i64 = event.origin_server_ts.0.into();
                let timestamp = chrono::DateTime::from_timestamp(
                    millis / 1000,
                    ((millis % 1000) * 1_000_000) as u32,
                )
                .unwrap_or_default()
                .with_timezone(&chrono::Local);

                let (body, is_emote, is_notice) = match &event.content.msgtype {
                    MessageType::Text(text) => (text.body.clone(), false, false),
                    MessageType::Emote(emote) => (emote.body.clone(), true, false),
                    MessageType::Notice(notice) => (notice.body.clone(), false, true),
                    _ => ("[unsupported message type]".to_string(), false, false),
                };

                let msg = crate::state::DisplayMessage {
                    event_id: event.event_id.to_string(),
                    sender,
                    body,
                    timestamp,
                    is_emote,
                    is_notice,
                    pending: false,
                    verified: None,
                };

                let _ = tx.send(AppEvent::NewMessage {
                    room_id,
                    message: msg,
                });
            }
        },
    );

    // Register event handler for typing indicators
    let typing_tx = tx.clone();
    client.add_event_handler(move |event: SyncTypingEvent, room: matrix_sdk::Room| {
        let tx = typing_tx.clone();
        async move {
            let room_id = room.room_id().to_string();
            let user_ids: Vec<String> = event
                .content
                .user_ids
                .iter()
                .map(|u| u.to_string())
                .collect();
            let _ = tx.send(AppEvent::TypingUsersUpdated { room_id, user_ids });
        }
    });

    // Register event handler for incoming verification requests
    let verify_tx = tx.clone();
    let incoming_verify = incoming_verification.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::ToDeviceEvent<
            matrix_sdk::ruma::events::key::verification::request::ToDeviceKeyVerificationRequestEventContent,
        >,
              client: matrix_sdk::Client| {
            let tx = verify_tx.clone();
            let incoming_verify = incoming_verify.clone();
            async move {
                let request = client
                    .encryption()
                    .get_verification_request(&event.sender, event.content.transaction_id.as_str())
                    .await;

                if let Some(request) = request {
                    info!("Incoming verification request from {}", event.sender);
                    // Store the request for main.rs to drive
                    *incoming_verify.lock().await = Some(request);
                    let _ = tx.send(crate::event::AppEvent::VerificationRequestReceived {
                        sender: event.sender.to_string(),
                        flow_id: event.content.transaction_id.to_string(),
                    });
                }
            }
        },
    );

    // Register event handlers for MatrixRTC call member events
    register_matrixrtc_handlers(&client, &tx);

    // Initial room list
    let room_list = rooms::get_room_list(&client).await;
    let _ = tx.send(AppEvent::RoomListUpdated(room_list));

    let _ = tx.send(AppEvent::SyncStatus("syncing...".to_string()));

    info!("Starting sync loop");

    // Retry loop wrapping sync_with_callback
    let retry_count = Arc::new(AtomicU32::new(0));

    loop {
        let sync_tx = tx.clone();
        let retry_reset = Arc::clone(&retry_count);
        let result = client
            .sync_with_callback(crate::matrix::client::default_sync_settings(), {
                let client = client.clone();
                move |response| {
                    let tx = sync_tx.clone();
                    let client = client.clone();
                    let retry_reset = retry_reset.clone();
                    async move {
                        retry_reset.store(0, Ordering::Relaxed);
                        let _ = tx.send(AppEvent::SyncTokenUpdated(response.next_batch.clone()));
                        let _ = tx.send(AppEvent::SyncStatus("synced".to_string()));

                        // Update room list after each sync
                        let room_list = rooms::get_room_list(&client).await;
                        let _ = tx.send(AppEvent::RoomListUpdated(room_list));

                        matrix_sdk::LoopCtrl::Continue
                    }
                }
            })
            .await;

        match result {
            Ok(()) => break,
            Err(e) => {
                let msg = sanitize_error(&e.to_string());

                if is_auth_error(&e.to_string()) {
                    warn!("Auth error during sync: {msg}");
                    // Delete stale session file
                    if let Ok(path) = config::session_path() {
                        let _ = session::delete_session(&path);
                    }
                    let _ = tx.send(AppEvent::LoggedOut);
                    break;
                }

                // Transient error — retry with backoff
                let attempt = retry_count.fetch_add(1, Ordering::Relaxed);
                let backoff_secs = match attempt {
                    0 => 2u64,
                    1 => 4,
                    2 => 8,
                    3 => 16,
                    _ => 30,
                };
                warn!(
                    "Sync error (attempt {}): {msg} — retrying in {backoff_secs}s",
                    attempt + 1
                );
                let _ = tx.send(AppEvent::SyncError(msg));

                // Countdown
                for remaining in (1..=backoff_secs).rev() {
                    let _ = tx.send(AppEvent::SyncStatus(format!(
                        "reconnecting in {remaining}s..."
                    )));
                    sleep(Duration::from_secs(1)).await;
                }
                let _ = tx.send(AppEvent::SyncStatus("reconnecting...".to_string()));
            }
        }
    }

    Ok(())
}

/// Check if an error indicates an auth failure (expired/invalid token).
fn is_auth_error(error: &str) -> bool {
    error.contains("[403]")
        || error.contains("[401]")
        || error.contains("M_UNKNOWN_TOKEN")
        || error.contains("M_MISSING_TOKEN")
}

/// Clean up error messages for display.
fn sanitize_error(error: &str) -> String {
    let mut msg = error.replace("<non-json bytes>", "(non-JSON response)");
    msg.truncate(120);
    msg
}

fn register_matrixrtc_handlers(client: &Client, tx: &EventSender) {
    // Handle raw m.call.member / org.matrix.msc3401.call.member state events
    let member_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::AnySyncStateEvent, room: matrix_sdk::Room| {
            let tx = member_tx.clone();
            async move {
                let event_type = event.event_type().to_string();
                if event_type != "org.matrix.msc3401.call.member" && event_type != "m.call.member" {
                    return;
                }

                let sender = event.sender().to_string();
                let room_id = room.room_id().to_string();

                let raw_content = event
                    .original_content()
                    .map(|raw| {
                        serde_json::to_string_pretty(&serde_json::to_value(raw).unwrap_or_default())
                            .unwrap_or_default()
                    })
                    .unwrap_or_else(|| "<redacted>".to_string());
                debug!(
                    "m.call.member raw: type={}, sender={}, room={}, state_key={}, content={}",
                    event_type,
                    sender,
                    room_id,
                    event.state_key(),
                    raw_content,
                );

                // Parse the content to check for active memberships
                // New format (MSC4143): top-level "application" field means joined
                // Old format (MSC3401): non-empty "memberships" array means joined
                // Leave: empty {} or empty memberships array
                let content = event.original_content();
                let has_active_memberships = content
                    .map(|raw| {
                        let json = serde_json::to_value(raw).unwrap_or_default();
                        // New format: presence of "application" field = active
                        if json.get("application").is_some() {
                            return true;
                        }
                        // Old format: non-empty memberships array = active
                        json.get("memberships")
                            .and_then(|m| m.as_array())
                            .map(|arr| !arr.is_empty())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);

                if has_active_memberships {
                    info!("m.call.member joined: {} in {}", sender, room_id);
                    let _ = tx.send(AppEvent::CallMemberJoined {
                        room_id,
                        user_id: sender,
                    });
                } else {
                    info!("m.call.member left: {} in {}", sender, room_id);
                    let _ = tx.send(AppEvent::CallMemberLeft {
                        room_id,
                        user_id: sender,
                    });
                }
            }
        },
    );
}
