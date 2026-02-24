use anyhow::Result;
use matrix_sdk::{Client, config::SyncSettings};
use tracing::info;

use crate::config;
use crate::event::{AppEvent, EventSender};
use crate::matrix::session::{self, StoredSession};

pub async fn try_restore_session(tx: &EventSender, accept_invalid_certs: bool) -> Result<Option<Client>> {
    let session_path = config::session_path()?;
    if !session_path.exists() {
        return Ok(None);
    }

    let stored = session::load_session(&session_path)?;
    info!("Restoring session for {}", stored.user_id);

    let store_path = config::store_path()?;
    let mut builder = Client::builder()
        .homeserver_url(&stored.homeserver)
        .sqlite_store(&store_path, None);

    if accept_invalid_certs {
        builder = builder.disable_ssl_verification();
    }

    let client = builder.build().await?;

    let session = matrix_sdk::authentication::matrix::MatrixSession {
        meta: matrix_sdk::SessionMeta {
            user_id: stored.user_id.as_str().try_into()?,
            device_id: stored.device_id.as_str().into(),
        },
        tokens: matrix_sdk::authentication::matrix::MatrixSessionTokens {
            access_token: stored.access_token,
            refresh_token: None,
        },
    };

    client.restore_session(session).await?;

    let _ = tx.send(AppEvent::LoginSuccess {
        user_id: stored.user_id,
        device_id: stored.device_id,
        homeserver: stored.homeserver,
    });

    Ok(Some(client))
}

pub async fn login(
    homeserver: &str,
    username: &str,
    password: &str,
    tx: &EventSender,
    accept_invalid_certs: bool,
) -> Result<Client> {
    let store_path = config::store_path()?;

    let client = {
        // Try with server discovery first
        let mut builder = Client::builder()
            .server_name_or_homeserver_url(homeserver)
            .sqlite_store(&store_path, None);
        if accept_invalid_certs {
            builder = builder.disable_ssl_verification();
        }
        match builder.build().await {
            Ok(client) => client,
            Err(discovery_err) => {
                tracing::info!(
                    "Server discovery failed ({}), trying direct URL",
                    discovery_err
                );
                let mut builder = Client::builder()
                    .homeserver_url(homeserver)
                    .sqlite_store(&store_path, None);
                if accept_invalid_certs {
                    builder = builder.disable_ssl_verification();
                }
                builder.build().await?
            }
        }
    };

    client
        .matrix_auth()
        .login_username(username, password)
        .initial_device_display_name("walrust")
        .await?;

    let user_id = client
        .user_id()
        .ok_or_else(|| anyhow::anyhow!("No user ID after login"))?
        .to_string();
    let device_id = client
        .device_id()
        .ok_or_else(|| anyhow::anyhow!("No device ID after login"))?
        .to_string();

    // Save session
    let session_path = config::session_path()?;
    let stored = StoredSession {
        homeserver: client.homeserver().to_string(),
        user_id: user_id.clone(),
        device_id: device_id.clone(),
        access_token: client
            .matrix_auth()
            .session()
            .ok_or_else(|| anyhow::anyhow!("No session after login"))?
            .tokens
            .access_token,
    };
    session::save_session(&session_path, &stored)?;

    let _ = tx.send(AppEvent::LoginSuccess {
        user_id,
        device_id,
        homeserver: client.homeserver().to_string(),
    });

    Ok(client)
}

pub async fn logout(client: &Client) -> Result<()> {
    let session_path = config::session_path()?;
    session::delete_session(&session_path)?;

    // Try to log out from the server, but don't fail if it doesn't work
    let _ = client.matrix_auth().logout().await;

    // Clean up store
    let store_path = config::store_path()?;
    let _ = std::fs::remove_dir_all(&store_path);

    Ok(())
}

pub fn default_sync_settings() -> SyncSettings {
    SyncSettings::default().timeout(std::time::Duration::from_secs(30))
}
