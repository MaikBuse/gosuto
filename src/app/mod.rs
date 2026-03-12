mod audio;
mod commands;
mod event_handler;
mod input_handler;
mod modal_keys;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::error;

use ratatui_image::protocol::StatefulProtocol;

pub type ImageDecodeResult = (String, Result<(StatefulProtocol, u32, u32), String>);

use crate::config::GosutoConfig;
use crate::event::{AppEvent, EventSender, WarnClosed};
use crate::input::{self, CommandAction, FocusPanel, InputResult, VimState};
use crate::state::{
    AudioSettingsAction, AudioSettingsState, AuthState, ChangePasswordAction, ChangePasswordState,
    CreateRoomAction, CreateRoomParams, CreateRoomState, ImageCache, MemberListState, MessageState,
    RecoveryAction, RecoveryModalState, RecoveryStage, RecoveryTransition, RoomInfoAction,
    RoomInfoState, RoomListState, UserConfigAction, UserConfigState, recovery_key_action,
};
use crate::ui::animation::AnimationClock;
use crate::ui::call_overlay::TransmissionPopup;
use crate::ui::effects::{EffectsState, TextReveal};
use crate::ui::login::LoginState;
use crate::ui::room_list::RoomListAnimState;
use crate::voip::audio::AudioPipeline;
use crate::voip::{CallCommand, CallCommandSender, CallInfo, CallState};

#[derive(Debug, Clone)]
pub struct ReplyContext {
    pub event_id: String,
    pub sender: String,
    pub body_preview: String,
}

#[derive(Debug, Clone)]
pub struct EditContext {
    pub event_id: String,
    pub original_body: String,
}

pub(crate) fn truncate_preview(text: &str, max_len: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    if first_line.len() <= max_len {
        first_line.to_string()
    } else {
        format!("{}...", &first_line[..max_len])
    }
}

pub struct PendingSend {
    pub room_id: String,
    pub body: String,
    pub reply_to: Option<ReplyContext>,
    pub edit: Option<EditContext>,
}

pub const QUICK_EMOJIS: &[&str] = &[
    "\u{1F44D}",
    "\u{2764}\u{FE0F}",
    "\u{1F602}",
    "\u{1F389}",
    "\u{1F62E}",
    "\u{1F622}",
    "\u{1F914}",
    "\u{1F440}",
];

pub struct ReactionPickerState {
    pub event_id: String,
    pub quick_pick_index: usize,
    pub existing_own_reactions: Vec<String>,
    pub in_grid: bool,
    pub grid_index: usize,
    pub filter: String,
    pub filter_active: bool,
    pub scroll_offset: usize,
}

pub struct PendingReaction {
    pub room_id: String,
    pub target_event_id: String,
    pub emoji_key: String,
    pub toggle_off_reaction_event_id: Option<String>,
}

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
    pub pending_load_more: bool,
    pending_send: Option<PendingSend>,
    pub reply_context: Option<ReplyContext>,
    pub edit_context: Option<EditContext>,
    pending_join: Option<String>,  // room_id_or_alias
    pending_leave: Option<String>, // room_id
    pending_dm: Option<String>,    // user_id
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
    pub anim_clock: AnimationClock,
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
    pub shared_audio_config: Option<Arc<parking_lot::RwLock<crate::config::AudioConfig>>>,
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
    // Recovery
    pub recovery: Option<RecoveryModalState>,
    pub pending_recovery: Option<RecoveryAction>,
    // Verification
    pub verification_modal: Option<crate::state::VerificationModalState>,
    pub pending_verify: Option<Option<String>>,
    pub pending_reset_cross_signing: bool,
    pub verify_confirm_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    pub verify_task_handle: Option<tokio::task::JoinHandle<()>>,
    pub self_verified: bool,
    pub recovery_status: crate::event::RecoveryStatus,
    // Invitation support
    pub invite_prompt_room: Option<String>,
    pending_accept_invite: Option<String>,
    pending_decline_invite: Option<String>,
    pending_invite_user: Option<(String, String)>,
    // Reaction picker
    pub reaction_picker: Option<ReactionPickerState>,
    pending_reaction: Option<PendingReaction>,
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
            pending_load_more: false,
            pending_send: None,
            reply_context: None,
            edit_context: None,
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
            anim_clock: AnimationClock::new(),
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
            shared_audio_config: None,
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
            recovery: None,
            pending_recovery: None,
            verification_modal: None,
            pending_verify: None,
            pending_reset_cross_signing: false,
            verify_confirm_tx: None,
            verify_task_handle: None,
            self_verified: false,
            recovery_status: crate::event::RecoveryStatus::Disabled,
            invite_prompt_room: None,
            pending_accept_invite: None,
            pending_decline_invite: None,
            pending_invite_user: None,
            reaction_picker: None,
            pending_reaction: None,
        }
    }

    pub(crate) fn send_message(&mut self, body: String) {
        if let Some(room_id) = self.messages.current_room_id.clone() {
            let edit = self.edit_context.take();

            if let Some(ref edit_ctx) = edit {
                // Editing: update the existing message in-place
                let new_content = crate::state::MessageContent::Text {
                    plain: body.clone(),
                    formatted_html: None,
                };
                self.messages
                    .update_message_content(&edit_ctx.event_id, new_content);
                self.pending_send = Some(PendingSend {
                    room_id,
                    body,
                    reply_to: None,
                    edit,
                });
            } else {
                // Get the actual user_id for the sender display
                let sender = match &self.auth {
                    AuthState::LoggedIn { user_id, .. } => user_id.clone(),
                    _ => "me".to_string(),
                };

                let reply_to = self.reply_context.take();
                let in_reply_to = reply_to.as_ref().map(|r| crate::state::ReplyInfo {
                    event_id: r.event_id.clone(),
                    sender: r.sender.clone(),
                    body_preview: r.body_preview.clone(),
                });

                // Add optimistic message
                let msg = crate::state::DisplayMessage {
                    event_id: String::new(),
                    sender,
                    content: crate::state::MessageContent::Text {
                        plain: body.clone(),
                        formatted_html: None,
                    },
                    timestamp: chrono::Local::now(),
                    is_emote: false,
                    is_notice: false,
                    pending: true,
                    verified: None,
                    in_reply_to,
                    reactions: Vec::new(),
                    edited: false,
                };
                self.messages.add_message(msg);
                self.messages.scroll_to_bottom();

                // Queue for main loop to send
                self.pending_send = Some(PendingSend {
                    room_id,
                    body,
                    reply_to,
                    edit: None,
                });
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

    pub fn take_pending_send(&mut self) -> Option<PendingSend> {
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

    pub fn take_pending_verify(&mut self) -> Option<Option<String>> {
        self.pending_verify.take()
    }

    pub fn take_pending_reaction(&mut self) -> Option<PendingReaction> {
        self.pending_reaction.take()
    }
}
