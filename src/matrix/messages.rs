use std::collections::HashMap;

use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::OwnedUserId;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::uint;
use tracing::{debug, error, trace};

use crate::event::{AppEvent, EventSender, WarnClosed};
use crate::matrix::message_parsing::{
    ParsedMessage, millis_to_local, parse_message_type, spawn_image_fetch,
};
use crate::state::{DisplayMessage, MessageContent, Reaction, ReactionSender};

/// Minimum displayable messages to collect before stopping pagination.
const MIN_MESSAGES: usize = 20;
/// Maximum backward pages to fetch in a single load.
const MAX_PAGES: usize = 5;

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

    let mut messages: Vec<DisplayMessage> = Vec::new();
    let mut total_chunk_events = 0usize;
    let mut skipped = 0usize;
    let mut utd_count = 0usize;
    // key: target_event_id → Vec<(emoji_key, sender, reaction_event_id)>
    let mut reaction_map: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
    // key: target_event_id → new content from edit
    let mut edit_map: HashMap<String, MessageContent> = HashMap::new();

    let mut next_token = sync_token.clone();
    let had_sync_token = sync_token.is_some();

    for page in 0..MAX_PAGES {
        let mut options = MessagesOptions::backward();
        options.limit = uint!(50);
        if let Some(ref token) = next_token {
            options.from = Some(token.clone());
        }
        let response = room.messages(options).await?;

        debug!(
            "room.messages() page {} returned {} events for room {} (token: {})",
            page,
            response.chunk.len(),
            room_id,
            next_token.is_some()
        );

        if response.chunk.is_empty() {
            next_token = response.end;
            break;
        }

        total_chunk_events += response.chunk.len();

        for event in &response.chunk {
            trace!(
                "Event kind={}, event_id={:?}, event_type={:?}",
                match &event.kind {
                    matrix_sdk::deserialized_responses::TimelineEventKind::Decrypted(_) =>
                        "Decrypted",
                    matrix_sdk::deserialized_responses::TimelineEventKind::UnableToDecrypt {
                        ..
                    } => "UnableToDecrypt",
                    matrix_sdk::deserialized_responses::TimelineEventKind::PlainText { .. } =>
                        "PlainText",
                },
                event.event_id(),
                event.kind.event_type(),
            );

            // Handle unable-to-decrypt events before attempting deserialization,
            // since raw() returns encrypted JSON that won't deserialize to message types
            if event.kind.is_utd() {
                utd_count += 1;
                let event_id = event
                    .event_id()
                    .map(|id| id.to_string())
                    .unwrap_or_default();
                let sender = event
                    .raw()
                    .get_field::<OwnedUserId>("sender")
                    .ok()
                    .flatten()
                    .map_or_else(|| "unknown".to_string(), |u| u.to_string());
                let timestamp = match event.timestamp() {
                    Some(ts) => {
                        let millis: i64 = ts.0.into();
                        millis_to_local(millis)
                    }
                    None => chrono::Local::now(),
                };
                messages.push(DisplayMessage {
                    event_id,
                    sender,
                    content: MessageContent::Text {
                        plain: "[Unable to decrypt]".to_string(),
                        formatted_html: None,
                    },
                    timestamp,
                    is_emote: false,
                    is_notice: false,
                    pending: false,
                    verified: None,
                    in_reply_to: None,
                    reactions: Vec::new(),
                    edited: false,
                    redacted: false,
                });
                continue;
            }

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
                        _ => {
                            debug!("Skipping redacted RoomMessage event");
                            continue;
                        }
                    };

                    // Check for edit (replacement) events FIRST, before parsing fallback content
                    if let Some(matrix_sdk::ruma::events::room::message::Relation::Replacement(
                        replacement,
                    )) = &content.relates_to
                    {
                        let target_eid = replacement.event_id.to_string();
                        let edit_parsed = parse_message_type(&replacement.new_content.msgtype);
                        if let ParsedMessage::Message {
                            content: edit_content,
                            ..
                        } = edit_parsed
                        {
                            edit_map.entry(target_eid).or_insert(edit_content);
                        }
                        debug!(
                            "Skipping edit/replacement event for {}",
                            replacement.event_id
                        );
                        continue;
                    }

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
                        debug!("Skipping unsupported message type in event {}", event_id);
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
                        reactions: Vec::new(),
                        edited: false,
                        redacted: false,
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
                        content: MessageContent::Text {
                            plain: "[Unable to decrypt]".to_string(),
                            formatted_html: None,
                        },
                        timestamp,
                        is_emote: false,
                        is_notice: false,
                        pending: false,
                        verified: None,
                        in_reply_to: None,
                        reactions: Vec::new(),
                        edited: false,
                        redacted: false,
                    });
                }
                Ok(matrix_sdk::ruma::events::AnySyncTimelineEvent::MessageLike(
                    matrix_sdk::ruma::events::AnySyncMessageLikeEvent::Reaction(reaction_event),
                )) => {
                    if let matrix_sdk::ruma::events::reaction::SyncReactionEvent::Original(orig) =
                        reaction_event
                    {
                        let target = orig.content.relates_to.event_id.to_string();
                        let emoji_key = orig.content.relates_to.key.clone();
                        let sender = orig.sender.to_string();
                        let reaction_eid = orig.event_id.to_string();
                        reaction_map.entry(target).or_default().push((
                            emoji_key,
                            sender,
                            reaction_eid,
                        ));
                    }
                }
                Ok(other) => {
                    debug!("Skipped non-message event type: {:?}", other.event_type());
                    skipped += 1;
                }
                Err(e) => {
                    debug!("Failed to deserialize timeline event: {e}");
                    skipped += 1;
                }
            }
        }

        next_token = response.end;

        if messages.len() >= MIN_MESSAGES || next_token.is_none() {
            break;
        }

        debug!(
            "Only {} messages so far (need {}), fetching page {}...",
            messages.len(),
            MIN_MESSAGES,
            page + 1
        );
    }

    // Attach historical reactions to their target messages
    let mut total_reactions = 0usize;
    for msg in &mut messages {
        if let Some(entries) = reaction_map.remove(&msg.event_id) {
            for (emoji_key, sender, reaction_eid) in entries {
                if let Some(reaction) = msg.reactions.iter_mut().find(|r| r.key == emoji_key) {
                    if !reaction.senders.iter().any(|s| s.user_id == sender) {
                        reaction.senders.push(ReactionSender {
                            user_id: sender,
                            reaction_event_id: reaction_eid,
                        });
                    }
                } else {
                    msg.reactions.push(Reaction {
                        key: emoji_key,
                        senders: vec![ReactionSender {
                            user_id: sender,
                            reaction_event_id: reaction_eid,
                        }],
                    });
                }
            }
            total_reactions += msg.reactions.len();
        }
    }

    // Apply edits from edit_map
    for msg in &mut messages {
        if let Some(new_content) = edit_map.remove(&msg.event_id) {
            msg.content = new_content;
            msg.edited = true;
        }
    }

    let pagination_token = next_token;
    let has_more = pagination_token.is_some();

    debug!(
        "Fetched {} events for room {}: {} messages, {} reactions, {} UTD, {} skipped (sync_token: {})",
        total_chunk_events,
        room_id,
        messages.len(),
        total_reactions,
        utd_count,
        skipped,
        had_sync_token
    );

    // Reverse so oldest is first
    messages.reverse();

    tx.send(AppEvent::MessagesLoaded {
        room_id: room_id.to_string(),
        messages,
        has_more,
        pagination_token,
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

    let content = if has_markdown(body) {
        let html_body = markdown_to_html(body);
        RoomMessageEventContent::text_html(body, html_body)
    } else if body.contains('\n') {
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

pub async fn send_edit(
    client: &Client,
    room_id: &str,
    target_event_id: &str,
    new_body: &str,
    tx: &EventSender,
) -> Result<()> {
    use matrix_sdk::ruma::events::room::message::ReplacementMetadata;

    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let target_eid: matrix_sdk::ruma::OwnedEventId = target_event_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let mut content = if has_markdown(new_body) {
        let html_body = markdown_to_html(new_body);
        RoomMessageEventContent::text_html(new_body, html_body)
    } else if new_body.contains('\n') {
        let html_body = new_body
            .split('\n')
            .map(escape_html)
            .collect::<Vec<_>>()
            .join("<br>");
        RoomMessageEventContent::text_html(new_body, html_body)
    } else {
        RoomMessageEventContent::text_plain(new_body)
    };

    let metadata = ReplacementMetadata::new(target_eid, None);
    content = content.make_replacement(metadata);

    match room.send(content).await {
        Ok(_response) => {
            tx.send(AppEvent::MessageSent {
                room_id: room_id.to_string(),
                event_id: target_event_id.to_string(),
                body: new_body.to_string(),
            })
            .warn_closed("MessageSent(edit)");
        }
        Err(e) => {
            error!("Failed to send edit: {}", e);
            tx.send(AppEvent::SendError {
                error: e.to_string(),
            })
            .warn_closed("SendError(edit)");
        }
    }

    Ok(())
}

pub async fn send_reaction(
    client: &Client,
    room_id: &str,
    event_id: &str,
    emoji_key: &str,
    sender: &str,
    tx: &EventSender,
) -> Result<()> {
    use matrix_sdk::ruma::events::reaction::ReactionEventContent;
    use matrix_sdk::ruma::events::relation::Annotation;

    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let event_id_parsed: matrix_sdk::ruma::OwnedEventId = event_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let annotation = Annotation::new(event_id_parsed, emoji_key.to_string());
    let content = ReactionEventContent::new(annotation);
    match room.send(content).await {
        Ok(response) => {
            tx.send(AppEvent::ReactionSent {
                room_id: room_id.to_string(),
                target_event_id: event_id.to_string(),
                reaction_event_id: response.event_id.to_string(),
                emoji_key: emoji_key.to_string(),
                sender: sender.to_string(),
            })
            .warn_closed("ReactionSent");
        }
        Err(e) => {
            error!("Failed to send reaction: {}", e);
            tx.send(AppEvent::SendError {
                error: e.to_string(),
            })
            .warn_closed("SendError");
        }
    }
    Ok(())
}

pub async fn redact_reaction(
    client: &Client,
    room_id: &str,
    reaction_event_id: &str,
    tx: &EventSender,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let reaction_eid: matrix_sdk::ruma::OwnedEventId = reaction_event_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    match room.redact(&reaction_eid, None, None).await {
        Ok(_) => {
            tx.send(AppEvent::ReactionRedacted {
                room_id: room_id.to_string(),
                reaction_event_id: reaction_event_id.to_string(),
            })
            .warn_closed("ReactionRedacted");
        }
        Err(e) => {
            error!("Failed to redact reaction: {}", e);
            tx.send(AppEvent::SendError {
                error: e.to_string(),
            })
            .warn_closed("SendError");
        }
    }
    Ok(())
}

pub async fn redact_message(
    client: &Client,
    room_id: &str,
    event_id: &str,
    tx: &EventSender,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let event_id_parsed: matrix_sdk::ruma::OwnedEventId = event_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    match room.redact(&event_id_parsed, None, None).await {
        Ok(_) => {
            tx.send(AppEvent::MessageRedacted {
                room_id: room_id.to_string(),
                target_event_id: event_id.to_string(),
            })
            .warn_closed("MessageRedacted");
        }
        Err(e) => {
            error!("Failed to redact message: {}", e);
            tx.send(AppEvent::SendError {
                error: e.to_string(),
            })
            .warn_closed("SendError");
        }
    }
    Ok(())
}

fn has_markdown(text: &str) -> bool {
    use pulldown_cmark::{Event, Options, Parser as MdParser, Tag, TagEnd};

    let parser = MdParser::new_ext(text, Options::ENABLE_STRIKETHROUGH);
    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                _ => return true,
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {}
                _ => return true,
            },
            Event::Code(_) | Event::Html(_) | Event::InlineHtml(_) | Event::Rule => {
                return true;
            }
            Event::Text(_) | Event::SoftBreak | Event::HardBreak => {}
            _ => {}
        }
    }
    false
}

fn markdown_to_html(text: &str) -> String {
    use pulldown_cmark::{Options, Parser as MdParser};

    let parser = MdParser::new_ext(text, Options::ENABLE_STRIKETHROUGH);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    // Trim trailing newline that pulldown-cmark appends
    if html_output.ends_with('\n') {
        html_output.pop();
    }

    // Strip outer <p>...</p> for single-paragraph messages
    if html_output.starts_with("<p>")
        && html_output.ends_with("</p>")
        && html_output.matches("<p>").count() == 1
    {
        html_output = html_output[3..html_output.len() - 4].to_string();
    }

    html_output
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

    #[test]
    fn has_markdown_plain_text() {
        assert!(!has_markdown("hello world"));
        assert!(!has_markdown("hello\nworld"));
        assert!(!has_markdown("3*4 is twelve"));
        assert!(!has_markdown("some punctuation: colons, dashes - done."));
    }

    #[test]
    fn has_markdown_bold() {
        assert!(has_markdown("**bold**"));
    }

    #[test]
    fn has_markdown_italic() {
        assert!(has_markdown("*italic*"));
    }

    #[test]
    fn has_markdown_inline_code() {
        assert!(has_markdown("`code`"));
    }

    #[test]
    fn has_markdown_strikethrough() {
        assert!(has_markdown("~~strike~~"));
    }

    #[test]
    fn has_markdown_heading() {
        assert!(has_markdown("# heading"));
    }

    #[test]
    fn has_markdown_list() {
        assert!(has_markdown("- item"));
    }

    #[test]
    fn has_markdown_blockquote() {
        assert!(has_markdown("> quote"));
    }

    #[test]
    fn has_markdown_link() {
        assert!(has_markdown("[text](url)"));
    }

    #[test]
    fn has_markdown_code_block() {
        assert!(has_markdown("```\ncode\n```"));
    }

    #[test]
    fn markdown_to_html_bold() {
        assert_eq!(markdown_to_html("**bold**"), "<strong>bold</strong>");
    }

    #[test]
    fn markdown_to_html_italic() {
        assert_eq!(markdown_to_html("*italic*"), "<em>italic</em>");
    }

    #[test]
    fn markdown_to_html_inline_code() {
        assert_eq!(markdown_to_html("`code`"), "<code>code</code>");
    }

    #[test]
    fn markdown_to_html_strikethrough() {
        assert_eq!(markdown_to_html("~~strike~~"), "<del>strike</del>");
    }

    #[test]
    fn markdown_to_html_link() {
        assert_eq!(markdown_to_html("[text](url)"), "<a href=\"url\">text</a>");
    }

    #[test]
    fn markdown_to_html_multi_paragraph() {
        let result = markdown_to_html("para one\n\npara two");
        assert!(result.contains("<p>"));
    }

    #[test]
    fn markdown_to_html_single_paragraph_strips_p() {
        let result = markdown_to_html("**bold** text");
        assert!(!result.starts_with("<p>"));
        assert_eq!(result, "<strong>bold</strong> text");
    }
}
