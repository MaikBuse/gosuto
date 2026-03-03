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
    RegisterFailure(String),
    LoggedOut,
    AutoLogin {
        homeserver: String,
        username: String,
        password: String,
    },
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
    FetchError {
        room_id: String,
        error: String,
    },
    // Member events
    MembersLoaded {
        room_id: String,
        members: Vec<RoomMember>,
    },
    // Typing events
    TypingUsersUpdated {
        room_id: String,
        user_ids: Vec<String>,
    },
    DmRoomReady {
        room_id: String,
    },
    RoomCreated {
        room_id: String,
    },
    // Sync events
    SyncError(String),
    SyncStatus(String),
    SyncTokenUpdated(String),
    // VoIP events (MatrixRTC)
    CallMemberJoined {
        room_id: String,
        user_id: String,
    },
    CallMemberLeft {
        room_id: String,
        user_id: String,
    },
    CallParticipantUpdate {
        participants: Vec<String>,
    },
    CallStateChanged {
        room_id: String,
        state: CallState,
    },
    CallError(String),
    CallEnded,
    // Room info events
    RoomInfoLoaded {
        room_id: String,
        name: Option<String>,
        topic: Option<String>,
        history_visibility: String,
        encrypted: bool,
    },
    RoomSettingUpdated {
        room_id: String,
    },
    RoomSettingError {
        error: String,
    },
    // User config events
    UserConfigLoaded {
        display_name: Option<String>,
        verified: bool,
    },
    UserConfigUpdated,
    UserConfigError(String),
    // Audio settings events
    MicLevel(f32),
    KeyRelease(KeyEvent),
    // Verification events
    VerificationRequestReceived {
        sender: String,
        flow_id: String,
    },
    VerificationSasEmoji {
        emojis: Vec<(String, String)>,
        flow_id: String,
        sender: String,
    },
    VerificationCompleted {
        sender: String,
    },
    VerificationCancelled {
        reason: String,
    },
    VerificationError(String),
    // Recovery events
    RecoveryState(String),
    RecoveryKeyReady(String),
    RecoveryError(String),
    RecoveryRecovered,
}

pub type EventSender = mpsc::UnboundedSender<AppEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<AppEvent>;

pub fn event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
