use std::collections::HashMap;

use matrix_sdk::Client;
use matrix_sdk::RoomMemberships;
use matrix_sdk::room::{Receipts, Room};
use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::ruma::events::room::history_visibility::HistoryVisibility;
use matrix_sdk::ruma::events::space::child::SpaceChildEventContent;
use tracing::{error, warn};

use matrix_sdk::ruma::events::room::power_levels::RoomPowerLevelsEventContent;
use matrix_sdk::ruma::serde::Raw;

use crate::event::AppEvent;
use crate::event::EventSender;
use crate::event::WarnClosed;
use crate::state::{RoomCategory, RoomMember, RoomSummary};

pub fn call_power_level_override() -> Option<Raw<RoomPowerLevelsEventContent>> {
    let pl = serde_json::json!({
        "events": {
            "m.call.member": 0,
            "org.matrix.msc3401.call.member": 0
        }
    });
    match serde_json::value::to_raw_value(&pl) {
        Ok(raw) => Some(Raw::from_json(raw)),
        Err(e) => {
            warn!("Failed to serialize call power level override: {e}");
            None
        }
    }
}

fn resolve_room(client: &Client, room_id: &str, on_error: impl FnOnce(String)) -> Option<Room> {
    let rid: OwnedRoomId = match room_id.try_into() {
        Ok(rid) => rid,
        Err(_) => {
            on_error(format!("Invalid room ID: {}", room_id));
            return None;
        }
    };
    client.get_room(&rid).or_else(|| {
        on_error("Room not found".to_string());
        None
    })
}

pub async fn get_room_list(client: &Client) -> Vec<RoomSummary> {
    let joined = client.joined_rooms();
    let mut rooms = Vec::new();

    for room in &joined {
        let id = room.room_id().to_string();
        let is_dm = room.is_direct().await.unwrap_or(false);

        let name = if is_dm {
            let target_uid = room
                .direct_targets()
                .iter()
                .next()
                .and_then(|uid| uid.as_user_id().map(|u| u.to_owned()));

            if let Some(uid) = target_uid {
                let id_without_at = uid
                    .as_str()
                    .strip_prefix('@')
                    .unwrap_or(uid.as_str())
                    .to_string();
                let local_part = uid.localpart();

                // Try to fetch the member's display name
                let display = room
                    .get_member(&uid)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|m| m.display_name().map(|n| n.to_string()));

                match display {
                    Some(dn) if dn != local_part => format!("{} ({})", dn, id_without_at),
                    _ => id_without_at,
                }
            } else {
                match room.display_name().await {
                    Ok(dn) => dn.to_string(),
                    Err(_) => id.clone(),
                }
            }
        } else {
            match room.display_name().await {
                Ok(dn) => dn.to_string(),
                Err(_) => id.clone(),
            }
        };
        let is_space = room.is_space();

        let category = if is_space {
            RoomCategory::Space
        } else if is_dm {
            RoomCategory::DirectMessage
        } else {
            RoomCategory::Room
        };

        let unread_count = room.unread_notification_counts().notification_count;

        rooms.push(RoomSummary {
            id,
            name,
            category,
            unread_count,
            is_space_child: false,
            parent_space_id: None,
        });
    }

    // Fetch space children via m.space.child state events
    let mut child_to_space: HashMap<String, String> = HashMap::new();
    for room in &joined {
        if !room.is_space() {
            continue;
        }
        let space_id = room.room_id().to_string();
        match room
            .get_state_events_static::<SpaceChildEventContent>()
            .await
        {
            Ok(events) => {
                for raw in events {
                    if let Ok(ev) = raw.deserialize() {
                        let child_id = ev.state_key().to_string();
                        // First space wins for multi-space rooms
                        child_to_space.entry(child_id).or_insert(space_id.clone());
                    }
                }
            }
            Err(e) => {
                warn!("Failed to fetch space children for {}: {}", space_id, e);
            }
        }
    }

    // Populate parent_space_id on child rooms
    for room_summary in &mut rooms {
        if let Some(space_id) = child_to_space.get(&room_summary.id) {
            room_summary.parent_space_id = Some(space_id.clone());
            room_summary.is_space_child = true;
        }
    }

    // Fetch invited rooms
    for room in &client.invited_rooms() {
        let id = room.room_id().to_string();
        let name = match room.display_name().await {
            Ok(dn) => dn.to_string(),
            Err(_) => id.clone(),
        };
        rooms.push(RoomSummary {
            id,
            name,
            category: RoomCategory::Invitation,
            unread_count: 0,
            is_space_child: false,
            parent_space_id: None,
        });
    }

    // Sort: invitations first, then spaces, rooms, DMs
    rooms.sort_by(|a, b| {
        let cat_order = |c: &RoomCategory| match c {
            RoomCategory::Invitation => 0,
            RoomCategory::Space => 1,
            RoomCategory::Room => 2,
            RoomCategory::DirectMessage => 3,
        };
        cat_order(&a.category)
            .cmp(&cat_order(&b.category))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    rooms
}

pub async fn fetch_room_members(client: &Client, room_id: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |e| error!("member fetch: {}", e)) else {
        return;
    };

    match room.members(RoomMemberships::JOIN).await {
        Ok(members) => {
            let room_members: Vec<RoomMember> = members
                .iter()
                .map(|m| {
                    let display_name = m
                        .display_name()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| m.user_id().to_string());
                    RoomMember {
                        user_id: m.user_id().to_string(),
                        display_name,
                        power_level: match m.power_level() {
                            matrix_sdk::ruma::events::room::power_levels::UserPowerLevel::Int(
                                i,
                            ) => i64::from(i),
                            _ => i64::MAX,
                        },
                        verified: None,
                    }
                })
                .collect();

            tx.send(AppEvent::MembersLoaded {
                room_id: room_id.to_string(),
                members: room_members,
            })
            .warn_closed("MembersLoaded");
        }
        Err(e) => {
            error!("Failed to fetch room members: {}", e);
        }
    }
}

pub async fn check_member_verification(client: &Client, room_id: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |_| {}) else {
        return;
    };

    let members = match room.members(RoomMemberships::JOIN).await {
        Ok(m) => m,
        Err(_) => return,
    };

    for member in &members {
        let uid = member.user_id();
        let verified = match client.encryption().get_user_identity(uid).await {
            Ok(Some(identity)) => identity.is_verified(),
            _ => false,
        };
        tx.send(AppEvent::MemberVerificationStatus {
            room_id: room_id.to_string(),
            user_id: uid.to_string(),
            verified,
        })
        .warn_closed("MemberVerificationStatus");
    }
}

pub async fn fetch_room_info(client: &Client, room_id: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |e| {
        error!("info fetch: {}", e);
        tx.send(AppEvent::RoomSettingError { error: e })
            .warn_closed("RoomSettingError");
    }) else {
        return;
    };

    let name = match room.display_name().await {
        Ok(dn) => Some(dn.to_string()),
        Err(_) => None,
    };
    let topic = room.topic();
    let visibility = room.history_visibility_or_default();
    let encrypted = room
        .latest_encryption_state()
        .await
        .map(|s| s.is_encrypted())
        .unwrap_or(false);

    tx.send(AppEvent::RoomInfoLoaded {
        room_id: room_id.to_string(),
        name,
        topic,
        history_visibility: visibility.as_ref().to_string(),
        encrypted,
    })
    .warn_closed("RoomInfoLoaded");
}

pub async fn set_room_name(client: &Client, room_id: &str, name: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |e| {
        tx.send(AppEvent::RoomSettingError { error: e })
            .warn_closed("RoomSettingError");
    }) else {
        return;
    };

    match room.set_name(name.to_string()).await {
        Ok(_) => {
            tx.send(AppEvent::RoomSettingUpdated {
                room_id: room_id.to_string(),
            })
            .warn_closed("RoomSettingUpdated");
        }
        Err(e) => {
            tx.send(AppEvent::RoomSettingError {
                error: format!("Failed to set room name: {}", e),
            })
            .warn_closed("RoomSettingError");
        }
    }
}

pub async fn set_room_topic(client: &Client, room_id: &str, topic: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |e| {
        tx.send(AppEvent::RoomSettingError { error: e })
            .warn_closed("RoomSettingError");
    }) else {
        return;
    };

    match room.set_room_topic(topic).await {
        Ok(_) => {
            tx.send(AppEvent::RoomSettingUpdated {
                room_id: room_id.to_string(),
            })
            .warn_closed("RoomSettingUpdated");
        }
        Err(e) => {
            tx.send(AppEvent::RoomSettingError {
                error: format!("Failed to set room topic: {}", e),
            })
            .warn_closed("RoomSettingError");
        }
    }
}

pub async fn enable_encryption(client: &Client, room_id: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |e| {
        tx.send(AppEvent::RoomSettingError { error: e })
            .warn_closed("RoomSettingError");
    }) else {
        return;
    };

    match room.enable_encryption().await {
        Ok(_) => {
            tx.send(AppEvent::RoomSettingUpdated {
                room_id: room_id.to_string(),
            })
            .warn_closed("RoomSettingUpdated");
        }
        Err(e) => {
            tx.send(AppEvent::RoomSettingError {
                error: format!("Failed to enable encryption: {}", e),
            })
            .warn_closed("RoomSettingError");
        }
    }
}

pub async fn set_history_visibility(
    client: &Client,
    room_id: &str,
    visibility: &str,
    tx: &EventSender,
) {
    let Some(room) = resolve_room(client, room_id, |e| {
        tx.send(AppEvent::RoomSettingError { error: e })
            .warn_closed("RoomSettingError");
    }) else {
        return;
    };

    let vis = match visibility {
        "shared" => HistoryVisibility::Shared,
        "invited" => HistoryVisibility::Invited,
        "joined" => HistoryVisibility::Joined,
        "world_readable" => HistoryVisibility::WorldReadable,
        other => {
            tx.send(AppEvent::RoomSettingError {
                error: format!("Invalid visibility: {}", other),
            })
            .warn_closed("RoomSettingError");
            return;
        }
    };

    match room
        .privacy_settings()
        .update_room_history_visibility(vis)
        .await
    {
        Ok(()) => {
            tx.send(AppEvent::RoomSettingUpdated {
                room_id: room_id.to_string(),
            })
            .warn_closed("RoomSettingUpdated");
        }
        Err(e) => {
            tx.send(AppEvent::RoomSettingError {
                error: format!("Failed to update visibility: {}", e),
            })
            .warn_closed("RoomSettingError");
        }
    }
}

pub async fn mark_room_as_read(client: &Client, room_id: &str, event_id_hint: Option<&str>) {
    let Some(room) = resolve_room(client, room_id, |e| warn!("read receipt: {}", e)) else {
        return;
    };

    // Try the caller-provided event_id first, fall back to room.latest_event()
    let event_id = if let Some(hint) = event_id_hint {
        let parsed: Result<matrix_sdk::ruma::OwnedEventId, _> = hint.try_into();
        match parsed {
            Ok(eid) => Some(eid),
            Err(e) => {
                warn!("Invalid event_id hint '{}' for {}: {}", hint, room_id, e);
                None
            }
        }
    } else {
        None
    };

    let event_id = event_id.or_else(|| {
        let ev = room.latest_event();
        if ev.is_none() {
            warn!(
                "No latest_event for room {} (encrypted DM?), cannot send read receipt",
                room_id
            );
        }
        ev.and_then(|e| {
            let eid = e.event_id();
            if eid.is_none() {
                warn!("latest_event for room {} has no event_id", room_id);
            }
            eid
        })
    });

    let Some(event_id) = event_id else {
        return;
    };

    if let Err(e) = room
        .send_multiple_receipts(
            Receipts::new()
                .fully_read_marker(Some(event_id.clone()))
                .public_read_receipt(Some(event_id)),
        )
        .await
    {
        warn!("Failed to send read receipt for {}: {}", room_id, e);
    }
}

pub async fn accept_invite(client: &Client, room_id: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |e| {
        tx.send(AppEvent::InviteError { error: e })
            .warn_closed("InviteError");
    }) else {
        return;
    };

    match room.join().await {
        Ok(_) => {
            tx.send(AppEvent::InviteAccepted {
                room_id: room_id.to_string(),
            })
            .warn_closed("InviteAccepted");
        }
        Err(e) => {
            tx.send(AppEvent::InviteError {
                error: format!("Failed to accept invite: {}", e),
            })
            .warn_closed("InviteError");
        }
    }
}

pub async fn decline_invite(client: &Client, room_id: &str, tx: &EventSender) {
    let Some(room) = resolve_room(client, room_id, |e| {
        tx.send(AppEvent::InviteError { error: e })
            .warn_closed("InviteError");
    }) else {
        return;
    };

    match room.leave().await {
        Ok(_) => {
            tx.send(AppEvent::InviteDeclined)
                .warn_closed("InviteDeclined");
        }
        Err(e) => {
            tx.send(AppEvent::InviteError {
                error: format!("Failed to decline invite: {}", e),
            })
            .warn_closed("InviteError");
        }
    }
}

pub async fn invite_user(client: &Client, room_id: &str, user_id: &str, tx: &EventSender) {
    let uid_parsed: Result<matrix_sdk::ruma::OwnedUserId, _> = user_id.try_into();
    let Ok(uid) = uid_parsed else {
        tx.send(AppEvent::InviteError {
            error: format!("Invalid user ID: {}", user_id),
        })
        .warn_closed("InviteError");
        return;
    };

    let Some(room) = resolve_room(client, room_id, |e| {
        tx.send(AppEvent::InviteError { error: e })
            .warn_closed("InviteError");
    }) else {
        return;
    };

    match room.invite_user_by_id(&uid).await {
        Ok(_) => {
            tx.send(AppEvent::UserInvited {
                user_id: user_id.to_string(),
            })
            .warn_closed("UserInvited");
        }
        Err(e) => {
            tx.send(AppEvent::InviteError {
                error: format!("Failed to invite user: {}", e),
            })
            .warn_closed("InviteError");
        }
    }
}
