use crossterm::event::KeyEvent;

use super::command::handle_command;
use super::insert::handle_insert;
use super::normal::handle_normal;
use super::{InputResult, VimMode, VimState};

pub fn handle_key(key: KeyEvent, vim: &mut VimState) -> InputResult {
    match vim.mode {
        VimMode::Normal => handle_normal(key, vim),
        VimMode::Insert => handle_insert(key, vim),
        VimMode::Command => handle_command(key, vim),
    }
}
