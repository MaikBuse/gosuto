use crossterm::event::{KeyCode, KeyEvent};

use super::HISTORY_VISIBILITY_OPTIONS;

pub struct CreateRoomState {
    pub open: bool,
    pub selected_field: usize, // 0=name, 1=topic, 2=history, 3=encrypted, 4=create button
    pub name_buffer: String,
    pub editing_name: bool,
    pub topic_buffer: String,
    pub editing_topic: bool,
    pub history_visibility: String,
    pub encrypted: String, // "yes" (default) or "no"
    pub creating: bool,
}

impl CreateRoomState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_field: 0,
            name_buffer: String::new(),
            editing_name: false,
            topic_buffer: String::new(),
            editing_topic: false,
            history_visibility: "shared".to_string(),
            encrypted: "yes".to_string(),
            creating: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> CreateRoomAction {
        if self.creating {
            if key.code == KeyCode::Esc {
                self.open = false;
                self.creating = false;
            }
            return CreateRoomAction::None;
        }

        // Inline name editing mode
        if self.editing_name {
            match key.code {
                KeyCode::Esc => {
                    self.editing_name = false;
                    self.name_buffer.clear();
                }
                KeyCode::Enter => {
                    self.editing_name = false;
                }
                KeyCode::Backspace => {
                    self.name_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.name_buffer.push(c);
                }
                _ => {}
            }
            return CreateRoomAction::None;
        }

        // Inline topic editing mode
        if self.editing_topic {
            match key.code {
                KeyCode::Esc => {
                    self.editing_topic = false;
                    self.topic_buffer.clear();
                }
                KeyCode::Enter => {
                    self.editing_topic = false;
                }
                KeyCode::Backspace => {
                    self.topic_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.topic_buffer.push(c);
                }
                _ => {}
            }
            return CreateRoomAction::None;
        }

        match key.code {
            KeyCode::Esc => CreateRoomAction::Close,
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected_field < 4 {
                    self.selected_field += 1;
                }
                CreateRoomAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_field > 0 {
                    self.selected_field -= 1;
                }
                CreateRoomAction::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cycle_field(-1);
                CreateRoomAction::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cycle_field(1);
                CreateRoomAction::None
            }
            KeyCode::Enter => match self.selected_field {
                0 => {
                    self.editing_name = true;
                    CreateRoomAction::None
                }
                1 => {
                    self.editing_topic = true;
                    CreateRoomAction::None
                }
                2 | 3 => CreateRoomAction::None,
                4 => {
                    if self.name_buffer.trim().is_empty() {
                        return CreateRoomAction::Error("Room name is required".to_string());
                    }
                    let topic = if self.topic_buffer.trim().is_empty() {
                        None
                    } else {
                        Some(self.topic_buffer.clone())
                    };
                    self.creating = true;
                    CreateRoomAction::Create(CreateRoomParams {
                        name: self.name_buffer.clone(),
                        topic,
                        history_visibility: self.history_visibility.clone(),
                        encrypted: self.encrypted == "yes",
                    })
                }
                _ => CreateRoomAction::None,
            },
            _ => CreateRoomAction::None,
        }
    }

    fn cycle_field(&mut self, dir: i32) {
        if self.selected_field == 2 {
            let opts = HISTORY_VISIBILITY_OPTIONS;
            let current_idx = opts
                .iter()
                .position(|&v| v == self.history_visibility)
                .unwrap_or(0);
            let len = opts.len();
            let new_idx = if dir > 0 {
                (current_idx + 1) % len
            } else {
                (current_idx + len - 1) % len
            };
            self.history_visibility = opts[new_idx].to_string();
        } else if self.selected_field == 3 {
            self.encrypted = if self.encrypted == "no" {
                "yes".to_string()
            } else {
                "no".to_string()
            };
        }
    }
}

pub struct CreateRoomParams {
    pub name: String,
    pub topic: Option<String>,
    pub history_visibility: String,
    pub encrypted: bool,
}

pub enum CreateRoomAction {
    None,
    Close,
    Error(String),
    Create(CreateRoomParams),
}
