pub mod command;
pub mod handler;
pub mod insert;
pub mod normal;
pub mod vim;

pub use handler::handle_key;
pub use vim::{FocusPanel, VimMode, VimState};

#[derive(Debug, Clone)]
pub enum InputResult {
    None,
    Quit,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    Select,
    SwitchPanel,
    FocusLeft,
    FocusRight,
    SendMessage(String),
    Command(CommandAction),
    Search(String),
    ClearSearch,
    CallMember,
    AnswerCall,
    RejectCall,
    ShowWhichKey,
    TypingActivity,
}

#[derive(Debug, Clone)]
pub enum CommandAction {
    Quit,
    Join(String),
    Leave,
    DirectMessage(String),
    Logout,
    Call,
    Answer,
    Reject,
    Hangup,
    Rain,
    Glitch,
    AudioSettings,
    CreateRoom,
    RoomInfo,
    Configure,
    NerdFonts,
    Recovery,
}
