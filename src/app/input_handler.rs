use super::*;

impl App {
    /// Ensure selection state is consistent with the current focus panel.
    /// Call after any focus change.
    fn sync_message_selection(&mut self) {
        if self.vim.focus == FocusPanel::Messages {
            if !self.messages.is_selecting() {
                self.messages.select_newest();
            }
        } else {
            self.messages.deselect();
        }
    }

    fn maybe_trigger_load_more(&mut self) {
        if self.messages.selected_index == Some(0)
            && self.messages.has_more
            && !self.messages.loading
        {
            self.messages.loading = true;
            self.pending_load_more = true;
        }
    }

    pub(crate) fn process_input(&mut self, result: InputResult) {
        match result {
            InputResult::None => {}
            InputResult::Quit | InputResult::Command(CommandAction::Quit) => {
                self.running = false;
            }
            InputResult::Escape => {
                self.reply_context = None;
                self.edit_context = None;
            }
            InputResult::MoveUp => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_up(),
                FocusPanel::Messages => {
                    self.messages.select_up();
                    self.maybe_trigger_load_more();
                }
                FocusPanel::Members => self.members_list.move_up(),
            },
            InputResult::MoveDown => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_down(),
                FocusPanel::Messages => self.messages.select_down(),
                FocusPanel::Members => self.members_list.move_down(),
            },
            InputResult::MoveTop => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_top(),
                FocusPanel::Messages => {
                    self.messages.select_top();
                    self.maybe_trigger_load_more();
                }
                FocusPanel::Members => self.members_list.move_top(),
            },
            InputResult::MoveBottom => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_bottom(),
                FocusPanel::Messages => self.messages.select_bottom(),
                FocusPanel::Members => self.members_list.move_bottom(),
            },
            InputResult::Select => {
                if self.vim.focus == FocusPanel::RoomList {
                    use crate::state::DisplayRow;
                    match self.room_list.selected_display_row() {
                        Some(DisplayRow::SpaceHeader { .. }) => {
                            self.room_list.toggle_space();
                        }
                        Some(DisplayRow::Room { .. }) => {
                            self.room_list_anim.trigger_flash(self.room_list.selected);
                            self.effects
                                .emp_pulse
                                .trigger_burst(self.room_list.selected as u16);
                            if let Some(room) = self.room_list.selected_room() {
                                if matches!(room.category, crate::state::RoomCategory::Invitation) {
                                    self.invite_prompt_room = Some(room.id.clone());
                                } else {
                                    let room_id = room.id.clone();
                                    self.messages.set_room(Some(room_id));
                                    self.chat_title_reveal.trigger();
                                }
                            }
                        }
                        _ => {} // SectionHeader: no-op
                    }
                } else if self.vim.focus == FocusPanel::Members
                    && let Some(member) = self.members_list.selected_member()
                {
                    if let AuthState::LoggedIn { ref user_id, .. } = self.auth
                        && member.user_id == *user_id
                    {
                        self.last_error = Some("Cannot DM yourself".to_string());
                        return;
                    }
                    self.effects
                        .members_emp_pulse
                        .trigger_burst(self.members_list.selected as u16);
                    self.pending_dm = Some(member.user_id.clone());
                }
            }
            InputResult::CallMember => {
                if self.call_info.is_some() {
                    // Toggle: c during active call = hangup
                    self.handle_command(CommandAction::Hangup);
                } else {
                    // Initiate call in current room
                    self.handle_command(CommandAction::Call);
                }
            }
            InputResult::AnswerCall => {
                self.handle_command(CommandAction::Answer);
            }
            InputResult::RejectCall => {
                self.handle_command(CommandAction::Reject);
            }
            InputResult::SwitchPanel => {
                self.vim.focus = match self.vim.focus {
                    FocusPanel::RoomList => FocusPanel::Messages,
                    FocusPanel::Messages => FocusPanel::Members,
                    FocusPanel::Members => FocusPanel::RoomList,
                };
                self.sync_message_selection();
            }
            InputResult::FocusRight => {
                self.vim.focus = match self.vim.focus {
                    FocusPanel::RoomList => FocusPanel::Messages,
                    FocusPanel::Messages => FocusPanel::Members,
                    FocusPanel::Members => FocusPanel::Members,
                };
                self.sync_message_selection();
            }
            InputResult::FocusLeft => {
                self.vim.focus = match self.vim.focus {
                    FocusPanel::RoomList => FocusPanel::RoomList,
                    FocusPanel::Messages => FocusPanel::RoomList,
                    FocusPanel::Members => FocusPanel::Messages,
                };
                self.sync_message_selection();
            }
            InputResult::TypingActivity => {
                let should_send = self
                    .last_typing_sent
                    .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(4));
                if should_send && let Some(room_id) = self.messages.current_room_id.clone() {
                    self.pending_typing_notice = Some((room_id, true));
                    self.last_typing_sent = Some(Instant::now());
                }
            }
            InputResult::SendMessage(msg) => {
                if let Some(room_id) = self.messages.current_room_id.clone() {
                    self.pending_typing_notice = Some((room_id, false));
                    self.last_typing_sent = None;
                }
                self.send_message(msg);
                self.vim.enter_normal();
            }
            InputResult::Command(action) => self.handle_command(action),
            InputResult::Search(query) => {
                let filter = if query.is_empty() { None } else { Some(query) };
                self.room_list.set_filter(filter);
            }
            InputResult::ClearSearch => {
                self.room_list.set_filter(None);
            }
            InputResult::ShowWhichKey => {
                self.which_key = Some(None);
            }
            InputResult::ReplyToSelected => {
                if let Some(idx) = self.messages.selected_index
                    && let Some(msg) = self.messages.messages.get(idx)
                {
                    if msg.event_id.is_empty() {
                        return; // can't reply to pending messages
                    }
                    self.reply_context = Some(ReplyContext {
                        event_id: msg.event_id.clone(),
                        sender: msg.sender.clone(),
                        body_preview: truncate_preview(msg.body_text(), 50),
                    });
                    self.messages.deselect();
                    self.vim.enter_insert();
                }
            }
            InputResult::EditSelected => {
                if let Some(idx) = self.messages.selected_index
                    && let Some(msg) = self.messages.messages.get(idx)
                {
                    if msg.event_id.is_empty() {
                        return; // can't edit pending messages
                    }
                    let own_id = match &self.auth {
                        AuthState::LoggedIn { user_id, .. } => user_id.clone(),
                        _ => return,
                    };
                    if msg.sender != own_id {
                        self.last_error = Some("Can only edit your own messages".to_string());
                        return;
                    }
                    let body = match &msg.content {
                        crate::state::MessageContent::Text { plain, .. } => plain.clone(),
                        _ => {
                            self.last_error = Some("Can only edit text messages".to_string());
                            return;
                        }
                    };
                    self.edit_context = Some(EditContext {
                        event_id: msg.event_id.clone(),
                        original_body: body.clone(),
                    });
                    self.vim.input_buffer = body;
                    self.vim.input_cursor = self.vim.input_buffer.len();
                    self.messages.deselect();
                    self.vim.enter_insert();
                }
            }
            InputResult::ReactToSelected => {
                if let Some(idx) = self.messages.selected_index
                    && let Some(msg) = self.messages.messages.get(idx)
                {
                    if msg.event_id.is_empty() {
                        return; // can't react to pending messages
                    }
                    let own_id = match &self.auth {
                        AuthState::LoggedIn { user_id, .. } => user_id.clone(),
                        _ => return,
                    };
                    let existing_own_reactions: Vec<String> = msg
                        .reactions
                        .iter()
                        .filter(|r| r.senders.iter().any(|s| s.user_id == own_id))
                        .map(|r| r.key.clone())
                        .collect();
                    self.reaction_picker = Some(ReactionPickerState {
                        event_id: msg.event_id.clone(),
                        quick_pick_index: 0,
                        existing_own_reactions,
                        in_grid: false,
                        grid_index: 0,
                        filter: String::new(),
                        filter_active: false,
                        scroll_offset: 0,
                    });
                }
            }
            InputResult::VerifyMember => {
                if self.verification_modal.is_some() {
                    self.last_error = Some("A verification is already in progress".to_string());
                } else if let Some(member) = self.members_list.selected_member() {
                    let uid = member.user_id.clone();
                    self.verification_modal = Some(crate::state::VerificationModalState {
                        stage: crate::state::VerificationStage::WaitingForOtherDevice,
                        sender: uid.clone(),
                        emojis: vec![],
                        user_id_buffer: String::new(),
                    });
                    self.pending_verify = Some(Some(uid));
                }
            }
        }
    }
}
