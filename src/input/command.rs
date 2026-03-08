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
        description: "Exit gōsuto",
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
        syntax: ":call",
        description: "Start a call in current room",
        takes_arg: false,
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
    CommandDef {
        name: "create",
        aliases: &["new"],
        syntax: ":create",
        description: "Create a new room",
        takes_arg: false,
    },
    CommandDef {
        name: "edit",
        aliases: &["roominfo"],
        syntax: ":edit",
        description: "Edit room settings",
        takes_arg: false,
    },
    CommandDef {
        name: "profile",
        aliases: &["configure", "config"],
        syntax: ":profile",
        description: "Edit user profile",
        takes_arg: false,
    },
    CommandDef {
        name: "nerdfonts",
        aliases: &["nerd", "icons"],
        syntax: ":nerdfonts",
        description: "Toggle Nerd Font icons",
        takes_arg: false,
    },
    CommandDef {
        name: "recovery",
        aliases: &["recover"],
        syntax: ":recovery",
        description: "Manage recovery key",
        takes_arg: false,
    },
    CommandDef {
        name: "password",
        aliases: &["passwd", "pw"],
        syntax: ":password",
        description: "Change account password",
        takes_arg: false,
    },
    CommandDef {
        name: "verify",
        aliases: &["v"],
        syntax: ":verify [user]",
        description: "Start verification",
        takes_arg: true,
    },
];

pub fn filtered_commands(prefix: &str) -> Vec<&'static CommandDef> {
    if prefix.is_empty() {
        return COMMANDS.iter().collect();
    }
    COMMANDS
        .iter()
        .filter(|cmd| {
            cmd.name.starts_with(prefix) || cmd.aliases.iter().any(|a| a.starts_with(prefix))
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
            if !vim.command_buffer.contains(' ')
                && let Some(idx) = vim.completion.selected
            {
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
        "call" => InputResult::Command(CommandAction::Call),
        "answer" | "accept" => InputResult::Command(CommandAction::Answer),
        "reject" | "decline" => InputResult::Command(CommandAction::Reject),
        "hangup" | "end" => InputResult::Command(CommandAction::Hangup),
        "rain" | "matrix" | "effects" => InputResult::Command(CommandAction::Rain),
        "glitch" => InputResult::Command(CommandAction::Glitch),
        "audio" | "sound" => InputResult::Command(CommandAction::AudioSettings),
        "create" | "new" => InputResult::Command(CommandAction::CreateRoom),
        "edit" | "roominfo" => InputResult::Command(CommandAction::RoomInfo),
        "profile" | "configure" | "config" => InputResult::Command(CommandAction::Configure),
        "nerdfonts" | "nerd" | "icons" => InputResult::Command(CommandAction::NerdFonts),
        "recovery" | "recover" => InputResult::Command(CommandAction::Recovery),
        "password" | "passwd" | "pw" => InputResult::Command(CommandAction::ChangePassword),
        "verify" | "v" => {
            let user = if arg.is_empty() {
                None
            } else {
                Some(arg.to_string())
            };
            InputResult::Command(CommandAction::Verify(user))
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

    // --- filtered_commands ---

    #[test]
    fn filtered_commands_empty_returns_all() {
        let result = filtered_commands("");
        assert_eq!(result.len(), COMMANDS.len());
    }

    #[test]
    fn filtered_commands_partial_filters() {
        let result = filtered_commands("qu");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "quit");
    }

    #[test]
    fn filtered_commands_alias_matches() {
        let result = filtered_commands("q");
        assert!(result.iter().any(|c| c.name == "quit"));
    }

    #[test]
    fn filtered_commands_no_match() {
        let result = filtered_commands("zzzzz");
        assert!(result.is_empty());
    }

    #[test]
    fn filtered_commands_multiple_matches() {
        let result = filtered_commands("c");
        // "call", "create", "config" (alias) all start with "c"
        assert!(result.len() >= 2);
    }

    // --- parse_command ---

    #[test]
    fn parse_quit() {
        let result = parse_command("quit");
        assert!(matches!(result, InputResult::Command(CommandAction::Quit)));
    }

    #[test]
    fn parse_q_alias() {
        let result = parse_command("q");
        assert!(matches!(result, InputResult::Command(CommandAction::Quit)));
    }

    #[test]
    fn parse_join_with_arg() {
        let result = parse_command("join #room:matrix.org");
        assert!(
            matches!(result, InputResult::Command(CommandAction::Join(ref r)) if r == "#room:matrix.org")
        );
    }

    #[test]
    fn parse_join_no_arg_returns_none() {
        let result = parse_command("join");
        assert!(matches!(result, InputResult::None));
    }

    #[test]
    fn parse_leave() {
        let result = parse_command("leave");
        assert!(matches!(result, InputResult::Command(CommandAction::Leave)));
    }

    #[test]
    fn parse_dm_with_arg() {
        let result = parse_command("dm @user:matrix.org");
        assert!(
            matches!(result, InputResult::Command(CommandAction::DirectMessage(ref u)) if u == "@user:matrix.org")
        );
    }

    #[test]
    fn parse_dm_no_arg_returns_none() {
        let result = parse_command("dm");
        assert!(matches!(result, InputResult::None));
    }

    #[test]
    fn parse_logout() {
        let result = parse_command("logout");
        assert!(matches!(
            result,
            InputResult::Command(CommandAction::Logout)
        ));
    }

    #[test]
    fn parse_call() {
        let result = parse_command("call");
        assert!(matches!(result, InputResult::Command(CommandAction::Call)));
    }

    #[test]
    fn parse_answer_alias_accept() {
        assert!(matches!(
            parse_command("answer"),
            InputResult::Command(CommandAction::Answer)
        ));
        assert!(matches!(
            parse_command("accept"),
            InputResult::Command(CommandAction::Answer)
        ));
    }

    #[test]
    fn parse_reject_alias_decline() {
        assert!(matches!(
            parse_command("reject"),
            InputResult::Command(CommandAction::Reject)
        ));
        assert!(matches!(
            parse_command("decline"),
            InputResult::Command(CommandAction::Reject)
        ));
    }

    #[test]
    fn parse_hangup_alias_end() {
        assert!(matches!(
            parse_command("hangup"),
            InputResult::Command(CommandAction::Hangup)
        ));
        assert!(matches!(
            parse_command("end"),
            InputResult::Command(CommandAction::Hangup)
        ));
    }

    #[test]
    fn parse_rain_aliases() {
        assert!(matches!(
            parse_command("rain"),
            InputResult::Command(CommandAction::Rain)
        ));
        assert!(matches!(
            parse_command("matrix"),
            InputResult::Command(CommandAction::Rain)
        ));
        assert!(matches!(
            parse_command("effects"),
            InputResult::Command(CommandAction::Rain)
        ));
    }

    #[test]
    fn parse_glitch() {
        assert!(matches!(
            parse_command("glitch"),
            InputResult::Command(CommandAction::Glitch)
        ));
    }

    #[test]
    fn parse_audio_alias_sound() {
        assert!(matches!(
            parse_command("audio"),
            InputResult::Command(CommandAction::AudioSettings)
        ));
        assert!(matches!(
            parse_command("sound"),
            InputResult::Command(CommandAction::AudioSettings)
        ));
    }

    #[test]
    fn parse_create_alias_new() {
        assert!(matches!(
            parse_command("create"),
            InputResult::Command(CommandAction::CreateRoom)
        ));
        assert!(matches!(
            parse_command("new"),
            InputResult::Command(CommandAction::CreateRoom)
        ));
    }

    #[test]
    fn parse_edit_alias_roominfo() {
        assert!(matches!(
            parse_command("edit"),
            InputResult::Command(CommandAction::RoomInfo)
        ));
        assert!(matches!(
            parse_command("roominfo"),
            InputResult::Command(CommandAction::RoomInfo)
        ));
    }

    #[test]
    fn parse_profile_aliases() {
        assert!(matches!(
            parse_command("profile"),
            InputResult::Command(CommandAction::Configure)
        ));
        assert!(matches!(
            parse_command("configure"),
            InputResult::Command(CommandAction::Configure)
        ));
        assert!(matches!(
            parse_command("config"),
            InputResult::Command(CommandAction::Configure)
        ));
    }

    #[test]
    fn parse_recovery() {
        assert!(matches!(
            parse_command("recovery"),
            InputResult::Command(CommandAction::Recovery)
        ));
    }

    #[test]
    fn parse_recover_alias() {
        assert!(matches!(
            parse_command("recover"),
            InputResult::Command(CommandAction::Recovery)
        ));
    }

    #[test]
    fn parse_verify_no_arg() {
        let result = parse_command("verify");
        assert!(matches!(
            result,
            InputResult::Command(CommandAction::Verify(None))
        ));
    }

    #[test]
    fn parse_verify_with_arg() {
        let result = parse_command("verify @alice:matrix.org");
        assert!(
            matches!(result, InputResult::Command(CommandAction::Verify(Some(ref u))) if u == "@alice:matrix.org")
        );
    }

    #[test]
    fn parse_verify_alias_v() {
        let result = parse_command("v");
        assert!(matches!(
            result,
            InputResult::Command(CommandAction::Verify(None))
        ));
    }

    #[test]
    fn parse_password_aliases() {
        assert!(matches!(
            parse_command("password"),
            InputResult::Command(CommandAction::ChangePassword)
        ));
        assert!(matches!(
            parse_command("passwd"),
            InputResult::Command(CommandAction::ChangePassword)
        ));
        assert!(matches!(
            parse_command("pw"),
            InputResult::Command(CommandAction::ChangePassword)
        ));
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert!(matches!(parse_command("xyzzy"), InputResult::None));
    }

    #[test]
    fn parse_whitespace_trimmed() {
        let result = parse_command("  quit  ");
        assert!(matches!(result, InputResult::Command(CommandAction::Quit)));
    }

    // --- handle_command key handling ---

    #[test]
    fn esc_exits_command_mode() {
        let mut vim = VimState::new();
        vim.enter_command();
        let result = handle_command(key(KeyCode::Esc), &mut vim);
        assert!(matches!(result, InputResult::None));
        assert_eq!(vim.mode, VimMode::Normal);
    }

    #[test]
    fn ctrl_c_quits_in_command_mode() {
        let mut vim = VimState::new();
        vim.enter_command();
        let result = handle_command(ctrl('c'), &mut vim);
        assert!(matches!(result, InputResult::Quit));
    }

    #[test]
    fn char_appends_to_buffer() {
        let mut vim = VimState::new();
        vim.enter_command();
        handle_command(key(KeyCode::Char('q')), &mut vim);
        assert_eq!(vim.command_buffer, "q");
        handle_command(key(KeyCode::Char('u')), &mut vim);
        assert_eq!(vim.command_buffer, "qu");
    }

    #[test]
    fn backspace_last_char_exits() {
        let mut vim = VimState::new();
        vim.enter_command();
        handle_command(key(KeyCode::Char('q')), &mut vim);
        handle_command(key(KeyCode::Backspace), &mut vim);
        assert_eq!(vim.mode, VimMode::Normal);
    }

    #[test]
    fn backspace_with_remaining_chars() {
        let mut vim = VimState::new();
        vim.enter_command();
        handle_command(key(KeyCode::Char('q')), &mut vim);
        handle_command(key(KeyCode::Char('u')), &mut vim);
        handle_command(key(KeyCode::Backspace), &mut vim);
        assert_eq!(vim.command_buffer, "q");
        assert_eq!(vim.mode, VimMode::Command);
    }

    #[test]
    fn enter_executes_command() {
        let mut vim = VimState::new();
        vim.enter_command();
        vim.command_buffer = "quit".to_string();
        let result = handle_command(key(KeyCode::Enter), &mut vim);
        assert!(matches!(result, InputResult::Command(CommandAction::Quit)));
        assert_eq!(vim.mode, VimMode::Normal);
    }
}
