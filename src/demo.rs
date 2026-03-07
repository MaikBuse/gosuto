use chrono::{Local, TimeDelta};

use crate::state::members::RoomMember;
use crate::state::messages::{DisplayMessage, MessageContent};
use crate::state::rooms::{RoomCategory, RoomSummary};

pub fn is_demo_mode() -> bool {
    std::env::args().any(|a| a == "--demo")
}

// Room IDs
const SPACE_HQ: &str = "!hq:gosuto.dev";
const SPACE_COMMUNITY: &str = "!community:gosuto.dev";
const ROOM_GENERAL: &str = "!general:gosuto.dev";
const ROOM_DEVELOPMENT: &str = "!development:gosuto.dev";
const ROOM_DESIGN: &str = "!design:gosuto.dev";
const ROOM_ANNOUNCEMENTS: &str = "!announcements:gosuto.dev";
const ROOM_SUPPORT: &str = "!support:gosuto.dev";
const ROOM_RANDOM: &str = "!random:gosuto.dev";
const DM_ALICE: &str = "!dm-alice:gosuto.dev";
const DM_BOB: &str = "!dm-bob:gosuto.dev";

// Users
const USER_GHOST: &str = "@ghost:gosuto.dev";
const USER_ALICE: &str = "@alice:gosuto.dev";
const USER_BOB: &str = "@bob:gosuto.dev";
const USER_CAROL: &str = "@carol:gosuto.dev";
const USER_DAVE: &str = "@dave:gosuto.dev";
const USER_EVE: &str = "@eve:gosuto.dev";

pub fn demo_rooms() -> Vec<RoomSummary> {
    vec![
        // Spaces
        RoomSummary {
            id: SPACE_HQ.to_string(),
            name: "Gosuto HQ".to_string(),
            category: RoomCategory::Space,
            unread_count: 0,
            is_space_child: false,
            parent_space_id: None,
        },
        RoomSummary {
            id: SPACE_COMMUNITY.to_string(),
            name: "Matrix Community".to_string(),
            category: RoomCategory::Space,
            unread_count: 0,
            is_space_child: false,
            parent_space_id: None,
        },
        // Gosuto HQ children
        RoomSummary {
            id: ROOM_GENERAL.to_string(),
            name: "general".to_string(),
            category: RoomCategory::Room,
            unread_count: 3,
            is_space_child: true,
            parent_space_id: Some(SPACE_HQ.to_string()),
        },
        RoomSummary {
            id: ROOM_DEVELOPMENT.to_string(),
            name: "development".to_string(),
            category: RoomCategory::Room,
            unread_count: 0,
            is_space_child: true,
            parent_space_id: Some(SPACE_HQ.to_string()),
        },
        RoomSummary {
            id: ROOM_DESIGN.to_string(),
            name: "design".to_string(),
            category: RoomCategory::Room,
            unread_count: 1,
            is_space_child: true,
            parent_space_id: Some(SPACE_HQ.to_string()),
        },
        // Matrix Community children
        RoomSummary {
            id: ROOM_ANNOUNCEMENTS.to_string(),
            name: "announcements".to_string(),
            category: RoomCategory::Room,
            unread_count: 0,
            is_space_child: true,
            parent_space_id: Some(SPACE_COMMUNITY.to_string()),
        },
        RoomSummary {
            id: ROOM_SUPPORT.to_string(),
            name: "support".to_string(),
            category: RoomCategory::Room,
            unread_count: 2,
            is_space_child: true,
            parent_space_id: Some(SPACE_COMMUNITY.to_string()),
        },
        // Orphan room
        RoomSummary {
            id: ROOM_RANDOM.to_string(),
            name: "random".to_string(),
            category: RoomCategory::Room,
            unread_count: 0,
            is_space_child: false,
            parent_space_id: None,
        },
        // DMs
        RoomSummary {
            id: DM_ALICE.to_string(),
            name: "Alice".to_string(),
            category: RoomCategory::DirectMessage,
            unread_count: 1,
            is_space_child: false,
            parent_space_id: None,
        },
        RoomSummary {
            id: DM_BOB.to_string(),
            name: "Bob".to_string(),
            category: RoomCategory::DirectMessage,
            unread_count: 0,
            is_space_child: false,
            parent_space_id: None,
        },
    ]
}

fn msg(event_id: &str, sender: &str, body: &str, mins_ago: i64) -> DisplayMessage {
    DisplayMessage {
        event_id: event_id.to_string(),
        sender: sender.to_string(),
        content: MessageContent::Text(body.to_string()),
        timestamp: Local::now() - TimeDelta::minutes(mins_ago),
        is_emote: false,
        is_notice: false,
        pending: false,
        verified: Some(true),
    }
}

fn emote(event_id: &str, sender: &str, body: &str, mins_ago: i64) -> DisplayMessage {
    DisplayMessage {
        is_emote: true,
        ..msg(event_id, sender, body, mins_ago)
    }
}

fn notice(event_id: &str, sender: &str, body: &str, mins_ago: i64) -> DisplayMessage {
    DisplayMessage {
        is_notice: true,
        ..msg(event_id, sender, body, mins_ago)
    }
}

pub fn demo_messages_for_room(room_id: &str) -> Vec<DisplayMessage> {
    match room_id {
        ROOM_GENERAL => vec![
            msg(
                "$g1",
                USER_ALICE,
                "hey everyone! has anyone tried the new TUI framework?",
                45,
            ),
            msg(
                "$g2",
                USER_BOB,
                "yeah, ratatui is amazing. the immediate mode rendering is so clean",
                42,
            ),
            msg(
                "$g3",
                USER_CAROL,
                "I've been working on a Matrix client with it",
                38,
            ),
            emote("$g4", USER_DAVE, "nods enthusiastically", 35),
            msg(
                "$g5",
                USER_GHOST,
                "welcome to gosuto! this is a demo of the app running without a server",
                30,
            ),
            msg(
                "$g6",
                USER_EVE,
                "the effects are really cool - try :rain or :glitch",
                25,
            ),
            notice(
                "$g7",
                USER_GHOST,
                "tip: press ? for keybindings, : for commands",
                20,
            ),
            msg("$g8", USER_ALICE, "I love how snappy everything feels", 15),
            msg(
                "$g9",
                USER_BOB,
                "agreed. and the vim keybindings are perfect",
                10,
            ),
            msg(
                "$g10",
                USER_CAROL,
                "hjkl to navigate, i to type, Esc to go back to normal mode",
                5,
            ),
        ],
        ROOM_DEVELOPMENT => vec![
            msg(
                "$d1",
                USER_CAROL,
                "pushed the new event handler refactor",
                120,
            ),
            msg(
                "$d2",
                USER_DAVE,
                "nice! the async architecture is looking solid",
                115,
            ),
            msg(
                "$d3",
                USER_GHOST,
                "let's make sure we handle all the edge cases in sync",
                110,
            ),
            msg(
                "$d4",
                USER_ALICE,
                "I'll add tests for the room list state machine",
                100,
            ),
            msg(
                "$d5",
                USER_BOB,
                "don't forget to run `cargo clippy` before pushing",
                90,
            ),
        ],
        ROOM_DESIGN => vec![
            msg("$ds1", USER_EVE, "thoughts on the color scheme?", 200),
            msg(
                "$ds2",
                USER_ALICE,
                "I think the cyan accents work well on dark terminals",
                195,
            ),
            msg(
                "$ds3",
                USER_GHOST,
                "we should support both dark and light themes eventually",
                190,
            ),
            msg(
                "$ds4",
                USER_EVE,
                "agreed. the border styles could use some love too",
                185,
            ),
        ],
        ROOM_ANNOUNCEMENTS => vec![
            notice("$a1", USER_GHOST, "gosuto v0.1.0 released!", 1440),
            msg(
                "$a2",
                USER_GHOST,
                "features: rooms, messages, spaces, DMs, calls, effects, inline images",
                1430,
            ),
            notice(
                "$a3",
                USER_GHOST,
                "gosuto v0.2.0 coming soon with E2EE verification",
                60,
            ),
        ],
        ROOM_SUPPORT => vec![
            msg("$s1", USER_DAVE, "how do I set up the config file?", 300),
            msg(
                "$s2",
                USER_GHOST,
                "check ~/.config/gosuto/config.toml - you can set homeserver, theme, and effects",
                295,
            ),
            msg(
                "$s3",
                USER_DAVE,
                "thanks! and how do I enable push-to-talk?",
                290,
            ),
            msg(
                "$s4",
                USER_CAROL,
                "set ptt_enabled = true in [audio] section, then hold Space during a call",
                285,
            ),
        ],
        ROOM_RANDOM => vec![
            msg(
                "$r1",
                USER_BOB,
                "anyone else here use a tiling window manager?",
                500,
            ),
            msg("$r2", USER_EVE, "sway + foot terminal is my setup", 495),
            msg("$r3", USER_ALICE, "i3 + alacritty here", 490),
            emote("$r4", USER_DAVE, "uses tmux for everything", 485),
        ],
        DM_ALICE => vec![
            msg(
                "$dm_a1",
                USER_ALICE,
                "hey! want to pair on the UI refactor tomorrow?",
                60,
            ),
            msg("$dm_a2", USER_GHOST, "sure, morning works for me", 55),
            msg(
                "$dm_a3",
                USER_ALICE,
                "great, I'll set up a call in the development room",
                50,
            ),
        ],
        DM_BOB => vec![
            msg(
                "$dm_b1",
                USER_BOB,
                "did you see the new Matrix spec update?",
                180,
            ),
            msg("$dm_b2", USER_GHOST, "not yet, anything interesting?", 175),
            msg(
                "$dm_b3",
                USER_BOB,
                "MatrixRTC is getting standardized, great for our call implementation",
                170,
            ),
            msg(
                "$dm_b4",
                USER_GHOST,
                "nice, we're already using the draft spec",
                165,
            ),
        ],
        _ => vec![],
    }
}

pub fn demo_members_for_room(room_id: &str) -> Vec<RoomMember> {
    let ghost = RoomMember {
        user_id: USER_GHOST.to_string(),
        display_name: "Ghost".to_string(),
        power_level: 100,
    };
    let alice = RoomMember {
        user_id: USER_ALICE.to_string(),
        display_name: "Alice".to_string(),
        power_level: 50,
    };
    let bob = RoomMember {
        user_id: USER_BOB.to_string(),
        display_name: "Bob".to_string(),
        power_level: 50,
    };
    let carol = RoomMember {
        user_id: USER_CAROL.to_string(),
        display_name: "Carol".to_string(),
        power_level: 0,
    };
    let dave = RoomMember {
        user_id: USER_DAVE.to_string(),
        display_name: "Dave".to_string(),
        power_level: 0,
    };
    let eve = RoomMember {
        user_id: USER_EVE.to_string(),
        display_name: "Eve".to_string(),
        power_level: 0,
    };

    match room_id {
        ROOM_GENERAL => vec![
            ghost,
            alice.clone(),
            bob.clone(),
            carol.clone(),
            dave.clone(),
            eve.clone(),
        ],
        ROOM_DEVELOPMENT => vec![ghost, alice, carol, dave.clone()],
        ROOM_DESIGN => vec![ghost, alice, eve.clone()],
        ROOM_ANNOUNCEMENTS => vec![ghost, alice, bob, carol, dave, eve],
        ROOM_SUPPORT => vec![ghost, carol, dave],
        ROOM_RANDOM => vec![ghost, alice, bob, dave, eve],
        DM_ALICE => vec![ghost, alice],
        DM_BOB => vec![ghost, bob],
        _ => vec![ghost],
    }
}
