use super::*;

impl App {
    pub(crate) fn handle_login_key(&mut self, key: crossterm::event::KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }

        if matches!(
            self.auth,
            AuthState::LoggingIn | AuthState::AutoLoggingIn | AuthState::Registering
        ) {
            return;
        }

        match key.code {
            KeyCode::Tab => self.login.next_field(),
            KeyCode::BackTab => self.login.prev_field(),
            KeyCode::F(2) => self.login.toggle_mode(),
            KeyCode::Enter => {
                if self.login.mode == crate::ui::login::FormMode::Register {
                    self.initiate_registration();
                } else {
                    self.initiate_login();
                }
            }
            KeyCode::Backspace => self.login.backspace(),
            KeyCode::Char(c) => self.login.insert_char(c),
            _ => {}
        }
    }

    pub(crate) fn initiate_login(&mut self) {
        if self.login.username.is_empty() || self.login.password.is_empty() {
            self.auth = AuthState::Error("Username and password required".to_string());
            return;
        }
        self.auth = AuthState::LoggingIn;
    }

    pub(crate) fn initiate_registration(&mut self) {
        if self.login.username.is_empty() || self.login.password.is_empty() {
            self.auth = AuthState::Error("Username and password required".to_string());
            return;
        }
        if self.login.password != self.login.confirm_password {
            self.auth = AuthState::Error("Passwords do not match".to_string());
            return;
        }
        self.auth = AuthState::Registering;
    }

    // ── Invite Prompt ────────────────────────────────

    pub(crate) fn handle_invite_prompt_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.invite_prompt_room = None;
            self.running = false;
            return;
        }

        match key.code {
            KeyCode::Enter => {
                if let Some(room_id) = self.invite_prompt_room.take() {
                    self.pending_accept_invite = Some(room_id);
                }
            }
            KeyCode::Char('d') => {
                if let Some(room_id) = self.invite_prompt_room.take() {
                    self.pending_decline_invite = Some(room_id);
                }
            }
            KeyCode::Esc => {
                self.invite_prompt_room = None;
            }
            _ => {}
        }
    }

    // ── Recovery Modal ────────────────────────────────

    pub(crate) fn handle_recovery_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.recovery = None;
            self.running = false;
            return;
        }

        let Some(ref mut modal) = self.recovery else {
            return;
        };

        let transition =
            recovery_key_action(modal, key.code, key.modifiers, self.clipboard.as_mut());
        match transition {
            RecoveryTransition::None => {}
            RecoveryTransition::Close => {
                self.recovery = None;
            }
            RecoveryTransition::Pending(action) => {
                self.pending_recovery = Some(action);
            }
        }
    }

    // ── Verification Modal ────────────────────────────

    pub(crate) fn cancel_verification(&mut self) {
        self.verify_confirm_tx = None;
        self.verification_modal = None;
        if let Some(handle) = self.verify_task_handle.take() {
            handle.abort();
        }
    }

    pub(crate) fn handle_verify_modal_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.cancel_verification();
            self.running = false;
            return;
        }

        let stage = self.verification_modal.as_ref().map(|m| &m.stage);

        match stage {
            Some(crate::state::VerificationStage::ChooseAction { selected }) => {
                let sel = *selected;
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(ref mut modal) = self.verification_modal {
                            modal.stage = crate::state::VerificationStage::ChooseAction {
                                selected: sel.saturating_sub(1),
                            };
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(ref mut modal) = self.verification_modal {
                            modal.stage = crate::state::VerificationStage::ChooseAction {
                                selected: (sel + 1).min(2),
                            };
                        }
                    }
                    KeyCode::Enter => match sel {
                        0 => {
                            // Self-verify
                            if let Some(ref mut modal) = self.verification_modal {
                                modal.stage =
                                    crate::state::VerificationStage::WaitingForOtherDevice;
                            }
                            self.pending_verify = Some(None);
                        }
                        1 => {
                            // Verify a user — show text input
                            if let Some(ref mut modal) = self.verification_modal {
                                modal.user_id_buffer.clear();
                                modal.stage = crate::state::VerificationStage::EnterUserId;
                            }
                        }
                        2 => {
                            // Reset cross-signing keys
                            self.pending_reset_cross_signing = true;
                            self.cancel_verification();
                        }
                        _ => {}
                    },
                    KeyCode::Esc => {
                        self.cancel_verification();
                    }
                    _ => {}
                }
            }
            Some(crate::state::VerificationStage::EnterUserId) => match key.code {
                KeyCode::Char(c) => {
                    if let Some(ref mut modal) = self.verification_modal {
                        modal.user_id_buffer.push(c);
                    }
                }
                KeyCode::Backspace => {
                    if let Some(ref mut modal) = self.verification_modal {
                        modal.user_id_buffer.pop();
                    }
                }
                KeyCode::Enter => {
                    let user_id = self
                        .verification_modal
                        .as_ref()
                        .map(|m| m.user_id_buffer.clone())
                        .unwrap_or_default();
                    if user_id.is_empty() {
                        self.last_error = Some("User ID cannot be empty".to_string());
                        return;
                    }
                    if let Some(ref mut modal) = self.verification_modal {
                        modal.sender = user_id.clone();
                        modal.stage = crate::state::VerificationStage::WaitingForOtherDevice;
                    }
                    self.pending_verify = Some(Some(user_id));
                }
                KeyCode::Esc => {
                    if let Some(ref mut modal) = self.verification_modal {
                        modal.stage = crate::state::VerificationStage::ChooseAction { selected: 1 };
                    }
                }
                _ => {}
            },
            Some(crate::state::VerificationStage::EmojiConfirmation) => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(tx) = self.verify_confirm_tx.take()
                        && tx.send(true).is_err()
                    {
                        tracing::warn!("verify confirm: receiver dropped");
                    }
                    if let Some(ref mut modal) = self.verification_modal {
                        modal.stage = crate::state::VerificationStage::WaitingForOtherDevice;
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if let Some(tx) = self.verify_confirm_tx.take()
                        && tx.send(false).is_err()
                    {
                        tracing::warn!("verify reject: receiver dropped");
                    }
                }
                KeyCode::Esc => {
                    self.cancel_verification();
                }
                _ => {}
            },
            Some(crate::state::VerificationStage::Completed)
            | Some(crate::state::VerificationStage::Failed(_)) => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.cancel_verification();
                }
                _ => {}
            },
            Some(crate::state::VerificationStage::WaitingForOtherDevice) => {
                if key.code == KeyCode::Esc {
                    self.cancel_verification();
                }
            }
            None => {}
        }
    }

    // ── Which-Key Leader Popup ────────────────────────

    pub(crate) fn handle_which_key(&mut self, key: KeyEvent) {
        use crate::ui::which_key::WhichKeyCategory;

        match self.which_key {
            Some(None) => {
                // Root menu
                match key.code {
                    KeyCode::Char('r') => self.which_key = Some(Some(WhichKeyCategory::Room)),
                    KeyCode::Char('c') => self.which_key = Some(Some(WhichKeyCategory::Call)),
                    KeyCode::Char('s') => self.which_key = Some(Some(WhichKeyCategory::Security)),
                    KeyCode::Char('e') => self.which_key = Some(Some(WhichKeyCategory::Effects)),
                    KeyCode::Char('u') => self.which_key = Some(Some(WhichKeyCategory::User)),
                    KeyCode::Char('q') => {
                        self.which_key = None;
                        self.running = false;
                    }
                    KeyCode::Char('l') => {
                        self.which_key = None;
                        self.pending_logout = true;
                        self.pending_credential_clear = true;
                    }
                    _ => self.which_key = None,
                }
            }
            Some(Some(cat)) => match key.code {
                KeyCode::Esc => self.which_key = None,
                KeyCode::Backspace => self.which_key = Some(None),
                KeyCode::Char(ch) => {
                    self.which_key = None;
                    self.dispatch_which_key_action(cat, ch);
                }
                _ => self.which_key = None,
            },
            None => {}
        }
    }

    // ── Room Info ───────────────────────────────────────

    pub(crate) fn handle_room_info_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.room_info.open = false;
            self.running = false;
            return;
        }

        match self.room_info.handle_key(key) {
            RoomInfoAction::None => {}
            RoomInfoAction::Close => {
                self.room_info.open = false;
            }
            RoomInfoAction::SetName(room_id, name) => {
                self.pending_set_room_name = Some((room_id, name));
            }
            RoomInfoAction::SetTopic(room_id, topic) => {
                self.pending_set_room_topic = Some((room_id, topic));
            }
            RoomInfoAction::SetVisibility(room_id, vis) => {
                self.pending_set_visibility = Some((room_id, vis));
            }
            RoomInfoAction::EnableEncryption(room_id) => {
                self.pending_enable_encryption = Some(room_id);
            }
        }
    }

    // ── Create Room Modal ────────────────────────────────

    pub(crate) fn handle_create_room_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.create_room.open = false;
            self.running = false;
            return;
        }

        match self.create_room.handle_key(key) {
            CreateRoomAction::None => {}
            CreateRoomAction::Close => {
                self.create_room.open = false;
            }
            CreateRoomAction::Error(msg) => {
                self.last_error = Some(msg);
            }
            CreateRoomAction::Create(params) => {
                self.pending_create_room = Some(params);
            }
        }
    }

    // ── User Config ─────────────────────────────────────

    pub(crate) fn handle_user_config_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.user_config.open = false;
            self.running = false;
            return;
        }

        match self.user_config.handle_key(key) {
            UserConfigAction::None => {}
            UserConfigAction::Close => {
                self.user_config.open = false;
            }
            UserConfigAction::SetDisplayName(name) => {
                self.pending_set_display_name = Some(name);
            }
            UserConfigAction::OpenChangePassword => {
                self.user_config.open = false;
                self.handle_command(CommandAction::ChangePassword);
            }
        }
    }

    // ── Change Password ─────────────────────────────────

    pub(crate) fn handle_change_password_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.change_password.open = false;
            self.running = false;
            return;
        }

        match self.change_password.handle_key(key) {
            ChangePasswordAction::None => {}
            ChangePasswordAction::Close => {
                self.change_password.open = false;
            }
            ChangePasswordAction::Error(msg) => {
                self.last_error = Some(msg);
            }
            ChangePasswordAction::Submit(current, new) => {
                self.pending_change_password = Some((current, new));
            }
        }
    }
}
