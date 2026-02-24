use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent,
};
use tracing::info;

use crate::event::{AppEvent, EventSender};
use crate::matrix::rooms;

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

    // Sync loop
    let sync_tx = tx.clone();
    client
        .sync_with_callback(crate::matrix::client::default_sync_settings(), {
            let client = client.clone();
            move |_response| {
                let tx = sync_tx.clone();
                let client = client.clone();
                async move {
                    let _ = tx.send(AppEvent::SyncStatus("synced".to_string()));

                    // Update room list after each sync
                    let room_list = rooms::get_room_list(&client).await;
                    let _ = tx.send(AppEvent::RoomListUpdated(room_list));

                    matrix_sdk::LoopCtrl::Continue
                }
            }
        })
        .await?;

    Ok(())
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
