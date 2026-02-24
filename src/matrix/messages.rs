use anyhow::Result;
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::events::room::message::{MessageType, RoomMessageEventContent};
use matrix_sdk::Client;
use tracing::{debug, error};

use crate::event::{AppEvent, EventSender};
use crate::state::DisplayMessage;

pub async fn fetch_messages(
    client: &Client,
    room_id: &str,
    tx: &EventSender,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let options = MessagesOptions::backward();
    let response = room.messages(options).await?;

    let mut messages: Vec<DisplayMessage> = Vec::new();
    let chunk_size = response.chunk.len();
    let mut skipped = 0usize;

    for event in &response.chunk {
        let timeline_event = event.raw().deserialize();
        if let Ok(matrix_sdk::ruma::events::AnySyncTimelineEvent::MessageLike(
            matrix_sdk::ruma::events::AnySyncMessageLikeEvent::RoomMessage(msg_event),
        )) = timeline_event
        {
            let (sender, event_id, timestamp, content) = match msg_event {
                matrix_sdk::ruma::events::room::message::SyncRoomMessageEvent::Original(orig) => {
                    let millis: i64 = orig.origin_server_ts.0.into();
                    let ts = chrono::DateTime::from_timestamp(
                        millis / 1000,
                        ((millis % 1000) * 1_000_000) as u32,
                    )
                    .unwrap_or_default()
                    .with_timezone(&chrono::Local);
                    (orig.sender.to_string(), orig.event_id.to_string(), ts, orig.content)
                }
                _ => continue,
            };

            let (body, is_emote, is_notice) = match &content.msgtype {
                MessageType::Text(text) => (text.body.clone(), false, false),
                MessageType::Emote(emote) => (emote.body.clone(), true, false),
                MessageType::Notice(notice) => (notice.body.clone(), false, true),
                _ => ("[unsupported message type]".to_string(), false, false),
            };

            messages.push(DisplayMessage {
                event_id,
                sender,
                body,
                timestamp,
                is_emote,
                is_notice,
                pending: false,
            });
        } else {
            skipped += 1;
        }
    }

    debug!(
        "Fetched {} events for room {}: {} messages, {} skipped non-message events",
        chunk_size, room_id, messages.len(), skipped
    );

    // Reverse so oldest is first
    messages.reverse();

    let has_more = response.end.is_some();

    let _ = tx.send(AppEvent::MessagesLoaded {
        room_id: room_id.to_string(),
        messages,
        has_more,
    });

    Ok(())
}

pub async fn send_message(
    client: &Client,
    room_id: &str,
    body: &str,
    tx: &EventSender,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let content = RoomMessageEventContent::text_plain(body);
    match room.send(content).await {
        Ok(response) => {
            let _ = tx.send(AppEvent::MessageSent {
                room_id: room_id.to_string(),
                event_id: response.event_id.to_string(),
                body: body.to_string(),
            });
        }
        Err(e) => {
            error!("Failed to send message: {}", e);
            let _ = tx.send(AppEvent::SendError {
                room_id: room_id.to_string(),
                error: e.to_string(),
            });
        }
    }

    Ok(())
}
