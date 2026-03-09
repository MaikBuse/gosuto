use crossterm::event::{KeyCode, KeyEvent};

pub struct UserConfigState {
    pub open: bool,
    pub user_id: String,
    pub device_id: String,
    pub homeserver: String,
    pub display_name: Option<String>,
    pub display_name_buffer: String,
    pub editing_display_name: bool,
    pub verified: bool,
    pub recovery_status: crate::event::RecoveryStatus,
    pub selected_field: usize, // 0=display name
    pub loading: bool,
    pub saving: bool,
}

impl UserConfigState {
    pub fn new() -> Self {
        Self {
            open: false,
            user_id: String::new(),
            device_id: String::new(),
            homeserver: String::new(),
            display_name: None,
            display_name_buffer: String::new(),
            editing_display_name: false,
            verified: false,
            recovery_status: crate::event::RecoveryStatus::Disabled,
            selected_field: 0,
            loading: false,
            saving: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> UserConfigAction {
        if self.loading || self.saving {
            if key.code == KeyCode::Esc {
                return UserConfigAction::Close;
            }
            return UserConfigAction::None;
        }

        // Inline display name editing mode
        if self.editing_display_name {
            match key.code {
                KeyCode::Esc => {
                    self.editing_display_name = false;
                    self.display_name_buffer.clear();
                }
                KeyCode::Enter => {
                    let new_name = self.display_name_buffer.clone();
                    if !new_name.is_empty() {
                        self.saving = true;
                        self.editing_display_name = false;
                        return UserConfigAction::SetDisplayName(new_name);
                    }
                    self.editing_display_name = false;
                    self.display_name_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.display_name_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.display_name_buffer.push(c);
                }
                _ => {}
            }
            return UserConfigAction::None;
        }

        match key.code {
            KeyCode::Esc => UserConfigAction::Close,
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected_field = (self.selected_field + 1).min(1);
                UserConfigAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_field = self.selected_field.saturating_sub(1);
                UserConfigAction::None
            }
            KeyCode::Enter => {
                if self.selected_field == 0 {
                    // Enter display name editing mode
                    self.editing_display_name = true;
                    self.display_name_buffer = self.display_name.clone().unwrap_or_default();
                    UserConfigAction::None
                } else if self.selected_field == 1 {
                    UserConfigAction::OpenChangePassword
                } else {
                    UserConfigAction::None
                }
            }
            _ => UserConfigAction::None,
        }
    }
}

pub enum UserConfigAction {
    None,
    Close,
    SetDisplayName(String),
    OpenChangePassword,
}
