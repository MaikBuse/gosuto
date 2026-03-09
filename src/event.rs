use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use tokio::sync::mpsc;

use crate::state::{DisplayMessage, RoomMember, RoomSummary};
use crate::voip::CallState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecoveryStatus {
    #[default]
    Disabled,
    Incomplete,
    Enabled,
}

/// A oneshot sender wrapped so `AppEvent` can derive `Clone` + `Debug`.
/// The first consumer calls `.take()` to extract the real sender.
#[derive(Clone)]
pub struct PasswordSender(pub Arc<Mutex<Option<tokio::sync::oneshot::Sender<String>>>>);

impl PasswordSender {
    pub fn new(tx: tokio::sync::oneshot::Sender<String>) -> Self {
        Self(Arc::new(Mutex::new(Some(tx))))
    }

    pub fn take(&self) -> Option<tokio::sync::oneshot::Sender<String>> {
        self.0.lock().ok()?.take()
    }
}

impl std::fmt::Debug for PasswordSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordSender(..)")
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize,
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
        recovery_status: RecoveryStatus,
    },
    UserConfigUpdated,
    UserConfigError(String),
    // Audio settings events
    MicLevel(f32),
    KeyRelease,
    PttKeyCaptured(String),
    PttListenerFailed(String),
    // Recovery events
    RecoveryStateChecked(crate::app::RecoveryStage),
    RecoveryKeyReady(String),
    RecoveryHealingProgress(crate::app::HealingStep),
    RecoveryRecovered,
    RecoveryNeedPassword(PasswordSender),
    RecoveryError(String),
    // Verification events
    VerificationRequestReceived {
        sender: String,
    },
    VerificationSasEmoji {
        emojis: Vec<(String, String)>,
        sender: String,
    },
    VerificationCompleted,
    VerificationCancelled {
        reason: String,
    },
    VerificationError(String),
    // Member verification status
    MemberVerificationStatus {
        room_id: String,
        user_id: String,
        verified: bool,
    },
    // Invitation events
    InviteAccepted {
        room_id: String,
    },
    InviteDeclined,
    UserInvited {
        user_id: String,
    },
    InviteError {
        error: String,
    },
    // Image events
    ImageLoaded {
        event_id: String,
        image_data: Vec<u8>,
    },
    ImageFailed {
        event_id: String,
        error: String,
    },
}

pub type EventSender = mpsc::UnboundedSender<AppEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<AppEvent>;

pub fn event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
