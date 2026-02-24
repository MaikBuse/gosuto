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
            InputResult::None
        }
        KeyCode::Char(c) => {
            vim.insert_char(c);
            InputResult::None
        }
        _ => InputResult::None,
    }
}
