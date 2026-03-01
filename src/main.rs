#![recursion_limit = "256"]

mod app;
mod config;
mod event;
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

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file-based logging (controlled by GOSUTO_LOG env var)
    let log_path = config::log_path()?;
    let file_appender = tracing_appender::rolling::daily(&log_path, "gosuto.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(
            EnvFilter::try_from_env("GOSUTO_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info,matrix_sdk=warn,hyper=warn")),
        )
        .init();

    config::cleanup_old_logs(&log_path, 7);
    info!("Starting gōsuto");

    // Create event channel
    let (event_tx, mut event_rx) = event::event_channel();

    // Initialize terminal
    let mut tui = terminal::init()?;

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
    let mut app = App::new(event_tx.clone(), gosuto_config);

    // Shared Matrix client
    let matrix_client: Arc<Mutex<Option<matrix_sdk::Client>>> = Arc::new(Mutex::new(None));

    // Shared state for incoming verification requests
    let incoming_verification: matrix::sync::IncomingVerification =
        Arc::new(Mutex::new(None));

    // Try to restore session
    match matrix::client::try_restore_session(&event_tx, accept_invalid_certs).await {
        Ok(Some(client)) => {
            *matrix_client.lock().await = Some(client.clone());
            let tx = event_tx.clone();
            let iv = incoming_verification.clone();
            tokio::spawn(async move {
                if let Err(e) = matrix::sync::start_sync(client, tx.clone(), iv).await {
                    error!("Sync error: {}", e);
                    let _ = tx.send(AppEvent::SyncError(e.to_string()));
                }
            });
        }
        Ok(None) => {
            info!("No stored session found");
            if let Some(creds) = matrix::credentials::load_credentials() {
                let _ = event_tx.send(AppEvent::AutoLogin {
                    homeserver: creds.homeserver,
                    username: creds.username,
                    password: creds.password,
                });
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
                let _ = matrix::session::delete_session(&session_path);
            }
            if let Some(creds) = matrix::credentials::load_credentials() {
                let _ = event_tx.send(AppEvent::AutoLogin {
                    homeserver: creds.homeserver,
                    username: creds.username,
                    password: creds.password,
                });
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
                        let _ = input_tx.send(AppEvent::Key(key));
                    }
                    KeyEventKind::Release => {
                        let _ = input_tx.send(AppEvent::KeyRelease(key));
                    }
                },
                Event::Resize(w, h) => {
                    let _ = input_tx.send(AppEvent::Resize(w, h));
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
    let shared_audio_config = Arc::new(std::sync::Mutex::new(app.config.audio.clone()));
    let ptt_transmitting = app.ptt_transmitting.clone();

    // Spawn CallManager
    let (call_cmd_tx, call_cmd_rx) = voip::manager::command_channel();
    app.call_cmd_tx = Some(call_cmd_tx.clone());
    let call_manager = voip::manager::CallManager::new(
        call_cmd_rx,
        event_tx.clone(),
        matrix_client.clone(),
        shared_audio_config.clone(),
        ptt_transmitting,
    );
    tokio::spawn(call_manager.run());

    // Track login/registration state to avoid re-triggering
    let mut login_in_progress = false;
    let mut registration_in_progress = false;

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
                            // Clear stale members immediately so guards
                            // don't use the previous room's member list.
                            app.members_list.clear();

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
                                            let _ = tx.send(AppEvent::FetchError {
                                                room_id: rid,
                                                error: e.to_string(),
                                            });
                                        }
                                    } else {
                                        let _ = tx.send(AppEvent::FetchError {
                                            room_id: rid,
                                            error: "Not connected".to_string(),
                                        });
                                    }
                                });

                                // Fetch members
                                let client_holder2 = matrix_client.clone();
                                let tx2 = event_tx.clone();
                                let rid2 = room_id.clone();
                                tokio::spawn(async move {
                                    let client = { client_holder2.lock().await.clone() };
                                    if let Some(client) = client {
                                        matrix::rooms::fetch_room_members(&client, &rid2, &tx2).await;
                                    }
                                });
                            }
                        }

                        // Handle message sending
                        if let Some((room_id, body)) = app.take_pending_send() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client
                                    && let Err(e) = matrix::messages::send_message(&client, &room_id, &body, &tx).await
                                {
                                    error!("Failed to send message: {}", e);
                                }
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
                                let _ = tx.send(AppEvent::LoggedOut);
                            });
                        }

                        // Handle room join
                        if let Some(room_id) = app.take_pending_join() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    let room_id_parsed: Result<matrix_sdk::ruma::OwnedRoomOrAliasId, _> = room_id.as_str().try_into();
                                    match room_id_parsed {
                                        Ok(id) => {
                                            if let Err(e) = client.join_room_by_id_or_alias(&id, &[]).await {
                                                let _ = tx.send(AppEvent::SyncError(format!("Join failed: {}", e)));
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(AppEvent::SyncError(format!("Invalid room: {}", e)));
                                        }
                                    }
                                }
                            });
                        }

                        // Handle pending DM
                        if let Some(user_id_str) = app.take_pending_dm() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    let user_id: Result<matrix_sdk::ruma::OwnedUserId, _> = user_id_str.as_str().try_into();
                                    match user_id {
                                        Ok(uid) => {
                                            // Check for existing DM room
                                            if let Some(room) = client.get_dm_room(&uid) {
                                                let _ = tx.send(AppEvent::DmRoomReady {
                                                    room_id: room.room_id().to_string(),
                                                });
                                            } else {
                                                // Create new DM room
                                                use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
                                                let mut request = CreateRoomRequest::new();
                                                request.invite = vec![uid.clone()];
                                                request.is_direct = true;

                                                match client.create_room(request).await {
                                                    Ok(response) => {
                                                        let _ = tx.send(AppEvent::DmRoomReady {
                                                            room_id: response.room_id().to_string(),
                                                        });
                                                    }
                                                    Err(e) => {
                                                        let _ = tx.send(AppEvent::SyncError(format!("Failed to create DM: {}", e)));
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(AppEvent::SyncError(format!("Invalid user ID: {}", e)));
                                        }
                                    }
                                }
                            });
                        }

                        // Handle room creation
                        if let Some((room_name, visibility)) = app.take_pending_create_room() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
                                    use matrix_sdk::ruma::events::InitialStateEvent;
                                    use matrix_sdk::ruma::events::room::history_visibility::{
                                        HistoryVisibility, RoomHistoryVisibilityEventContent,
                                    };
                                    let mut request = CreateRoomRequest::new();
                                    request.name = Some(room_name);

                                    // Set history visibility as initial state
                                    let vis = match visibility.as_str() {
                                        "invited" => HistoryVisibility::Invited,
                                        "joined" => HistoryVisibility::Joined,
                                        "world_readable" => HistoryVisibility::WorldReadable,
                                        _ => HistoryVisibility::Shared,
                                    };
                                    let vis_content = RoomHistoryVisibilityEventContent::new(vis);
                                    let initial_event = InitialStateEvent::with_empty_state_key(vis_content);
                                    request.initial_state.push(initial_event.to_raw_any());

                                    match client.create_room(request).await {
                                        Ok(response) => {
                                            let _ = tx.send(AppEvent::RoomCreated {
                                                room_id: response.room_id().to_string(),
                                            });
                                        }
                                        Err(e) => {
                                            let _ = tx.send(AppEvent::SyncError(format!("Failed to create room: {}", e)));
                                        }
                                    }
                                }
                            });
                        }

                        // Handle room leave
                        if let Some(room_id) = app.take_pending_leave() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    let room_id_parsed: Result<matrix_sdk::ruma::OwnedRoomId, _> = room_id.as_str().try_into();
                                    if let Ok(id) = room_id_parsed
                                        && let Some(room) = client.get_room(&id)
                                        && let Err(e) = room.leave().await
                                    {
                                        let _ = tx.send(AppEvent::SyncError(format!("Leave failed: {}", e)));
                                    }
                                }
                            });
                        }
                        // Handle room info fetch
                        if app.pending_room_info {
                            app.pending_room_info = false;
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            let rid = app.room_info.room_id.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    matrix::rooms::fetch_room_info(&client, &rid, &tx).await;
                                }
                            });
                        }

                        // Handle visibility update
                        if let Some((room_id, visibility)) = app.pending_set_visibility.take() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    matrix::rooms::set_history_visibility(&client, &room_id, &visibility, &tx).await;
                                }
                            });
                        }

                        // Handle room name update
                        if let Some((room_id, name)) = app.pending_set_room_name.take() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    matrix::rooms::set_room_name(&client, &room_id, &name, &tx).await;
                                }
                            });
                        }

                        // Handle outgoing verification (:verify command)
                        if let Some(target) = app.take_pending_verify() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            let (confirm_tx, confirm_rx) = tokio::sync::oneshot::channel();
                            app.verify_confirm_tx = Some(confirm_tx);

                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
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
                                }
                            });
                        }

                        // Handle recovery state check
                        if app.pending_recovery {
                            app.pending_recovery = false;
                            app.recovery_modal = Some(crate::state::RecoveryModalState {
                                stage: crate::state::RecoveryStage::Checking,
                                confirm_buffer: String::new(),
                                key_buffer: String::new(),
                                copied: false,
                            });
                            let client = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = client.lock().await.clone();
                                if let Some(client) = client {
                                    let recovery = client.encryption().recovery();
                                    let state = recovery.state();
                                    let state_str = format!("{:?}", state);
                                    let _ = tx.send(AppEvent::RecoveryState(state_str));
                                }
                            });
                        }

                        // Handle recovery key creation
                        if app.pending_recovery_create {
                            app.pending_recovery_create = false;
                            let client = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = client.lock().await.clone();
                                if let Some(client) = client {
                                    match client
                                        .encryption()
                                        .recovery()
                                        .enable()
                                        .wait_for_backups_to_upload()
                                        .await
                                    {
                                        Ok(key) => {
                                            let _ = tx.send(AppEvent::RecoveryKeyReady(key));
                                        }
                                        Err(e) => {
                                            let _ =
                                                tx.send(AppEvent::RecoveryError(e.to_string()));
                                        }
                                    }
                                }
                            });
                        }

                        // Handle recovery key reset
                        if app.pending_recovery_reset {
                            app.pending_recovery_reset = false;
                            let client = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = client.lock().await.clone();
                                if let Some(client) = client {
                                    match client
                                        .encryption()
                                        .recovery()
                                        .reset_key()
                                        .await
                                    {
                                        Ok(key) => {
                                            let _ = tx.send(AppEvent::RecoveryKeyReady(key));
                                        }
                                        Err(e) => {
                                            let _ =
                                                tx.send(AppEvent::RecoveryError(e.to_string()));
                                        }
                                    }
                                }
                            });
                        }

                        // Handle recovery key import
                        if let Some(recovery_key) = app.pending_recovery_recover.take() {
                            let client = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = client.lock().await.clone();
                                if let Some(client) = client {
                                    match client
                                        .encryption()
                                        .recovery()
                                        .recover(&recovery_key)
                                        .await
                                    {
                                        Ok(()) => {
                                            let _ = tx.send(AppEvent::RecoveryRecovered);
                                        }
                                        Err(e) => {
                                            let _ =
                                                tx.send(AppEvent::RecoveryError(e.to_string()));
                                        }
                                    }
                                }
                            });
                        }

                        // Handle incoming verification requests
                        {
                            let mut iv_guard = incoming_verification.lock().await;
                            if let Some(request) = iv_guard.take() {
                                let tx = event_tx.clone();
                                let (confirm_tx, confirm_rx) = tokio::sync::oneshot::channel();
                                app.verify_confirm_tx = Some(confirm_tx);

                                tokio::spawn(async move {
                                    matrix::verification::handle_incoming_verification(
                                        request, tx, confirm_rx,
                                    )
                                    .await;
                                });
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
                            if let Err(e) = matrix::sync::start_sync(client, sync_tx.clone(), iv).await
                            {
                                error!("Sync error: {}", e);
                                let _ = sync_tx.send(AppEvent::SyncError(e.to_string()));
                            }
                        });
                    }
                    Err(e) => {
                        error!("Login failed: {:#}", e);
                        let _ = tx.send(AppEvent::LoginFailure(e.to_string()));
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
                            if let Err(e) = matrix::sync::start_sync(client, sync_tx.clone(), iv).await
                            {
                                error!("Sync error: {}", e);
                                let _ = sync_tx.send(AppEvent::SyncError(e.to_string()));
                            }
                        });
                    }
                    Err(e) => {
                        error!("Registration failed: {:#}", e);
                        let _ = tx.send(AppEvent::RegisterFailure(e.to_string()));
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
            let room_focused =
                app.vim.focus == crate::input::FocusPanel::RoomList;
            let room_area = ratatui::layout::Rect::new(
                term_area.x,
                term_area.y,
                24, // matches layout::compute_layout Constraint::Length(24)
                term_area.height.saturating_sub(1),
            );
            app.effects.tick_emp(dt, room_area, room_focused);

            // Tick EMP effect for members pane
            let members_focused =
                app.vim.focus == crate::input::FocusPanel::Members;
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

            tui.draw(|frame| ui::render(&app, frame))?;
        }

        if !app.running {
            break;
        }
    }

    // Cleanup
    let _ = call_cmd_tx.send(voip::manager::CallCommand::Shutdown);
    terminal::restore()?;
    info!("gōsuto shut down cleanly");

    Ok(())
}
