use chrono::{DateTime, Local};
use matrix_sdk::ruma::events::room::message::{
    ImageMessageEventContent, MessageFormat, MessageType,
};

use crate::event::EventSender;
use crate::state::MessageContent;

pub fn millis_to_local(millis: i64) -> DateTime<Local> {
    chrono::DateTime::from_timestamp(millis / 1000, ((millis % 1000) * 1_000_000) as u32)
        .unwrap_or_default()
        .with_timezone(&Local)
}

pub enum ParsedMessage {
    Skip,
    Message {
        content: MessageContent,
        is_emote: bool,
        is_notice: bool,
        image_to_fetch: Option<ImageMessageEventContent>,
    },
}

pub fn parse_message_type(msgtype: &MessageType) -> ParsedMessage {
    match msgtype {
        MessageType::Text(text) => ParsedMessage::Message {
            content: MessageContent::Text {
                plain: text.body.clone(),
                formatted_html: text
                    .formatted
                    .as_ref()
                    .filter(|f| f.format == MessageFormat::Html)
                    .map(|f| f.body.clone()),
            },
            is_emote: false,
            is_notice: false,
            image_to_fetch: None,
        },
        MessageType::Emote(emote) => ParsedMessage::Message {
            content: MessageContent::Text {
                plain: emote.body.clone(),
                formatted_html: emote
                    .formatted
                    .as_ref()
                    .filter(|f| f.format == MessageFormat::Html)
                    .map(|f| f.body.clone()),
            },
            is_emote: true,
            is_notice: false,
            image_to_fetch: None,
        },
        MessageType::Notice(notice) => ParsedMessage::Message {
            content: MessageContent::Text {
                plain: notice.body.clone(),
                formatted_html: notice
                    .formatted
                    .as_ref()
                    .filter(|f| f.format == MessageFormat::Html)
                    .map(|f| f.body.clone()),
            },
            is_emote: false,
            is_notice: true,
            image_to_fetch: None,
        },
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
            ParsedMessage::Message {
                content: MessageContent::Image {
                    body: img.body.clone(),
                    width: w,
                    height: h,
                },
                is_emote: false,
                is_notice: false,
                image_to_fetch: Some(img.clone()),
            }
        }
        MessageType::VerificationRequest(_) => ParsedMessage::Skip,
        _ => ParsedMessage::Message {
            content: MessageContent::Text {
                plain: "[unsupported message type]".to_string(),
                formatted_html: None,
            },
            is_emote: false,
            is_notice: false,
            image_to_fetch: None,
        },
    }
}

pub fn spawn_image_fetch(
    client: &matrix_sdk::Client,
    event_id: String,
    img: &ImageMessageEventContent,
    tx: &EventSender,
) {
    let img_content = img.clone();
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        crate::matrix::media::fetch_image(&client, event_id, &img_content, &tx).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millis_to_local_zero() {
        let dt = millis_to_local(0);
        assert_eq!(dt.timestamp(), 0);
    }

    #[test]
    fn millis_to_local_specific() {
        let dt = millis_to_local(1_700_000_000_000);
        assert_eq!(dt.timestamp(), 1_700_000_000);
    }

    #[test]
    fn millis_to_local_preserves_subsec() {
        let dt = millis_to_local(1_700_000_000_123);
        assert_eq!(dt.timestamp_millis(), 1_700_000_000_123);
    }

    #[test]
    fn millis_to_local_negative_returns_default() {
        // Negative timestamps before epoch should still produce a valid DateTime
        let dt = millis_to_local(-1000);
        assert_eq!(dt.timestamp(), -1);
    }

    #[test]
    fn parse_text_message() {
        use matrix_sdk::ruma::events::room::message::TextMessageEventContent;
        let msgtype = MessageType::Text(TextMessageEventContent::plain("hello"));
        match parse_message_type(&msgtype) {
            ParsedMessage::Message {
                content,
                is_emote,
                is_notice,
                image_to_fetch,
            } => {
                assert!(
                    matches!(content, MessageContent::Text { ref plain, .. } if plain == "hello")
                );
                assert!(!is_emote);
                assert!(!is_notice);
                assert!(image_to_fetch.is_none());
            }
            ParsedMessage::Skip => panic!("expected Message"),
        }
    }

    #[test]
    fn parse_emote_message() {
        use matrix_sdk::ruma::events::room::message::EmoteMessageEventContent;
        let msgtype = MessageType::Emote(EmoteMessageEventContent::plain("waves"));
        match parse_message_type(&msgtype) {
            ParsedMessage::Message {
                is_emote,
                is_notice,
                ..
            } => {
                assert!(is_emote);
                assert!(!is_notice);
            }
            ParsedMessage::Skip => panic!("expected Message"),
        }
    }

    #[test]
    fn parse_notice_message() {
        use matrix_sdk::ruma::events::room::message::NoticeMessageEventContent;
        let msgtype = MessageType::Notice(NoticeMessageEventContent::plain("bot says hi"));
        match parse_message_type(&msgtype) {
            ParsedMessage::Message {
                is_emote,
                is_notice,
                ..
            } => {
                assert!(!is_emote);
                assert!(is_notice);
            }
            ParsedMessage::Skip => panic!("expected Message"),
        }
    }

    #[test]
    fn parse_verification_skips() {
        use matrix_sdk::ruma::OwnedDeviceId;
        use matrix_sdk::ruma::events::room::message::KeyVerificationRequestEventContent;
        let content = KeyVerificationRequestEventContent::new(
            "body".to_string(),
            vec![],
            OwnedDeviceId::from("DEV"),
            matrix_sdk::ruma::OwnedUserId::try_from("@u:x").unwrap(),
        );
        let msgtype = MessageType::VerificationRequest(content);
        assert!(matches!(parse_message_type(&msgtype), ParsedMessage::Skip));
    }
}
