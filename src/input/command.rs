use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{CommandAction, InputResult, VimState};

pub fn handle_command(key: KeyEvent, vim: &mut VimState) -> InputResult {
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
            let cmd = std::mem::take(&mut vim.command_buffer);
            vim.enter_normal();
            parse_command(&cmd)
        }
        KeyCode::Backspace => {
            vim.command_buffer.pop();
            if vim.command_buffer.is_empty() {
                vim.enter_normal();
            }
            InputResult::None
        }
        KeyCode::Char(c) => {
            vim.command_buffer.push(c);
            InputResult::None
        }
        _ => InputResult::None,
    }
}

fn parse_command(input: &str) -> InputResult {
    let input = input.trim();
    let mut parts = input.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let arg = parts.next().unwrap_or("").trim();

    match cmd {
        "q" | "quit" => InputResult::Command(CommandAction::Quit),
        "join" => {
            if arg.is_empty() {
                InputResult::None
            } else {
                InputResult::Command(CommandAction::Join(arg.to_string()))
            }
        }
        "leave" => InputResult::Command(CommandAction::Leave),
        "dm" => {
            if arg.is_empty() {
                InputResult::None
            } else {
                InputResult::Command(CommandAction::DirectMessage(arg.to_string()))
            }
        }
        "logout" => InputResult::Command(CommandAction::Logout),
        "call" => {
            if arg.is_empty() {
                InputResult::None
            } else {
                InputResult::Command(CommandAction::Call(arg.to_string()))
            }
        }
        "answer" | "accept" => InputResult::Command(CommandAction::Answer),
        "reject" | "decline" => InputResult::Command(CommandAction::Reject),
        "hangup" | "end" => InputResult::Command(CommandAction::Hangup),
        "rain" | "matrix" | "effects" => InputResult::Command(CommandAction::Rain),
        _ => InputResult::None,
    }
}
