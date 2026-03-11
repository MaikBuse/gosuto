use super::*;

impl App {
    pub fn handle_event(&mut self, event: AppEvent) {
        // Clear transient errors on any key press
        if matches!(event, AppEvent::Key(_)) {
            self.last_error = None;
            self.anim_clock.reset_cursor();
        }

        match event {
            AppEvent::Key(key) => {
                if !self.auth.is_logged_in() {
                    self.handle_login_key(key);
                } else if self.change_password.open {
                    self.handle_change_password_key(key);
                } else if self.user_config.open {
                    self.handle_user_config_key(key);
                } else if self.room_info.open {
                    self.handle_room_info_key(key);
                } else if self.create_room.open {
                    self.handle_create_room_key(key);
                } else if self.audio_settings.open {
                    self.handle_audio_settings_key(key);
                } else if self.verification_modal.is_some() {
                    self.handle_verify_modal_key(key);
                } else if self.recovery.is_some() {
                    self.handle_recovery_key(key);
                } else if self.invite_prompt_room.is_some() {
                    self.handle_invite_prompt_key(key);
                } else if self.reaction_picker.is_some() {
                    self.handle_reaction_picker_key(key);
                } else if self.which_key.is_some() {
                    self.handle_which_key(key);
                } else {
                    let result = input::handle_key(key, &mut self.vim);
                    self.process_input(result);
                }
            }
            AppEvent::KeyRelease => {}
            AppEvent::PttKeyCaptured(name) => {
                if self.audio_settings.capturing_ptt_key {
                    self.audio_settings.push_to_talk_key = Some(name.clone());
                    self.audio_settings.capturing_ptt_key = false;
                    if let Some(ref handle) = self.global_ptt {
                        *handle.ptt_key.lock() = name;
                    }
                }
            }
            AppEvent::PttListenerFailed(message) => {
                self.audio_settings.ptt_error = Some(message);
            }
            AppEvent::MicLevel(level) => {
                self.audio_settings.mic_level = level;
            }
            AppEvent::Resize => {}
            AppEvent::Tick => {}
            AppEvent::LoginSuccess {
                user_id,
                device_id,
                homeserver,
            } => {
                // Save credentials to keyring if we have a password (skip on session restore)
                if !self.login.password.is_empty() {
                    crate::matrix::credentials::save_credentials(
                        &self.login.homeserver,
                        &self.login.username,
                        &self.login.password,
                    );
                }
                self.login.password.clear();
                self.login.confirm_password.clear();
                self.auth = AuthState::LoggedIn {
                    user_id,
                    device_id,
                    homeserver,
                };
                self.sync_status = "syncing...".to_string();
                self.room_list.loading = true;
            }
            AppEvent::LoginFailure(err) => {
                if matches!(self.auth, AuthState::AutoLoggingIn) {
                    self.login.password.clear();
                    self.auth = AuthState::Error(format!("Auto-login failed: {err}"));
                } else {
                    self.auth = AuthState::Error(err);
                }
            }
            AppEvent::RegisterFailure(err) => {
                self.auth = AuthState::Error(err);
            }
            AppEvent::LoggedOut => {
                let was_logged_in = self.auth.is_logged_in();
                self.room_list = RoomListState::new();
                self.messages = MessageState::new();
                self.members_list = MemberListState::new();
                self.login = LoginState::new();
                self.sync_status = "disconnected".to_string();
                self.self_verified = false;
                self.recovery_status = crate::event::RecoveryStatus::Disabled;
                self.typing_users.clear();
                self.last_typing_sent = None;
                self.pending_typing_notice = None;

                if self.pending_credential_clear {
                    self.pending_credential_clear = false;
                    crate::matrix::credentials::delete_credentials();
                    self.auth = AuthState::LoggedOut;
                } else if was_logged_in {
                    self.auth =
                        AuthState::Error("Session expired — please log in again".to_string());
                } else if !self.auto_login_attempted
                    && let Some(creds) = crate::matrix::credentials::load_credentials()
                {
                    self.event_tx
                        .send(AppEvent::AutoLogin {
                            homeserver: creds.homeserver,
                            username: creds.username,
                            password: creds.password,
                        })
                        .warn_closed("AutoLogin");
                } else {
                    self.auth = AuthState::LoggedOut;
                }
            }
            AppEvent::AutoLogin {
                homeserver,
                username,
                password,
            } => {
                if !self.auto_login_attempted {
                    self.auto_login_attempted = true;
                    self.login.homeserver = homeserver;
                    self.login.username = username;
                    self.login.password = password;
                    self.login.cursor_pos = self.login.active_buffer().len();
                    self.auth = AuthState::AutoLoggingIn;
                }
            }
            AppEvent::RoomListUpdated(rooms) => {
                self.room_list.set_rooms(rooms);
                // Clear unread badge for the room we're currently viewing
                if let Some(ref current_id) = self.messages.current_room_id {
                    self.room_list.clear_unread(current_id);
                }
            }
            AppEvent::NewMessage {
                room_id,
                mut message,
            } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    if let Some(ref mut reply) = message.in_reply_to
                        && let Some(original) = self
                            .messages
                            .messages
                            .iter()
                            .find(|m| m.event_id == reply.event_id)
                    {
                        reply.sender = original.sender.clone();
                        reply.body_preview = truncate_preview(original.body_text(), 50);
                    }
                    let eid = message.event_id.clone();
                    self.messages.add_message(message);
                    self.pending_read_receipt = Some((room_id.clone(), Some(eid)));
                }
            }
            AppEvent::MessagesLoaded {
                room_id,
                messages,
                has_more,
            } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    let last_event_id = messages.last().map(|m| m.event_id.clone());
                    self.messages.prepend_messages(messages, has_more);
                    // Resolve reply info from local messages
                    let resolutions: HashMap<String, (String, String)> = self
                        .messages
                        .messages
                        .iter()
                        .filter(|m| !m.event_id.is_empty())
                        .map(|m| {
                            (
                                m.event_id.clone(),
                                (m.sender.clone(), truncate_preview(m.body_text(), 50)),
                            )
                        })
                        .collect();
                    for msg in &mut self.messages.messages {
                        if let Some(ref mut reply) = msg.in_reply_to
                            && reply.sender.is_empty()
                            && let Some((sender, preview)) = resolutions.get(&reply.event_id)
                        {
                            reply.sender.clone_from(sender);
                            reply.body_preview.clone_from(preview);
                        }
                    }
                    self.pending_read_receipt = Some((room_id.clone(), last_event_id));
                }
            }
            AppEvent::MessageSent {
                room_id,
                event_id,
                body,
            } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.messages.confirm_sent(&body, &event_id);
                }
            }
            AppEvent::MessageEdited {
                room_id,
                target_event_id,
                new_content,
            } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.messages
                        .update_message_content(&target_event_id, new_content);
                }
            }
            AppEvent::SendError { error, .. } => {
                self.last_error = Some(error);
            }
            AppEvent::FetchError { room_id, error } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.messages.set_fetch_error(error);
                }
            }
            AppEvent::MembersLoaded { room_id, members } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.members_list.set_members(&room_id, members);
                    self.members_title_reveal.trigger();
                }
            }
            AppEvent::DmRoomReady { room_id } => {
                self.messages.set_room(Some(room_id));
                self.vim.focus = FocusPanel::Messages;
                self.chat_title_reveal.trigger();
            }
            AppEvent::RoomCreated { room_id } => {
                self.create_room.open = false;
                self.create_room.creating = false;
                self.messages.set_room(Some(room_id));
                self.vim.focus = FocusPanel::Messages;
                self.chat_title_reveal.trigger();
            }
            AppEvent::SyncError(err) => {
                if self.create_room.creating {
                    self.create_room.creating = false;
                }
                self.last_error = Some(err);
            }
            AppEvent::SyncStatus(status) => {
                self.sync_status = status;
            }
            AppEvent::SyncTokenUpdated(token) => {
                if self.sync_token.is_none() {
                    self.pending_user_config = true;
                }
                self.sync_token = Some(token);
            }
            // VoIP events (MatrixRTC)
            AppEvent::CallMemberJoined { room_id, user_id } => {
                // Update room call members for sidebar display
                self.room_list
                    .room_call_members
                    .entry(room_id.clone())
                    .or_default()
                    .insert(user_id.clone());
                self.room_list.rebuild_display_rows();

                // If we're already in a call, ignore ringing logic
                if self.call_info.is_some() {
                    return;
                }
                // Ignore call events during initial sync (before first sync token)
                if self.sync_token.is_none() {
                    return;
                }
                // If it's us, ignore
                if let AuthState::LoggedIn {
                    user_id: ref our_id,
                    ..
                } = self.auth
                    && user_id == *our_id
                {
                    return;
                }
                // Resolve room name from room list
                let room_name = self
                    .room_list
                    .rooms
                    .iter()
                    .find(|r| r.id == room_id)
                    .map(|r| r.name.clone());
                // Someone started a call — show ringing UI
                self.incoming_call_room = Some(room_id);
                self.incoming_call_user = Some(user_id);
                self.incoming_call_room_name = room_name;
            }
            AppEvent::CallMemberLeft { room_id, user_id } => {
                // Update room call members for sidebar display
                if let Some(members) = self.room_list.room_call_members.get_mut(&room_id) {
                    members.remove(&user_id);
                    if members.is_empty() {
                        self.room_list.room_call_members.remove(&room_id);
                    }
                }
                self.room_list.rebuild_display_rows();

                // If it was the incoming caller, clear ringing state
                if self.incoming_call_room.as_deref() == Some(&room_id)
                    && self.incoming_call_user.as_deref() == Some(&user_id)
                {
                    self.incoming_call_room = None;
                    self.incoming_call_user = None;
                    self.incoming_call_room_name = None;
                }
                // Update participants if in active call
                if let Some(ref mut info) = self.call_info {
                    info.participants.retain(|p| p != &user_id);
                }
            }
            AppEvent::CallParticipantUpdate { participants } => {
                if let Some(ref mut info) = self.call_info {
                    info.participants = participants;
                }
            }
            AppEvent::CallStateChanged { room_id, state } => {
                if let Some(ref mut info) = self.call_info
                    && info.room_id == room_id
                {
                    if state == CallState::Active && info.started_at.is_none() {
                        info.started_at = Some(Instant::now());
                    }
                    info.state = state;
                }
            }
            // Room info events
            AppEvent::RoomInfoLoaded {
                room_id,
                name,
                topic,
                history_visibility,
                encrypted,
            } => {
                if self.room_info.open && self.room_info.room_id == room_id {
                    self.room_info.name = name;
                    self.room_info.topic = topic;
                    self.room_info.history_visibility = history_visibility;
                    self.room_info.encryption_selection =
                        if encrypted { "yes" } else { "no" }.to_string();
                    self.room_info.encrypted = encrypted;
                    self.room_info.loading = false;
                }
            }
            AppEvent::RoomSettingUpdated { room_id } => {
                if self.room_info.open && self.room_info.room_id == room_id {
                    // If we just saved a name, update it in state
                    if !self.room_info.name_buffer.is_empty() {
                        self.room_info.name = Some(self.room_info.name_buffer.clone());
                        self.room_info.name_buffer.clear();
                    }
                    // If we just saved a topic, update it in state
                    if self.room_info.topic_save_pending {
                        if self.room_info.topic_buffer.is_empty() {
                            self.room_info.topic = None;
                        } else {
                            self.room_info.topic = Some(self.room_info.topic_buffer.clone());
                        }
                        self.room_info.topic_buffer.clear();
                        self.room_info.topic_save_pending = false;
                    }
                    // If encryption was just enabled, reflect it
                    if self.room_info.encryption_selection == "yes" && !self.room_info.encrypted {
                        self.room_info.encrypted = true;
                    }
                    self.room_info.saving = false;
                }
            }
            AppEvent::RoomSettingError { error } => {
                self.room_info.saving = false;
                self.last_error = Some(error);
            }
            // User config events
            AppEvent::UserConfigLoaded {
                display_name,
                verified,
                recovery_status,
            } => {
                if verified {
                    self.self_verified = true;
                }
                self.recovery_status = recovery_status;
                self.user_config.loading = false;
                if self.user_config.open {
                    self.user_config.display_name = display_name;
                    self.user_config.verified = verified || self.self_verified;
                    self.user_config.recovery_status = recovery_status;
                }
            }
            AppEvent::UserConfigUpdated => {
                if self.change_password.open {
                    self.change_password.open = false;
                    self.change_password.saving = false;
                }
                if self.user_config.open {
                    if !self.user_config.display_name_buffer.is_empty() {
                        self.user_config.display_name =
                            Some(self.user_config.display_name_buffer.clone());
                        self.user_config.display_name_buffer.clear();
                    }
                    self.user_config.saving = false;
                }
            }
            AppEvent::UserConfigError(error) => {
                self.change_password.saving = false;
                self.user_config.saving = false;
                self.user_config.loading = false;
                self.last_error = Some(error);
            }
            AppEvent::CallError(err) => {
                self.last_error = Some(err);
                self.call_info = None;
                self.set_global_ptt_active(false);
            }
            AppEvent::CallEnded => {
                self.call_info = None;
                self.set_global_ptt_active(false);
            }
            // Recovery events
            AppEvent::RecoveryStateChecked(stage) => {
                if let Some(ref mut modal) = self.recovery {
                    modal.stage = stage;
                }
            }
            AppEvent::RecoveryKeyReady(key) => {
                if let Some(ref mut modal) = self.recovery {
                    modal.stage = RecoveryStage::ShowKey(key);
                    modal.copied = false;
                }
                self.pending_refetch = true;
            }
            AppEvent::RecoveryHealingProgress(step) => {
                if let Some(ref mut modal) = self.recovery {
                    modal.stage = RecoveryStage::Healing(step);
                }
            }
            AppEvent::RecoveryRecovered => {
                if let Some(ref mut modal) = self.recovery {
                    modal.stage = RecoveryStage::Enabled;
                }
                self.pending_refetch = true;
            }
            AppEvent::RecoveryNeedPassword(sender) => {
                if let Some(ref mut modal) = self.recovery {
                    modal.stage = RecoveryStage::NeedPassword;
                    modal.password_tx = sender.take();
                    modal.password_buffer.clear();
                }
            }
            AppEvent::RecoveryError(err) => {
                if let Some(ref mut modal) = self.recovery {
                    modal.stage = RecoveryStage::Error(err);
                }
            }
            // Verification events
            AppEvent::VerificationRequestReceived { sender } => {
                if self.verification_modal.is_none() {
                    self.verification_modal = Some(crate::state::VerificationModalState {
                        stage: crate::state::VerificationStage::WaitingForOtherDevice,
                        sender,
                        emojis: vec![],
                        user_id_buffer: String::new(),
                    });
                }
            }
            AppEvent::VerificationSasEmoji { emojis, sender } => {
                self.verification_modal = Some(crate::state::VerificationModalState {
                    stage: crate::state::VerificationStage::EmojiConfirmation,
                    sender,
                    emojis,
                    user_id_buffer: String::new(),
                });
            }
            AppEvent::VerificationCompleted => {
                if let Some(ref mut modal) = self.verification_modal {
                    modal.stage = crate::state::VerificationStage::Completed;
                }
                self.self_verified = true;
                self.pending_refetch = true;
                self.user_config.verified = true;
            }
            AppEvent::VerificationCancelled { reason } => {
                if let Some(ref mut modal) = self.verification_modal {
                    modal.stage = crate::state::VerificationStage::Failed(reason);
                }
            }
            AppEvent::CrossSigningResetCompleted => {
                self.last_error = Some("Cross-signing keys reset successfully".to_string());
                self.pending_refetch = true;
            }
            AppEvent::CrossSigningResetError(err) => {
                self.last_error = Some(err);
            }
            AppEvent::VerificationError(err) => {
                if self.verification_modal.is_some() {
                    if let Some(ref mut modal) = self.verification_modal {
                        modal.stage = crate::state::VerificationStage::Failed(err);
                    }
                } else {
                    self.last_error = Some(err);
                }
            }
            AppEvent::MemberVerificationStatus {
                room_id,
                user_id,
                verified,
            } => {
                if self.members_list.current_room_id.as_deref() == Some(&room_id)
                    && let Some(member) = self
                        .members_list
                        .members
                        .iter_mut()
                        .find(|m| m.user_id == user_id)
                {
                    member.verified = Some(verified);
                }
            }
            // Invitation events
            AppEvent::InviteAccepted { room_id } => {
                self.messages.set_room(Some(room_id));
                self.vim.focus = FocusPanel::Messages;
                self.chat_title_reveal.trigger();
                self.pending_refetch = true;
            }
            AppEvent::InviteDeclined => {
                self.pending_refetch = true;
            }
            AppEvent::UserInvited { user_id } => {
                self.last_error = Some(format!("Invited {}", user_id));
            }
            AppEvent::InviteError { error } => {
                self.last_error = Some(error);
            }
            // Reaction events
            AppEvent::ReactionReceived {
                room_id,
                target_event_id,
                reaction_event_id,
                emoji_key,
                sender,
            }
            | AppEvent::ReactionSent {
                room_id,
                target_event_id,
                reaction_event_id,
                emoji_key,
                sender,
            } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.messages.add_reaction(
                        &target_event_id,
                        &emoji_key,
                        &sender,
                        &reaction_event_id,
                    );
                }
            }
            AppEvent::ReactionRedacted {
                room_id,
                reaction_event_id,
            } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.messages
                        .remove_reaction_by_event_id(&reaction_event_id);
                }
            }
            // Image events
            AppEvent::ImageLoaded {
                event_id,
                image_data,
            } => {
                if self.image_cache.is_loaded(&event_id) || self.image_cache.is_failed(&event_id) {
                    return;
                }
                let picker = self.picker.clone();
                let tx = self.image_decode_tx.clone();
                tokio::task::spawn_blocking(move || {
                    let result = image::load_from_memory(&image_data)
                        .map(|img| {
                            let (w, h) = (img.width(), img.height());
                            (picker.new_resize_protocol(img), w, h)
                        })
                        .map_err(|e| e.to_string());
                    if tx.send((event_id, result)).is_err() {
                        tracing::warn!("image decode result: receiver dropped");
                    }
                });
            }
            AppEvent::ImageFailed { event_id, error } => {
                error!("Image download failed for {}: {}", event_id, error);
                self.image_cache.mark_failed(&event_id);
            }
            // Typing events
            AppEvent::TypingUsersUpdated { room_id, user_ids } => {
                let own_id = match &self.auth {
                    AuthState::LoggedIn { user_id, .. } => Some(user_id.as_str()),
                    _ => None,
                };
                let display_names: Vec<String> = user_ids
                    .iter()
                    .filter(|uid| own_id != Some(uid.as_str()))
                    .map(|uid| {
                        // Resolve to display name from loaded members
                        self.members_list
                            .members
                            .iter()
                            .find(|m| m.user_id == *uid)
                            .map(|m| m.display_name.clone())
                            .unwrap_or_else(|| {
                                // Fall back to localpart
                                uid.strip_prefix('@')
                                    .and_then(|s| s.split(':').next())
                                    .unwrap_or(uid)
                                    .to_string()
                            })
                    })
                    .collect();
                if display_names.is_empty() {
                    self.typing_users.remove(&room_id);
                } else {
                    self.typing_users.insert(room_id, display_names);
                }
            }
        }
    }
}
