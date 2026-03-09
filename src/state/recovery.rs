use crossterm::event::{KeyCode, KeyModifiers};

/// Steps in the automatic healing process that runs when `recover()` succeeds
/// but the account's encryption state is still incomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealingStep {
    /// Generating and uploading cross-signing keys (master, self-signing, user-signing).
    CrossSigning,
    /// Creating or enabling server-side key backup.
    Backup,
    /// Re-exporting all secrets into a new secret storage key.
    ExportSecrets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStage {
    Checking,
    Disabled,
    Enabled,
    Incomplete,
    EnterKey,
    Recovering,
    Creating,
    NeedPassword,
    Healing(HealingStep),
    ShowKey(String),
    ConfirmReset,
    Resetting,
    Error(String),
}

pub struct RecoveryModalState {
    pub stage: RecoveryStage,
    pub key_buffer: String,
    pub confirm_buffer: String,
    pub copied: bool,
    pub password_buffer: String,
    pub password_tx: Option<tokio::sync::oneshot::Sender<String>>,
}

impl RecoveryModalState {
    pub fn new() -> Self {
        Self {
            stage: RecoveryStage::Checking,
            key_buffer: String::new(),
            confirm_buffer: String::new(),
            copied: false,
            password_buffer: String::new(),
            password_tx: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    Check,
    Create,
    Recover(String),
    Reset,
    SubmitPassword(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryTransition {
    None,
    Close,
    Pending(RecoveryAction),
}

pub fn recovery_key_action(
    state: &mut RecoveryModalState,
    code: KeyCode,
    _modifiers: KeyModifiers,
    clipboard: Option<&mut arboard::Clipboard>,
) -> RecoveryTransition {
    match &state.stage {
        RecoveryStage::Checking
        | RecoveryStage::Recovering
        | RecoveryStage::Creating
        | RecoveryStage::Healing(_)
        | RecoveryStage::Resetting => {
            if code == KeyCode::Esc {
                return RecoveryTransition::Close;
            }
            RecoveryTransition::None
        }
        RecoveryStage::NeedPassword => match code {
            KeyCode::Char(c) => {
                state.password_buffer.push(c);
                RecoveryTransition::None
            }
            KeyCode::Backspace => {
                state.password_buffer.pop();
                RecoveryTransition::None
            }
            KeyCode::Enter => {
                if state.password_buffer.is_empty() {
                    RecoveryTransition::None
                } else {
                    let pw = state.password_buffer.clone();
                    state.password_buffer.clear();
                    state.stage = RecoveryStage::Healing(HealingStep::CrossSigning);
                    RecoveryTransition::Pending(RecoveryAction::SubmitPassword(pw))
                }
            }
            KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
        RecoveryStage::Disabled => match code {
            KeyCode::Enter => {
                state.stage = RecoveryStage::Creating;
                RecoveryTransition::Pending(RecoveryAction::Create)
            }
            KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
        RecoveryStage::Enabled => match code {
            KeyCode::Char('r') => {
                state.stage = RecoveryStage::ConfirmReset;
                RecoveryTransition::None
            }
            KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
        RecoveryStage::Incomplete => match code {
            KeyCode::Char('e') => {
                state.stage = RecoveryStage::EnterKey;
                RecoveryTransition::None
            }
            KeyCode::Char('r') => {
                state.stage = RecoveryStage::ConfirmReset;
                RecoveryTransition::None
            }
            KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
        RecoveryStage::EnterKey => match code {
            KeyCode::Char(c) => {
                state.key_buffer.push(c);
                RecoveryTransition::None
            }
            KeyCode::Backspace => {
                state.key_buffer.pop();
                RecoveryTransition::None
            }
            KeyCode::Enter => {
                if state.key_buffer.is_empty() {
                    RecoveryTransition::None
                } else {
                    let key = state.key_buffer.clone();
                    state.stage = RecoveryStage::Recovering;
                    RecoveryTransition::Pending(RecoveryAction::Recover(key))
                }
            }
            KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
        RecoveryStage::ConfirmReset => match code {
            KeyCode::Char(c) => {
                state.confirm_buffer.push(c);
                RecoveryTransition::None
            }
            KeyCode::Backspace => {
                state.confirm_buffer.pop();
                RecoveryTransition::None
            }
            KeyCode::Enter => {
                if state.confirm_buffer == "yes" {
                    state.stage = RecoveryStage::Resetting;
                    RecoveryTransition::Pending(RecoveryAction::Reset)
                } else {
                    state.confirm_buffer.clear();
                    RecoveryTransition::None
                }
            }
            KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
        RecoveryStage::ShowKey(_) => match code {
            KeyCode::Char('c') => {
                if let RecoveryStage::ShowKey(ref key) = state.stage
                    && let Some(clip) = clipboard
                {
                    if let Err(e) = clip.set_text(key.clone()) {
                        tracing::warn!("clipboard set_text failed: {e}");
                    }
                    state.copied = true;
                }
                RecoveryTransition::None
            }
            KeyCode::Enter | KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
        RecoveryStage::Error(_) => match code {
            KeyCode::Enter | KeyCode::Esc => RecoveryTransition::Close,
            _ => RecoveryTransition::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modal() -> RecoveryModalState {
        RecoveryModalState::new()
    }

    fn act(m: &mut RecoveryModalState, code: KeyCode) -> RecoveryTransition {
        recovery_key_action(m, code, KeyModifiers::NONE, None)
    }

    #[test]
    fn checking_esc_closes() {
        let mut m = modal();
        assert_eq!(act(&mut m, KeyCode::Esc), RecoveryTransition::Close);
    }

    #[test]
    fn disabled_enter_creates() {
        let mut m = modal();
        m.stage = RecoveryStage::Disabled;
        let t = act(&mut m, KeyCode::Enter);
        assert_eq!(m.stage, RecoveryStage::Creating);
        assert_eq!(t, RecoveryTransition::Pending(RecoveryAction::Create));
    }

    #[test]
    fn disabled_esc_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::Disabled;
        assert_eq!(act(&mut m, KeyCode::Esc), RecoveryTransition::Close);
    }

    #[test]
    fn enabled_r_confirm_reset() {
        let mut m = modal();
        m.stage = RecoveryStage::Enabled;
        let t = act(&mut m, KeyCode::Char('r'));
        assert_eq!(m.stage, RecoveryStage::ConfirmReset);
        assert_eq!(t, RecoveryTransition::None);
    }

    #[test]
    fn enabled_esc_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::Enabled;
        assert_eq!(act(&mut m, KeyCode::Esc), RecoveryTransition::Close);
    }

    #[test]
    fn incomplete_e_enter_key() {
        let mut m = modal();
        m.stage = RecoveryStage::Incomplete;
        let t = act(&mut m, KeyCode::Char('e'));
        assert_eq!(m.stage, RecoveryStage::EnterKey);
        assert_eq!(t, RecoveryTransition::None);
    }

    #[test]
    fn incomplete_r_confirm_reset() {
        let mut m = modal();
        m.stage = RecoveryStage::Incomplete;
        let t = act(&mut m, KeyCode::Char('r'));
        assert_eq!(m.stage, RecoveryStage::ConfirmReset);
        assert_eq!(t, RecoveryTransition::None);
    }

    #[test]
    fn enter_key_char_appends() {
        let mut m = modal();
        m.stage = RecoveryStage::EnterKey;
        act(&mut m, KeyCode::Char('a'));
        act(&mut m, KeyCode::Char('b'));
        assert_eq!(m.key_buffer, "ab");
    }

    #[test]
    fn enter_key_backspace_pops() {
        let mut m = modal();
        m.stage = RecoveryStage::EnterKey;
        m.key_buffer = "abc".to_string();
        act(&mut m, KeyCode::Backspace);
        assert_eq!(m.key_buffer, "ab");
    }

    #[test]
    fn enter_key_enter_nonempty_recovers() {
        let mut m = modal();
        m.stage = RecoveryStage::EnterKey;
        m.key_buffer = "my-key".to_string();
        let t = act(&mut m, KeyCode::Enter);
        assert_eq!(m.stage, RecoveryStage::Recovering);
        assert_eq!(
            t,
            RecoveryTransition::Pending(RecoveryAction::Recover("my-key".to_string()))
        );
    }

    #[test]
    fn enter_key_enter_empty_noop() {
        let mut m = modal();
        m.stage = RecoveryStage::EnterKey;
        let t = act(&mut m, KeyCode::Enter);
        assert_eq!(m.stage, RecoveryStage::EnterKey);
        assert_eq!(t, RecoveryTransition::None);
    }

    #[test]
    fn confirm_reset_yes_resets() {
        let mut m = modal();
        m.stage = RecoveryStage::ConfirmReset;
        act(&mut m, KeyCode::Char('y'));
        act(&mut m, KeyCode::Char('e'));
        act(&mut m, KeyCode::Char('s'));
        let t = act(&mut m, KeyCode::Enter);
        assert_eq!(m.stage, RecoveryStage::Resetting);
        assert_eq!(t, RecoveryTransition::Pending(RecoveryAction::Reset));
    }

    #[test]
    fn confirm_reset_no_clears() {
        let mut m = modal();
        m.stage = RecoveryStage::ConfirmReset;
        act(&mut m, KeyCode::Char('n'));
        act(&mut m, KeyCode::Char('o'));
        let t = act(&mut m, KeyCode::Enter);
        assert_eq!(m.stage, RecoveryStage::ConfirmReset);
        assert_eq!(t, RecoveryTransition::None);
        assert!(m.confirm_buffer.is_empty());
    }

    #[test]
    fn show_key_c_copies() {
        let mut m = modal();
        m.stage = RecoveryStage::ShowKey("key123".to_string());
        // Without clipboard, copied stays false but no crash
        let t = act(&mut m, KeyCode::Char('c'));
        assert_eq!(t, RecoveryTransition::None);
    }

    #[test]
    fn show_key_enter_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::ShowKey("key123".to_string());
        assert_eq!(act(&mut m, KeyCode::Enter), RecoveryTransition::Close);
    }

    #[test]
    fn show_key_esc_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::ShowKey("key123".to_string());
        assert_eq!(act(&mut m, KeyCode::Esc), RecoveryTransition::Close);
    }

    #[test]
    fn creating_ignores_keys() {
        let mut m = modal();
        m.stage = RecoveryStage::Creating;
        assert_eq!(act(&mut m, KeyCode::Char('x')), RecoveryTransition::None);
    }

    #[test]
    fn recovering_ignores_keys() {
        let mut m = modal();
        m.stage = RecoveryStage::Recovering;
        assert_eq!(act(&mut m, KeyCode::Char('x')), RecoveryTransition::None);
    }

    #[test]
    fn resetting_ignores_keys() {
        let mut m = modal();
        m.stage = RecoveryStage::Resetting;
        assert_eq!(act(&mut m, KeyCode::Char('x')), RecoveryTransition::None);
    }

    #[test]
    fn error_enter_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::Error("oops".to_string());
        assert_eq!(act(&mut m, KeyCode::Enter), RecoveryTransition::Close);
    }

    #[test]
    fn error_esc_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::Error("oops".to_string());
        assert_eq!(act(&mut m, KeyCode::Esc), RecoveryTransition::Close);
    }

    #[test]
    fn healing_stage_esc_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::Healing(HealingStep::CrossSigning);
        assert_eq!(act(&mut m, KeyCode::Esc), RecoveryTransition::Close);
    }

    #[test]
    fn healing_stage_ignores_keys() {
        let mut m = modal();
        m.stage = RecoveryStage::Healing(HealingStep::Backup);
        assert_eq!(act(&mut m, KeyCode::Char('x')), RecoveryTransition::None);
    }

    #[test]
    fn need_password_char_appends() {
        let mut m = modal();
        m.stage = RecoveryStage::NeedPassword;
        act(&mut m, KeyCode::Char('a'));
        act(&mut m, KeyCode::Char('b'));
        assert_eq!(m.password_buffer, "ab");
    }

    #[test]
    fn need_password_backspace_pops() {
        let mut m = modal();
        m.stage = RecoveryStage::NeedPassword;
        m.password_buffer = "abc".to_string();
        act(&mut m, KeyCode::Backspace);
        assert_eq!(m.password_buffer, "ab");
    }

    #[test]
    fn need_password_enter_submits() {
        let mut m = modal();
        m.stage = RecoveryStage::NeedPassword;
        m.password_buffer = "secret".to_string();
        let t = act(&mut m, KeyCode::Enter);
        assert_eq!(m.stage, RecoveryStage::Healing(HealingStep::CrossSigning));
        assert_eq!(
            t,
            RecoveryTransition::Pending(RecoveryAction::SubmitPassword("secret".to_string()))
        );
        assert!(m.password_buffer.is_empty());
    }

    #[test]
    fn need_password_enter_empty_noop() {
        let mut m = modal();
        m.stage = RecoveryStage::NeedPassword;
        let t = act(&mut m, KeyCode::Enter);
        assert_eq!(m.stage, RecoveryStage::NeedPassword);
        assert_eq!(t, RecoveryTransition::None);
    }

    #[test]
    fn need_password_esc_closes() {
        let mut m = modal();
        m.stage = RecoveryStage::NeedPassword;
        assert_eq!(act(&mut m, KeyCode::Esc), RecoveryTransition::Close);
    }
}
