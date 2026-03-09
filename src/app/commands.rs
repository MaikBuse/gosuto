use super::*;

impl App {
    pub(crate) fn handle_command(&mut self, action: CommandAction) {
        match action {
            CommandAction::Quit => self.running = false,
            CommandAction::Logout => {
                self.pending_logout = true;
                self.pending_credential_clear = true;
            }
            CommandAction::Join(room) => {
                self.pending_join = Some(room);
            }
            CommandAction::Leave => {
                if let Some(room) = self.room_list.selected_room() {
                    self.pending_leave = Some(room.id.clone());
                }
            }
            CommandAction::DirectMessage(user) => {
                if let AuthState::LoggedIn { ref user_id, .. } = self.auth
                    && user == *user_id
                {
                    self.last_error = Some("Cannot DM yourself".to_string());
                    return;
                }
                self.pending_dm = Some(user);
            }
            CommandAction::Call => {
                if self.call_info.is_some() {
                    self.last_error = Some("Already in a call".to_string());
                    return;
                }
                if let Some(room_id) = self.messages.current_room_id.clone() {
                    if let AuthState::LoggedIn { ref user_id, .. } = self.auth {
                        // Only trust the members list if it's loaded for the current room
                        let members_loaded =
                            self.members_list.current_room_id.as_deref() == Some(&room_id);
                        let others = members_loaded
                            && self
                                .members_list
                                .members
                                .iter()
                                .any(|m| m.user_id != *user_id);
                        if members_loaded && !others {
                            self.last_error = Some("Cannot call yourself".to_string());
                            return;
                        }
                    }
                    let room_name = self
                        .room_list
                        .rooms
                        .iter()
                        .find(|r| r.id == room_id)
                        .map(|r| r.name.clone());
                    self.call_info = Some(CallInfo::new_outgoing(room_id.clone(), room_name));
                    self.set_global_ptt_active(true);
                    if let Some(ref tx) = self.call_cmd_tx {
                        tx.send(CallCommand::Initiate { room_id })
                            .warn_closed("CallCommand::Initiate");
                    }
                } else {
                    self.last_error = Some("No room selected".to_string());
                }
            }
            CommandAction::Answer => {
                if let Some(room_id) = self.incoming_call_room.take() {
                    let caller = self.incoming_call_user.take().unwrap_or_default();
                    let room_name = self.incoming_call_room_name.take();
                    self.call_info =
                        Some(CallInfo::new_incoming(room_id.clone(), caller, room_name));
                    self.set_global_ptt_active(true);
                    if let Some(ref tx) = self.call_cmd_tx {
                        tx.send(CallCommand::Initiate { room_id })
                            .warn_closed("CallCommand::Initiate");
                    }
                } else {
                    self.last_error = Some("No incoming call".to_string());
                }
            }
            CommandAction::Reject => {
                if self.incoming_call_room.is_some() {
                    self.incoming_call_room = None;
                    self.incoming_call_user = None;
                    self.incoming_call_room_name = None;
                } else {
                    self.last_error = Some("No incoming call".to_string());
                }
            }
            CommandAction::Hangup => {
                if self.call_info.is_some() {
                    if let Some(ref tx) = self.call_cmd_tx {
                        tx.send(CallCommand::Leave)
                            .warn_closed("CallCommand::Leave");
                    }
                    self.call_info = None;
                    self.set_global_ptt_active(false);
                } else {
                    self.last_error = Some("No active call".to_string());
                }
            }
            CommandAction::Rain => {
                self.effects.toggle();
                self.config.effects.rain = self.effects.enabled;
                crate::config::save_config(&self.config);
            }
            CommandAction::NerdFonts => {
                self.config.ui.use_nerd_fonts = !self.config.ui.use_nerd_fonts;
                crate::config::save_config(&self.config);
            }
            CommandAction::Glitch => {
                self.effects.toggle_glitch();
                self.config.effects.glitch = self.effects.glitch_enabled;
                crate::config::save_config(&self.config);
            }
            CommandAction::AudioSettings => {
                self.open_audio_settings();
            }
            CommandAction::CreateRoom => {
                self.create_room = CreateRoomState {
                    open: true,
                    selected_field: 0,
                    name_buffer: String::new(),
                    editing_name: true,
                    topic_buffer: String::new(),
                    editing_topic: false,
                    history_visibility: "shared".to_string(),
                    encrypted: "yes".to_string(),
                    creating: false,
                };
            }
            CommandAction::Configure => {
                if let AuthState::LoggedIn {
                    ref user_id,
                    ref device_id,
                    ref homeserver,
                } = self.auth
                {
                    self.user_config = UserConfigState {
                        open: true,
                        user_id: user_id.clone(),
                        device_id: device_id.clone(),
                        homeserver: homeserver.clone(),
                        display_name: None,
                        display_name_buffer: String::new(),
                        editing_display_name: false,
                        verified: self.self_verified,
                        recovery_status: self.recovery_status,
                        selected_field: 0,
                        loading: true,
                        saving: false,
                    };
                    self.pending_user_config = true;
                }
            }
            CommandAction::RoomInfo => {
                if let Some(room_id) = self.messages.current_room_id.clone() {
                    self.room_info = RoomInfoState {
                        open: true,
                        room_id,
                        name: None,
                        topic: None,
                        history_visibility: String::new(),
                        encrypted: false,
                        encryption_selection: "no".to_string(),
                        selected_field: 0,
                        loading: true,
                        saving: false,
                        editing_name: false,
                        name_buffer: String::new(),
                        editing_topic: false,
                        topic_buffer: String::new(),
                        topic_save_pending: false,
                    };
                    self.pending_room_info = true;
                } else {
                    self.last_error = Some("No room selected".to_string());
                }
            }
            CommandAction::ChangePassword => {
                self.change_password = ChangePasswordState {
                    open: true,
                    selected_field: 0,
                    current_buffer: String::new(),
                    new_buffer: String::new(),
                    confirm_buffer: String::new(),
                    saving: false,
                };
            }
            CommandAction::Recovery => {
                self.recovery = Some(RecoveryModalState::new());
                self.pending_recovery = Some(RecoveryAction::Check);
            }
            CommandAction::Verify(target) => {
                if self.verification_modal.is_some() {
                    self.last_error = Some("A verification is already in progress".to_string());
                } else if let Some(ref user_id) = target {
                    // Explicit target — skip menu, go straight to verification
                    self.verification_modal = Some(crate::state::VerificationModalState {
                        stage: crate::state::VerificationStage::WaitingForOtherDevice,
                        sender: user_id.clone(),
                        emojis: vec![],
                        user_id_buffer: String::new(),
                    });
                    self.pending_verify = Some(target);
                } else {
                    // No target — show action menu
                    let sender = match &self.auth {
                        AuthState::LoggedIn { user_id, .. } => user_id.clone(),
                        _ => String::new(),
                    };
                    self.verification_modal = Some(crate::state::VerificationModalState {
                        stage: crate::state::VerificationStage::ChooseAction { selected: 0 },
                        sender,
                        emojis: vec![],
                        user_id_buffer: String::new(),
                    });
                }
            }
            CommandAction::AcceptInvite => {
                if let Some(room) = self.room_list.selected_room() {
                    if matches!(room.category, crate::state::RoomCategory::Invitation) {
                        self.pending_accept_invite = Some(room.id.clone());
                    } else {
                        self.last_error = Some("Selected room is not an invitation".to_string());
                    }
                } else {
                    self.last_error = Some("No room selected".to_string());
                }
            }
            CommandAction::DeclineInvite => {
                if let Some(room) = self.room_list.selected_room() {
                    if matches!(room.category, crate::state::RoomCategory::Invitation) {
                        self.pending_decline_invite = Some(room.id.clone());
                    } else {
                        self.last_error = Some("Selected room is not an invitation".to_string());
                    }
                } else {
                    self.last_error = Some("No room selected".to_string());
                }
            }
            CommandAction::InviteUser(user) => {
                if let Some(room_id) = self.messages.current_room_id.clone() {
                    self.pending_invite_user = Some((room_id, user));
                } else {
                    self.last_error = Some("No room selected".to_string());
                }
            }
        }
    }

    pub(crate) fn dispatch_which_key_action(
        &mut self,
        cat: crate::ui::which_key::WhichKeyCategory,
        key: char,
    ) {
        use crate::input::CommandAction;
        use crate::ui::which_key::WhichKeyCategory;

        match cat {
            WhichKeyCategory::Room => match key {
                'j' => self.vim.enter_command_with("join "),
                'l' => self.handle_command(CommandAction::Leave),
                'c' => self.handle_command(CommandAction::CreateRoom),
                'e' => self.handle_command(CommandAction::RoomInfo),
                'd' => self.vim.enter_command_with("dm "),
                'i' => self.vim.enter_command_with("invite "),
                _ => {}
            },
            WhichKeyCategory::Call => match key {
                'c' => self.handle_command(CommandAction::Call),
                'a' => self.handle_command(CommandAction::Answer),
                'd' => self.handle_command(CommandAction::Reject),
                'h' => self.handle_command(CommandAction::Hangup),
                _ => {}
            },
            WhichKeyCategory::Effects => match key {
                'r' => self.handle_command(CommandAction::Rain),
                'g' => self.handle_command(CommandAction::Glitch),
                _ => {}
            },
            WhichKeyCategory::User => match key {
                'p' => self.handle_command(CommandAction::Configure),
                'a' => self.handle_command(CommandAction::AudioSettings),
                _ => {}
            },
            WhichKeyCategory::Security => match key {
                'r' => self.handle_command(CommandAction::Recovery),
                'v' => self.handle_command(CommandAction::Verify(None)),
                'p' => self.handle_command(CommandAction::ChangePassword),
                _ => {}
            },
        }
    }
}
