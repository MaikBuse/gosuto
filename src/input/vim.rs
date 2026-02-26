use std::fmt;

use super::command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Command,
}

impl fmt::Display for VimMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VimMode::Normal => write!(f, "NORMAL"),
            VimMode::Insert => write!(f, "INSERT"),
            VimMode::Command => write!(f, "COMMAND"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    RoomList,
    Messages,
    Members,
}

#[derive(Debug)]
pub struct CompletionState {
    pub selected: Option<usize>,
    pub match_count: usize,
}

impl CompletionState {
    pub fn new(match_count: usize) -> Self {
        Self {
            selected: None,
            match_count,
        }
    }

    pub fn next(&mut self) {
        if self.match_count == 0 {
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1) % self.match_count,
            None => 0,
        });
    }

    pub fn prev(&mut self) {
        if self.match_count == 0 {
            return;
        }
        self.selected = Some(match self.selected {
            Some(0) => self.match_count - 1,
            Some(i) => i - 1,
            None => self.match_count.saturating_sub(1),
        });
    }

    pub fn reset(&mut self, match_count: usize) {
        self.selected = None;
        self.match_count = match_count;
    }
}

#[derive(Debug)]
pub struct VimState {
    pub mode: VimMode,
    pub focus: FocusPanel,
    pub pending_g: bool,
    pub command_buffer: String,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub search_query: String,
    pub searching: bool,
    pub completion: CompletionState,
}

impl VimState {
    pub fn new() -> Self {
        Self {
            mode: VimMode::Normal,
            focus: FocusPanel::RoomList,
            pending_g: false,
            command_buffer: String::new(),
            input_buffer: String::new(),
            input_cursor: 0,
            search_query: String::new(),
            searching: false,
            completion: CompletionState::new(command::COMMANDS.len()),
        }
    }

    pub fn enter_insert(&mut self) {
        self.mode = VimMode::Insert;
        self.pending_g = false;
    }

    pub fn enter_normal(&mut self) {
        self.mode = VimMode::Normal;
        self.pending_g = false;
        self.searching = false;
        self.completion.reset(command::COMMANDS.len());
    }

    pub fn enter_command(&mut self) {
        self.mode = VimMode::Command;
        self.command_buffer.clear();
        self.pending_g = false;
        self.completion.reset(command::COMMANDS.len());
    }

    #[allow(dead_code)]
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.input_cursor = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.input_cursor > 0 {
            // Find the previous char boundary
            let prev = self.input_buffer[..self.input_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input_buffer.remove(prev);
            self.input_cursor = prev;
        }
    }

    pub fn take_input(&mut self) -> String {
        self.input_cursor = 0;
        std::mem::take(&mut self.input_buffer)
    }
}
