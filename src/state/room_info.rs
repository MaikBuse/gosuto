use crossterm::event::{KeyCode, KeyEvent};

use super::HISTORY_VISIBILITY_OPTIONS;

pub struct RoomInfoState {
    pub open: bool,
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub history_visibility: String,
    pub encrypted: bool,
    pub encryption_selection: String,
    pub selected_field: usize,
    pub loading: bool,
    pub saving: bool,
    pub editing_name: bool,
    pub name_buffer: String,
    pub editing_topic: bool,
    pub topic_buffer: String,
    pub topic_save_pending: bool,
}

impl RoomInfoState {
    pub fn new() -> Self {
        Self {
            open: false,
            room_id: String::new(),
            name: None,
            topic: None,
            history_visibility: "shared".to_string(),
            encrypted: false,
            encryption_selection: "no".to_string(),
            selected_field: 0,
            loading: false,
            saving: false,
            editing_name: false,
            name_buffer: String::new(),
            editing_topic: false,
            topic_buffer: String::new(),
            topic_save_pending: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> RoomInfoAction {
        if self.loading || self.saving {
            if key.code == KeyCode::Esc {
                return RoomInfoAction::Close;
            }
            return RoomInfoAction::None;
        }

        // Inline name editing mode
        if self.editing_name {
            match key.code {
                KeyCode::Esc => {
                    self.editing_name = false;
                    self.name_buffer.clear();
                }
                KeyCode::Enter => {
                    let new_name = self.name_buffer.clone();
                    if !new_name.is_empty() {
                        let room_id = self.room_id.clone();
                        self.saving = true;
                        self.editing_name = false;
                        return RoomInfoAction::SetName(room_id, new_name);
                    }
                    self.editing_name = false;
                    self.name_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.name_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.name_buffer.push(c);
                }
                _ => {}
            }
            return RoomInfoAction::None;
        }

        // Inline topic editing mode
        if self.editing_topic {
            match key.code {
                KeyCode::Esc => {
                    self.editing_topic = false;
                    self.topic_buffer.clear();
                }
                KeyCode::Enter => {
                    let new_topic = self.topic_buffer.clone();
                    let room_id = self.room_id.clone();
                    self.saving = true;
                    self.editing_topic = false;
                    self.topic_save_pending = true;
                    return RoomInfoAction::SetTopic(room_id, new_topic);
                }
                KeyCode::Backspace => {
                    self.topic_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.topic_buffer.push(c);
                }
                _ => {}
            }
            return RoomInfoAction::None;
        }

        match key.code {
            KeyCode::Esc => RoomInfoAction::Close,
            KeyCode::Char('j') | KeyCode::Down => {
                let max_field = if self.encrypted { 2 } else { 3 };
                if self.selected_field < max_field {
                    self.selected_field += 1;
                }
                RoomInfoAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_field > 0 {
                    self.selected_field -= 1;
                }
                RoomInfoAction::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cycle_field(-1);
                RoomInfoAction::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cycle_field(1);
                RoomInfoAction::None
            }
            KeyCode::Enter => match self.selected_field {
                0 => {
                    // Enter name editing mode
                    self.editing_name = true;
                    self.name_buffer = self.name.clone().unwrap_or_default();
                    RoomInfoAction::None
                }
                1 => {
                    // Enter topic editing mode
                    self.editing_topic = true;
                    self.topic_buffer = self.topic.clone().unwrap_or_default();
                    RoomInfoAction::None
                }
                2 => {
                    // Save current history visibility
                    let room_id = self.room_id.clone();
                    let vis = self.history_visibility.clone();
                    self.saving = true;
                    RoomInfoAction::SetVisibility(room_id, vis)
                }
                3 => {
                    // Enable encryption (only reachable when not already encrypted)
                    if self.encryption_selection == "yes" {
                        let room_id = self.room_id.clone();
                        self.saving = true;
                        RoomInfoAction::EnableEncryption(room_id)
                    } else {
                        RoomInfoAction::None
                    }
                }
                _ => RoomInfoAction::None,
            },
            _ => RoomInfoAction::None,
        }
    }

    fn cycle_field(&mut self, dir: i32) {
        if self.selected_field == 2 {
            // Cycle history visibility
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
            // Toggle encryption selection between "no" and "yes"
            let _ = dir; // direction doesn't matter for a binary toggle
            self.encryption_selection = if self.encryption_selection == "no" {
                "yes".to_string()
            } else {
                "no".to_string()
            };
        }
    }
}

pub enum RoomInfoAction {
    None,
    Close,
    SetName(String, String),
    SetTopic(String, String),
    SetVisibility(String, String),
    EnableEncryption(String),
}
