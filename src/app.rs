use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::error;

use ratatui_image::protocol::StatefulProtocol;

use crate::config::GosutoConfig;
use crate::event::{AppEvent, EventSender};
use crate::input::{self, CommandAction, FocusPanel, InputResult, VimState};
use crate::state::{AuthState, ImageCache, MemberListState, MessageState, RoomListState};
use crate::ui::call_overlay::TransmissionPopup;
use crate::ui::effects::{EffectsState, TextReveal};
use crate::ui::login::LoginState;
use crate::ui::room_list::RoomListAnimState;
use crate::voip::audio::AudioPipeline;
use crate::voip::{CallCommand, CallCommandSender, CallInfo, CallState};

pub const HISTORY_VISIBILITY_OPTIONS: &[&str] = &["shared", "invited", "joined", "world_readable"];

pub struct RoomInfoState {
    pub open: bool,
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub history_visibility: String,
    pub encrypted: bool,
    pub encryption_selection: String,
    pub selected_field: usize,
    pub loading: bool,
    pub saving: bool,
    pub editing_name: bool,
    pub name_buffer: String,
    pub editing_topic: bool,
    pub topic_buffer: String,
    pub topic_save_pending: bool,
}

pub struct CreateRoomState {
    pub open: bool,
    pub selected_field: usize, // 0=name, 1=topic, 2=history, 3=encrypted, 4=create button
    pub name_buffer: String,
    pub editing_name: bool,
    pub topic_buffer: String,
    pub editing_topic: bool,
    pub history_visibility: String,
    pub encrypted: String, // "yes" (default) or "no"
    pub creating: bool,
}

impl CreateRoomState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_field: 0,
            name_buffer: String::new(),
            editing_name: false,
            topic_buffer: String::new(),
            editing_topic: false,
            history_visibility: "shared".to_string(),
            encrypted: "yes".to_string(),
            creating: false,
        }
    }
}

pub struct CreateRoomParams {
    pub name: String,
    pub topic: Option<String>,
    pub history_visibility: String,
    pub encrypted: bool,
}

impl RoomInfoState {
    pub fn new() -> Self {
        Self {
            open: false,
            room_id: String::new(),
            name: None,
            topic: None,
            history_visibility: "shared".to_string(),
            encrypted: false,
            encryption_selection: "no".to_string(),
            selected_field: 0,
            loading: false,
            saving: false,
            editing_name: false,
            name_buffer: String::new(),
            editing_topic: false,
            topic_buffer: String::new(),
            topic_save_pending: false,
        }
    }
}

pub struct AudioSettingsState {
    pub open: bool,
    pub selected_field: usize,
    pub input_devices: Vec<String>,
    pub output_devices: Vec<String>,
    pub input_device_idx: usize,
    pub output_device_idx: usize,
    pub input_volume: f32,
    pub output_volume: f32,
    pub voice_activity: bool,
    pub sensitivity: f32,
    pub push_to_talk: bool,
    pub push_to_talk_key: Option<String>,
    pub capturing_ptt_key: bool,
    pub mic_level: f32,
    pub mic_test_running: Arc<AtomicBool>,
}

impl AudioSettingsState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_field: 0,
            input_devices: vec!["Default".to_string()],
            output_devices: vec!["Default".to_string()],
            input_device_idx: 0,
            output_device_idx: 0,
            input_volume: 1.0,
            output_volume: 1.0,
            voice_activity: false,
            sensitivity: 0.15,
            push_to_talk: false,
            push_to_talk_key: None,
            capturing_ptt_key: false,
            mic_level: 0.0,
            mic_test_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn visible_fields(&self) -> Vec<usize> {
        let mut fields = vec![0, 1, 2, 3, 4];
        if self.voice_activity {
            fields.push(5);
        }
        fields.push(6);
        if self.push_to_talk {
            fields.push(7);
        }
        fields
    }

    pub fn current_field(&self) -> usize {
        let visible = self.visible_fields();
        visible.get(self.selected_field).copied().unwrap_or(0)
    }
}

pub struct UserConfigState {
    pub open: bool,
    pub user_id: String,
    pub device_id: String,
    pub homeserver: String,
    pub display_name: Option<String>,
    pub display_name_buffer: String,
    pub editing_display_name: bool,
    pub verified: bool,
    pub selected_field: usize, // 0=display name, 1=verified
    pub loading: bool,
    pub saving: bool,
}

impl UserConfigState {
    pub fn new() -> Self {
        Self {
            open: false,
            user_id: String::new(),
            device_id: String::new(),
            homeserver: String::new(),
            display_name: None,
            display_name_buffer: String::new(),
            editing_display_name: false,
            verified: false,
            selected_field: 0,
            loading: false,
            saving: false,
        }
    }
}

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
    pub config: GosutoConfig,
    // Pending actions for main loop to process
    pub pending_logout: bool,
    pub pending_refetch: bool,
    pending_send: Option<(String, String)>, // (room_id, body)
    pending_join: Option<String>,           // room_id_or_alias
    pending_leave: Option<String>,          // room_id
    pending_dm: Option<String>,             // user_id
    pending_create_room: Option<CreateRoomParams>,
    pub pending_room_info: bool,
    pub pending_set_visibility: Option<(String, String)>, // (room_id, visibility)
    pub pending_set_room_name: Option<(String, String)>,  // (room_id, new_name)
    pub pending_set_room_topic: Option<(String, String)>, // (room_id, new_topic)
    pub pending_enable_encryption: Option<String>,        // room_id
    pub pending_read_receipt: Option<(String, Option<String>)>, // (room_id, event_id hint)
    // VoIP
    pub call_info: Option<CallInfo>,
    pub call_cmd_tx: Option<CallCommandSender>,
    pub incoming_call_room: Option<String>,
    pub incoming_call_user: Option<String>,
    pub incoming_call_room_name: Option<String>,
    // Auto-login
    pub auto_login_attempted: bool,
    pub pending_credential_clear: bool,
    // Visual effects
    pub effects: EffectsState,
    pub call_popup: TransmissionPopup,
    pub room_list_anim: RoomListAnimState,
    pub chat_title_reveal: TextReveal,
    pub members_title_reveal: TextReveal,
    // Room info
    pub room_info: RoomInfoState,
    // Create room
    pub create_room: CreateRoomState,
    // User config
    pub user_config: UserConfigState,
    pub pending_user_config: bool,
    pub pending_set_display_name: Option<String>,
    // Audio settings
    pub audio_settings: AudioSettingsState,
    pub ptt_transmitting: Arc<AtomicBool>,
    pub sync_token: Option<String>,
    // Verification
    pub verification_modal: Option<crate::state::VerificationModalState>,
    pub pending_verify: Option<Option<String>>,
    pub verify_confirm_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    pub self_verified: bool,
    pub recovery_enabled: bool,
    // Which-key leader popup
    pub which_key: Option<Option<crate::ui::which_key::WhichKeyCategory>>,
    // Recovery
    pub recovery_modal: Option<crate::state::RecoveryModalState>,
    pub pending_recovery: bool,
    pub pending_recovery_create: bool,
    pub pending_recovery_reset: bool,
    pub pending_recovery_recover: Option<String>,
    clipboard: Option<arboard::Clipboard>,
    // Typing indicators
    pub typing_users: HashMap<String, Vec<String>>,
    pub last_typing_sent: Option<Instant>,
    pub pending_typing_notice: Option<(String, bool)>,
    // Inline images
    pub picker: ratatui_image::picker::Picker,
    pub image_cache: ImageCache,
    pub image_decode_tx: std::sync::mpsc::Sender<(String, Result<StatefulProtocol, String>)>,
    pub encode_tx:
        tokio::sync::mpsc::UnboundedSender<(String, StatefulProtocol, ratatui::layout::Rect)>,
}

impl App {
    pub fn new(
        event_tx: EventSender,
        config: GosutoConfig,
        picker: ratatui_image::picker::Picker,
        image_decode_tx: std::sync::mpsc::Sender<(String, Result<StatefulProtocol, String>)>,
        encode_tx: tokio::sync::mpsc::UnboundedSender<(
            String,
            StatefulProtocol,
            ratatui::layout::Rect,
        )>,
    ) -> Self {
        let rain_enabled = config.effects.rain;
        let glitch_enabled = config.effects.glitch;
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
            pending_refetch: false,
            pending_send: None,
            pending_join: None,
            pending_leave: None,
            pending_dm: None,
            pending_create_room: None,
            pending_room_info: false,
            pending_set_visibility: None,
            pending_set_room_name: None,
            pending_set_room_topic: None,
            pending_enable_encryption: None,
            pending_read_receipt: None,
            call_info: None,
            call_cmd_tx: None,
            incoming_call_room: None,
            incoming_call_user: None,
            incoming_call_room_name: None,
            auto_login_attempted: false,
            pending_credential_clear: false,
            effects: EffectsState::new(rain_enabled, glitch_enabled),
            call_popup: TransmissionPopup::new(),
            room_list_anim: RoomListAnimState::new(),
            chat_title_reveal: TextReveal::new(0xC0DE_CAFE_0001),
            members_title_reveal: TextReveal::new(0xC0DE_CAFE_0002),
            room_info: RoomInfoState::new(),
            create_room: CreateRoomState::new(),
            user_config: UserConfigState::new(),
            pending_user_config: false,
            pending_set_display_name: None,
            audio_settings: AudioSettingsState::new(),
            ptt_transmitting: Arc::new(AtomicBool::new(true)), // default: always transmit (no PTT)
            sync_token: None,
            verification_modal: None,
            pending_verify: None,
            verify_confirm_tx: None,
            self_verified: false,
            recovery_enabled: false,
            which_key: None,
            recovery_modal: None,
            pending_recovery: false,
            pending_recovery_create: false,
            pending_recovery_reset: false,
            pending_recovery_recover: None,
            clipboard: arboard::Clipboard::new().ok(),
            typing_users: HashMap::new(),
            last_typing_sent: None,
            pending_typing_notice: None,
            picker,
            image_cache: ImageCache::new(),
            image_decode_tx,
            encode_tx,
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
                } else if self.verification_modal.is_some() {
                    self.handle_verify_modal_key(key);
                } else if self.recovery_modal.is_some() {
                    self.handle_recovery_modal_key(key);
                } else if self.user_config.open {
                    self.handle_user_config_key(key);
                } else if self.room_info.open {
                    self.handle_room_info_key(key);
                } else if self.create_room.open {
                    self.handle_create_room_key(key);
                } else if self.audio_settings.open {
                    self.handle_audio_settings_key(key);
                } else if self.which_key.is_some() {
                    self.handle_which_key(key);
                } else {
                    // PTT: key press sets transmitting
                    if self.config.audio.push_to_talk
                        && self.call_info.is_some()
                        && self.key_matches_ptt(&key)
                    {
                        self.ptt_transmitting.store(true, Ordering::Relaxed);
                    }
                    let result = input::handle_key(key, &mut self.vim);
                    self.process_input(result);
                }
            }
            AppEvent::KeyRelease(key) => {
                // PTT: key release stops transmitting
                if self.config.audio.push_to_talk
                    && self.call_info.is_some()
                    && self.key_matches_ptt(&key)
                {
                    self.ptt_transmitting.store(false, Ordering::Relaxed);
                }
            }
            AppEvent::MicLevel(level) => {
                self.audio_settings.mic_level = level;
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
                self.login.confirm_password.clear();
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
                self.recovery_enabled = false;
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
                    let _ = self.event_tx.send(AppEvent::AutoLogin {
                        homeserver: creds.homeserver,
                        username: creds.username,
                        password: creds.password,
                    });
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
            AppEvent::NewMessage { room_id, message } => {
                if self.messages.current_room_id.as_deref() == Some(&room_id) {
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
            } => {
                if self.user_config.open {
                    self.user_config.display_name = display_name;
                    self.user_config.verified = verified || self.self_verified;
                    self.user_config.loading = false;
                }
            }
            AppEvent::UserConfigUpdated => {
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
                self.user_config.saving = false;
                self.last_error = Some(error);
            }
            AppEvent::CallError(err) => {
                self.last_error = Some(err);
                self.call_info = None;
            }
            AppEvent::CallEnded => {
                self.call_info = None;
            }
            // Verification events
            AppEvent::VerificationRequestReceived { sender, flow_id: _ } => {
                self.verification_modal = Some(crate::state::VerificationModalState {
                    stage: crate::state::VerificationStage::WaitingForOtherDevice,
                    sender,
                    emojis: vec![],
                });
            }
            AppEvent::VerificationSasEmoji {
                emojis,
                flow_id: _,
                sender,
            } => {
                self.verification_modal = Some(crate::state::VerificationModalState {
                    stage: crate::state::VerificationStage::EmojiConfirmation,
                    sender,
                    emojis,
                });
            }
            AppEvent::VerificationCompleted { sender: _ } => {
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
            AppEvent::VerificationError(err) => {
                if self.verification_modal.is_some() {
                    if let Some(ref mut modal) = self.verification_modal {
                        modal.stage = crate::state::VerificationStage::Failed(err);
                    }
                } else {
                    self.last_error = Some(err);
                }
            }
            // Recovery events
            AppEvent::RecoveryState(state_str) => {
                let stage = if state_str.contains("Enabled") || self.recovery_enabled {
                    crate::state::RecoveryStage::Enabled
                } else if state_str.contains("Incomplete") {
                    crate::state::RecoveryStage::Incomplete
                } else {
                    crate::state::RecoveryStage::Setup
                };
                if let Some(ref mut modal) = self.recovery_modal {
                    modal.stage = stage;
                }
            }
            AppEvent::RecoveryKeyReady(key) => {
                if let Some(ref mut modal) = self.recovery_modal {
                    modal.stage = crate::state::RecoveryStage::ShowKey(key);
                    modal.copied = false;
                }
                self.recovery_enabled = true;
                self.self_verified = true;
                self.user_config.verified = true;
            }
            AppEvent::RecoveryRecovered => {
                if let Some(ref mut modal) = self.recovery_modal {
                    modal.stage = crate::state::RecoveryStage::Enabled;
                }
                self.recovery_enabled = true;
                self.self_verified = true;
                self.user_config.verified = true;
                self.pending_refetch = true;
            }
            AppEvent::RecoveryError(err) => {
                if let Some(ref mut modal) = self.recovery_modal {
                    if err.contains("backup already exists") {
                        modal.stage = crate::state::RecoveryStage::Incomplete;
                    } else {
                        modal.stage = crate::state::RecoveryStage::Failed(err);
                    }
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
                        .map(|img| picker.new_resize_protocol(img))
                        .map_err(|e| e.to_string());
                    let _ = tx.send((event_id, result));
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

    fn handle_login_key(&mut self, key: crossterm::event::KeyEvent) {
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
                    // Scroll to top - rendering will clamp to actual max
                    self.messages.scroll_offset = usize::MAX;
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
                                let room_id = room.id.clone();
                                self.messages.set_room(Some(room_id));
                                self.chat_title_reveal.trigger();
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
                content: crate::state::MessageContent::Text(body.clone()),
                timestamp: chrono::Local::now(),
                is_emote: false,
                is_notice: false,
                pending: true,
                verified: None,
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
                    if let Some(ref tx) = self.call_cmd_tx {
                        let _ = tx.send(CallCommand::Initiate { room_id });
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
                    if let Some(ref tx) = self.call_cmd_tx {
                        let _ = tx.send(CallCommand::Initiate { room_id });
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
                        let _ = tx.send(CallCommand::Leave);
                    }
                    self.call_info = None;
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
            CommandAction::Verify(target) => {
                self.pending_verify = Some(target);
            }
            CommandAction::Recovery => {
                self.pending_recovery = true;
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
        }
    }

    // ── Which-Key Leader Popup ────────────────────────

    fn handle_which_key(&mut self, key: KeyEvent) {
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

    fn dispatch_which_key_action(
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
                _ => {}
            },
            WhichKeyCategory::Call => match key {
                'c' => self.handle_command(CommandAction::Call),
                'a' => self.handle_command(CommandAction::Answer),
                'd' => self.handle_command(CommandAction::Reject),
                'h' => self.handle_command(CommandAction::Hangup),
                _ => {}
            },
            WhichKeyCategory::Security => match key {
                'v' => self.handle_command(CommandAction::Verify(None)),
                'r' => self.handle_command(CommandAction::Recovery),
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

    fn initiate_registration(&mut self) {
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

    pub fn is_registering(&self) -> bool {
        matches!(self.auth, AuthState::Registering)
    }

    pub fn registration_credentials(&self) -> (String, String, String, String) {
        (
            self.login.homeserver.clone(),
            self.login.username.clone(),
            self.login.password.clone(),
            self.login.registration_token.clone(),
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

    pub fn take_pending_create_room(&mut self) -> Option<CreateRoomParams> {
        self.pending_create_room.take()
    }

    pub fn take_pending_verify(&mut self) -> Option<Option<String>> {
        self.pending_verify.take()
    }

    pub fn take_pending_typing_notice(&mut self) -> Option<(String, bool)> {
        self.pending_typing_notice.take()
    }

    // ── Verification Modal ────────────────────────────

    fn handle_verify_modal_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.verification_modal = None;
            self.verify_confirm_tx = None;
            self.running = false;
            return;
        }

        let stage = self.verification_modal.as_ref().map(|m| &m.stage);

        match stage {
            Some(crate::state::VerificationStage::EmojiConfirmation) => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(tx) = self.verify_confirm_tx.take() {
                            let _ = tx.send(true);
                        }
                        if let Some(ref mut modal) = self.verification_modal {
                            modal.stage = crate::state::VerificationStage::WaitingForOtherDevice;
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        if let Some(tx) = self.verify_confirm_tx.take() {
                            let _ = tx.send(false);
                        }
                    }
                    KeyCode::Esc => {
                        // Drop the sender to cancel verification
                        self.verify_confirm_tx = None;
                        self.verification_modal = None;
                    }
                    _ => {}
                }
            }
            Some(crate::state::VerificationStage::Completed)
            | Some(crate::state::VerificationStage::Failed(_)) => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.verification_modal = None;
                    self.verify_confirm_tx = None;
                }
                _ => {}
            },
            Some(crate::state::VerificationStage::WaitingForOtherDevice) => {
                if key.code == KeyCode::Esc {
                    self.verify_confirm_tx = None;
                    self.verification_modal = None;
                }
            }
            None => {}
        }
    }

    // ── Recovery Modal ─────────────────────────────────

    fn handle_recovery_modal_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.recovery_modal = None;
            self.running = false;
            return;
        }

        let stage = self.recovery_modal.as_ref().map(|m| &m.stage);

        match stage {
            Some(crate::state::RecoveryStage::Checking) => {
                if key.code == KeyCode::Esc {
                    self.recovery_modal = None;
                }
            }
            Some(crate::state::RecoveryStage::Setup) => match key.code {
                KeyCode::Enter => {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.stage = crate::state::RecoveryStage::Creating;
                    }
                    self.pending_recovery_create = true;
                }
                KeyCode::Esc => {
                    self.recovery_modal = None;
                }
                _ => {}
            },
            Some(crate::state::RecoveryStage::Creating)
            | Some(crate::state::RecoveryStage::Resetting)
            | Some(crate::state::RecoveryStage::Recovering) => {
                // In progress — no keys accepted
            }
            Some(crate::state::RecoveryStage::ShowKey(_)) => match key.code {
                KeyCode::Char('c') => {
                    if let Some(crate::state::RecoveryStage::ShowKey(key_str)) =
                        self.recovery_modal.as_ref().map(|m| &m.stage)
                    {
                        let key_clone = key_str.clone();
                        if let Some(clipboard) = self.clipboard.as_mut()
                            && clipboard.set_text(key_clone).is_ok()
                            && let Some(ref mut modal) = self.recovery_modal
                        {
                            modal.copied = true;
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Esc => {
                    self.recovery_modal = None;
                }
                _ => {}
            },
            Some(crate::state::RecoveryStage::Incomplete) => match key.code {
                KeyCode::Char('e') => {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.key_buffer.clear();
                        modal.stage = crate::state::RecoveryStage::EnterKey;
                    }
                }
                KeyCode::Char('r') => {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.confirm_buffer.clear();
                        modal.stage = crate::state::RecoveryStage::ConfirmReset;
                    }
                }
                KeyCode::Esc => {
                    self.recovery_modal = None;
                }
                _ => {}
            },
            Some(crate::state::RecoveryStage::EnterKey) => match key.code {
                KeyCode::Backspace => {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.key_buffer.pop();
                    }
                }
                KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(ref mut modal) = self.recovery_modal
                        && let Some(clipboard) = self.clipboard.as_mut()
                        && let Ok(text) = clipboard.get_text()
                    {
                        modal.key_buffer.push_str(text.trim());
                    }
                }
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.key_buffer.push(c);
                    }
                }
                KeyCode::Enter => {
                    let key_input = self
                        .recovery_modal
                        .as_ref()
                        .map(|m| m.key_buffer.trim().to_string())
                        .unwrap_or_default();
                    if !key_input.is_empty() {
                        if let Some(ref mut modal) = self.recovery_modal {
                            modal.stage = crate::state::RecoveryStage::Recovering;
                        }
                        self.pending_recovery_recover = Some(key_input);
                    }
                }
                KeyCode::Esc => {
                    self.recovery_modal = None;
                }
                _ => {}
            },
            Some(crate::state::RecoveryStage::Enabled) => match key.code {
                KeyCode::Char('r') => {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.confirm_buffer.clear();
                        modal.stage = crate::state::RecoveryStage::ConfirmReset;
                    }
                }
                KeyCode::Esc => {
                    self.recovery_modal = None;
                }
                _ => {}
            },
            Some(crate::state::RecoveryStage::ConfirmReset) => match key.code {
                KeyCode::Backspace => {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.confirm_buffer.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut modal) = self.recovery_modal {
                        modal.confirm_buffer.push(c);
                    }
                }
                KeyCode::Enter => {
                    let confirmed = self
                        .recovery_modal
                        .as_ref()
                        .map(|m| m.confirm_buffer == "yes")
                        .unwrap_or(false);
                    if confirmed {
                        if let Some(ref mut modal) = self.recovery_modal {
                            modal.stage = crate::state::RecoveryStage::Resetting;
                        }
                        self.pending_recovery_reset = true;
                        self.recovery_enabled = false;
                    }
                }
                KeyCode::Esc => {
                    self.recovery_modal = None;
                }
                _ => {}
            },
            Some(crate::state::RecoveryStage::Failed(_)) => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.recovery_modal = None;
                }
                _ => {}
            },
            None => {}
        }
    }

    // ── Room Info ───────────────────────────────────────

    fn handle_room_info_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.room_info.open = false;
            self.running = false;
            return;
        }

        if self.room_info.loading || self.room_info.saving {
            if key.code == KeyCode::Esc {
                self.room_info.open = false;
            }
            return;
        }

        // Inline name editing mode
        if self.room_info.editing_name {
            match key.code {
                KeyCode::Esc => {
                    self.room_info.editing_name = false;
                    self.room_info.name_buffer.clear();
                }
                KeyCode::Enter => {
                    let new_name = self.room_info.name_buffer.clone();
                    if !new_name.is_empty() {
                        let room_id = self.room_info.room_id.clone();
                        self.room_info.saving = true;
                        self.room_info.editing_name = false;
                        self.pending_set_room_name = Some((room_id, new_name));
                    } else {
                        self.room_info.editing_name = false;
                        self.room_info.name_buffer.clear();
                    }
                }
                KeyCode::Backspace => {
                    self.room_info.name_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.room_info.name_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        // Inline topic editing mode
        if self.room_info.editing_topic {
            match key.code {
                KeyCode::Esc => {
                    self.room_info.editing_topic = false;
                    self.room_info.topic_buffer.clear();
                }
                KeyCode::Enter => {
                    let new_topic = self.room_info.topic_buffer.clone();
                    let room_id = self.room_info.room_id.clone();
                    self.room_info.saving = true;
                    self.room_info.editing_topic = false;
                    self.room_info.topic_save_pending = true;
                    self.pending_set_room_topic = Some((room_id, new_topic));
                }
                KeyCode::Backspace => {
                    self.room_info.topic_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.room_info.topic_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.room_info.open = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max_field = if self.room_info.encrypted { 2 } else { 3 };
                if self.room_info.selected_field < max_field {
                    self.room_info.selected_field += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.room_info.selected_field > 0 {
                    self.room_info.selected_field -= 1;
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cycle_room_info_field(-1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cycle_room_info_field(1);
            }
            KeyCode::Enter => {
                match self.room_info.selected_field {
                    0 => {
                        // Enter name editing mode
                        self.room_info.editing_name = true;
                        self.room_info.name_buffer =
                            self.room_info.name.clone().unwrap_or_default();
                    }
                    1 => {
                        // Enter topic editing mode
                        self.room_info.editing_topic = true;
                        self.room_info.topic_buffer =
                            self.room_info.topic.clone().unwrap_or_default();
                    }
                    2 => {
                        // Save current history visibility
                        let room_id = self.room_info.room_id.clone();
                        let vis = self.room_info.history_visibility.clone();
                        self.room_info.saving = true;
                        self.pending_set_visibility = Some((room_id, vis));
                    }
                    3 => {
                        // Enable encryption (only reachable when not already encrypted)
                        if self.room_info.encryption_selection == "yes" {
                            let room_id = self.room_info.room_id.clone();
                            self.room_info.saving = true;
                            self.pending_enable_encryption = Some(room_id);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn cycle_room_info_field(&mut self, dir: i32) {
        if self.room_info.selected_field == 2 {
            // Cycle history visibility
            let opts = HISTORY_VISIBILITY_OPTIONS;
            let current_idx = opts
                .iter()
                .position(|&v| v == self.room_info.history_visibility)
                .unwrap_or(0);
            let len = opts.len();
            let new_idx = if dir > 0 {
                (current_idx + 1) % len
            } else {
                (current_idx + len - 1) % len
            };
            self.room_info.history_visibility = opts[new_idx].to_string();
        } else if self.room_info.selected_field == 3 {
            // Toggle encryption selection between "no" and "yes"
            let _ = dir; // direction doesn't matter for a binary toggle
            self.room_info.encryption_selection = if self.room_info.encryption_selection == "no" {
                "yes".to_string()
            } else {
                "no".to_string()
            };
        }
    }

    // ── Create Room Modal ────────────────────────────────

    fn handle_create_room_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.create_room.open = false;
            self.running = false;
            return;
        }

        if self.create_room.creating {
            if key.code == KeyCode::Esc {
                self.create_room.open = false;
                self.create_room.creating = false;
            }
            return;
        }

        // Inline name editing mode
        if self.create_room.editing_name {
            match key.code {
                KeyCode::Esc => {
                    self.create_room.editing_name = false;
                    self.create_room.name_buffer.clear();
                }
                KeyCode::Enter => {
                    self.create_room.editing_name = false;
                }
                KeyCode::Backspace => {
                    self.create_room.name_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.create_room.name_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        // Inline topic editing mode
        if self.create_room.editing_topic {
            match key.code {
                KeyCode::Esc => {
                    self.create_room.editing_topic = false;
                    self.create_room.topic_buffer.clear();
                }
                KeyCode::Enter => {
                    self.create_room.editing_topic = false;
                }
                KeyCode::Backspace => {
                    self.create_room.topic_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.create_room.topic_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.create_room.open = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.create_room.selected_field < 4 {
                    self.create_room.selected_field += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.create_room.selected_field > 0 {
                    self.create_room.selected_field -= 1;
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cycle_create_room_field(-1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cycle_create_room_field(1);
            }
            KeyCode::Enter => {
                match self.create_room.selected_field {
                    0 => {
                        self.create_room.editing_name = true;
                    }
                    1 => {
                        self.create_room.editing_topic = true;
                    }
                    2 | 3 => {
                        // h/l already handles cycling; Enter does nothing extra here
                    }
                    4 => {
                        // CREATE button
                        if self.create_room.name_buffer.trim().is_empty() {
                            self.last_error = Some("Room name is required".to_string());
                            return;
                        }
                        let topic = if self.create_room.topic_buffer.trim().is_empty() {
                            None
                        } else {
                            Some(self.create_room.topic_buffer.clone())
                        };
                        self.pending_create_room = Some(CreateRoomParams {
                            name: self.create_room.name_buffer.clone(),
                            topic,
                            history_visibility: self.create_room.history_visibility.clone(),
                            encrypted: self.create_room.encrypted == "yes",
                        });
                        self.create_room.creating = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn cycle_create_room_field(&mut self, dir: i32) {
        if self.create_room.selected_field == 2 {
            let opts = HISTORY_VISIBILITY_OPTIONS;
            let current_idx = opts
                .iter()
                .position(|&v| v == self.create_room.history_visibility)
                .unwrap_or(0);
            let len = opts.len();
            let new_idx = if dir > 0 {
                (current_idx + 1) % len
            } else {
                (current_idx + len - 1) % len
            };
            self.create_room.history_visibility = opts[new_idx].to_string();
        } else if self.create_room.selected_field == 3 {
            self.create_room.encrypted = if self.create_room.encrypted == "no" {
                "yes".to_string()
            } else {
                "no".to_string()
            };
        }
    }

    // ── User Config ─────────────────────────────────────

    fn handle_user_config_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.user_config.open = false;
            self.running = false;
            return;
        }

        if self.user_config.loading || self.user_config.saving {
            if key.code == KeyCode::Esc {
                self.user_config.open = false;
            }
            return;
        }

        // Inline display name editing mode
        if self.user_config.editing_display_name {
            match key.code {
                KeyCode::Esc => {
                    self.user_config.editing_display_name = false;
                    self.user_config.display_name_buffer.clear();
                }
                KeyCode::Enter => {
                    let new_name = self.user_config.display_name_buffer.clone();
                    if !new_name.is_empty() {
                        self.user_config.saving = true;
                        self.user_config.editing_display_name = false;
                        self.pending_set_display_name = Some(new_name);
                    } else {
                        self.user_config.editing_display_name = false;
                        self.user_config.display_name_buffer.clear();
                    }
                }
                KeyCode::Backspace => {
                    self.user_config.display_name_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.user_config.display_name_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.user_config.open = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.user_config.selected_field < 1 {
                    self.user_config.selected_field += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.user_config.selected_field > 0 {
                    self.user_config.selected_field -= 1;
                }
            }
            KeyCode::Enter => {
                match self.user_config.selected_field {
                    0 => {
                        // Enter display name editing mode
                        self.user_config.editing_display_name = true;
                        self.user_config.display_name_buffer =
                            self.user_config.display_name.clone().unwrap_or_default();
                    }
                    1 => {
                        // Trigger self-verification and close modal
                        self.pending_verify = Some(None);
                        self.user_config.open = false;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // ── Audio Settings ──────────────────────────────────

    fn open_audio_settings(&mut self) {
        // Enumerate devices
        let mut input_devices = vec!["Default".to_string()];
        input_devices.extend(AudioPipeline::enumerate_input_devices());
        let mut output_devices = vec!["Default".to_string()];
        output_devices.extend(AudioPipeline::enumerate_output_devices());

        // Find current device indices
        let input_idx = self
            .config
            .audio
            .input_device
            .as_ref()
            .and_then(|name| input_devices.iter().position(|d| d == name))
            .unwrap_or(0);
        let output_idx = self
            .config
            .audio
            .output_device
            .as_ref()
            .and_then(|name| output_devices.iter().position(|d| d == name))
            .unwrap_or(0);

        self.audio_settings = AudioSettingsState {
            open: true,
            selected_field: 0,
            input_devices,
            output_devices,
            input_device_idx: input_idx,
            output_device_idx: output_idx,
            input_volume: self.config.audio.input_volume,
            output_volume: self.config.audio.output_volume,
            voice_activity: self.config.audio.voice_activity,
            sensitivity: self.config.audio.sensitivity,
            push_to_talk: self.config.audio.push_to_talk,
            push_to_talk_key: self.config.audio.push_to_talk_key.clone(),
            capturing_ptt_key: false,
            mic_level: 0.0,
            mic_test_running: Arc::new(AtomicBool::new(false)),
        };

        // Start mic test
        self.start_mic_test();
    }

    fn close_audio_settings(&mut self) {
        // Stop mic test
        self.audio_settings
            .mic_test_running
            .store(false, Ordering::Relaxed);

        // Sync state back to config
        let s = &self.audio_settings;
        self.config.audio.input_device = if s.input_device_idx == 0 {
            None
        } else {
            s.input_devices.get(s.input_device_idx).cloned()
        };
        self.config.audio.output_device = if s.output_device_idx == 0 {
            None
        } else {
            s.output_devices.get(s.output_device_idx).cloned()
        };
        self.config.audio.input_volume = s.input_volume;
        self.config.audio.output_volume = s.output_volume;
        self.config.audio.voice_activity = s.voice_activity;
        self.config.audio.sensitivity = s.sensitivity;
        self.config.audio.push_to_talk = s.push_to_talk;
        self.config.audio.push_to_talk_key = s.push_to_talk_key.clone();

        // Update PTT transmitting default
        if !self.config.audio.push_to_talk {
            self.ptt_transmitting.store(true, Ordering::Relaxed);
        } else {
            self.ptt_transmitting.store(false, Ordering::Relaxed);
        }

        crate::config::save_config(&self.config);
        self.audio_settings.open = false;
    }

    pub fn start_mic_test(&mut self) {
        // Stop any existing mic test (old Arc stays false, old thread exits)
        self.audio_settings
            .mic_test_running
            .store(false, Ordering::Relaxed);

        // Create a fresh running flag for the new test
        let running = Arc::new(AtomicBool::new(true));
        self.audio_settings.mic_test_running = running.clone();

        let device_name = if self.audio_settings.input_device_idx == 0 {
            None
        } else {
            self.audio_settings
                .input_devices
                .get(self.audio_settings.input_device_idx)
                .cloned()
        };
        let volume = self.audio_settings.input_volume;
        let tx = self.event_tx.clone();

        std::thread::spawn(move || {
            if let Err(e) =
                AudioPipeline::start_mic_test(device_name.as_deref(), volume, tx, running)
            {
                error!("Mic test error: {}", e);
            }
        });
    }

    fn handle_audio_settings_key(&mut self, key: KeyEvent) {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.close_audio_settings();
            self.running = false;
            return;
        }

        // PTT key capture mode
        if self.audio_settings.capturing_ptt_key {
            let key_name = key_event_name(&key);
            self.audio_settings.push_to_talk_key = Some(key_name);
            self.audio_settings.capturing_ptt_key = false;
            return;
        }

        let visible = self.audio_settings.visible_fields();
        let max_sel = visible.len().saturating_sub(1);

        match key.code {
            KeyCode::Esc => {
                self.close_audio_settings();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.audio_settings.selected_field =
                    (self.audio_settings.selected_field + 1).min(max_sel);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.audio_settings.selected_field =
                    self.audio_settings.selected_field.saturating_sub(1);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.adjust_audio_field(-1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.adjust_audio_field(1);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let field = self.audio_settings.current_field();
                match field {
                    4 => {
                        self.audio_settings.voice_activity = !self.audio_settings.voice_activity;
                        if self.audio_settings.voice_activity {
                            self.audio_settings.push_to_talk = false;
                        }
                    }
                    6 => {
                        self.audio_settings.push_to_talk = !self.audio_settings.push_to_talk;
                        if self.audio_settings.push_to_talk {
                            self.audio_settings.voice_activity = false;
                        }
                    }
                    7 => self.audio_settings.capturing_ptt_key = true,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn adjust_audio_field(&mut self, dir: i32) {
        let field = self.audio_settings.current_field();
        match field {
            0 => {
                // Input device
                let len = self.audio_settings.input_devices.len();
                if dir > 0 {
                    self.audio_settings.input_device_idx =
                        (self.audio_settings.input_device_idx + 1) % len;
                } else {
                    self.audio_settings.input_device_idx =
                        (self.audio_settings.input_device_idx + len - 1) % len;
                }
                // Restart mic test with new device
                self.start_mic_test();
            }
            1 => {
                // Output device
                let len = self.audio_settings.output_devices.len();
                if dir > 0 {
                    self.audio_settings.output_device_idx =
                        (self.audio_settings.output_device_idx + 1) % len;
                } else {
                    self.audio_settings.output_device_idx =
                        (self.audio_settings.output_device_idx + len - 1) % len;
                }
            }
            2 => {
                // Input volume
                let step = 0.05;
                self.audio_settings.input_volume =
                    (self.audio_settings.input_volume + dir as f32 * step).clamp(0.0, 1.0);
            }
            3 => {
                // Output volume
                let step = 0.05;
                self.audio_settings.output_volume =
                    (self.audio_settings.output_volume + dir as f32 * step).clamp(0.0, 1.0);
            }
            4 => {
                // Voice activity toggle
                self.audio_settings.voice_activity = dir > 0;
                if self.audio_settings.voice_activity {
                    self.audio_settings.push_to_talk = false;
                }
            }
            5 => {
                // Sensitivity
                let step = 0.05;
                self.audio_settings.sensitivity =
                    (self.audio_settings.sensitivity + dir as f32 * step).clamp(0.0, 1.0);
            }
            6 => {
                // Push to talk toggle
                self.audio_settings.push_to_talk = dir > 0;
                if self.audio_settings.push_to_talk {
                    self.audio_settings.voice_activity = false;
                }
            }
            _ => {}
        }
    }

    fn key_matches_ptt(&self, key: &KeyEvent) -> bool {
        if let Some(ref ptt_key) = self.config.audio.push_to_talk_key {
            let name = key_event_name(key);
            &name == ptt_key
        } else {
            false
        }
    }
}

fn key_event_name(key: &KeyEvent) -> String {
    match key.code {
        KeyCode::Char(c) => {
            if c == ' ' {
                "Space".to_string()
            } else {
                c.to_uppercase().to_string()
            }
        }
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Modifier(m) => {
            use crossterm::event::ModifierKeyCode::*;
            match m {
                LeftControl | RightControl => "Ctrl",
                LeftShift | RightShift => "Shift",
                LeftAlt | RightAlt => "Alt",
                LeftSuper | RightSuper => "Super",
                LeftHyper | RightHyper => "Hyper",
                LeftMeta | RightMeta => "Meta",
                IsoLevel3Shift => "IsoLevel3Shift",
                IsoLevel5Shift => "IsoLevel5Shift",
            }
            .to_string()
        }
        _ => format!("{:?}", key.code),
    }
}
