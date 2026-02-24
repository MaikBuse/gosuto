use crossterm::event::KeyEvent;
use tokio::sync::mpsc;

use crate::state::{DisplayMessage, RoomMember, RoomSummary};
use crate::voip::CallState;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    // Auth events
    LoginSuccess {
        user_id: String,
        device_id: String,
        homeserver: String,
    },
    LoginFailure(String),
    LoggedOut,
    // Room events
    RoomListUpdated(Vec<RoomSummary>),
    // Message events
    NewMessage {
        room_id: String,
        message: DisplayMessage,
    },
    MessagesLoaded {
        room_id: String,
        messages: Vec<DisplayMessage>,
        has_more: bool,
    },
    MessageSent {
        room_id: String,
        event_id: String,
        body: String,
    },
    SendError {
        room_id: String,
        error: String,
    },
    // Member events
    MembersLoaded {
        room_id: String,
        members: Vec<RoomMember>,
    },
    // Sync events
    SyncError(String),
    SyncStatus(String),
    // VoIP events
    CallInvite {
        call_id: String,
        room_id: String,
        sender: String,
        sdp: String,
    },
    CallAnswer {
        call_id: String,
        room_id: String,
        sdp: String,
    },
    CallCandidates {
        call_id: String,
        room_id: String,
        candidates: Vec<String>,
    },
    CallHangup {
        call_id: String,
        room_id: String,
    },
    CallStateChanged {
        call_id: String,
        state: CallState,
    },
    CallError(String),
    CallEnded,
}

pub type EventSender = mpsc::UnboundedSender<AppEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<AppEvent>;

pub fn event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
