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

    pub fn enter_command_with(&mut self, prefix: &str) {
        self.mode = VimMode::Command;
        self.command_buffer = prefix.to_string();
        self.pending_g = false;
        self.completion.reset(0);
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- CompletionState ---

    #[test]
    fn completion_next_from_none() {
        let mut c = CompletionState::new(5);
        assert!(c.selected.is_none());
        c.next();
        assert_eq!(c.selected, Some(0));
    }

    #[test]
    fn completion_next_wraps() {
        let mut c = CompletionState::new(3);
        c.selected = Some(2);
        c.next();
        assert_eq!(c.selected, Some(0));
    }

    #[test]
    fn completion_next_increments() {
        let mut c = CompletionState::new(5);
        c.selected = Some(1);
        c.next();
        assert_eq!(c.selected, Some(2));
    }

    #[test]
    fn completion_prev_from_none() {
        let mut c = CompletionState::new(3);
        c.prev();
        assert_eq!(c.selected, Some(2));
    }

    #[test]
    fn completion_prev_wraps() {
        let mut c = CompletionState::new(3);
        c.selected = Some(0);
        c.prev();
        assert_eq!(c.selected, Some(2));
    }

    #[test]
    fn completion_prev_decrements() {
        let mut c = CompletionState::new(5);
        c.selected = Some(3);
        c.prev();
        assert_eq!(c.selected, Some(2));
    }

    #[test]
    fn completion_next_empty_match_count() {
        let mut c = CompletionState::new(0);
        c.next();
        assert!(c.selected.is_none());
    }

    #[test]
    fn completion_prev_empty_match_count() {
        let mut c = CompletionState::new(0);
        c.prev();
        assert!(c.selected.is_none());
    }

    #[test]
    fn completion_reset() {
        let mut c = CompletionState::new(5);
        c.selected = Some(3);
        c.reset(10);
        assert!(c.selected.is_none());
        assert_eq!(c.match_count, 10);
    }

    // --- VimState ---

    #[test]
    fn vim_new_defaults() {
        let vim = VimState::new();
        assert_eq!(vim.mode, VimMode::Normal);
        assert_eq!(vim.focus, FocusPanel::RoomList);
        assert!(!vim.pending_g);
        assert!(vim.command_buffer.is_empty());
        assert!(vim.input_buffer.is_empty());
        assert_eq!(vim.input_cursor, 0);
        assert!(vim.search_query.is_empty());
        assert!(!vim.searching);
    }

    #[test]
    fn enter_insert_sets_mode() {
        let mut vim = VimState::new();
        vim.pending_g = true;
        vim.enter_insert();
        assert_eq!(vim.mode, VimMode::Insert);
        assert!(!vim.pending_g);
    }

    #[test]
    fn enter_normal_clears_state() {
        let mut vim = VimState::new();
        vim.enter_insert();
        vim.searching = true;
        vim.pending_g = true;
        vim.enter_normal();
        assert_eq!(vim.mode, VimMode::Normal);
        assert!(!vim.pending_g);
        assert!(!vim.searching);
    }

    #[test]
    fn enter_command_clears_buffer() {
        let mut vim = VimState::new();
        vim.command_buffer = "old".to_string();
        vim.pending_g = true;
        vim.enter_command();
        assert_eq!(vim.mode, VimMode::Command);
        assert!(vim.command_buffer.is_empty());
        assert!(!vim.pending_g);
    }

    #[test]
    fn enter_command_with_prefills() {
        let mut vim = VimState::new();
        vim.enter_command_with("join ");
        assert_eq!(vim.mode, VimMode::Command);
        assert_eq!(vim.command_buffer, "join ");
    }

    #[test]
    fn insert_char_ascii() {
        let mut vim = VimState::new();
        vim.insert_char('h');
        vim.insert_char('i');
        assert_eq!(vim.input_buffer, "hi");
        assert_eq!(vim.input_cursor, 2);
    }

    #[test]
    fn insert_char_multibyte() {
        let mut vim = VimState::new();
        vim.insert_char('é');
        assert_eq!(vim.input_buffer, "é");
        assert_eq!(vim.input_cursor, 2); // é is 2 bytes in UTF-8
        vim.insert_char('!');
        assert_eq!(vim.input_buffer, "é!");
        assert_eq!(vim.input_cursor, 3);
    }

    #[test]
    fn backspace_removes_char() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.backspace();
        assert_eq!(vim.input_buffer, "a");
        assert_eq!(vim.input_cursor, 1);
    }

    #[test]
    fn backspace_at_start_noop() {
        let mut vim = VimState::new();
        vim.backspace();
        assert!(vim.input_buffer.is_empty());
        assert_eq!(vim.input_cursor, 0);
    }

    #[test]
    fn backspace_multibyte() {
        let mut vim = VimState::new();
        vim.insert_char('日');
        vim.insert_char('本');
        vim.backspace();
        assert_eq!(vim.input_buffer, "日");
        assert_eq!(vim.input_cursor, 3); // 日 is 3 bytes
    }

    #[test]
    fn take_input_returns_and_clears() {
        let mut vim = VimState::new();
        vim.insert_char('h');
        vim.insert_char('i');
        let input = vim.take_input();
        assert_eq!(input, "hi");
        assert!(vim.input_buffer.is_empty());
        assert_eq!(vim.input_cursor, 0);
    }

    #[test]
    fn clear_input() {
        let mut vim = VimState::new();
        vim.insert_char('x');
        vim.clear_input();
        assert!(vim.input_buffer.is_empty());
        assert_eq!(vim.input_cursor, 0);
    }

    #[test]
    fn vim_mode_display() {
        assert_eq!(format!("{}", VimMode::Normal), "NORMAL");
        assert_eq!(format!("{}", VimMode::Insert), "INSERT");
        assert_eq!(format!("{}", VimMode::Command), "COMMAND");
    }
}
