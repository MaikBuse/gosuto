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
        .with(EnvFilter::from_env("WALRUST_LOG"))
        .init();

    info!("Starting walrust");

    // Create event channel
    let (event_tx, mut event_rx) = event::event_channel();

    // Initialize terminal
    let mut tui = terminal::init()?;

    // Create app
    let mut app = App::new(event_tx.clone());

    // Shared Matrix client
    let matrix_client: Arc<Mutex<Option<matrix_sdk::Client>>> = Arc::new(Mutex::new(None));

    // Try to restore session
    match matrix::client::try_restore_session(&event_tx).await {
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
        }
        Err(e) => {
            info!("Failed to restore session: {}", e);
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
        // Render
        tui.draw(|frame| ui::render(&app, frame))?;

        // Check for login trigger
        if app.is_logging_in() && !login_in_progress {
            login_in_progress = true;
            let (homeserver, username, password) = app.login_credentials();
            let tx = event_tx.clone();
            let client_holder = matrix_client.clone();
            tokio::spawn(async move {
                match matrix::client::login(&homeserver, &username, &password, &tx).await {
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
                                    let guard = client_holder.lock().await;
                                    if let Some(ref client) = *guard {
                                        if let Err(e) = matrix::messages::fetch_messages(client, &rid, &tx).await {
                                            error!("Failed to fetch messages: {}", e);
                                        }
                                    }
                                });

                                // Fetch members
                                let client_holder2 = matrix_client.clone();
                                let tx2 = event_tx.clone();
                                let rid2 = room_id.clone();
                                tokio::spawn(async move {
                                    let guard = client_holder2.lock().await;
                                    if let Some(ref client) = *guard {
                                        matrix::rooms::fetch_room_members(client, &rid2, &tx2).await;
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
                                let guard = client_holder.lock().await;
                                if let Some(ref client) = *guard {
                                    if let Err(e) = matrix::messages::send_message(client, &room_id, &body, &tx).await {
                                        error!("Failed to send message: {}", e);
                                    }
                                }
                            });
                        }

                        // Handle logout
                        if app.pending_logout {
                            app.pending_logout = false;
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let mut guard = client_holder.lock().await;
                                if let Some(ref client) = *guard {
                                    if let Err(e) = matrix::client::logout(client).await {
                                        error!("Logout error: {}", e);
                                    }
                                }
                                *guard = None;
                                let _ = tx.send(AppEvent::LoggedOut);
                            });
                        }

                        // Handle room join
                        if let Some(room_id) = app.take_pending_join() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let guard = client_holder.lock().await;
                                if let Some(ref client) = *guard {
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

                        // Handle room leave
                        if let Some(room_id) = app.take_pending_leave() {
                            let client_holder = matrix_client.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let guard = client_holder.lock().await;
                                if let Some(ref client) = *guard {
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
