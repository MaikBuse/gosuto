use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{CommandAction, InputResult, VimState};

pub struct CommandDef {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub syntax: &'static str,
    pub description: &'static str,
    pub takes_arg: bool,
}

pub const COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "quit",
        aliases: &["q"],
        syntax: ":quit",
        description: "Exit walrust",
        takes_arg: false,
    },
    CommandDef {
        name: "join",
        aliases: &[],
        syntax: ":join <room>",
        description: "Join a room",
        takes_arg: true,
    },
    CommandDef {
        name: "leave",
        aliases: &[],
        syntax: ":leave",
        description: "Leave current room",
        takes_arg: false,
    },
    CommandDef {
        name: "dm",
        aliases: &[],
        syntax: ":dm <user>",
        description: "Direct message a user",
        takes_arg: true,
    },
    CommandDef {
        name: "logout",
        aliases: &[],
        syntax: ":logout",
        description: "Log out of session",
        takes_arg: false,
    },
    CommandDef {
        name: "call",
        aliases: &[],
        syntax: ":call <user>",
        description: "Start a VoIP call",
        takes_arg: true,
    },
    CommandDef {
        name: "answer",
        aliases: &["accept"],
        syntax: ":answer",
        description: "Answer incoming call",
        takes_arg: false,
    },
    CommandDef {
        name: "reject",
        aliases: &["decline"],
        syntax: ":reject",
        description: "Reject incoming call",
        takes_arg: false,
    },
    CommandDef {
        name: "hangup",
        aliases: &["end"],
        syntax: ":hangup",
        description: "End active call",
        takes_arg: false,
    },
    CommandDef {
        name: "rain",
        aliases: &["matrix", "effects"],
        syntax: ":rain",
        description: "Toggle matrix rain",
        takes_arg: false,
    },
    CommandDef {
        name: "glitch",
        aliases: &[],
        syntax: ":glitch",
        description: "Toggle glitch effect",
        takes_arg: false,
    },
    CommandDef {
        name: "audio",
        aliases: &["sound"],
        syntax: ":audio",
        description: "Audio configuration",
        takes_arg: false,
    },
];

pub fn filtered_commands(prefix: &str) -> Vec<&'static CommandDef> {
    if prefix.is_empty() {
        return COMMANDS.iter().collect();
    }
    COMMANDS
        .iter()
        .filter(|cmd| {
            cmd.name.starts_with(prefix)
                || cmd.aliases.iter().any(|a| a.starts_with(prefix))
        })
        .collect()
}

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
            // If popup is showing with a selection, accept it first
            if !vim.command_buffer.contains(' ') {
                if let Some(idx) = vim.completion.selected {
                    let matches = filtered_commands(&vim.command_buffer);
                    if let Some(cmd) = matches.get(idx) {
                        if cmd.takes_arg {
                            // Fill the command name + space, stay in command mode
                            vim.command_buffer = format!("{} ", cmd.name);
                            vim.completion.reset(0);
                            return InputResult::None;
                        }
                        // No arg needed — execute it directly
                        vim.command_buffer = cmd.name.to_string();
                    }
                }
            }
            let cmd = std::mem::take(&mut vim.command_buffer);
            vim.enter_normal();
            parse_command(&cmd)
        }
        KeyCode::Tab | KeyCode::BackTab => {
            let cmd_part = if let Some(pos) = vim.command_buffer.find(' ') {
                &vim.command_buffer[..pos]
            } else {
                &vim.command_buffer
            };

            // Only complete the command name part (before any space)
            if vim.command_buffer.contains(' ') {
                return InputResult::None;
            }

            let matches = filtered_commands(cmd_part);
            if matches.is_empty() {
                return InputResult::None;
            }

            if matches.len() == 1 {
                // Single match — accept it
                let cmd = matches[0];
                if cmd.takes_arg {
                    vim.command_buffer = format!("{} ", cmd.name);
                } else {
                    vim.command_buffer = cmd.name.to_string();
                }
                vim.completion.reset(0);
                return InputResult::None;
            }

            if vim.completion.selected.is_some() {
                // Already navigating — accept current selection
                let idx = vim.completion.selected.unwrap();
                if let Some(cmd) = matches.get(idx) {
                    if cmd.takes_arg {
                        vim.command_buffer = format!("{} ", cmd.name);
                    } else {
                        vim.command_buffer = cmd.name.to_string();
                    }
                    vim.completion.reset(0);
                    return InputResult::None;
                }
            }

            // Multiple matches, no selection yet — start navigating
            if key.code == KeyCode::BackTab {
                vim.completion.prev();
            } else {
                vim.completion.next();
            }
            InputResult::None
        }
        KeyCode::Up => {
            if !vim.command_buffer.contains(' ') {
                vim.completion.prev();
            }
            InputResult::None
        }
        KeyCode::Down => {
            if !vim.command_buffer.contains(' ') {
                vim.completion.next();
            }
            InputResult::None
        }
        KeyCode::Backspace => {
            vim.command_buffer.pop();
            if vim.command_buffer.is_empty() {
                vim.enter_normal();
            } else {
                let matches = filtered_commands(&vim.command_buffer);
                vim.completion.reset(matches.len());
            }
            InputResult::None
        }
        KeyCode::Char(c) => {
            vim.command_buffer.push(c);
            if !vim.command_buffer.contains(' ') {
                let matches = filtered_commands(&vim.command_buffer);
                vim.completion.reset(matches.len());
            } else {
                vim.completion.reset(0);
            }
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
        "glitch" => InputResult::Command(CommandAction::Glitch),
        "audio" | "sound" => InputResult::Command(CommandAction::AudioSettings),
        _ => InputResult::None,
    }
}
