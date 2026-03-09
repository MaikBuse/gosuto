use crossterm::event::{KeyCode, KeyEvent};

pub struct ChangePasswordState {
    pub open: bool,
    pub selected_field: usize, // 0=current, 1=new, 2=confirm
    pub current_buffer: String,
    pub new_buffer: String,
    pub confirm_buffer: String,
    pub saving: bool,
}

impl ChangePasswordState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_field: 0,
            current_buffer: String::new(),
            new_buffer: String::new(),
            confirm_buffer: String::new(),
            saving: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ChangePasswordAction {
        if self.saving {
            if key.code == KeyCode::Esc {
                return ChangePasswordAction::Close;
            }
            return ChangePasswordAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                *self = Self::new();
                ChangePasswordAction::Close
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected_field = (self.selected_field + 1).min(2);
                ChangePasswordAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_field = self.selected_field.saturating_sub(1);
                ChangePasswordAction::None
            }
            KeyCode::Enter => {
                if self.selected_field < 2 {
                    self.selected_field += 1;
                    ChangePasswordAction::None
                } else {
                    // Submit
                    if self.new_buffer != self.confirm_buffer {
                        *self = Self::new();
                        return ChangePasswordAction::Error("Passwords do not match".to_string());
                    }
                    if self.current_buffer.is_empty() || self.new_buffer.is_empty() {
                        *self = Self::new();
                        return ChangePasswordAction::Error("Password cannot be empty".to_string());
                    }
                    let current = std::mem::take(&mut self.current_buffer);
                    let new = std::mem::take(&mut self.new_buffer);
                    self.confirm_buffer.clear();
                    self.saving = true;
                    ChangePasswordAction::Submit(current, new)
                }
            }
            KeyCode::Backspace => {
                match self.selected_field {
                    0 => {
                        self.current_buffer.pop();
                    }
                    1 => {
                        self.new_buffer.pop();
                    }
                    _ => {
                        self.confirm_buffer.pop();
                    }
                }
                ChangePasswordAction::None
            }
            KeyCode::Char(c) => {
                match self.selected_field {
                    0 => self.current_buffer.push(c),
                    1 => self.new_buffer.push(c),
                    _ => self.confirm_buffer.push(c),
                }
                ChangePasswordAction::None
            }
            _ => ChangePasswordAction::None,
        }
    }
}

pub enum ChangePasswordAction {
    None,
    Close,
    Error(String),
    Submit(String, String),
}
