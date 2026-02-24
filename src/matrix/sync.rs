use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent,
};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::config;
use crate::event::{AppEvent, EventSender};
use crate::matrix::{rooms, session};

pub async fn start_sync(client: Client, tx: EventSender) -> Result<()> {
    // Register event handler for messages
    let msg_tx = tx.clone();
    client.add_event_handler(
        move |event: OriginalSyncRoomMessageEvent,
              room: matrix_sdk::Room| {
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
                };

                let _ = tx.send(AppEvent::NewMessage {
                    room_id,
                    message: msg,
                });
            }
        },
    );

    // Register event handlers for VoIP call events
    register_call_event_handlers(&client, &tx);

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
                move |_response| {
                    let tx = sync_tx.clone();
                    let client = client.clone();
                    let retry_reset = retry_reset.clone();
                    async move {
                        retry_reset.store(0, Ordering::Relaxed);
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
                warn!("Sync error (attempt {}): {msg} — retrying in {backoff_secs}s", attempt + 1);
                let _ = tx.send(AppEvent::SyncError(msg));

                // Countdown
                for remaining in (1..=backoff_secs).rev() {
                    let _ = tx.send(AppEvent::SyncStatus(
                        format!("reconnecting in {remaining}s..."),
                    ));
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
    error.contains("[403]") || error.contains("[401]")
}

/// Clean up error messages for display.
fn sanitize_error(error: &str) -> String {
    let mut msg = error.replace("<non-json bytes>", "(non-JSON response)");
    msg.truncate(120);
    msg
}

fn register_call_event_handlers(client: &Client, tx: &EventSender) {
    // m.call.invite handler
    let invite_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::call::invite::OriginalSyncCallInviteEvent,
              room: matrix_sdk::Room| {
            let tx = invite_tx.clone();
            async move {
                let room_id = room.room_id().to_string();
                let sender = event.sender.to_string();
                let call_id = event.content.call_id.to_string();
                let sdp = event.content.offer.sdp.clone();
                info!("Received m.call.invite from {} (call_id: {})", sender, call_id);
                let _ = tx.send(AppEvent::CallInvite {
                    call_id,
                    room_id,
                    sender,
                    sdp,
                });
            }
        },
    );

    // m.call.answer handler
    let answer_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::call::answer::OriginalSyncCallAnswerEvent,
              room: matrix_sdk::Room| {
            let tx = answer_tx.clone();
            async move {
                let room_id = room.room_id().to_string();
                let call_id = event.content.call_id.to_string();
                let sdp = event.content.answer.sdp.clone();
                info!("Received m.call.answer (call_id: {})", call_id);
                let _ = tx.send(AppEvent::CallAnswer {
                    call_id,
                    room_id,
                    sdp,
                });
            }
        },
    );

    // m.call.candidates handler
    let candidates_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::call::candidates::OriginalSyncCallCandidatesEvent,
              room: matrix_sdk::Room| {
            let tx = candidates_tx.clone();
            async move {
                let room_id = room.room_id().to_string();
                let call_id = event.content.call_id.to_string();
                let candidates: Vec<String> = event
                    .content
                    .candidates
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "candidate": c.candidate,
                            "sdpMid": c.sdp_mid,
                            "sdpMLineIndex": c.sdp_m_line_index,
                        })
                        .to_string()
                    })
                    .collect();
                info!(
                    "Received m.call.candidates (call_id: {}, count: {})",
                    call_id,
                    candidates.len()
                );
                let _ = tx.send(AppEvent::CallCandidates {
                    call_id,
                    room_id,
                    candidates,
                });
            }
        },
    );

    // m.call.hangup handler
    let hangup_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::call::hangup::OriginalSyncCallHangupEvent,
              room: matrix_sdk::Room| {
            let tx = hangup_tx.clone();
            async move {
                let room_id = room.room_id().to_string();
                let call_id = event.content.call_id.to_string();
                info!("Received m.call.hangup (call_id: {})", call_id);
                let _ = tx.send(AppEvent::CallHangup { call_id, room_id });
            }
        },
    );
}
