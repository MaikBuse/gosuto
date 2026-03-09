use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::uint;
use tracing::{debug, error};

use crate::event::{AppEvent, EventSender, WarnClosed};
use crate::matrix::message_parsing::{
    ParsedMessage, millis_to_local, parse_message_type, spawn_image_fetch,
};
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
                        (
                            orig.sender.to_string(),
                            orig.event_id.to_string(),
                            millis_to_local(millis),
                            orig.content,
                        )
                    }
                    _ => continue,
                };

                let in_reply_to = match &content.relates_to {
                    Some(matrix_sdk::ruma::events::room::message::Relation::Reply {
                        in_reply_to,
                    }) => Some(crate::state::ReplyInfo {
                        event_id: in_reply_to.event_id.to_string(),
                        sender: String::new(),
                        body_preview: String::new(),
                    }),
                    _ => None,
                };

                let parsed = parse_message_type(&content.msgtype);
                let ParsedMessage::Message {
                    content: msg_content,
                    is_emote,
                    is_notice,
                    image_to_fetch,
                } = parsed
                else {
                    continue;
                };

                if let Some(ref img) = image_to_fetch {
                    spawn_image_fetch(client, event_id.clone(), img, tx);
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
                    in_reply_to,
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
                        (
                            orig.sender.to_string(),
                            orig.event_id.to_string(),
                            millis_to_local(millis),
                        )
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
                    in_reply_to: None,
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

    tx.send(AppEvent::MessagesLoaded {
        room_id: room_id.to_string(),
        messages,
        has_more,
    })
    .warn_closed("MessagesLoaded");

    Ok(())
}

pub async fn send_message(
    client: &Client,
    room_id: &str,
    body: &str,
    reply_to: Option<(&str, &str)>,
    tx: &EventSender,
) -> Result<()> {
    use matrix_sdk::ruma::events::room::message::{AddMentions, ForwardThread, ReplyMetadata};

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
    let content = if let Some((reply_eid_str, reply_sender_str)) = reply_to {
        let reply_eid: matrix_sdk::ruma::OwnedEventId = reply_eid_str.try_into()?;
        let reply_sender: matrix_sdk::ruma::OwnedUserId = reply_sender_str.try_into()?;
        let metadata = ReplyMetadata::new(&reply_eid, &reply_sender, None);
        content.make_reply_to(metadata, ForwardThread::Yes, AddMentions::No)
    } else {
        content
    };
    match room.send(content).await {
        Ok(response) => {
            tx.send(AppEvent::MessageSent {
                room_id: room_id.to_string(),
                event_id: response.event_id.to_string(),
                body: body.to_string(),
            })
            .warn_closed("MessageSent");
        }
        Err(e) => {
            error!("Failed to send message: {}", e);
            tx.send(AppEvent::SendError {
                error: e.to_string(),
            })
            .warn_closed("SendError");
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
