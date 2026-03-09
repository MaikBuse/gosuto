pub mod audio_settings;
pub mod auth;
pub mod change_password;
pub mod create_room;
pub mod image_cache;
pub mod members;
pub mod messages;
pub mod recovery;
pub mod room_info;
pub mod rooms;
pub mod user_config;

pub use audio_settings::{AudioSettingsAction, AudioSettingsState};
pub use auth::AuthState;
pub use change_password::{ChangePasswordAction, ChangePasswordState};
pub use create_room::{CreateRoomAction, CreateRoomParams, CreateRoomState};
pub use image_cache::ImageCache;
pub use members::{MemberListState, RoomMember};
pub use messages::{DisplayMessage, MessageContent, MessageState, ReplyInfo};
pub use recovery::{
    HealingStep, RecoveryAction, RecoveryModalState, RecoveryStage, RecoveryTransition,
    recovery_key_action,
};
pub use room_info::{RoomInfoAction, RoomInfoState};
pub use rooms::{DisplayRow, RoomCategory, RoomListState, RoomSummary};
pub use user_config::{UserConfigAction, UserConfigState};

pub const HISTORY_VISIBILITY_OPTIONS: &[&str] = &["shared", "invited", "joined", "world_readable"];

#[derive(Debug)]
pub struct VerificationModalState {
    pub stage: VerificationStage,
    pub sender: String,
    pub emojis: Vec<(String, String)>,
    pub user_id_buffer: String,
}

#[derive(Debug, PartialEq)]
pub enum VerificationStage {
    ChooseAction { selected: u8 },
    EnterUserId,
    WaitingForOtherDevice,
    EmojiConfirmation,
    Completed,
    Failed(String),
}
