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

    // ── Redact Confirm ────────────────────────────────

    pub(crate) fn handle_redact_confirm_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.redact_confirm = None;
            self.running = false;
            return;
        }

        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if let Some(confirm) = self.redact_confirm.take()
                    && let Some(room_id) = self.messages.current_room_id.clone()
                {
                    self.pending_redact = Some(PendingRedact {
                        room_id,
                        event_id: confirm.event_id,
                    });
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.redact_confirm = None;
            }
            _ => {}
        }
    }

    // ── Reaction Picker ────────────────────────────────

    pub(crate) fn handle_reaction_picker_key(&mut self, key: KeyEvent) {
        use crate::ui::emoji_data::filtered_emojis;

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.reaction_picker = None;
            self.running = false;
            return;
        }

        let Some(ref mut picker) = self.reaction_picker else {
            return;
        };

        const GRID_COLS: usize = 8;

        if picker.filter_active {
            // Filter input mode
            match key.code {
                KeyCode::Char(c) => {
                    picker.filter.push(c);
                    let count = filtered_emojis(&picker.filter).len();
                    if picker.grid_index >= count {
                        picker.grid_index = count.saturating_sub(1);
                    }
                }
                KeyCode::Backspace => {
                    picker.filter.pop();
                    let count = filtered_emojis(&picker.filter).len();
                    if picker.grid_index >= count {
                        picker.grid_index = count.saturating_sub(1);
                    }
                }
                KeyCode::Enter => {
                    picker.filter_active = false;
                    picker.in_grid = true;
                    picker.grid_index = 0;
                    picker.scroll_offset = 0;
                }
                KeyCode::Esc => {
                    if picker.filter.is_empty() {
                        picker.filter_active = false;
                    } else {
                        picker.filter.clear();
                        picker.filter_active = false;
                    }
                }
                _ => {}
            }
            return;
        }

        if picker.in_grid {
            // Grid navigation mode
            let filtered = filtered_emojis(&picker.filter);
            let count = filtered.len();
            if count == 0 {
                match key.code {
                    KeyCode::Char('/') => {
                        picker.filter_active = true;
                    }
                    KeyCode::Esc => {
                        picker.in_grid = false;
                    }
                    _ => {}
                }
                return;
            }

            match key.code {
                KeyCode::Char('h') | KeyCode::Left => {
                    picker.grid_index = picker.grid_index.saturating_sub(1);
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    picker.grid_index = (picker.grid_index + 1).min(count - 1);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let next = picker.grid_index + GRID_COLS;
                    if next < count {
                        picker.grid_index = next;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if picker.grid_index < GRID_COLS {
                        // Move back to quick row
                        picker.in_grid = false;
                    } else {
                        picker.grid_index -= GRID_COLS;
                    }
                }
                KeyCode::Enter => {
                    let emoji = filtered[picker.grid_index].emoji.to_string();
                    self.confirm_reaction(emoji);
                }
                KeyCode::Char('/') => {
                    picker.filter_active = true;
                }
                KeyCode::Esc => {
                    picker.in_grid = false;
                }
                _ => {}
            }
        } else {
            // Quick row mode
            match key.code {
                KeyCode::Char('h') | KeyCode::Left => {
                    picker.quick_pick_index = picker.quick_pick_index.saturating_sub(1);
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    let max = QUICK_EMOJIS.len().saturating_sub(1);
                    picker.quick_pick_index = (picker.quick_pick_index + 1).min(max);
                }
                KeyCode::Char(c @ '1'..='8') => {
                    let idx = (c as usize) - ('1' as usize);
                    if idx < QUICK_EMOJIS.len() {
                        picker.quick_pick_index = idx;
                    }
                }
                KeyCode::Enter => {
                    let emoji = QUICK_EMOJIS[picker.quick_pick_index].to_string();
                    self.confirm_reaction(emoji);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    picker.in_grid = true;
                    picker.grid_index = 0;
                    picker.scroll_offset = 0;
                }
                KeyCode::Char('/') => {
                    picker.filter_active = true;
                }
                KeyCode::Esc => {
                    self.reaction_picker = None;
                }
                _ => {}
            }
        }
    }

    fn confirm_reaction(&mut self, emoji: String) {
        let Some(picker) = self.reaction_picker.take() else {
            return;
        };
        let Some(room_id) = self.messages.current_room_id.clone() else {
            return;
        };

        let toggle_off = if picker.existing_own_reactions.contains(&emoji) {
            // Find the reaction_event_id for this emoji from the message
            self.messages
                .messages
                .iter()
                .find(|m| m.event_id == picker.event_id)
                .and_then(|msg| {
                    msg.reactions.iter().find(|r| r.key == emoji).and_then(|r| {
                        let own_id = match &self.auth {
                            AuthState::LoggedIn { user_id, .. } => user_id.as_str(),
                            _ => return None,
                        };
                        r.senders
                            .iter()
                            .find(|s| s.user_id == own_id)
                            .map(|s| s.reaction_event_id.clone())
                    })
                })
        } else {
            None
        };

        self.pending_reaction = Some(PendingReaction {
            room_id,
            target_event_id: picker.event_id,
            emoji_key: emoji,
            toggle_off_reaction_event_id: toggle_off,
        });
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
