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
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use app::App;
use event::AppEvent;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file-based logging if WALRUST_LOG is set
    let log_path = config::log_path()?;
    let file_appender = tracing_appender::rolling::daily(&log_path, "walrust.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(
            EnvFilter::try_from_env("WALRUST_LOG")
                .unwrap_or_else(|_| EnvFilter::new("error")),
        )
        .init();

    info!("Starting walrust");

    // Create event channel
    let (event_tx, mut event_rx) = event::event_channel();

    // Initialize terminal
    let mut tui = terminal::init()?;

    // Load config
    let walrust_config = config::load_config();
    info!("Config: {:?}", walrust_config);

    // Create app
    let accept_invalid_certs = walrust_config.network.accept_invalid_certs;
    let mut app = App::new(event_tx.clone(), walrust_config);

    // Shared Matrix client
    let matrix_client: Arc<Mutex<Option<matrix_sdk::Client>>> = Arc::new(Mutex::new(None));

    // Try to restore session
    match matrix::client::try_restore_session(&event_tx, accept_invalid_certs).await {
        Ok(Some(client)) => {
            *matrix_client.lock().await = Some(client.clone());
            let tx = event_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = matrix::sync::start_sync(client, tx.clone()).await {
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
                    && let Ok(store_path) = config::store_path_for_homeserver_unchecked(&stored.homeserver)
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
                Event::Key(key) => {
                    let _ = input_tx.send(AppEvent::Key(key));
                }
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

    // Spawn CallManager
    let (call_cmd_tx, call_cmd_rx) = voip::manager::command_channel();
    app.call_cmd_tx = Some(call_cmd_tx.clone());
    let call_manager = voip::manager::CallManager::new(
        call_cmd_rx,
        event_tx.clone(),
        matrix_client.clone(),
    );
    tokio::spawn(call_manager.run());

    // Track login state to avoid re-triggering
    let mut login_in_progress = false;

    // Main loop
    let render_interval = Duration::from_millis(config::RENDER_RATE_MS);

    loop {
        // Tick effects before render
        let term_size = tui.size()?;
        let term_area = ratatui::layout::Rect::new(0, 0, term_size.width, term_size.height);
        app.effects.tick(render_interval.as_millis() as u64, term_area);

        // Render
        tui.draw(|frame| ui::render(&app, frame))?;

        // Check for login trigger
        if app.is_logging_in() && !login_in_progress {
            login_in_progress = true;
            let (homeserver, username, password) = app.login_credentials();
            let tx = event_tx.clone();
            let client_holder = matrix_client.clone();
            tokio::spawn(async move {
                match matrix::client::login(&homeserver, &username, &password, &tx, accept_invalid_certs).await {
                    Ok(client) => {
                        *client_holder.lock().await = Some(client.clone());
                        let sync_tx = tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                matrix::sync::start_sync(client, sync_tx.clone()).await
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

        // Wait for events with timeout for rendering
        tokio::select! {
            event = event_rx.recv() => {
                match event {
                    Some(ev) => {
                        // Track room change for message fetching
                        let prev_room = app.messages.current_room_id.clone();
                        app.handle_event(ev);
                        let new_room = app.messages.current_room_id.clone();

                        // Fetch messages and members when room changes
                        if prev_room != new_room {
                            if let Some(ref room_id) = new_room {
                                // Fetch messages
                                let client_holder = matrix_client.clone();
                                let tx = event_tx.clone();
                                let rid = room_id.clone();
                                tokio::spawn(async move {
                                    let client = { client_holder.lock().await.clone() };
                                    if let Some(client) = client
                                        && let Err(e) = matrix::messages::fetch_messages(&client, &rid, &tx).await
                                    {
                                        error!("Failed to fetch messages for {}: {:?}", rid, e);
                                        let _ = tx.send(AppEvent::FetchError {
                                            room_id: rid,
                                            error: e.to_string(),
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
                            } else {
                                app.members_list.clear();
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

                        // Handle room leave
                        if let Some(room_id) = app.take_pending_leave() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let client = { client_holder.lock().await.clone() };
                                if let Some(client) = client {
                                    let room_id_parsed: Result<matrix_sdk::ruma::OwnedRoomId, _> = room_id.as_str().try_into();
                                    if let Ok(id) = room_id_parsed {
                                        if let Some(room) = client.get_room(&id) {
                                            if let Err(e) = room.leave().await {
                                                let _ = tx.send(AppEvent::SyncError(format!("Leave failed: {}", e)));
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(render_interval) => {}
        }

        if !app.running {
            break;
        }
    }

    // Cleanup
    let _ = call_cmd_tx.send(voip::manager::CallCommand::Shutdown);
    terminal::restore()?;
    info!("walrust shut down cleanly");

    Ok(())
}
