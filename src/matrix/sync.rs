use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::encryption::verification::VerificationRequest;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::typing::SyncTypingEvent;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::config;
use crate::event::{AppEvent, EventSender, WarnClosed};
use crate::matrix::{rooms, session};
use crate::voip::CallCommandSender;

pub type IncomingVerification = Arc<Mutex<Option<VerificationRequest>>>;

pub async fn start_sync(
    client: Client,
    tx: EventSender,
    incoming_verification: IncomingVerification,
    call_cmd_tx: Option<CallCommandSender>,
) -> Result<()> {
    // Register event handler for messages
    let msg_tx = tx.clone();
    let msg_verify_tx = tx.clone();
    let msg_incoming_verify = incoming_verification.clone();
    client.add_event_handler(
        move |event: OriginalSyncRoomMessageEvent, room: matrix_sdk::Room| {
            let tx = msg_tx.clone();
            let verify_tx = msg_verify_tx.clone();
            let incoming_verify = msg_incoming_verify.clone();
            async move {
                use crate::matrix::message_parsing::{
                    ParsedMessage, millis_to_local, parse_message_type, spawn_image_fetch,
                };
                use matrix_sdk::ruma::events::room::message::MessageType;

                // Intercept in-room verification requests before normal message parsing
                if let MessageType::VerificationRequest(_) = &event.content.msgtype {
                    // Skip our own requests
                    if room.client().user_id() == Some(&event.sender) {
                        return;
                    }
                    let request = room
                        .client()
                        .encryption()
                        .get_verification_request(&event.sender, event.event_id.as_str())
                        .await;
                    if let Some(request) = request {
                        info!(
                            "Incoming in-room verification request from {}",
                            event.sender
                        );
                        *incoming_verify.lock().await = Some(request);
                        verify_tx
                            .send(AppEvent::VerificationRequestReceived {
                                sender: event.sender.to_string(),
                            })
                            .warn_closed("VerificationRequestReceived");
                    }
                    return;
                }

                let room_id = room.room_id().to_string();
                let sender = event.sender.to_string();
                let millis: i64 = event.origin_server_ts.0.into();
                let timestamp = millis_to_local(millis);

                // Check for edit (replacement) events FIRST, before parsing fallback content
                if let Some(matrix_sdk::ruma::events::room::message::Relation::Replacement(
                    replacement,
                )) = &event.content.relates_to
                {
                    let target_eid = replacement.event_id.to_string();
                    let parsed = parse_message_type(&replacement.new_content.msgtype);
                    if let ParsedMessage::Message {
                        content: new_content,
                        ..
                    } = parsed
                    {
                        tx.send(AppEvent::MessageEdited {
                            room_id: room_id.clone(),
                            target_event_id: target_eid,
                            new_content,
                        })
                        .warn_closed("MessageEdited");
                    }
                    return;
                }

                let parsed = parse_message_type(&event.content.msgtype);
                let ParsedMessage::Message {
                    content: msg_content,
                    is_emote,
                    is_notice,
                    image_to_fetch,
                } = parsed
                else {
                    return;
                };

                if let Some(ref img) = image_to_fetch {
                    spawn_image_fetch(&room.client(), event.event_id.to_string(), img, &tx);
                }

                let in_reply_to = match &event.content.relates_to {
                    Some(matrix_sdk::ruma::events::room::message::Relation::Reply {
                        in_reply_to,
                    }) => Some(crate::state::ReplyInfo {
                        event_id: in_reply_to.event_id.to_string(),
                        sender: String::new(),
                        body_preview: String::new(),
                    }),
                    _ => None,
                };

                let msg = crate::state::DisplayMessage {
                    event_id: event.event_id.to_string(),
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
                };

                tx.send(AppEvent::NewMessage {
                    room_id,
                    message: msg,
                })
                .warn_closed("NewMessage");
            }
        },
    );

    // Register event handler for reaction events
    let reaction_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::reaction::OriginalSyncReactionEvent,
              room: matrix_sdk::Room| {
            let tx = reaction_tx.clone();
            async move {
                let room_id = room.room_id().to_string();
                let sender = event.sender.to_string();
                let reaction_event_id = event.event_id.to_string();
                let emoji_key = event.content.relates_to.key.clone();
                let target_event_id = event.content.relates_to.event_id.to_string();

                tx.send(AppEvent::ReactionReceived {
                    room_id,
                    target_event_id,
                    reaction_event_id,
                    emoji_key,
                    sender,
                })
                .warn_closed("ReactionReceived");
            }
        },
    );

    // Register event handler for redaction events (reaction removal)
    let redaction_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::room::redaction::OriginalSyncRoomRedactionEvent,
              room: matrix_sdk::Room| {
            let tx = redaction_tx.clone();
            async move {
                let Some(redacted_id) = event.redacts else {
                    return;
                };
                let room_id = room.room_id().to_string();
                let reaction_event_id = redacted_id.to_string();

                tx.send(AppEvent::ReactionRedacted {
                    room_id,
                    reaction_event_id,
                })
                .warn_closed("ReactionRedacted");
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
            tx.send(AppEvent::TypingUsersUpdated { room_id, user_ids })
                .warn_closed("TypingUsersUpdated");
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
                    tx.send(crate::event::AppEvent::VerificationRequestReceived {
                        sender: event.sender.to_string(),
                    }).warn_closed("VerificationRequestReceived");
                }
            }
        },
    );

    // Register event handlers for MatrixRTC call member events and encryption keys
    register_matrixrtc_handlers(&client, &tx, call_cmd_tx);

    // Initial room list
    let room_list = rooms::get_room_list(&client).await;
    tx.send(AppEvent::RoomListUpdated(room_list))
        .warn_closed("RoomListUpdated");

    tx.send(AppEvent::SyncStatus("syncing...".to_string()))
        .warn_closed("SyncStatus");

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
                        tx.send(AppEvent::SyncTokenUpdated(response.next_batch.clone()))
                            .warn_closed("SyncTokenUpdated");
                        tx.send(AppEvent::SyncStatus("synced".to_string()))
                            .warn_closed("SyncStatus");

                        // Update room list in background so sync loop isn't blocked
                        let room_list_client = client.clone();
                        let room_list_tx = tx.clone();
                        tokio::spawn(async move {
                            let room_list = rooms::get_room_list(&room_list_client).await;
                            room_list_tx
                                .send(AppEvent::RoomListUpdated(room_list))
                                .warn_closed("RoomListUpdated");
                        });

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
                    if let Ok(path) = config::session_path()
                        && let Err(e) = session::delete_session(&path)
                    {
                        warn!("Failed to delete session: {e}");
                    }
                    tx.send(AppEvent::LoggedOut).warn_closed("LoggedOut");
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
                tx.send(AppEvent::SyncError(msg)).warn_closed("SyncError");

                // Countdown
                for remaining in (1..=backoff_secs).rev() {
                    tx.send(AppEvent::SyncStatus(format!(
                        "reconnecting in {remaining}s..."
                    )))
                    .warn_closed("SyncStatus");
                    sleep(Duration::from_secs(1)).await;
                }
                tx.send(AppEvent::SyncStatus("reconnecting...".to_string()))
                    .warn_closed("SyncStatus");
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

fn register_matrixrtc_handlers(
    client: &Client,
    tx: &EventSender,
    call_cmd_tx: Option<CallCommandSender>,
) {
    // Handle encryption key state events via sync
    if let Some(call_cmd_tx) = call_cmd_tx {
        // Handler for state events (our own key echo + legacy clients)
        let state_cmd_tx = call_cmd_tx.clone();
        client.add_event_handler(
            move |event: matrix_sdk::ruma::events::AnySyncStateEvent, room: matrix_sdk::Room| {
                let cmd_tx = state_cmd_tx.clone();
                async move {
                    let event_type = event.event_type().to_string();
                    if event_type != "io.element.call.encryption_keys" {
                        return;
                    }

                    let room_id = room.room_id().to_string();
                    let content = match event.original_content() {
                        Some(raw) => serde_json::to_value(raw).unwrap_or_default(),
                        None => return,
                    };

                    let device_id_str = content
                        .get("device_id")
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            content
                                .get("member")
                                .and_then(|m| m.get("claimed_device_id"))
                                .and_then(|v| v.as_str())
                        })
                        .unwrap_or("")
                        .to_string();
                    let sender = event.sender().to_string();

                    dispatch_encryption_keys(
                        &cmd_tx,
                        &content,
                        &room_id,
                        &sender,
                        &device_id_str,
                        "state",
                    );
                }
            },
        );

        // Handler for to-device events (Element X sends encryption keys this way)
        let td_cmd_tx = call_cmd_tx;
        client.add_event_handler(
            move |raw: matrix_sdk::ruma::serde::Raw<matrix_sdk::ruma::events::AnyToDeviceEvent>| {
                let cmd_tx = td_cmd_tx.clone();
                async move {
                    let json: serde_json::Value = match serde_json::from_str(raw.json().get()) {
                        Ok(v) => v,
                        Err(_) => return,
                    };
                    let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if event_type != "io.element.call.encryption_keys" {
                        return;
                    }

                    let sender = json
                        .get("sender")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let content = match json.get("content") {
                        Some(c) => c,
                        None => return,
                    };

                    let device_id_str = content
                        .get("device_id")
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            content
                                .get("member")
                                .and_then(|m| m.get("claimed_device_id"))
                                .and_then(|v| v.as_str())
                        })
                        .unwrap_or("")
                        .to_string();
                    // Element X sends call_id which is the room ID for MatrixRTC calls
                    let room_id = content
                        .get("call_id")
                        .and_then(|v| v.as_str())
                        .or_else(|| content.get("room_id").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();

                    debug!(
                        "Encryption key to-device event: sender={}, device={}, room={}, content={}",
                        sender,
                        device_id_str,
                        room_id,
                        serde_json::to_string_pretty(content).unwrap_or_default()
                    );

                    dispatch_encryption_keys(
                        &cmd_tx,
                        content,
                        &room_id,
                        &sender,
                        &device_id_str,
                        "to-device",
                    );
                }
            },
        );
    }

    // Handle typed m.call.member / org.matrix.msc3401.call.member state events
    let member_tx = tx.clone();
    client.add_event_handler(
        move |event: matrix_sdk::ruma::events::SyncStateEvent<
            matrix_sdk::ruma::events::call::member::CallMemberEventContent,
        >,
              room: matrix_sdk::Room| {
            let tx = member_tx.clone();
            async move {
                use matrix_sdk::ruma::events::SyncStateEvent;
                use matrix_sdk::ruma::events::call::member::CallMemberEventContent;

                let sender = event.sender().to_string();
                let room_id = room.room_id().to_string();

                debug!(
                    "m.call.member: sender={}, room={}, state_key={}",
                    sender,
                    room_id,
                    event.state_key().as_ref(),
                );

                let has_active_memberships = match &event {
                    SyncStateEvent::Original(ev) => {
                        !matches!(ev.content, CallMemberEventContent::Empty(_))
                    }
                    SyncStateEvent::Redacted(_) => false,
                };

                if has_active_memberships {
                    info!("m.call.member joined: {} in {}", sender, room_id);
                    tx.send(AppEvent::CallMemberJoined {
                        room_id,
                        user_id: sender,
                    })
                    .warn_closed("CallMemberJoined");
                } else {
                    info!("m.call.member left: {} in {}", sender, room_id);
                    tx.send(AppEvent::CallMemberLeft {
                        room_id,
                        user_id: sender,
                    })
                    .warn_closed("CallMemberLeft");
                }
            }
        },
    );
}

fn dispatch_encryption_keys(
    cmd_tx: &CallCommandSender,
    content: &serde_json::Value,
    room_id: &str,
    sender: &str,
    device_id_str: &str,
    source: &str,
) {
    // Handle keys as array (state event format) or single object (Element X to-device format)
    let key_entries: Vec<&serde_json::Value> =
        if let Some(arr) = content.get("keys").and_then(|v| v.as_array()) {
            arr.iter().collect()
        } else if let Some(keys) = content.get("keys").filter(|v| v.as_object().is_some()) {
            vec![keys]
        } else {
            return;
        };

    if key_entries.is_empty() {
        return;
    }

    for entry in key_entries {
        let index = entry.get("index").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let encoded = match entry.get("key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => continue,
        };

        let key_bytes = match crate::voip::matrixrtc::lenient_base64_decode(encoded) {
            Ok(b) => b,
            Err(e) => {
                warn!(
                    "Failed to decode encryption key ({}) for {}:{}: {}",
                    source, sender, device_id_str, e
                );
                continue;
            }
        };

        info!(
            "Encryption key received via {}: sender={}, device={}, index={}",
            source, sender, device_id_str, index
        );

        cmd_tx
            .send(crate::voip::CallCommand::EncryptionKeyReceived {
                room_id: room_id.to_string(),
                user_id: sender.to_string(),
                device_id: device_id_str.to_string(),
                key_index: index,
                key_bytes,
            })
            .warn_closed("EncryptionKeyReceived");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_auth_error ---

    #[test]
    fn auth_error_403() {
        assert!(is_auth_error("something [403] forbidden"));
    }

    #[test]
    fn auth_error_401() {
        assert!(is_auth_error("error [401] unauthorized"));
    }

    #[test]
    fn auth_error_unknown_token() {
        assert!(is_auth_error("M_UNKNOWN_TOKEN: token expired"));
    }

    #[test]
    fn auth_error_missing_token() {
        assert!(is_auth_error("M_MISSING_TOKEN"));
    }

    #[test]
    fn auth_error_embedded_in_message() {
        assert!(is_auth_error("Sync failed: HTTP error [403] at homeserver"));
    }

    #[test]
    fn not_auth_error_500() {
        assert!(!is_auth_error("[500] internal server error"));
    }

    #[test]
    fn not_auth_error_timeout() {
        assert!(!is_auth_error("connection timeout"));
    }

    #[test]
    fn not_auth_error_empty() {
        assert!(!is_auth_error(""));
    }

    // --- sanitize_error ---

    #[test]
    fn sanitize_replaces_non_json_bytes() {
        assert_eq!(
            sanitize_error("got <non-json bytes> from server"),
            "got (non-JSON response) from server"
        );
    }

    #[test]
    fn sanitize_truncates_long_message() {
        let long_msg = "x".repeat(200);
        let result = sanitize_error(&long_msg);
        assert_eq!(result.len(), 120);
    }

    #[test]
    fn sanitize_short_message_unchanged() {
        assert_eq!(sanitize_error("short error"), "short error");
    }

    #[test]
    fn sanitize_empty_unchanged() {
        assert_eq!(sanitize_error(""), "");
    }

    #[test]
    fn sanitize_exactly_120_chars() {
        let msg = "a".repeat(120);
        assert_eq!(sanitize_error(&msg), msg);
    }

    // --- dispatch_encryption_keys ---

    fn recv_all_commands(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::voip::CallCommand>,
    ) -> Vec<crate::voip::CallCommand> {
        let mut cmds = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            cmds.push(cmd);
        }
        cmds
    }

    #[test]
    fn dispatch_array_format_single_key() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = serde_json::json!({
            "keys": [{"index": 0, "key": "aGVsbG8"}]
        });
        dispatch_encryption_keys(&tx, &content, "!room:x", "@alice:x", "DEV1", "test");
        let cmds = recv_all_commands(&mut rx);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            crate::voip::CallCommand::EncryptionKeyReceived {
                room_id,
                user_id,
                device_id,
                key_index,
                key_bytes,
            } => {
                assert_eq!(room_id, "!room:x");
                assert_eq!(user_id, "@alice:x");
                assert_eq!(device_id, "DEV1");
                assert_eq!(*key_index, 0);
                assert_eq!(key_bytes, b"hello");
            }
            other => panic!("unexpected command: {:?}", other),
        }
    }

    #[test]
    fn dispatch_object_format_element_x() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = serde_json::json!({
            "keys": {"index": 1, "key": "dGVzdA"}
        });
        dispatch_encryption_keys(&tx, &content, "!r:x", "@bob:x", "D2", "test");
        let cmds = recv_all_commands(&mut rx);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            crate::voip::CallCommand::EncryptionKeyReceived {
                key_index,
                key_bytes,
                ..
            } => {
                assert_eq!(*key_index, 1);
                assert_eq!(key_bytes, b"test");
            }
            other => panic!("unexpected command: {:?}", other),
        }
    }

    #[test]
    fn dispatch_multiple_entries() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = serde_json::json!({
            "keys": [
                {"index": 0, "key": "YQ"},
                {"index": 1, "key": "Yg"},
            ]
        });
        dispatch_encryption_keys(&tx, &content, "!r:x", "@u:x", "D", "test");
        let cmds = recv_all_commands(&mut rx);
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn dispatch_missing_keys_field() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = serde_json::json!({"other": "data"});
        dispatch_encryption_keys(&tx, &content, "!r:x", "@u:x", "D", "test");
        let cmds = recv_all_commands(&mut rx);
        assert!(cmds.is_empty());
    }

    #[test]
    fn dispatch_empty_array() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = serde_json::json!({"keys": []});
        dispatch_encryption_keys(&tx, &content, "!r:x", "@u:x", "D", "test");
        let cmds = recv_all_commands(&mut rx);
        assert!(cmds.is_empty());
    }

    #[test]
    fn dispatch_invalid_base64_skipped() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = serde_json::json!({
            "keys": [
                {"index": 0, "key": "!!!invalid!!!"},
                {"index": 1, "key": "YQ"},
            ]
        });
        dispatch_encryption_keys(&tx, &content, "!r:x", "@u:x", "D", "test");
        let cmds = recv_all_commands(&mut rx);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            crate::voip::CallCommand::EncryptionKeyReceived {
                key_index,
                key_bytes,
                ..
            } => {
                assert_eq!(*key_index, 1);
                assert_eq!(key_bytes, b"a");
            }
            other => panic!("unexpected command: {:?}", other),
        }
    }

    #[test]
    fn dispatch_missing_key_string_skipped() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = serde_json::json!({
            "keys": [
                {"index": 0},
                {"index": 1, "key": "YQ"},
            ]
        });
        dispatch_encryption_keys(&tx, &content, "!r:x", "@u:x", "D", "test");
        let cmds = recv_all_commands(&mut rx);
        assert_eq!(cmds.len(), 1);
    }
}
