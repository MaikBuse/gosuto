use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::events::room::message::{MessageType, RoomMessageEventContent};
use matrix_sdk::ruma::uint;
use tracing::{debug, error};

use crate::event::{AppEvent, EventSender};
use crate::state::{DisplayMessage, MessageContent};

pub async fn fetch_messages(
    client: &Client,
    room_id: &str,
    tx: &EventSender,
    sync_token: Option<String>,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let mut options = MessagesOptions::backward();
    options.limit = uint!(50);
    if let Some(token) = sync_token {
        options.from = Some(token);
    }
    let response = room.messages(options).await?;

    let mut messages: Vec<DisplayMessage> = Vec::new();
    let chunk_size = response.chunk.len();
    let mut skipped = 0usize;

    for event in &response.chunk {
        let timeline_event = event.raw().deserialize();
        match timeline_event {
            Ok(matrix_sdk::ruma::events::AnySyncTimelineEvent::MessageLike(
                matrix_sdk::ruma::events::AnySyncMessageLikeEvent::RoomMessage(msg_event),
            )) => {
                let (sender, event_id, timestamp, content) = match msg_event {
                    matrix_sdk::ruma::events::room::message::SyncRoomMessageEvent::Original(
                        orig,
                    ) => {
                        let millis: i64 = orig.origin_server_ts.0.into();
                        let ts = chrono::DateTime::from_timestamp(
                            millis / 1000,
                            ((millis % 1000) * 1_000_000) as u32,
                        )
                        .unwrap_or_default()
                        .with_timezone(&chrono::Local);
                        (
                            orig.sender.to_string(),
                            orig.event_id.to_string(),
                            ts,
                            orig.content,
                        )
                    }
                    _ => continue,
                };

                let (msg_content, is_emote, is_notice) = match &content.msgtype {
                    MessageType::Text(text) => {
                        (MessageContent::Text(text.body.clone()), false, false)
                    }
                    MessageType::Emote(emote) => {
                        (MessageContent::Text(emote.body.clone()), true, false)
                    }
                    MessageType::Notice(notice) => {
                        (MessageContent::Text(notice.body.clone()), false, true)
                    }
                    MessageType::Image(img) => {
                        let (w, h) = img
                            .info
                            .as_ref()
                            .map(|i| {
                                (
                                    i.width.map(|v| u32::try_from(v).unwrap_or(0)),
                                    i.height.map(|v| u32::try_from(v).unwrap_or(0)),
                                )
                            })
                            .unwrap_or((None, None));
                        (
                            MessageContent::Image {
                                body: img.body.clone(),
                                width: w,
                                height: h,
                            },
                            false,
                            false,
                        )
                    }
                    MessageType::VerificationRequest(_) => continue,
                    _ => (
                        MessageContent::Text("[unsupported message type]".to_string()),
                        false,
                        false,
                    ),
                };

                // Spawn image download for image messages
                if let MessageType::Image(img) = &content.msgtype {
                    let img_content = img.clone();
                    let eid = event_id.clone();
                    let client = client.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        crate::matrix::media::fetch_image(&client, eid, &img_content, &tx).await;
                    });
                }

                messages.push(DisplayMessage {
                    event_id,
                    sender,
                    content: msg_content,
                    timestamp,
                    is_emote,
                    is_notice,
                    pending: false,
                    verified: None,
                });
            }
            Ok(matrix_sdk::ruma::events::AnySyncTimelineEvent::MessageLike(
                matrix_sdk::ruma::events::AnySyncMessageLikeEvent::RoomEncrypted(enc_event),
            )) => {
                let (sender, event_id, timestamp) = match enc_event {
                    matrix_sdk::ruma::events::room::encrypted::SyncRoomEncryptedEvent::Original(
                        orig,
                    ) => {
                        let millis: i64 = orig.origin_server_ts.0.into();
                        let ts = chrono::DateTime::from_timestamp(
                            millis / 1000,
                            ((millis % 1000) * 1_000_000) as u32,
                        )
                        .unwrap_or_default()
                        .with_timezone(&chrono::Local);
                        (orig.sender.to_string(), orig.event_id.to_string(), ts)
                    }
                    _ => continue,
                };

                messages.push(DisplayMessage {
                    event_id,
                    sender,
                    content: MessageContent::Text("[Unable to decrypt]".to_string()),
                    timestamp,
                    is_emote: false,
                    is_notice: false,
                    pending: false,
                    verified: None,
                });
            }
            _ => {
                skipped += 1;
            }
        }
    }

    debug!(
        "Fetched {} events for room {}: {} messages, {} skipped non-message events",
        chunk_size,
        room_id,
        messages.len(),
        skipped
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

    let content = if body.contains('\n') {
        let html_body = body
            .split('\n')
            .map(escape_html)
            .collect::<Vec<_>>()
            .join("<br>");
        RoomMessageEventContent::text_html(body, html_body)
    } else {
        RoomMessageEventContent::text_plain(body)
    };
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

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_plain() {
        assert_eq!(escape_html("hello"), "hello");
    }

    #[test]
    fn escape_html_special_chars() {
        assert_eq!(
            escape_html("<b>bold & \"quoted\" 'text'</b>"),
            "&lt;b&gt;bold &amp; &quot;quoted&quot; &#39;text&#39;&lt;/b&gt;"
        );
    }

    #[test]
    fn escape_html_empty() {
        assert_eq!(escape_html(""), "");
    }
}
