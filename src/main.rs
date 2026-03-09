#![recursion_limit = "256"]

mod app;
mod cli;
mod config;
mod event;
mod fs_utils;
mod global_ptt;
mod input;
mod matrix;
mod state;
mod terminal;
mod ui;
mod voip;

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use app::App;
use event::AppEvent;
use event::WarnClosed;

macro_rules! spawn_with_client {
    ($holder:expr, |$client:ident| $body:expr) => {{
        let holder = $holder.clone();
        tokio::spawn(async move {
            if let Some($client) = holder.lock().await.clone() {
                $body.await;
            }
        })
    }};
}

const NOISY_DEPS: &[&str] = &[
    "matrix_sdk",
    "hyper",
    "livekit",
    "tokio",
    "rustls",
    "reqwest",
    "cpal",
];

/// Build the tracing `EnvFilter` from the `GOSUTO_LOG` environment variable.
///
/// - **Unset**: `warn,gosuto=info,<noisy deps>=warn`
/// - **Simple level** (e.g. `debug`): `warn,gosuto=debug,<noisy deps>=warn`
/// - **Full filter** (contains `,` or `=`): used as-is
fn build_env_filter() -> EnvFilter {
    let filter = match std::env::var("GOSUTO_LOG") {
        Ok(val) if !val.is_empty() => {
            if val.contains(',') || val.contains('=') {
                // Advanced filter string — use as-is
                return EnvFilter::new(val);
            }
            // Simple level — scope to gosuto only
            let mut parts = vec![format!("warn,gosuto={val}")];
            for dep in NOISY_DEPS {
                parts.push(format!("{dep}=warn"));
            }
            parts.join(",")
        }
        _ => {
            // Default filter
            let mut parts = vec!["warn,gosuto=info".to_owned()];
            for dep in NOISY_DEPS {
                parts.push(format!("{dep}=warn"));
            }
            parts.join(",")
        }
    };
    EnvFilter::new(filter)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = cli::parse_args();
    config::init_profile(cli_args.profile);

    // Set up file-based logging (controlled by GOSUTO_LOG env var)
    let log_path = config::log_path()?;
    let file_appender = tracing_appender::rolling::daily(&log_path, "gosuto.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(build_env_filter())
        .init();

    config::cleanup_old_logs(&log_path, 7);
    info!("Starting gōsuto");

    // Create event channel
    let (event_tx, mut event_rx) = event::event_channel();

    // Initialize terminal
    let mut tui = terminal::init()?;

    let picker = terminal::init_picker();
    info!(
        "Image protocol: {:?}, font size: {:?}",
        picker.protocol_type(),
        picker.font_size()
    );

    terminal::init_keyboard_enhancement();

    // Restore terminal on panic so spawned-task panics produce readable output
    // instead of corrupting the raw-mode terminal.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::restore();
        default_hook(info);
    }));

    // Load config
    let gosuto_config = config::load_config();
    info!("Config: {:?}", gosuto_config);

    // Create app
    let accept_invalid_certs = gosuto_config.network.accept_invalid_certs;
    let (image_decode_tx, image_decode_rx) = std::sync::mpsc::channel();
    let mut app = App::new(event_tx.clone(), gosuto_config, picker, image_decode_tx);

    // Shared Matrix client
    let matrix_client: Arc<Mutex<Option<matrix_sdk::Client>>> = Arc::new(Mutex::new(None));

    // Shared state for incoming verification requests
    let incoming_verification: matrix::sync::IncomingVerification = Arc::new(Mutex::new(None));

    // Create CallManager command channel early so sync handlers can forward encryption keys
    let (call_cmd_tx, call_cmd_rx) = voip::manager::command_channel();
    app.call_cmd_tx = Some(call_cmd_tx.clone());

    // Try to restore session
    match matrix::client::try_restore_session(&event_tx, accept_invalid_certs).await {
        Ok(Some(client)) => {
            *matrix_client.lock().await = Some(client.clone());
            let tx = event_tx.clone();
            let iv = incoming_verification.clone();
            let cmd_tx = call_cmd_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = matrix::sync::start_sync(client, tx.clone(), iv, Some(cmd_tx)).await
                {
                    error!("Sync error: {}", e);
                    tx.send(AppEvent::SyncError(e.to_string()))
                        .warn_closed("SyncError");
                }
            });
        }
        Ok(None) => {
            info!("No stored session found");
            if let Some(creds) = matrix::credentials::load_credentials() {
                event_tx
                    .send(AppEvent::AutoLogin {
                        homeserver: creds.homeserver,
                        username: creds.username,
                        password: creds.password,
                    })
                    .warn_closed("AutoLogin");
            }
        }
        Err(e) => {
            info!("Failed to restore session: {}", e);
            // Clean up stale session and store to avoid repeated failures
            if let Ok(session_path) = config::session_path() {
                if let Ok(stored) = matrix::session::load_session(&session_path)
                    && let Ok(store_path) =
                        config::store_path_for_homeserver_unchecked(&stored.homeserver)
                {
                    info!("Removing stale store at {}", store_path.display());
                    if let Err(e) = std::fs::remove_dir_all(&store_path) {
                        info!("Could not remove store: {}", e);
                    }
                }
                if let Err(e) = matrix::session::delete_session(&session_path) {
                    tracing::warn!("Failed to delete session: {e}");
                }
            }
            if let Some(creds) = matrix::credentials::load_credentials() {
                event_tx
                    .send(AppEvent::AutoLogin {
                        homeserver: creds.homeserver,
                        username: creds.username,
                        password: creds.password,
                    })
                    .warn_closed("AutoLogin");
            }
        }
    }

    // Spawn crossterm event reader
    let input_tx = event_tx.clone();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            match event {
                Event::Key(key) => match key.kind {
                    KeyEventKind::Press | KeyEventKind::Repeat => {
                        input_tx.send(AppEvent::Key(key)).warn_closed("Key");
                    }
                    KeyEventKind::Release => {
                        input_tx
                            .send(AppEvent::KeyRelease)
                            .warn_closed("KeyRelease");
                    }
                },
                Event::Resize(_, _) => {
                    input_tx.send(AppEvent::Resize).warn_closed("Resize");
                }
                _ => {}
            }
        }
    });

    // Tick timer
    let tick_tx = event_tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(config::TICK_RATE_MS));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Shared audio config for CallManager
    let shared_audio_config = Arc::new(parking_lot::RwLock::new(app.config.audio.clone()));
    let ptt_transmitting = app.ptt_transmitting.clone();
    let mic_active = app.mic_active.clone();

    // Spawn CallManager
    let call_manager = voip::manager::CallManager::new(
        call_cmd_rx,
        event_tx.clone(),
        matrix_client.clone(),
        shared_audio_config.clone(),
        ptt_transmitting,
        mic_active,
    );
    tokio::spawn(call_manager.run());

    // Spawn global PTT listener when push-to-talk is enabled
    if app.config.audio.push_to_talk {
        let ptt_key = app
            .config
            .audio
            .push_to_talk_key
            .clone()
            .unwrap_or_default();
        let handle =
            global_ptt::spawn_listener(app.ptt_transmitting.clone(), ptt_key, event_tx.clone());
        app.global_ptt = Some(handle);
    }

    // Track login/registration state to avoid re-triggering
    let mut login_in_progress = false;
    let mut registration_in_progress = false;

    // Track popup state to clear terminal on close (restores Kitty images)

    // Main loop
    let render_interval = Duration::from_millis(config::RENDER_RATE_MS);
    let mut last_render = Instant::now();

    loop {
        // Calculate time until next render is due
        let until_render = render_interval.saturating_sub(last_render.elapsed());

        // Wait for at least one event, or wake when it's time to render
        tokio::select! {
            event = event_rx.recv() => {
                match event {
                    Some(ev) => {
                        // Track room change for message fetching
                        let prev_room = app.messages.current_room_id.clone();

                        app.handle_event(ev);

                        // Drain ALL remaining pending events so key events
                        // are never buried behind MicLevel floods
                        while let Ok(ev) = event_rx.try_recv() {
                            app.handle_event(ev);
                        }

                        let new_room = app.messages.current_room_id.clone();

                        // Fetch messages and members when room changes
                        if prev_room != new_room {
                            // Clear stale members and image cache immediately
                            app.members_list.clear();
                            app.image_cache.clear();
                            while image_decode_rx.try_recv().is_ok() {}

                            if let Some(ref room_id) = new_room {
                                // Fetch messages
                                let client_holder = matrix_client.clone();
                                let tx = event_tx.clone();
                                let rid = room_id.clone();
                                let sync_token = app.sync_token.clone();
                                tokio::spawn(async move {
                                    let client = { client_holder.lock().await.clone() };
                                    if let Some(client) = client {
                                        if let Err(e) = matrix::messages::fetch_messages(&client, &rid, &tx, sync_token).await {
                                            error!("Failed to fetch messages for {}: {:?}", rid, e);
                                            tx.send(AppEvent::FetchError {
                                                room_id: rid,
                                                error: e.to_string(),
                                            }).warn_closed("FetchError");
                                        }
                                    } else {
                                        tx.send(AppEvent::FetchError {
                                            room_id: rid,
                                            error: "Not connected".to_string(),
                                        }).warn_closed("FetchError");
                                    }
                                });

                                // Fetch members
                                let tx2 = event_tx.clone();
                                let rid2 = room_id.clone();
                                spawn_with_client!(matrix_client, |client| async move {
                                    matrix::rooms::fetch_room_members(&client, &rid2, &tx2).await;
                                    matrix::rooms::check_member_verification(&client, &rid2, &tx2).await;
                                });

                                // Send read receipt
                                let rid3 = room_id.clone();
                                spawn_with_client!(matrix_client, |client| async move {
                                    matrix::rooms::mark_room_as_read(&client, &rid3, None).await;
                                });
                            }
                        }

                        // Re-fetch messages after verification
                        if app.pending_refetch {
                            app.pending_refetch = false;
                            if let Some(ref room_id) = app.messages.current_room_id.clone() {
                                app.messages.messages.clear();
                                app.messages.loading = true;
                                let tx = event_tx.clone();
                                let rid = room_id.clone();
                                let sync_token = app.sync_token.clone();
                                spawn_with_client!(matrix_client, |client| async move {
                                    if let Err(e) = matrix::messages::fetch_messages(&client, &rid, &tx, sync_token).await {
                                        error!("Failed to re-fetch messages for {}: {:?}", rid, e);
                                        tx.send(AppEvent::FetchError {
                                            room_id: rid,
                                            error: e.to_string(),
                                        }).warn_closed("FetchError");
                                    }
                                });
                            }
                        }

                        // Handle message sending
                        if let Some((room_id, body)) = app.take_pending_send() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                if let Err(e) = matrix::messages::send_message(&client, &room_id, &body, &tx).await {
                                    error!("Failed to send message: {}", e);
                                }
                            });
                        }

                        // Handle outgoing typing notice
                        if let Some((room_id, typing)) = app.take_pending_typing_notice() {
                            spawn_with_client!(matrix_client, |client| async move {
                                let room_id_parsed: Result<matrix_sdk::ruma::OwnedRoomId, _> = room_id.as_str().try_into();
                                if let Ok(id) = room_id_parsed
                                    && let Some(room) = client.get_room(&id)
                                    && let Err(e) = room.typing_notice(typing).await
                                {
                                    tracing::warn!("typing_notice failed: {e}");
                                }
                            });
                        }

                        // Handle read receipt for new messages in open room
                        if let Some((room_id, event_id)) = app.pending_read_receipt.take() {
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::mark_room_as_read(&client, &room_id, event_id.as_deref()).await;
                            });
                        }

                        // Handle logout
                        if app.pending_logout {
                            app.pending_logout = false;
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.take() };
                                if let Some(client) = client
                                    && let Err(e) = matrix::client::logout(&client).await
                                {
                                    error!("Logout error: {}", e);
                                }
                                tx.send(AppEvent::LoggedOut).warn_closed("LoggedOut");
                            });
                        }

                        // Handle room join
                        if let Some(room_id) = app.take_pending_join() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                let room_id_parsed: Result<matrix_sdk::ruma::OwnedRoomOrAliasId, _> = room_id.as_str().try_into();
                                match room_id_parsed {
                                    Ok(id) => {
                                        if let Err(e) = client.join_room_by_id_or_alias(&id, &[]).await {
                                            tx.send(AppEvent::SyncError(format!("Join failed: {}", e))).warn_closed("SyncError");
                                        }
                                    }
                                    Err(e) => {
                                        tx.send(AppEvent::SyncError(format!("Invalid room: {}", e))).warn_closed("SyncError");
                                    }
                                }
                            });
                        }

                        // Handle pending DM
                        if let Some(user_id_str) = app.take_pending_dm() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                let user_id: Result<matrix_sdk::ruma::OwnedUserId, _> = user_id_str.as_str().try_into();
                                match user_id {
                                        Ok(uid) => {
                                            // Check for existing DM room
                                            if let Some(room) = client.get_dm_room(&uid) {
                                                tx.send(AppEvent::DmRoomReady {
                                                    room_id: room.room_id().to_string(),
                                                }).warn_closed("DmRoomReady");
                                            } else {
                                                // Create new DM room
                                                use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
                                                let mut request = CreateRoomRequest::new();
                                                request.invite = vec![uid.clone()];
                                                request.is_direct = true;

                                                use matrix_sdk::ruma::events::InitialStateEvent;
                                                use matrix_sdk::ruma::events::room::encryption::RoomEncryptionEventContent;
                                                let enc = RoomEncryptionEventContent::with_recommended_defaults();
                                                let enc_event = InitialStateEvent::with_empty_state_key(enc);
                                                request.initial_state.push(enc_event.to_raw_any());

                                                // Set call member event PL to 0 so both DM participants can use calls
                                                request.power_level_content_override = matrix::rooms::call_power_level_override();

                                                match client.create_room(request).await {
                                                    Ok(response) => {
                                                        tx.send(AppEvent::DmRoomReady {
                                                            room_id: response.room_id().to_string(),
                                                        }).warn_closed("DmRoomReady");
                                                    }
                                                    Err(e) => {
                                                        tx.send(AppEvent::SyncError(format!("Failed to create DM: {}", e))).warn_closed("SyncError");
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tx.send(AppEvent::SyncError(format!("Invalid user ID: {}", e))).warn_closed("SyncError");
                                        }
                                    }
                            });
                        }

                        // Handle room creation
                        if let Some(params) = app.take_pending_create_room() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
                                    use matrix_sdk::ruma::events::InitialStateEvent;
                                    use matrix_sdk::ruma::events::room::history_visibility::{
                                        HistoryVisibility, RoomHistoryVisibilityEventContent,
                                    };
                                    let mut request = CreateRoomRequest::new();
                                    request.name = Some(params.name);

                                    // Set topic as initial state if provided
                                    if let Some(topic) = params.topic {
                                        use matrix_sdk::ruma::events::room::topic::RoomTopicEventContent;
                                        let topic_content = RoomTopicEventContent::new(topic);
                                        let topic_event = InitialStateEvent::with_empty_state_key(topic_content);
                                        request.initial_state.push(topic_event.to_raw_any());
                                    }

                                    // Set history visibility as initial state
                                    let vis = match params.history_visibility.as_str() {
                                        "invited" => HistoryVisibility::Invited,
                                        "joined" => HistoryVisibility::Joined,
                                        "world_readable" => HistoryVisibility::WorldReadable,
                                        _ => HistoryVisibility::Shared,
                                    };
                                    let vis_content = RoomHistoryVisibilityEventContent::new(vis);
                                    let initial_event = InitialStateEvent::with_empty_state_key(vis_content);
                                    request.initial_state.push(initial_event.to_raw_any());

                                    // Enable encryption if requested
                                    if params.encrypted {
                                        use matrix_sdk::ruma::events::room::encryption::RoomEncryptionEventContent;
                                        let enc = RoomEncryptionEventContent::with_recommended_defaults();
                                        let enc_event = InitialStateEvent::with_empty_state_key(enc);
                                        request.initial_state.push(enc_event.to_raw_any());
                                    }

                                    // Set call member event PL to 0 so all participants can use calls
                                    request.power_level_content_override = matrix::rooms::call_power_level_override();

                                    match client.create_room(request).await {
                                        Ok(response) => {
                                            tx.send(AppEvent::RoomCreated {
                                                room_id: response.room_id().to_string(),
                                            }).warn_closed("RoomCreated");
                                        }
                                        Err(e) => {
                                            tx.send(AppEvent::SyncError(format!("Failed to create room: {}", e))).warn_closed("SyncError");
                                        }
                                    }
                            });
                        }

                        // Handle room leave
                        if let Some(room_id) = app.take_pending_leave() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                let room_id_parsed: Result<matrix_sdk::ruma::OwnedRoomId, _> = room_id.as_str().try_into();
                                if let Ok(id) = room_id_parsed
                                    && let Some(room) = client.get_room(&id)
                                    && let Err(e) = room.leave().await
                                {
                                    tx.send(AppEvent::SyncError(format!("Leave failed: {}", e))).warn_closed("SyncError");
                                }
                            });
                        }

                        // Handle accept invite
                        if let Some(room_id) = app.take_pending_accept_invite() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::accept_invite(&client, &room_id, &tx).await;
                            });
                        }

                        // Handle decline invite
                        if let Some(room_id) = app.take_pending_decline_invite() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::decline_invite(&client, &room_id, &tx).await;
                            });
                        }

                        // Handle invite user
                        if let Some((room_id, user_id)) = app.take_pending_invite_user() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::invite_user(&client, &room_id, &user_id, &tx).await;
                            });
                        }

                        // Handle user config fetch
                        if app.pending_user_config {
                            app.pending_user_config = false;
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::profile::fetch_user_config(&client, &tx).await;
                            });
                        }

                        // Handle display name update
                        if let Some(name) = app.pending_set_display_name.take() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::profile::set_user_display_name(&client, &name, &tx).await;
                            });
                        }

                        // Handle password change
                        if let Some((current, new)) = app.pending_change_password.take() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::profile::change_user_password(&client, &current, &new, &tx).await;
                            });
                        }

                        // Handle room info fetch
                        if app.pending_room_info {
                            app.pending_room_info = false;
                            let tx = event_tx.clone();
                            let rid = app.room_info.room_id.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::fetch_room_info(&client, &rid, &tx).await;
                            });
                        }

                        // Handle visibility update
                        if let Some((room_id, visibility)) = app.pending_set_visibility.take() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::set_history_visibility(&client, &room_id, &visibility, &tx).await;
                            });
                        }

                        // Handle room topic update
                        if let Some((room_id, topic)) = app.pending_set_room_topic.take() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::set_room_topic(&client, &room_id, &topic, &tx).await;
                            });
                        }

                        // Handle room name update
                        if let Some((room_id, name)) = app.pending_set_room_name.take() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::set_room_name(&client, &room_id, &name, &tx).await;
                            });
                        }

                        // Handle encryption enable
                        if let Some(room_id) = app.pending_enable_encryption.take() {
                            let tx = event_tx.clone();
                            spawn_with_client!(matrix_client, |client| async move {
                                matrix::rooms::enable_encryption(&client, &room_id, &tx).await;
                            });
                        }

                        // Handle recovery actions
                        if let Some(action) = app.pending_recovery.take() {
                            use state::RecoveryAction;
                            match action {
                                RecoveryAction::SubmitPassword(password) => {
                                    if let Some(ref mut modal) = app.recovery
                                        && let Some(tx) = modal.password_tx.take()
                                        && tx.send(password).is_err()
                                    {
                                        tracing::warn!("password oneshot: receiver dropped");
                                    }
                                }
                                other => {
                                    let tx = event_tx.clone();
                                    spawn_with_client!(matrix_client, |client| async move {
                                        match other {
                                                RecoveryAction::Check => {
                                                    client.encryption().wait_for_e2ee_initialization_tasks().await;
                                                    let state = client.encryption().recovery().state();
                                                    use matrix_sdk::encryption::recovery::RecoveryState;
                                                    let stage = match state {
                                                        RecoveryState::Enabled => state::RecoveryStage::Enabled,
                                                        RecoveryState::Disabled => state::RecoveryStage::Disabled,
                                                        RecoveryState::Incomplete => state::RecoveryStage::Incomplete,
                                                        _ => state::RecoveryStage::Disabled,
                                                    };
                                                    tx.send(AppEvent::RecoveryStateChecked(stage)).warn_closed("RecoveryStateChecked");
                                                }
                                                RecoveryAction::Create => {
                                                    match client.encryption().recovery()
                                                        .enable().wait_for_backups_to_upload().await {
                                                        Ok(key) => { tx.send(AppEvent::RecoveryKeyReady(key)).warn_closed("RecoveryKeyReady"); }
                                                        Err(e) => { tx.send(AppEvent::RecoveryError(e.to_string())).warn_closed("RecoveryError"); }
                                                    }
                                                }
                                                RecoveryAction::Recover(phrase) => {
                                                    match client.encryption().recovery().recover(&phrase).await {
                                                        Ok(()) => {
                                                            let state = client.encryption().recovery().state();
                                                            if matches!(state, matrix_sdk::encryption::recovery::RecoveryState::Incomplete) {
                                                                match matrix::client::heal_recovery(&client, &tx).await {
                                                                    Ok(new_key) => {
                                                                        tx.send(AppEvent::RecoveryKeyReady(new_key)).warn_closed("RecoveryKeyReady");
                                                                    }
                                                                    Err(e) => {
                                                                        tx.send(AppEvent::RecoveryError(
                                                                            format!("Recovery succeeded but healing failed: {}", e),
                                                                        )).warn_closed("RecoveryError");
                                                                    }
                                                                }
                                                            } else {
                                                                // Download room keys from backup so messages can be decrypted
                                                                let rooms: Vec<_> = client.joined_rooms().iter()
                                                                    .map(|r| r.room_id().to_owned())
                                                                    .collect();
                                                                for room_id in &rooms {
                                                                    if let Err(e) = client.encryption().backups().download_room_keys_for_room(room_id).await { tracing::warn!("download_room_keys failed for {room_id}: {e}"); }
                                                                }
                                                                tx.send(AppEvent::RecoveryRecovered).warn_closed("RecoveryRecovered");
                                                            }
                                                        }
                                                        Err(e) => { tx.send(AppEvent::RecoveryError(e.to_string())).warn_closed("RecoveryError"); }
                                                    }
                                                }
                                                RecoveryAction::Reset => {
                                                    use matrix_sdk::encryption::recovery::RecoveryState;
                                                    let is_incomplete = matches!(
                                                        client.encryption().recovery().state(),
                                                        RecoveryState::Incomplete
                                                    );
                                                    if is_incomplete {
                                                        match matrix::client::heal_recovery(&client, &tx).await {
                                                            Ok(key) => { tx.send(AppEvent::RecoveryKeyReady(key)).warn_closed("RecoveryKeyReady"); }
                                                            Err(e) => { tx.send(AppEvent::RecoveryError(e.to_string())).warn_closed("RecoveryError"); }
                                                        }
                                                    } else {
                                                        match client.encryption().recovery().reset_key().await {
                                                            Ok(key) => { tx.send(AppEvent::RecoveryKeyReady(key)).warn_closed("RecoveryKeyReady"); }
                                                            Err(e) => { tx.send(AppEvent::RecoveryError(e.to_string())).warn_closed("RecoveryError"); }
                                                        }
                                                    }
                                                }
                                                RecoveryAction::SubmitPassword(_) => unreachable!(),
                                            }
                                    });
                                }
                            }
                        }

                        // Handle outgoing verification (:verify command)
                        if let Some(target) = app.take_pending_verify() {
                            if let Some(handle) = app.verify_task_handle.take() {
                                handle.abort();
                            }
                            let tx = event_tx.clone();
                            let (confirm_tx, confirm_rx) = tokio::sync::oneshot::channel();
                            app.verify_confirm_tx = Some(confirm_tx);

                            let handle = spawn_with_client!(matrix_client, |client| async move {
                                match target {
                                    None => {
                                        matrix::verification::start_self_verification(
                                            client, tx, confirm_rx,
                                        )
                                        .await;
                                    }
                                    Some(user_id) => {
                                        matrix::verification::start_user_verification(
                                            client, &user_id, tx, confirm_rx,
                                        )
                                        .await;
                                    }
                                }
                            });
                            app.verify_task_handle = Some(handle);
                        }

                        // Handle incoming verification requests
                        {
                            let mut iv_guard = incoming_verification.lock().await;
                            if let Some(request) = iv_guard.take() {
                                if let Some(handle) = app.verify_task_handle.take() {
                                    handle.abort();
                                }
                                let tx = event_tx.clone();
                                let (confirm_tx, confirm_rx) = tokio::sync::oneshot::channel();
                                app.verify_confirm_tx = Some(confirm_tx);

                                let handle = tokio::spawn(async move {
                                    matrix::verification::handle_incoming_verification(
                                        request, tx, confirm_rx,
                                    )
                                    .await;
                                });
                                app.verify_task_handle = Some(handle);
                            }
                        }
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(until_render) => {}
        }

        // Check for login trigger
        if app.is_logging_in() && !login_in_progress {
            login_in_progress = true;
            let (homeserver, username, password) = app.login_credentials();
            let tx = event_tx.clone();
            let client_holder = matrix_client.clone();
            let iv = incoming_verification.clone();
            let cmd_tx = call_cmd_tx.clone();
            tokio::spawn(async move {
                match matrix::client::login(
                    &homeserver,
                    &username,
                    &password,
                    &tx,
                    accept_invalid_certs,
                )
                .await
                {
                    Ok(client) => {
                        *client_holder.lock().await = Some(client.clone());
                        let sync_tx = tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                matrix::sync::start_sync(client, sync_tx.clone(), iv, Some(cmd_tx))
                                    .await
                            {
                                error!("Sync error: {}", e);
                                sync_tx
                                    .send(AppEvent::SyncError(e.to_string()))
                                    .warn_closed("SyncError");
                            }
                        });
                    }
                    Err(e) => {
                        error!("Login failed: {:#}", e);
                        tx.send(AppEvent::LoginFailure(e.to_string()))
                            .warn_closed("LoginFailure");
                    }
                }
            });
        }

        // Reset login tracking when auth state changes away from LoggingIn
        if !app.is_logging_in() {
            login_in_progress = false;
        }

        // Check for registration trigger
        if app.is_registering() && !registration_in_progress {
            registration_in_progress = true;
            let (homeserver, username, password, token) = app.registration_credentials();
            let tx = event_tx.clone();
            let client_holder = matrix_client.clone();
            let iv = incoming_verification.clone();
            let cmd_tx = call_cmd_tx.clone();
            tokio::spawn(async move {
                match matrix::client::register(
                    &homeserver,
                    &username,
                    &password,
                    &token,
                    &tx,
                    accept_invalid_certs,
                )
                .await
                {
                    Ok(client) => {
                        *client_holder.lock().await = Some(client.clone());
                        let sync_tx = tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                matrix::sync::start_sync(client, sync_tx.clone(), iv, Some(cmd_tx))
                                    .await
                            {
                                error!("Sync error: {}", e);
                                sync_tx
                                    .send(AppEvent::SyncError(e.to_string()))
                                    .warn_closed("SyncError");
                            }
                        });
                    }
                    Err(e) => {
                        error!("Registration failed: {:#}", e);
                        tx.send(AppEvent::RegisterFailure(e.to_string()))
                            .warn_closed("RegisterFailure");
                    }
                }
            });
        }

        // Reset registration tracking
        if !app.is_registering() {
            registration_in_progress = false;
        }

        // Tick + render only when render_interval has elapsed
        let now = Instant::now();
        let elapsed = now.duration_since(last_render);
        if elapsed >= render_interval {
            let dt = elapsed.as_millis() as u64;
            last_render = now;

            let term_size = tui.size()?;
            let term_area = ratatui::layout::Rect::new(0, 0, term_size.width, term_size.height);
            app.effects.tick(dt, term_area);

            // Tick EMP effect with approximate room pane area
            let room_focused = app.vim.focus == crate::input::FocusPanel::RoomList;
            let room_area = ratatui::layout::Rect::new(
                term_area.x,
                term_area.y,
                24, // matches layout::compute_layout Constraint::Length(24)
                term_area.height.saturating_sub(1),
            );
            app.effects.tick_emp(dt, room_area, room_focused);

            // Tick EMP effect for members pane
            let members_focused = app.vim.focus == crate::input::FocusPanel::Members;
            let members_area = ratatui::layout::Rect::new(
                term_area.width.saturating_sub(20),
                term_area.y,
                20, // matches layout::compute_layout Constraint::Length(20)
                term_area.height.saturating_sub(1),
            );
            app.effects
                .tick_members_emp(dt, members_area, members_focused);

            app.room_list_anim.tick(dt);
            app.chat_title_reveal.tick(dt);
            app.members_title_reveal.tick(dt);
            if let Some(ref info) = app.call_info {
                let ds = match info.state {
                    voip::CallState::Connecting => ui::call_overlay::CallDisplayState::Connecting,
                    voip::CallState::Active => ui::call_overlay::CallDisplayState::Active,
                };
                app.call_popup.tick(dt, &ds);
            } else if app.incoming_call_room.is_some() {
                app.call_popup
                    .tick(dt, &ui::call_overlay::CallDisplayState::Ringing);
            }

            // Process at most one decoded image per frame to avoid batch freeze
            if let Ok((event_id, result)) = image_decode_rx.try_recv() {
                match result {
                    Ok((protocol, width, height)) => {
                        app.image_cache.insert(
                            event_id,
                            state::image_cache::CachedImage {
                                protocol: Some(protocol),
                                width: Some(width),
                                height: Some(height),
                                last_encoded_rect: None,
                            },
                        );
                    }
                    Err(e) => {
                        error!("Failed to decode image {}: {}", event_id, e);
                        app.image_cache.mark_failed(&event_id);
                    }
                }
            }

            tui.draw(|frame| ui::render(&mut app, frame))?;
        }

        if !app.running {
            break;
        }
    }

    // Cleanup
    if let Some(ref handle) = app.global_ptt {
        handle
            .active
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
    let _ = call_cmd_tx.send(voip::manager::CallCommand::Shutdown);
    terminal::restore()?;
    info!("gōsuto shut down cleanly");

    // Flush logs before exit (process::exit skips destructors)
    drop(_guard);

    // Force-exit to terminate the blocking rdev listener thread and any
    // in-flight async tasks that would otherwise delay shutdown.
    std::process::exit(0);
}
