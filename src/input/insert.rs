use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{InputResult, VimState};

pub fn handle_insert(key: KeyEvent, vim: &mut VimState) -> InputResult {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return InputResult::Quit;
    }

    match key.code {
        KeyCode::Esc => {
            vim.enter_normal();
            InputResult::None
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
            vim.insert_char('\n');
            InputResult::TypingActivity
        }
        KeyCode::Enter => {
            let msg = vim.take_input();
            if msg.is_empty() {
                InputResult::None
            } else {
                InputResult::SendMessage(msg)
            }
        }
        KeyCode::Backspace => {
            vim.backspace();
            InputResult::TypingActivity
        }
        KeyCode::Char(c) => {
            vim.insert_char(c);
            InputResult::TypingActivity
        }
        _ => InputResult::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::vim::VimMode;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn esc_returns_to_normal() {
        let mut vim = VimState::new();
        vim.enter_insert();
        let result = handle_insert(key(KeyCode::Esc), &mut vim);
        assert!(matches!(result, InputResult::None));
        assert_eq!(vim.mode, VimMode::Normal);
    }

    #[test]
    fn enter_sends_message() {
        let mut vim = VimState::new();
        vim.enter_insert();
        vim.insert_char('h');
        vim.insert_char('i');
        let result = handle_insert(key(KeyCode::Enter), &mut vim);
        assert!(matches!(result, InputResult::SendMessage(ref s) if s == "hi"));
    }

    #[test]
    fn enter_empty_returns_none() {
        let mut vim = VimState::new();
        vim.enter_insert();
        let result = handle_insert(key(KeyCode::Enter), &mut vim);
        assert!(matches!(result, InputResult::None));
    }

    #[test]
    fn char_inserts() {
        let mut vim = VimState::new();
        vim.enter_insert();
        let result = handle_insert(key(KeyCode::Char('x')), &mut vim);
        assert!(matches!(result, InputResult::TypingActivity));
        assert_eq!(vim.input_buffer, "x");
    }

    #[test]
    fn backspace_delegates() {
        let mut vim = VimState::new();
        vim.enter_insert();
        vim.insert_char('a');
        vim.insert_char('b');
        let result = handle_insert(key(KeyCode::Backspace), &mut vim);
        assert!(matches!(result, InputResult::TypingActivity));
        assert_eq!(vim.input_buffer, "a");
    }

    #[test]
    fn ctrl_c_quits() {
        let mut vim = VimState::new();
        vim.enter_insert();
        let result = handle_insert(ctrl('c'), &mut vim);
        assert!(matches!(result, InputResult::Quit));
    }

    fn alt_enter() -> KeyEvent {
        KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)
    }

    #[test]
    fn alt_enter_inserts_newline() {
        let mut vim = VimState::new();
        vim.enter_insert();
        vim.insert_char('h');
        vim.insert_char('i');
        let result = handle_insert(alt_enter(), &mut vim);
        assert!(matches!(result, InputResult::TypingActivity));
        assert_eq!(vim.input_buffer, "hi\n");
    }

    #[test]
    fn alt_enter_does_not_send() {
        let mut vim = VimState::new();
        vim.enter_insert();
        vim.insert_char('a');
        let result = handle_insert(alt_enter(), &mut vim);
        assert!(matches!(result, InputResult::TypingActivity));
        // Buffer should still contain text (not sent)
        assert_eq!(vim.input_buffer, "a\n");
    }

    #[test]
    fn enter_sends_multiline_message() {
        let mut vim = VimState::new();
        vim.enter_insert();
        vim.insert_char('a');
        handle_insert(alt_enter(), &mut vim);
        vim.insert_char('b');
        let result = handle_insert(key(KeyCode::Enter), &mut vim);
        assert!(matches!(result, InputResult::SendMessage(ref s) if s == "a\nb"));
    }
}
