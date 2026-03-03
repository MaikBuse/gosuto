pub mod auth;
pub mod image_cache;
pub mod members;
pub mod messages;
pub mod rooms;

pub use auth::AuthState;
pub use image_cache::ImageCache;
pub use members::{MemberListState, RoomMember};
pub use messages::{DisplayMessage, MessageContent, MessageState};
pub use rooms::{DisplayRow, RoomCategory, RoomListState, RoomSummary};

#[derive(Debug)]
pub struct VerificationModalState {
    pub stage: VerificationStage,
    pub sender: String,
    pub emojis: Vec<(String, String)>,
}

#[derive(Debug, PartialEq)]
pub enum VerificationStage {
    WaitingForOtherDevice,
    EmojiConfirmation,
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct RecoveryModalState {
    pub stage: RecoveryStage,
    pub confirm_buffer: String,
    pub key_buffer: String,
    pub copied: bool,
}

#[derive(Debug)]
pub enum RecoveryStage {
    Checking,
    Setup,
    Incomplete,
    EnterKey,
    Recovering,
    Creating,
    ShowKey(String),
    Enabled,
    ConfirmReset,
    Resetting,
    Failed(String),
}
