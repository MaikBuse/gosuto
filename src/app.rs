use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::error;

use ratatui_image::protocol::StatefulProtocol;

pub type ImageDecodeResult = (String, Result<(StatefulProtocol, u32, u32), String>);

use crate::config::GosutoConfig;
use crate::event::{AppEvent, EventSender};
use crate::input::{self, CommandAction, FocusPanel, InputResult, VimState};
use crate::state::{
    AudioSettingsAction, AudioSettingsState, AuthState, ChangePasswordAction, ChangePasswordState,
    CreateRoomAction, CreateRoomParams, CreateRoomState, ImageCache, MemberListState, MessageState,
    RecoveryAction, RecoveryModalState, RecoveryStage, RecoveryTransition, RoomInfoAction,
    RoomInfoState, RoomListState, UserConfigAction, UserConfigState, recovery_key_action,
};
use crate::ui::call_overlay::TransmissionPopup;
use crate::ui::effects::{EffectsState, TextReveal};
use crate::ui::login::LoginState;
use crate::ui::room_list::RoomListAnimState;
use crate::voip::audio::AudioPipeline;
use crate::voip::{CallCommand, CallCommandSender, CallInfo, CallState};

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
    // Change password
    pub change_password: ChangePasswordState,
    pub pending_change_password: Option<(String, String)>,
    // Audio settings
    pub audio_settings: AudioSettingsState,
    pub ptt_transmitting: Arc<AtomicBool>,
    pub mic_active: Arc<AtomicBool>,
    pub global_ptt: Option<crate::global_ptt::GlobalPttHandle>,
    pub sync_token: Option<String>,
    // Which-key leader popup
    pub which_key: Option<Option<crate::ui::which_key::WhichKeyCategory>>,
    clipboard: Option<arboard::Clipboard>,
    // Typing indicators
    pub typing_users: HashMap<String, Vec<String>>,
    pub last_typing_sent: Option<Instant>,
    pub pending_typing_notice: Option<(String, bool)>,
    // Inline images
    pub picker: ratatui_image::picker::Picker,
    pub image_cache: ImageCache,
    pub image_decode_tx: std::sync::mpsc::Sender<ImageDecodeResult>,
    pub demo_mode: bool,
    // Recovery
    pub recovery: Option<RecoveryModalState>,
    pub pending_recovery: Option<RecoveryAction>,
    // Verification
    pub verification_modal: Option<crate::state::VerificationModalState>,
    pub pending_verify: Option<Option<String>>,
    pub verify_confirm_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    pub self_verified: bool,
    pub recovery_status: crate::event::RecoveryStatus,
    // Invitation support
    pub invite_prompt_room: Option<String>,
    pending_accept_invite: Option<String>,
    pending_decline_invite: Option<String>,
    pending_invite_user: Option<(String, String)>,
}

impl App {
    pub fn new(
        event_tx: EventSender,
        config: GosutoConfig,
        picker: ratatui_image::picker::Picker,
        image_decode_tx: std::sync::mpsc::Sender<ImageDecodeResult>,
    ) -> Self {
        let rain_enabled = config.effects.rain;
        let glitch_enabled = config.effects.glitch;
        let ptt_enabled = config.audio.push_to_talk;
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
            change_password: ChangePasswordState::new(),
            pending_change_password: None,
            audio_settings: AudioSettingsState::new(),
            ptt_transmitting: Arc::new(AtomicBool::new(!ptt_enabled)),
            mic_active: Arc::new(AtomicBool::new(false)),
            global_ptt: None,
            sync_token: None,
            which_key: None,
            clipboard: arboard::Clipboard::new().ok(),
            typing_users: HashMap::new(),
            last_typing_sent: None,
            pending_typing_notice: None,
            picker,
            image_cache: ImageCache::new(),
            image_decode_tx,
            demo_mode: false,
            recovery: None,
            pending_recovery: None,
            verification_modal: None,
            pending_verify: None,
            verify_confirm_tx: None,
            self_verified: false,
            recovery_status: crate::event::RecoveryStatus::Disabled,
            invite_prompt_room: None,
            pending_accept_invite: None,
            pending_decline_invite: None,
            pending_invite_user: None,
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
                        *handle.ptt_key.lock().unwrap() = name;
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
                if self.user_config.open {
                    self.user_config.display_name = display_name;
                    self.user_config.verified = verified || self.self_verified;
                    self.user_config.recovery_status = recovery_status;
                    self.user_config.loading = false;
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
                self.verification_modal = Some(crate::state::VerificationModalState {
                    stage: crate::state::VerificationStage::WaitingForOtherDevice,
                    sender,
                    emojis: vec![],
                });
            }
            AppEvent::VerificationSasEmoji { emojis, sender } => {
                self.verification_modal = Some(crate::state::VerificationModalState {
                    stage: crate::state::VerificationStage::EmojiConfirmation,
                    sender,
                    emojis,
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
            InputResult::VerifyMember => {
                if let Some(member) = self.members_list.selected_member() {
                    let uid = member.user_id.clone();
                    self.pending_verify = Some(Some(uid));
                }
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
                    self.set_global_ptt_active(true);
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
                    self.set_global_ptt_active(true);
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
                self.pending_verify = Some(target);
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

    // ── Invite Prompt ────────────────────────────────

    fn handle_invite_prompt_key(&mut self, key: KeyEvent) {
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

    fn handle_recovery_key(&mut self, key: KeyEvent) {
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

    fn handle_verify_modal_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.verification_modal = None;
            self.verify_confirm_tx = None;
            self.running = false;
            return;
        }

        let stage = self.verification_modal.as_ref().map(|m| &m.stage);

        match stage {
            Some(crate::state::VerificationStage::EmojiConfirmation) => match key.code {
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
                    self.verify_confirm_tx = None;
                    self.verification_modal = None;
                }
                _ => {}
            },
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

    pub fn take_pending_verify(&mut self) -> Option<Option<String>> {
        self.pending_verify.take()
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

    pub fn take_pending_typing_notice(&mut self) -> Option<(String, bool)> {
        self.pending_typing_notice.take()
    }

    pub fn take_pending_accept_invite(&mut self) -> Option<String> {
        self.pending_accept_invite.take()
    }

    pub fn take_pending_decline_invite(&mut self) -> Option<String> {
        self.pending_decline_invite.take()
    }

    pub fn take_pending_invite_user(&mut self) -> Option<(String, String)> {
        self.pending_invite_user.take()
    }

    // ── Room Info ───────────────────────────────────────

    fn handle_room_info_key(&mut self, key: KeyEvent) {
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

    fn handle_create_room_key(&mut self, key: KeyEvent) {
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

    fn handle_user_config_key(&mut self, key: KeyEvent) {
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

    fn handle_change_password_key(&mut self, key: KeyEvent) {
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
            ptt_error: None,
            vad_hold_ms: self.config.audio.vad_hold_ms,
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
        self.config.audio.vad_hold_ms = s.vad_hold_ms;

        // Update PTT transmitting default
        if !self.config.audio.push_to_talk {
            self.ptt_transmitting.store(true, Ordering::Relaxed);
        } else {
            self.ptt_transmitting.store(false, Ordering::Relaxed);
        }

        // Spawn global PTT listener on demand, or sync key if already running
        if self.config.audio.push_to_talk {
            self.ensure_global_ptt_listener();
            if let Some(ref handle) = self.global_ptt {
                *handle.ptt_key.lock().unwrap() = self
                    .config
                    .audio
                    .push_to_talk_key
                    .clone()
                    .unwrap_or_default();
            }
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

        match self.audio_settings.handle_key(key) {
            AudioSettingsAction::None => {}
            AudioSettingsAction::Close => {
                self.close_audio_settings();
            }
            AudioSettingsAction::StartMicTest => {
                self.start_mic_test();
            }
            AudioSettingsAction::CapturePttKey => {
                self.ensure_global_ptt_listener();
                if let Some(ref handle) = self.global_ptt {
                    self.audio_settings.capturing_ptt_key = true;
                    handle.capturing.store(true, Ordering::Relaxed);
                }
            }
            AudioSettingsAction::ToggleVad | AudioSettingsAction::TogglePtt => {}
        }
    }

    fn ensure_global_ptt_listener(&mut self) {
        if let Some(ref handle) = self.global_ptt
            && !handle.alive.load(Ordering::Relaxed)
        {
            self.global_ptt = None;
        }
        if self.global_ptt.is_none() {
            if let Some(error) = crate::global_ptt::check_linux_prerequisites() {
                self.audio_settings.ptt_error = Some(error);
                return;
            }
            let ptt_key = self
                .config
                .audio
                .push_to_talk_key
                .clone()
                .unwrap_or_default();
            let handle = crate::global_ptt::spawn_listener(
                self.ptt_transmitting.clone(),
                ptt_key,
                self.event_tx.clone(),
            );
            self.global_ptt = Some(handle);
        }
    }

    fn set_global_ptt_active(&self, active: bool) {
        if let Some(ref handle) = self.global_ptt {
            handle.active.store(active, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GosutoConfig;
    use crate::event::AppEvent;
    use crate::state::{HealingStep, RecoveryStage};

    fn test_app() -> App {
        let (event_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let config = GosutoConfig::default();
        let picker = ratatui_image::picker::Picker::halfblocks();
        let (image_decode_tx, _image_decode_rx) = std::sync::mpsc::channel();
        App::new(event_tx, config, picker, image_decode_tx)
    }

    #[test]
    fn user_config_loaded_sets_display_name_when_open() {
        let mut app = test_app();
        app.user_config.open = true;
        app.user_config.loading = true;

        app.handle_event(AppEvent::UserConfigLoaded {
            display_name: Some("Alice".to_string()),
            verified: true,
            recovery_status: crate::event::RecoveryStatus::Disabled,
        });

        assert!(!app.user_config.loading);
        assert_eq!(app.user_config.display_name, Some("Alice".to_string()));
        assert!(app.self_verified);
        assert!(app.user_config.verified);
    }

    #[test]
    fn user_config_loaded_sets_self_verified_when_modal_closed() {
        let mut app = test_app();
        assert!(!app.self_verified);
        assert!(!app.user_config.open);

        app.handle_event(AppEvent::UserConfigLoaded {
            display_name: None,
            verified: true,
            recovery_status: crate::event::RecoveryStatus::Disabled,
        });

        assert!(app.self_verified);
        assert!(!app.user_config.loading);
    }

    #[test]
    fn full_restart_flow_sets_verified_before_modal_open() {
        let mut app = test_app();
        assert!(!app.self_verified);
        assert!(app.sync_token.is_none());

        // First sync triggers pending fetch
        app.handle_event(AppEvent::SyncTokenUpdated("tok1".to_string()));
        assert!(app.pending_user_config);

        // Main loop consumes the flag
        app.pending_user_config = false;

        // SDK responds with verified=true
        app.handle_event(AppEvent::UserConfigLoaded {
            display_name: Some("Bob".to_string()),
            verified: true,
            recovery_status: crate::event::RecoveryStatus::Disabled,
        });
        assert!(app.self_verified);

        // User opens :configure — verified should propagate from self_verified
        app.user_config = UserConfigState {
            open: true,
            verified: app.self_verified,
            loading: true,
            ..UserConfigState::new()
        };
        assert!(app.user_config.verified);
    }

    #[test]
    fn logout_resets_self_verified() {
        let mut app = test_app();
        app.self_verified = true;
        app.auto_login_attempted = true; // prevent auto-login side effects

        app.handle_event(AppEvent::LoggedOut);

        assert!(!app.self_verified);
    }

    #[test]
    fn first_sync_token_triggers_user_config_fetch() {
        let mut app = test_app();
        assert!(app.sync_token.is_none());

        app.handle_event(AppEvent::SyncTokenUpdated("tok1".to_string()));

        assert!(app.pending_user_config);
        assert_eq!(app.sync_token, Some("tok1".to_string()));
    }

    #[test]
    fn subsequent_sync_token_does_not_trigger_fetch() {
        let mut app = test_app();
        app.sync_token = Some("tok1".to_string());
        app.pending_user_config = false;

        app.handle_event(AppEvent::SyncTokenUpdated("tok2".to_string()));

        assert!(!app.pending_user_config);
        assert_eq!(app.sync_token, Some("tok2".to_string()));
    }

    #[test]
    fn recovery_command_opens_modal() {
        let mut app = test_app();
        app.auth = crate::state::AuthState::LoggedIn {
            user_id: "@test:example.com".to_string(),
            device_id: "DEV".to_string(),
            homeserver: "https://example.com".to_string(),
        };
        app.handle_command(CommandAction::Recovery);
        assert!(app.recovery.is_some());
        assert_eq!(app.pending_recovery, Some(RecoveryAction::Check));
    }

    #[test]
    fn recovery_event_updates_stage() {
        let mut app = test_app();
        app.recovery = Some(RecoveryModalState::new());

        app.handle_event(AppEvent::RecoveryStateChecked(RecoveryStage::Enabled));
        assert_eq!(app.recovery.as_ref().unwrap().stage, RecoveryStage::Enabled);

        app.handle_event(AppEvent::RecoveryKeyReady("key123".to_string()));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::ShowKey("key123".to_string())
        );

        app.recovery = Some(RecoveryModalState::new());
        app.handle_event(AppEvent::RecoveryRecovered);
        assert_eq!(app.recovery.as_ref().unwrap().stage, RecoveryStage::Enabled);

        app.recovery = Some(RecoveryModalState::new());
        app.handle_event(AppEvent::RecoveryError("bad".to_string()));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Error("bad".to_string())
        );
    }

    #[test]
    fn healing_progress_updates_stage() {
        let mut app = test_app();
        app.recovery = Some(RecoveryModalState::new());

        app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::CrossSigning));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::CrossSigning)
        );

        app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::Backup)
        );

        app.handle_event(AppEvent::RecoveryHealingProgress(
            HealingStep::ExportSecrets,
        ));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::ExportSecrets)
        );
    }

    #[test]
    fn need_password_event_sets_stage() {
        use crate::event::PasswordSender;

        let mut app = test_app();
        app.recovery = Some(RecoveryModalState::new());

        let (tx, _rx) = tokio::sync::oneshot::channel();
        app.handle_event(AppEvent::RecoveryNeedPassword(PasswordSender::new(tx)));

        let modal = app.recovery.as_ref().unwrap();
        assert_eq!(modal.stage, RecoveryStage::NeedPassword);
        assert!(modal.password_tx.is_some());
        assert!(modal.password_buffer.is_empty());
    }

    #[test]
    fn healing_skips_cross_signing_starts_at_backup() {
        let mut app = test_app();
        app.recovery = Some(RecoveryModalState::new());
        app.recovery.as_mut().unwrap().stage = RecoveryStage::Recovering;

        app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::Backup)
        );
    }

    #[test]
    fn healing_backup_then_export_without_cross_signing() {
        let mut app = test_app();
        app.recovery = Some(RecoveryModalState::new());

        app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::Backup)
        );

        app.handle_event(AppEvent::RecoveryHealingProgress(
            HealingStep::ExportSecrets,
        ));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::ExportSecrets)
        );

        app.handle_event(AppEvent::RecoveryKeyReady("newkey123".to_string()));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::ShowKey("newkey123".to_string())
        );
    }

    #[test]
    fn healing_full_path_with_cross_signing() {
        let mut app = test_app();
        app.recovery = Some(RecoveryModalState::new());

        app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::CrossSigning));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::CrossSigning)
        );

        let (tx, _rx) = tokio::sync::oneshot::channel();
        app.handle_event(AppEvent::RecoveryNeedPassword(
            crate::event::PasswordSender::new(tx),
        ));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::NeedPassword
        );

        app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::Backup)
        );

        app.handle_event(AppEvent::RecoveryHealingProgress(
            HealingStep::ExportSecrets,
        ));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::ExportSecrets)
        );

        app.handle_event(AppEvent::RecoveryKeyReady("abc".to_string()));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::ShowKey("abc".to_string())
        );
    }

    #[test]
    fn healing_from_resetting_stage() {
        let mut app = test_app();
        app.recovery = Some(RecoveryModalState::new());
        app.recovery.as_mut().unwrap().stage = RecoveryStage::Resetting;

        app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
        assert_eq!(
            app.recovery.as_ref().unwrap().stage,
            RecoveryStage::Healing(HealingStep::Backup)
        );
    }
}
