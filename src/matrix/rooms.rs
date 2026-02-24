use matrix_sdk::Client;
use matrix_sdk::RoomMemberships;
use tracing::error;

use crate::event::EventSender;
use crate::state::{RoomCategory, RoomMember, RoomSummary};
use crate::event::AppEvent;

pub async fn get_room_list(client: &Client) -> Vec<RoomSummary> {
    let joined = client.joined_rooms();
    let mut rooms = Vec::new();

    for room in joined {
        let id = room.room_id().to_string();
        let name = match room.display_name().await {
            Ok(dn) => dn.to_string(),
            Err(_) => id.clone(),
        };

        let is_dm = room.is_direct().await.unwrap_or(false);
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

    // Sort: spaces first, then rooms, then DMs
    rooms.sort_by(|a, b| {
        let cat_order = |c: &RoomCategory| match c {
            RoomCategory::Space => 0,
            RoomCategory::Room => 1,
            RoomCategory::DirectMessage => 2,
        };
        cat_order(&a.category)
            .cmp(&cat_order(&b.category))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    rooms
}

pub async fn fetch_room_members(client: &Client, room_id: &str, tx: &EventSender) {
    let room_id_parsed: Result<matrix_sdk::ruma::OwnedRoomId, _> = room_id.try_into();
    let Ok(rid) = room_id_parsed else {
        error!("Invalid room id for member fetch: {}", room_id);
        return;
    };

    let Some(room) = client.get_room(&rid) else {
        error!("Room not found for member fetch: {}", room_id);
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
                            matrix_sdk::ruma::events::room::power_levels::UserPowerLevel::Int(i) => i64::from(i),
                            _ => i64::MAX,
                        },
                    }
                })
                .collect();

            let _ = tx.send(AppEvent::MembersLoaded {
                room_id: room_id.to_string(),
                members: room_members,
            });
        }
        Err(e) => {
            error!("Failed to fetch room members: {}", e);
        }
    }
}
