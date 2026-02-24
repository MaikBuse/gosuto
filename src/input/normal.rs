use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{InputResult, VimState};

pub fn handle_normal(key: KeyEvent, vim: &mut VimState) -> InputResult {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return InputResult::Quit;
    }

    // Handle pending 'g' for 'gg'
    if vim.pending_g {
        vim.pending_g = false;
        if key.code == KeyCode::Char('g') {
            return InputResult::MoveTop;
        }
        // If not 'g', ignore the pending and process normally
    }

    // Handle search mode input
    if vim.searching {
        return handle_search(key, vim);
    }

    match key.code {
        KeyCode::Char('q') => InputResult::Quit,
        KeyCode::Char('j') | KeyCode::Down => InputResult::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => InputResult::MoveUp,
        KeyCode::Char('g') => {
            vim.pending_g = true;
            InputResult::None
        }
        KeyCode::Char('G') => InputResult::MoveBottom,
        KeyCode::Tab => InputResult::SwitchPanel,
        KeyCode::Char('l') => InputResult::FocusRight,
        KeyCode::Char('h') => InputResult::FocusLeft,
        KeyCode::Enter => InputResult::Select,
        KeyCode::Char('i') => {
            vim.enter_insert();
            InputResult::None
        }
        KeyCode::Char(':') => {
            vim.enter_command();
            InputResult::None
        }
        KeyCode::Char('/') => {
            vim.searching = true;
            vim.search_query.clear();
            InputResult::None
        }
        _ => InputResult::None,
    }
}

fn handle_search(key: KeyEvent, vim: &mut VimState) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            vim.searching = false;
            vim.search_query.clear();
            InputResult::ClearSearch
        }
        KeyCode::Enter => {
            vim.searching = false;
            let query = vim.search_query.clone();
            InputResult::Search(query)
        }
        KeyCode::Backspace => {
            vim.search_query.pop();
            InputResult::Search(vim.search_query.clone())
        }
        KeyCode::Char(c) => {
            vim.search_query.push(c);
            InputResult::Search(vim.search_query.clone())
        }
        _ => InputResult::None,
    }
}
