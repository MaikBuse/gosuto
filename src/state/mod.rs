pub mod auth;
pub mod members;
pub mod messages;
pub mod rooms;

pub use auth::AuthState;
pub use members::{MemberListState, RoomMember};
pub use messages::{DisplayMessage, MessageState};
pub use rooms::{DisplayRow, RoomCategory, RoomListState, RoomSummary};

#[derive(Debug)]
pub struct VerificationModalState {
    pub stage: VerificationStage,
    pub sender: String,
    pub emojis: Vec<(String, String)>,
    pub flow_id: String,
}

#[derive(Debug, PartialEq)]
pub enum VerificationStage {
    WaitingForOtherDevice,
    EmojiConfirmation,
    Completed,
    Failed(String),
}
