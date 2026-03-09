use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{FocusPanel, InputResult, VimState};

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
        KeyCode::Char('v') if vim.focus == FocusPanel::Members => InputResult::VerifyMember,
        KeyCode::Char('c') => InputResult::CallMember,
        KeyCode::Char('a') => {
            if vim.focus == FocusPanel::Messages {
                InputResult::ReactToSelected
            } else {
                InputResult::AnswerCall
            }
        }
        KeyCode::Char('r') => {
            if vim.focus == FocusPanel::Messages {
                InputResult::ReplyToSelected
            } else {
                InputResult::RejectCall
            }
        }
        KeyCode::Tab => InputResult::SwitchPanel,
        KeyCode::Char('l') => InputResult::FocusRight,
        KeyCode::Char('h') => InputResult::FocusLeft,
        KeyCode::Enter => InputResult::Select,
        KeyCode::Char('i') => {
            vim.enter_insert();
            InputResult::None
        }
        KeyCode::Char(' ') => InputResult::ShowWhichKey,
        KeyCode::Char(':') => {
            vim.enter_command();
            InputResult::None
        }
        KeyCode::Char('/') => {
            vim.searching = true;
            vim.search_query.clear();
            InputResult::None
        }
        KeyCode::Esc => InputResult::Escape,
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
    fn j_moves_down() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('j')), &mut vim);
        assert!(matches!(result, InputResult::MoveDown));
    }

    #[test]
    fn down_arrow_moves_down() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Down), &mut vim);
        assert!(matches!(result, InputResult::MoveDown));
    }

    #[test]
    fn k_moves_up() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('k')), &mut vim);
        assert!(matches!(result, InputResult::MoveUp));
    }

    #[test]
    fn up_arrow_moves_up() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Up), &mut vim);
        assert!(matches!(result, InputResult::MoveUp));
    }

    #[test]
    fn big_g_moves_bottom() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('G')), &mut vim);
        assert!(matches!(result, InputResult::MoveBottom));
    }

    #[test]
    fn gg_moves_top() {
        let mut vim = VimState::new();
        let r1 = handle_normal(key(KeyCode::Char('g')), &mut vim);
        assert!(matches!(r1, InputResult::None));
        assert!(vim.pending_g);
        let r2 = handle_normal(key(KeyCode::Char('g')), &mut vim);
        assert!(matches!(r2, InputResult::MoveTop));
        assert!(!vim.pending_g);
    }

    #[test]
    fn g_followed_by_non_g_cancels() {
        let mut vim = VimState::new();
        handle_normal(key(KeyCode::Char('g')), &mut vim);
        let result = handle_normal(key(KeyCode::Char('j')), &mut vim);
        // After pending_g is cleared, j should produce MoveDown
        assert!(matches!(result, InputResult::MoveDown));
        assert!(!vim.pending_g);
    }

    #[test]
    fn q_quits() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('q')), &mut vim);
        assert!(matches!(result, InputResult::Quit));
    }

    #[test]
    fn ctrl_c_quits() {
        let mut vim = VimState::new();
        let result = handle_normal(ctrl('c'), &mut vim);
        assert!(matches!(result, InputResult::Quit));
    }

    #[test]
    fn tab_switches_panel() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Tab), &mut vim);
        assert!(matches!(result, InputResult::SwitchPanel));
    }

    #[test]
    fn h_focus_left() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('h')), &mut vim);
        assert!(matches!(result, InputResult::FocusLeft));
    }

    #[test]
    fn l_focus_right() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('l')), &mut vim);
        assert!(matches!(result, InputResult::FocusRight));
    }

    #[test]
    fn enter_selects() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Enter), &mut vim);
        assert!(matches!(result, InputResult::Select));
    }

    #[test]
    fn i_enters_insert() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('i')), &mut vim);
        assert!(matches!(result, InputResult::None));
        assert_eq!(vim.mode, VimMode::Insert);
    }

    #[test]
    fn colon_enters_command() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char(':')), &mut vim);
        assert!(matches!(result, InputResult::None));
        assert_eq!(vim.mode, VimMode::Command);
    }

    #[test]
    fn slash_enters_search() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('/')), &mut vim);
        assert!(matches!(result, InputResult::None));
        assert!(vim.searching);
    }

    #[test]
    fn space_shows_which_key() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char(' ')), &mut vim);
        assert!(matches!(result, InputResult::ShowWhichKey));
    }

    #[test]
    fn c_calls_member() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Char('c')), &mut vim);
        assert!(matches!(result, InputResult::CallMember));
    }

    #[test]
    fn a_answers_call() {
        let mut vim = VimState::new();
        vim.focus = FocusPanel::RoomList;
        let result = handle_normal(key(KeyCode::Char('a')), &mut vim);
        assert!(matches!(result, InputResult::AnswerCall));
    }

    #[test]
    fn a_reacts_when_messages_focused() {
        let mut vim = VimState::new();
        vim.focus = FocusPanel::Messages;
        let result = handle_normal(key(KeyCode::Char('a')), &mut vim);
        assert!(matches!(result, InputResult::ReactToSelected));
    }

    #[test]
    fn r_rejects_call() {
        let mut vim = VimState::new();
        vim.focus = FocusPanel::RoomList;
        let result = handle_normal(key(KeyCode::Char('r')), &mut vim);
        assert!(matches!(result, InputResult::RejectCall));
    }

    #[test]
    fn r_replies_when_messages_focused() {
        let mut vim = VimState::new();
        vim.focus = FocusPanel::Messages;
        let result = handle_normal(key(KeyCode::Char('r')), &mut vim);
        assert!(matches!(result, InputResult::ReplyToSelected));
    }

    #[test]
    fn esc_returns_escape() {
        let mut vim = VimState::new();
        let result = handle_normal(key(KeyCode::Esc), &mut vim);
        assert!(matches!(result, InputResult::Escape));
    }

    // --- Search mode ---

    #[test]
    fn search_char_appends() {
        let mut vim = VimState::new();
        vim.searching = true;
        let result = handle_normal(key(KeyCode::Char('a')), &mut vim);
        assert!(matches!(result, InputResult::Search(ref q) if q == "a"));
        assert_eq!(vim.search_query, "a");
    }

    #[test]
    fn search_backspace_pops() {
        let mut vim = VimState::new();
        vim.searching = true;
        vim.search_query = "abc".to_string();
        let result = handle_normal(key(KeyCode::Backspace), &mut vim);
        assert!(matches!(result, InputResult::Search(ref q) if q == "ab"));
    }

    #[test]
    fn search_enter_confirms() {
        let mut vim = VimState::new();
        vim.searching = true;
        vim.search_query = "test".to_string();
        let result = handle_normal(key(KeyCode::Enter), &mut vim);
        assert!(matches!(result, InputResult::Search(ref q) if q == "test"));
        assert!(!vim.searching);
    }

    #[test]
    fn search_esc_cancels() {
        let mut vim = VimState::new();
        vim.searching = true;
        vim.search_query = "test".to_string();
        let result = handle_normal(key(KeyCode::Esc), &mut vim);
        assert!(matches!(result, InputResult::ClearSearch));
        assert!(!vim.searching);
        assert!(vim.search_query.is_empty());
    }
}
