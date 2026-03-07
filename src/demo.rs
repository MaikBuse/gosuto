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
                "imagine explaining to a normie that your chat client runs in a terminal",
                45,
            ),
            msg(
                "$g2",
                USER_BOB,
                "\"it's not a step backwards, it's a lifestyle choice\"",
                42,
            ),
            msg(
                "$g3",
                USER_CAROL,
                "my coworkers saw my screen and asked if I was hacking the mainframe",
                38,
            ),
            emote("$g4", USER_DAVE, "puts on sunglasses in solidarity", 35),
            msg(
                "$g5",
                USER_EVE,
                "speaking of vibes, try :rain or :glitch — this thing has actual effects",
                30,
            ),
            msg(
                "$g6",
                USER_GHOST,
                "welcome to the demo btw. no server needed, just vibes",
                25,
            ),
            notice(
                "$g7",
                USER_GHOST,
                "tip: press ? for keybindings, : for commands",
                20,
            ),
            msg(
                "$g8",
                USER_ALICE,
                "the fact that hjkl works here makes me unreasonably happy",
                15,
            ),
            msg(
                "$g9",
                USER_BOB,
                "vim keybindings in a chat client. we have ascended",
                10,
            ),
            msg(
                "$g10",
                USER_CAROL,
                "i to type, Esc to return to normal mode. it just makes sense",
                5,
            ),
        ],
        ROOM_DEVELOPMENT => vec![
            msg(
                "$d1",
                USER_CAROL,
                "just fought the borrow checker for 2 hours and won. barely.",
                120,
            ),
            msg(
                "$d2",
                USER_DAVE,
                "the borrow checker doesn't lose, it just lets you think you won",
                115,
            ),
            msg(
                "$d3",
                USER_GHOST,
                "anyway the event handler refactor landed. no more clone() spam",
                110,
            ),
            msg(
                "$d4",
                USER_ALICE,
                "nice. also can we settle this — vim or emacs for Rust?",
                100,
            ),
            msg(
                "$d5",
                USER_BOB,
                "neovim + rust-analyzer. this is not a debate, it's a fact",
                90,
            ),
        ],
        ROOM_DESIGN => vec![
            msg(
                "$ds1",
                USER_EVE,
                "I keep rewatching the Blade Runner 2049 UI scenes for \"research\"",
                200,
            ),
            msg(
                "$ds2",
                USER_ALICE,
                "the Ghost in the Shell terminal aesthetics are peak though",
                195,
            ),
            msg(
                "$ds3",
                USER_GHOST,
                "real talk: CRT phosphor glow is the gold standard for terminal vibes",
                190,
            ),
            msg(
                "$ds4",
                USER_EVE,
                "cyan on black. no rounded corners. no gradients. this is the way",
                185,
            ),
        ],
        ROOM_ANNOUNCEMENTS => vec![
            notice("$a1", USER_GHOST, "gosuto v0.1.0 — it lives.", 1440),
            msg(
                "$a2",
                USER_GHOST,
                "rooms, messages, spaces, DMs, VoIP calls, shader effects, inline images. all in your terminal. you're welcome",
                1430,
            ),
            notice(
                "$a3",
                USER_GHOST,
                "v0.2.0 incoming — E2EE verification, because privacy isn't optional",
                60,
            ),
        ],
        ROOM_SUPPORT => vec![
            msg(
                "$s1",
                USER_DAVE,
                "where does the config file live? I want to tweak things",
                300,
            ),
            msg(
                "$s2",
                USER_GHOST,
                "~/.config/gosuto/config.toml — homeserver, theme, effects, the works",
                295,
            ),
            msg(
                "$s3",
                USER_DAVE,
                "sweet. and push-to-talk? I don't want hot mic energy in calls",
                290,
            ),
            msg(
                "$s4",
                USER_CAROL,
                "ptt_enabled = true in [audio], then hold Space to talk. no one hears your mechanical keyboard",
                285,
            ),
        ],
        ROOM_RANDOM => vec![
            msg(
                "$r1",
                USER_BOB,
                "hot take: tabs vs spaces doesn't matter. ligatures are the real war",
                500,
            ),
            msg(
                "$r2",
                USER_EVE,
                "hotter take: light theme users are just built different",
                495,
            ),
            msg(
                "$r3",
                USER_ALICE,
                "the hottest take: every program eventually becomes a worse version of Emacs",
                490,
            ),
            emote("$r4", USER_DAVE, "stares in tmux", 485),
        ],
        DM_ALICE => vec![
            msg(
                "$dm_a1",
                USER_ALICE,
                "yo, you around tomorrow? wanna pair on that rendering bug",
                60,
            ),
            msg(
                "$dm_a2",
                USER_GHOST,
                "yeah morning works. coffee first though",
                55,
            ),
            msg(
                "$dm_a3",
                USER_ALICE,
                "obviously. I'll set up a call in #development, bring your worst code",
                50,
            ),
        ],
        DM_BOB => vec![
            msg(
                "$dm_b1",
                USER_BOB,
                "have you been following the MatrixRTC spec? it's actually happening",
                180,
            ),
            msg(
                "$dm_b2",
                USER_GHOST,
                "wait, they finalized the call signaling?",
                175,
            ),
            msg(
                "$dm_b3",
                USER_BOB,
                "basically. the ooo-call stuff we used from the draft is pretty close to what landed",
                170,
            ),
            msg(
                "$dm_b4",
                USER_GHOST,
                "nice, less spec churn to deal with for once",
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
