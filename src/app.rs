use std::time::Instant;

use crossterm::event::{KeyCode, KeyModifiers};
use tracing::info;

use crate::config::WalrustConfig;
use crate::event::{AppEvent, EventSender};
use crate::input::{self, CommandAction, FocusPanel, InputResult, VimState};
use crate::state::{AuthState, MemberListState, MessageState, RoomListState};
use crate::ui::login::LoginState;
use crate::voip::{CallCommand, CallCommandSender, CallInfo, CallState};

#[allow(dead_code)]
pub struct App {
    pub running: bool,
    pub vim: VimState,
    pub auth: AuthState,
    pub room_list: RoomListState,
    pub messages: MessageState,
    pub members_list: MemberListState,
    pub login: LoginState,
    pub sync_status: String,
    pub last_error: Option<String>,
    pub event_tx: EventSender,
    pub config: WalrustConfig,
    // Pending actions for main loop to process
    pub pending_logout: bool,
    pending_send: Option<(String, String)>,  // (room_id, body)
    pending_join: Option<String>,            // room_id_or_alias
    pending_leave: Option<String>,           // room_id
    pending_dm: Option<String>,              // user_id
    // VoIP
    pub call_info: Option<CallInfo>,
    pub call_cmd_tx: Option<CallCommandSender>,
    // Auto-login
    pub auto_login_attempted: bool,
    pub pending_credential_clear: bool,
}

impl App {
    pub fn new(event_tx: EventSender, config: WalrustConfig) -> Self {
        Self {
            running: true,
            vim: VimState::new(),
            auth: AuthState::LoggedOut,
            room_list: RoomListState::new(),
            messages: MessageState::new(),
            members_list: MemberListState::new(),
            login: LoginState::new(),
            sync_status: "disconnected".to_string(),
            last_error: None,
            event_tx,
            config,
            pending_logout: false,
            pending_send: None,
            pending_join: None,
            pending_leave: None,
            pending_dm: None,
            call_info: None,
            call_cmd_tx: None,
            auto_login_attempted: false,
            pending_credential_clear: false,
        }
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        // Clear transient errors on any key press
        if matches!(event, AppEvent::Key(_)) {
            self.last_error = None;
        }

        match event {
            AppEvent::Key(key) => {
                if !self.auth.is_logged_in() {
                    self.handle_login_key(key);
                } else {
                    let result = input::handle_key(key, &mut self.vim);
                    self.process_input(result);
                }
            }
            AppEvent::Resize(_, _) => {}
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
                self.auto_login_attempted = false;
                self.auth = AuthState::LoggedIn {
                    user_id,
                    device_id,
                    homeserver,
                };
                self.sync_status = "syncing...".to_string();
            }
            AppEvent::LoginFailure(err) => {
                if matches!(self.auth, AuthState::AutoLoggingIn) {
                    self.login.password.clear();
                    self.auth = AuthState::Error(format!("Auto-login failed: {err}"));
                } else {
                    self.auth = AuthState::Error(err);
                }
            }
            AppEvent::LoggedOut => {
                self.auth = AuthState::LoggedOut;
                self.room_list = RoomListState::new();
                self.messages = MessageState::new();
                self.members_list = MemberListState::new();
                self.login = LoginState::new();
                self.sync_status = "disconnected".to_string();

                if self.pending_credential_clear {
                    self.pending_credential_clear = false;
                    crate::matrix::credentials::delete_credentials();
                } else if !self.auto_login_attempted
                    && let Some(creds) = crate::matrix::credentials::load_credentials()
                {
                    let _ = self.event_tx.send(AppEvent::AutoLogin {
                        homeserver: creds.homeserver,
                        username: creds.username,
                        password: creds.password,
                    });
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
                    self.auth = AuthState::AutoLoggingIn;
                }
            }
            AppEvent::RoomListUpdated(rooms) => {
                self.room_list.set_rooms(rooms);
            }
            AppEvent::NewMessage { room_id, message } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.messages.add_message(message);
                }
            }
            AppEvent::MessagesLoaded {
                room_id,
                messages,
                has_more,
            } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
                    self.messages.prepend_messages(messages, has_more);
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
                }
            }
            AppEvent::DmRoomReady { room_id } => {
                self.messages.set_room(Some(room_id));
                self.vim.focus = FocusPanel::Messages;
            }
            AppEvent::SyncError(err) => {
                self.last_error = Some(err);
            }
            AppEvent::SyncStatus(status) => {
                self.sync_status = status;
            }
            // VoIP events
            AppEvent::CallInvite {
                call_id,
                room_id,
                sender,
                sdp,
            } => {
                if self.call_info.is_some() {
                    // Busy - auto-reject via CallManager
                    info!("Auto-rejecting call from {} (busy)", sender);
                    if let Some(ref tx) = self.call_cmd_tx {
                        let _ = tx.send(CallCommand::RejectIncoming {
                            call_id,
                            room_id,
                        });
                    }
                } else {
                    self.call_info = Some(CallInfo::new_incoming(
                        call_id.clone(),
                        room_id.clone(),
                        sender,
                    ));
                    if let Some(ref tx) = self.call_cmd_tx {
                        let _ = tx.send(CallCommand::RemoteInvite {
                            call_id,
                            room_id,
                            sdp,
                        });
                    }
                }
            }
            AppEvent::CallAnswer {
                call_id,
                room_id: _,
                sdp,
            } => {
                if let Some(ref tx) = self.call_cmd_tx {
                    let _ = tx.send(CallCommand::RemoteAnswer { call_id, sdp });
                }
            }
            AppEvent::CallCandidates {
                call_id,
                room_id: _,
                candidates,
            } => {
                if let Some(ref tx) = self.call_cmd_tx {
                    let _ = tx.send(CallCommand::RemoteCandidates {
                        call_id,
                        candidates,
                    });
                }
            }
            AppEvent::CallHangup {
                call_id,
                room_id: _,
            } => {
                if let Some(ref tx) = self.call_cmd_tx {
                    let _ = tx.send(CallCommand::RemoteHangup { call_id });
                }
            }
            AppEvent::CallStateChanged { call_id, state } => {
                if let Some(ref mut info) = self.call_info {
                    if info.call_id == call_id {
                        if state == CallState::Active && info.started_at.is_none() {
                            info.started_at = Some(Instant::now());
                        }
                        info.state = state;
                    }
                }
            }
            AppEvent::CallError(err) => {
                self.last_error = Some(err);
                self.call_info = None;
            }
            AppEvent::CallEnded => {
                self.call_info = None;
            }
        }
    }

    fn handle_login_key(&mut self, key: crossterm::event::KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }

        if matches!(self.auth, AuthState::LoggingIn | AuthState::AutoLoggingIn) {
            return;
        }

        match key.code {
            KeyCode::Tab => self.login.next_field(),
            KeyCode::BackTab => self.login.prev_field(),
            KeyCode::Enter => self.initiate_login(),
            KeyCode::Backspace => self.login.backspace(),
            KeyCode::Char(c) => self.login.insert_char(c),
            _ => {}
        }
    }

    fn initiate_login(&mut self) {
        if self.login.username.is_empty() || self.login.password.is_empty() {
            self.auth = AuthState::Error("Username and password required".to_string());
            return;
        }
        self.auth = AuthState::LoggingIn;
    }

    fn process_input(&mut self, result: InputResult) {
        match result {
            InputResult::None => {}
            InputResult::Quit | InputResult::Command(CommandAction::Quit) => {
                self.running = false;
            }
            InputResult::MoveUp => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_up(),
                FocusPanel::Messages => self.messages.scroll_up(),
                FocusPanel::Members => self.members_list.move_up(),
            },
            InputResult::MoveDown => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_down(),
                FocusPanel::Messages => self.messages.scroll_down(),
                FocusPanel::Members => self.members_list.move_down(),
            },
            InputResult::MoveTop => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_top(),
                FocusPanel::Messages => {
                    // Scroll to top - could trigger pagination
                    self.messages.scroll_offset = self.messages.messages.len().saturating_sub(1);
                }
                FocusPanel::Members => self.members_list.move_top(),
            },
            InputResult::MoveBottom => match self.vim.focus {
                FocusPanel::RoomList => self.room_list.move_bottom(),
                FocusPanel::Messages => self.messages.scroll_to_bottom(),
                FocusPanel::Members => self.members_list.move_bottom(),
            },
            InputResult::Select => {
                if self.vim.focus == FocusPanel::RoomList {
                    if let Some(room) = self.room_list.selected_room() {
                        let room_id = room.id.clone();
                        self.messages.set_room(Some(room_id));
                        self.vim.focus = FocusPanel::Messages;
                    }
                } else if self.vim.focus == FocusPanel::Members
                    && let Some(member) = self.members_list.selected_member()
                {
                    self.pending_dm = Some(member.user_id.clone());
                }
            }
            InputResult::CallMember => {
                if self.vim.focus == FocusPanel::Members
                    && let Some(member) = self.members_list.selected_member()
                {
                    if self.call_info.is_some() {
                        self.last_error = Some("Already in a call".to_string());
                    } else if let Some(room_id) = self.messages.current_room_id.clone() {
                        let call_id = uuid::Uuid::new_v4().to_string();
                        let user_id = member.user_id.clone();
                        self.call_info = Some(CallInfo::new_outgoing(
                            call_id.clone(),
                            room_id.clone(),
                            user_id,
                        ));
                        if let Some(ref tx) = self.call_cmd_tx {
                            let _ = tx.send(CallCommand::Initiate {
                                call_id,
                                room_id,
                            });
                        }
                    } else {
                        self.last_error = Some("No room selected".to_string());
                    }
                }
            }
            InputResult::SwitchPanel => {
                self.vim.focus = match self.vim.focus {
                    FocusPanel::RoomList => FocusPanel::Messages,
                    FocusPanel::Messages => FocusPanel::Members,
                    FocusPanel::Members => FocusPanel::RoomList,
                };
            }
            InputResult::FocusRight => {
                self.vim.focus = match self.vim.focus {
                    FocusPanel::RoomList => FocusPanel::Messages,
                    FocusPanel::Messages => FocusPanel::Members,
                    FocusPanel::Members => FocusPanel::Members,
                };
            }
            InputResult::FocusLeft => {
                self.vim.focus = match self.vim.focus {
                    FocusPanel::RoomList => FocusPanel::RoomList,
                    FocusPanel::Messages => FocusPanel::RoomList,
                    FocusPanel::Members => FocusPanel::Messages,
                };
            }
            InputResult::SendMessage(msg) => {
                self.send_message(msg);
            }
            InputResult::Command(action) => self.handle_command(action),
            InputResult::Search(query) => {
                let filter = if query.is_empty() { None } else { Some(query) };
                self.room_list.set_filter(filter);
            }
            InputResult::ClearSearch => {
                self.room_list.set_filter(None);
            }
        }
    }

    fn send_message(&mut self, body: String) {
        if let Some(room_id) = self.messages.current_room_id.clone() {
            // Get the actual user_id for the sender display
            let sender = match &self.auth {
                AuthState::LoggedIn { user_id, .. } => user_id.clone(),
                _ => "me".to_string(),
            };

            // Add optimistic message
            let msg = crate::state::DisplayMessage {
                event_id: String::new(),
                sender,
                body: body.clone(),
                timestamp: chrono::Local::now(),
                is_emote: false,
                is_notice: false,
                pending: true,
            };
            self.messages.add_message(msg);
            self.messages.scroll_to_bottom();

            // Queue for main loop to send
            self.pending_send = Some((room_id, body));
        }
    }

    fn handle_command(&mut self, action: CommandAction) {
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
                // DMs are just :join with the user as room target
                self.pending_join = Some(user);
            }
            CommandAction::Call(user) => {
                if self.call_info.is_some() {
                    self.last_error = Some("Already in a call".to_string());
                    return;
                }
                // Find or create DM room for this user, then initiate call
                let call_id = uuid::Uuid::new_v4().to_string();
                if let Some(room) = self.room_list.selected_room() {
                    let room_id = room.id.clone();
                    self.call_info = Some(CallInfo::new_outgoing(
                        call_id.clone(),
                        room_id.clone(),
                        user.clone(),
                    ));
                    if let Some(ref tx) = self.call_cmd_tx {
                        let _ = tx.send(CallCommand::Initiate {
                            call_id,
                            room_id,
                        });
                    }
                } else {
                    self.last_error = Some("Select a room first".to_string());
                }
            }
            CommandAction::Answer => {
                if let Some(ref info) = self.call_info {
                    if info.state == CallState::Ringing {
                        let call_id = info.call_id.clone();
                        if let Some(ref tx) = self.call_cmd_tx {
                            let _ = tx.send(CallCommand::Answer { call_id });
                        }
                    } else {
                        self.last_error = Some("No incoming call to answer".to_string());
                    }
                } else {
                    self.last_error = Some("No incoming call".to_string());
                }
            }
            CommandAction::Reject => {
                if let Some(ref info) = self.call_info {
                    if info.state == CallState::Ringing {
                        let call_id = info.call_id.clone();
                        let room_id = info.room_id.clone();
                        if let Some(ref tx) = self.call_cmd_tx {
                            let _ = tx.send(CallCommand::Reject { call_id, room_id });
                        }
                        self.call_info = None;
                    } else {
                        self.last_error = Some("No incoming call to reject".to_string());
                    }
                } else {
                    self.last_error = Some("No incoming call".to_string());
                }
            }
            CommandAction::Hangup => {
                if let Some(ref info) = self.call_info {
                    let call_id = info.call_id.clone();
                    let room_id = info.room_id.clone();
                    if let Some(ref tx) = self.call_cmd_tx {
                        let _ = tx.send(CallCommand::Hangup { call_id, room_id });
                    }
                    self.call_info = None;
                } else {
                    self.last_error = Some("No active call".to_string());
                }
            }
        }
    }

    pub fn is_logging_in(&self) -> bool {
        matches!(self.auth, AuthState::LoggingIn | AuthState::AutoLoggingIn)
    }

    pub fn login_credentials(&self) -> (String, String, String) {
        (
            self.login.homeserver.clone(),
            self.login.username.clone(),
            self.login.password.clone(),
        )
    }

    pub fn take_pending_send(&mut self) -> Option<(String, String)> {
        self.pending_send.take()
    }

    pub fn take_pending_join(&mut self) -> Option<String> {
        self.pending_join.take()
    }

    pub fn take_pending_leave(&mut self) -> Option<String> {
        self.pending_leave.take()
    }

    pub fn take_pending_dm(&mut self) -> Option<String> {
        self.pending_dm.take()
    }
}
