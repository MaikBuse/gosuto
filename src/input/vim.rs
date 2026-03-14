use std::fmt;

use unicode_width::UnicodeWidthChar;

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

    #[cfg(test)]
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

    pub fn move_left(&mut self) {
        if self.input_cursor > 0 {
            let prev = self.input_buffer[..self.input_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input_cursor = prev;
        }
    }

    pub fn move_right(&mut self) {
        if self.input_cursor < self.input_buffer.len() {
            let c = self.input_buffer[self.input_cursor..]
                .chars()
                .next()
                .unwrap();
            self.input_cursor += c.len_utf8();
        }
    }

    pub fn move_up(&mut self) {
        let (row, col) = self.cursor_row_col();
        if row == 0 {
            return;
        }
        let before_cursor = &self.input_buffer[..self.input_cursor];
        // Find the newline ending the previous line
        let cur_line_start = before_cursor.rfind('\n').unwrap();
        let prev_line = &self.input_buffer[..cur_line_start];
        let prev_line_start = match prev_line.rfind('\n') {
            Some(pos) => pos + 1,
            None => 0,
        };
        let prev_line_len = cur_line_start - prev_line_start;
        // Walk chars to find the correct byte position, clamping to line length
        let target_col = col.min(prev_line_len);
        let mut byte_pos = prev_line_start;
        let mut bytes_counted = 0;
        for c in self.input_buffer[prev_line_start..cur_line_start].chars() {
            if bytes_counted >= target_col {
                break;
            }
            let clen = c.len_utf8();
            if bytes_counted + clen > target_col {
                break;
            }
            byte_pos += clen;
            bytes_counted += clen;
        }
        self.input_cursor = byte_pos;
    }

    pub fn move_down(&mut self) {
        let (_, col) = self.cursor_row_col();
        let after_cursor = &self.input_buffer[self.input_cursor..];
        // Find the newline ending the current line
        let newline_offset = match after_cursor.find('\n') {
            Some(offset) => offset,
            None => return, // on last line
        };
        let next_line_start = self.input_cursor + newline_offset + 1;
        let next_line_end = match self.input_buffer[next_line_start..].find('\n') {
            Some(offset) => next_line_start + offset,
            None => self.input_buffer.len(),
        };
        let next_line_len = next_line_end - next_line_start;
        let target_col = col.min(next_line_len);
        let mut byte_pos = next_line_start;
        let mut bytes_counted = 0;
        for c in self.input_buffer[next_line_start..next_line_end].chars() {
            if bytes_counted >= target_col {
                break;
            }
            let clen = c.len_utf8();
            if bytes_counted + clen > target_col {
                break;
            }
            byte_pos += clen;
            bytes_counted += clen;
        }
        self.input_cursor = byte_pos;
    }

    pub fn move_line_start(&mut self) {
        let before_cursor = &self.input_buffer[..self.input_cursor];
        self.input_cursor = match before_cursor.rfind('\n') {
            Some(pos) => pos + 1,
            None => 0,
        };
    }

    pub fn move_line_end(&mut self) {
        let after_cursor = &self.input_buffer[self.input_cursor..];
        self.input_cursor = match after_cursor.find('\n') {
            Some(offset) => self.input_cursor + offset,
            None => self.input_buffer.len(),
        };
    }

    pub fn delete_char(&mut self) {
        if self.input_cursor < self.input_buffer.len() {
            self.input_buffer.remove(self.input_cursor);
        }
    }

    pub fn take_input(&mut self) -> String {
        self.input_cursor = 0;
        std::mem::take(&mut self.input_buffer)
    }

    #[cfg(test)]
    pub fn input_line_count(&self) -> usize {
        self.input_buffer.split('\n').count().max(1)
    }

    pub fn cursor_row_col(&self) -> (usize, usize) {
        let before_cursor = &self.input_buffer[..self.input_cursor];
        let row = before_cursor.matches('\n').count();
        let col = match before_cursor.rfind('\n') {
            Some(pos) => self.input_cursor - pos - 1,
            None => self.input_cursor,
        };
        (row, col)
    }

    /// Returns `(total_visual_lines, cursor_visual_row, cursor_visual_col)`
    /// by simulating ratatui's character-level wrapping with `Wrap { trim: false }`.
    pub fn visual_cursor_info(&self, text_width: u16) -> (usize, u16, u16) {
        let text_width = text_width.max(1) as usize;
        let mut vis_row: usize = 0;
        let mut vis_col: usize = 0;
        let mut cursor_row: u16 = 0;
        let mut cursor_col: u16 = 0;
        let mut byte_pos: usize = 0;
        let mut found_cursor = false;

        for c in self.input_buffer.chars() {
            if byte_pos == self.input_cursor && !found_cursor {
                cursor_row = vis_row as u16;
                cursor_col = vis_col as u16;
                found_cursor = true;
            }

            if c == '\n' {
                vis_row += 1;
                vis_col = 0;
            } else {
                let w = c.width().unwrap_or(0);
                if vis_col + w > text_width {
                    vis_row += 1;
                    vis_col = 0;
                }
                vis_col += w;
            }

            byte_pos += c.len_utf8();
        }

        if !found_cursor {
            cursor_row = vis_row as u16;
            cursor_col = vis_col as u16;
        }

        (vis_row + 1, cursor_row, cursor_col)
    }

    pub fn visual_line_count(&self, text_width: u16) -> usize {
        self.visual_cursor_info(text_width).0
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

    // --- input_line_count ---

    #[test]
    fn input_line_count_empty() {
        let vim = VimState::new();
        assert_eq!(vim.input_line_count(), 1);
    }

    #[test]
    fn input_line_count_single_line() {
        let mut vim = VimState::new();
        vim.insert_char('h');
        vim.insert_char('i');
        assert_eq!(vim.input_line_count(), 1);
    }

    #[test]
    fn input_line_count_multi_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('\n');
        vim.insert_char('b');
        vim.insert_char('\n');
        vim.insert_char('c');
        assert_eq!(vim.input_line_count(), 3);
    }

    #[test]
    fn input_line_count_trailing_newline() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('\n');
        assert_eq!(vim.input_line_count(), 2);
    }

    // --- cursor_row_col ---

    #[test]
    fn cursor_row_col_empty() {
        let vim = VimState::new();
        assert_eq!(vim.cursor_row_col(), (0, 0));
    }

    #[test]
    fn cursor_row_col_single_line() {
        let mut vim = VimState::new();
        vim.insert_char('h');
        vim.insert_char('i');
        assert_eq!(vim.cursor_row_col(), (0, 2));
    }

    #[test]
    fn cursor_row_col_after_newline() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('\n');
        assert_eq!(vim.cursor_row_col(), (1, 0));
    }

    #[test]
    fn cursor_row_col_second_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('\n');
        vim.insert_char('c');
        vim.insert_char('d');
        vim.insert_char('e');
        assert_eq!(vim.cursor_row_col(), (1, 3));
    }

    #[test]
    fn cursor_row_col_third_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('\n');
        vim.insert_char('b');
        vim.insert_char('\n');
        vim.insert_char('c');
        assert_eq!(vim.cursor_row_col(), (2, 1));
    }

    // --- move_left ---

    #[test]
    fn move_left_basic() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.move_left();
        assert_eq!(vim.input_cursor, 1);
    }

    #[test]
    fn move_left_at_start_noop() {
        let mut vim = VimState::new();
        vim.move_left();
        assert_eq!(vim.input_cursor, 0);
    }

    #[test]
    fn move_left_multibyte() {
        let mut vim = VimState::new();
        vim.insert_char('é'); // 2 bytes
        vim.insert_char('!');
        vim.move_left();
        assert_eq!(vim.input_cursor, 2); // at start of '!'
        vim.move_left();
        assert_eq!(vim.input_cursor, 0); // at start of 'é'
    }

    // --- move_right ---

    #[test]
    fn move_right_basic() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.input_cursor = 0;
        vim.move_right();
        assert_eq!(vim.input_cursor, 1);
    }

    #[test]
    fn move_right_at_end_noop() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.move_right();
        assert_eq!(vim.input_cursor, 1);
    }

    #[test]
    fn move_right_multibyte() {
        let mut vim = VimState::new();
        vim.insert_char('日'); // 3 bytes
        vim.insert_char('本'); // 3 bytes
        vim.input_cursor = 0;
        vim.move_right();
        assert_eq!(vim.input_cursor, 3);
        vim.move_right();
        assert_eq!(vim.input_cursor, 6);
    }

    // --- move_up ---

    #[test]
    fn move_up_basic() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('\n');
        vim.insert_char('c');
        vim.insert_char('d');
        // cursor at (1, 2), move up to (0, 2)
        vim.move_up();
        assert_eq!(vim.input_cursor, 2);
        assert_eq!(vim.cursor_row_col(), (0, 2));
    }

    #[test]
    fn move_up_clamps_to_shorter_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('\n');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.insert_char('d');
        // cursor at (1, 3), line 0 has len 1
        vim.move_up();
        assert_eq!(vim.input_cursor, 1);
        assert_eq!(vim.cursor_row_col(), (0, 1));
    }

    #[test]
    fn move_up_first_line_noop() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.move_up();
        assert_eq!(vim.input_cursor, 2);
    }

    #[test]
    fn move_up_multibyte() {
        let mut vim = VimState::new();
        // line 0: "日本" (6 bytes)
        vim.insert_char('日');
        vim.insert_char('本');
        vim.insert_char('\n');
        // line 1: "abc" cursor at col 3
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.move_up();
        // col 3 bytes into "日本" = after '日' (3 bytes)
        assert_eq!(vim.input_cursor, 3);
    }

    // --- move_down ---

    #[test]
    fn move_down_basic() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('\n');
        vim.insert_char('c');
        vim.insert_char('d');
        // Move cursor to line 0
        vim.input_cursor = 2; // (0, 2)
        vim.move_down();
        assert_eq!(vim.cursor_row_col(), (1, 2));
    }

    #[test]
    fn move_down_clamps_to_shorter_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.insert_char('\n');
        vim.insert_char('d');
        // cursor at (0, 3), line 1 has len 1
        vim.input_cursor = 3;
        vim.move_down();
        assert_eq!(vim.input_cursor, 5); // 'd' end
        assert_eq!(vim.cursor_row_col(), (1, 1));
    }

    #[test]
    fn move_down_last_line_noop() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.move_down();
        assert_eq!(vim.input_cursor, 1);
    }

    #[test]
    fn move_down_multibyte() {
        let mut vim = VimState::new();
        // line 0: "abc"
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.insert_char('\n');
        // line 1: "日本" (6 bytes)
        vim.insert_char('日');
        vim.insert_char('本');
        vim.input_cursor = 3; // (0, 3) at end of "abc"
        vim.move_down();
        // col 3 bytes into "日本" = after '日' (3 bytes), so byte 4+3=7
        assert_eq!(vim.input_cursor, 7);
    }

    // --- move_line_start ---

    #[test]
    fn move_line_start_single_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.move_line_start();
        assert_eq!(vim.input_cursor, 0);
    }

    #[test]
    fn move_line_start_middle_line() {
        let mut vim = VimState::new();
        // "a\nbc\nd" — line 0: "a", line 1: "bc", line 2: "d"
        vim.insert_char('a');
        vim.insert_char('\n');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.insert_char('\n');
        vim.insert_char('d');
        // cursor at end of line 2 (pos 6), move to start of line 2
        vim.move_line_start();
        assert_eq!(vim.input_cursor, 5); // start of line 2 (after second '\n')
    }

    // --- move_line_end ---

    #[test]
    fn move_line_end_single_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.input_cursor = 0;
        vim.move_line_end();
        assert_eq!(vim.input_cursor, 3);
    }

    #[test]
    fn move_line_end_middle_line() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('\n');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.insert_char('\n');
        vim.insert_char('d');
        vim.input_cursor = 2; // start of line 1
        vim.move_line_end();
        assert_eq!(vim.input_cursor, 4); // before '\n' on line 1
    }

    // --- delete_char ---

    #[test]
    fn delete_char_basic() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.insert_char('b');
        vim.insert_char('c');
        vim.input_cursor = 1;
        vim.delete_char();
        assert_eq!(vim.input_buffer, "ac");
        assert_eq!(vim.input_cursor, 1);
    }

    #[test]
    fn delete_char_at_end_noop() {
        let mut vim = VimState::new();
        vim.insert_char('a');
        vim.delete_char();
        assert_eq!(vim.input_buffer, "a");
        assert_eq!(vim.input_cursor, 1);
    }

    #[test]
    fn delete_char_multibyte() {
        let mut vim = VimState::new();
        vim.insert_char('日');
        vim.insert_char('本');
        vim.input_cursor = 0;
        vim.delete_char();
        assert_eq!(vim.input_buffer, "本");
        assert_eq!(vim.input_cursor, 0);
    }

    // --- visual_cursor_info ---

    #[test]
    fn visual_cursor_empty_buffer() {
        let vim = VimState::new();
        let (lines, row, col) = vim.visual_cursor_info(20);
        assert_eq!(lines, 1);
        assert_eq!(row, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn visual_cursor_short_text() {
        let mut vim = VimState::new();
        for c in "hello".chars() {
            vim.insert_char(c);
        }
        let (lines, row, col) = vim.visual_cursor_info(20);
        assert_eq!(lines, 1);
        assert_eq!(row, 0);
        assert_eq!(col, 5);
    }

    #[test]
    fn visual_cursor_wraps_at_boundary() {
        let mut vim = VimState::new();
        // 10 chars, width 5 => 2 visual lines
        for c in "abcdefghij".chars() {
            vim.insert_char(c);
        }
        let (lines, row, col) = vim.visual_cursor_info(5);
        assert_eq!(lines, 2);
        assert_eq!(row, 1);
        assert_eq!(col, 5);
    }

    #[test]
    fn visual_cursor_newline_and_wrap() {
        let mut vim = VimState::new();
        // "ab\ncdefgh" with width 4
        // line 0: "ab" (1 visual line)
        // line 1: "cdef" wraps to "cdef" | "gh" (2 visual lines)
        // total = 3 visual lines
        for c in "ab\ncdefgh".chars() {
            vim.insert_char(c);
        }
        let (lines, row, col) = vim.visual_cursor_info(4);
        assert_eq!(lines, 3);
        assert_eq!(row, 2);
        assert_eq!(col, 2);
    }

    #[test]
    fn visual_cursor_cjk_double_width() {
        let mut vim = VimState::new();
        // Each CJK char is 2 cells wide. With width 5, "日本語" = 6 cells
        // "日本" = 4 cells fits, "語" = 2 more would be 6 > 5, wraps
        for c in "日本語".chars() {
            vim.insert_char(c);
        }
        let (lines, row, col) = vim.visual_cursor_info(5);
        assert_eq!(lines, 2);
        assert_eq!(row, 1);
        assert_eq!(col, 2);
    }

    #[test]
    fn visual_cursor_cursor_in_middle() {
        let mut vim = VimState::new();
        // "abcdefgh" with width 4, cursor at byte 3 (after "abc")
        for c in "abcdefgh".chars() {
            vim.insert_char(c);
        }
        vim.input_cursor = 3;
        let (_lines, row, col) = vim.visual_cursor_info(4);
        assert_eq!(row, 0);
        assert_eq!(col, 3);
    }

    #[test]
    fn visual_line_count_matches_info() {
        let mut vim = VimState::new();
        for c in "abcdefghij".chars() {
            vim.insert_char(c);
        }
        assert_eq!(vim.visual_line_count(5), 2);
        assert_eq!(vim.visual_line_count(10), 1);
        assert_eq!(vim.visual_line_count(3), 4);
    }
}
