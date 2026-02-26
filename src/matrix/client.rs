use std::time::Duration;

use anyhow::{Context, Result};
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

    let store_path = config::store_path_for_homeserver(&stored.homeserver)?;
    info!("Using per-server store at {}", store_path.display());
    let mut builder = Client::builder()
        .homeserver_url(&stored.homeserver)
        .sqlite_store(&store_path, None);

    if accept_invalid_certs {
        builder = builder.disable_ssl_verification();
    }

    let client = match builder.build().await {
        Ok(c) => c,
        Err(e) => {
            // Server unreachable (DNS, network, etc.) — session is likely still valid.
            // Return Ok(None) so main.rs does NOT delete session.json.
            info!("Cannot reach homeserver during session restore, keeping session: {}", e);
            return Ok(None);
        }
    };

    let session = matrix_sdk::authentication::matrix::MatrixSession {
        meta: matrix_sdk::SessionMeta {
            user_id: stored.user_id.as_str().try_into()?,
            device_id: stored.device_id.as_str().into(),
        },
        tokens: matrix_sdk::authentication::SessionTokens {
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

fn normalize_homeserver_url(input: &str) -> String {
    let url = input.trim().trim_end_matches('/');
    if url.contains("://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

pub async fn login(
    homeserver: &str,
    username: &str,
    password: &str,
    tx: &EventSender,
    accept_invalid_certs: bool,
) -> Result<Client> {
    let homeserver = normalize_homeserver_url(homeserver);
    info!("Login requested for homeserver input: {}", homeserver);

    let store_path = config::store_path_for_homeserver(&homeserver)?;
    info!("Using per-server store at {}", store_path.display());

    let client = {
        // Try with server discovery first
        let mut builder = Client::builder()
            .server_name_or_homeserver_url(&homeserver)
            .sqlite_store(&store_path, None);
        if accept_invalid_certs {
            builder = builder.disable_ssl_verification();
        }
        match builder.build().await {
            Ok(client) => client,
            Err(discovery_err) => {
                info!(
                    "Server discovery failed ({}), trying direct URL",
                    discovery_err
                );
                let mut builder = Client::builder()
                    .homeserver_url(&homeserver)
                    .sqlite_store(&store_path, None);
                if accept_invalid_certs {
                    builder = builder.disable_ssl_verification();
                }
                builder.build().await?
            }
        }
    };

    info!(
        "Resolved homeserver URL: {} (input was: {})",
        client.homeserver(),
        homeserver
    );

    tokio::time::timeout(
        Duration::from_secs(30),
        client
            .matrix_auth()
            .login_username(username, password)
            .initial_device_display_name("walrust"),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Login timed out — server may be rate limiting. Wait a moment and try again."
        )
    })?
    .with_context(|| {
        format!(
            "Login failed against homeserver {} (resolved from input: {})",
            client.homeserver(),
            homeserver
        )
    })?;

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

    // Clean up only this server's store
    let store_path = config::store_path_for_homeserver_unchecked(client.homeserver().as_str())?;
    info!("Removing per-server store at {}", store_path.display());
    if let Err(e) = std::fs::remove_dir_all(&store_path) {
        info!("Could not remove store: {}", e);
    }

    Ok(())
}

pub fn default_sync_settings() -> SyncSettings {
    SyncSettings::default().timeout(std::time::Duration::from_secs(30))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_plain_domain_prepends_https() {
        assert_eq!(normalize_homeserver_url("matrix.org"), "https://matrix.org");
    }

    #[test]
    fn normalize_preserves_https() {
        assert_eq!(
            normalize_homeserver_url("https://matrix.org"),
            "https://matrix.org"
        );
    }

    #[test]
    fn normalize_preserves_http() {
        assert_eq!(
            normalize_homeserver_url("http://matrix.org"),
            "http://matrix.org"
        );
    }

    #[test]
    fn normalize_strips_trailing_slashes() {
        assert_eq!(normalize_homeserver_url("matrix.org/"), "https://matrix.org");
        assert_eq!(
            normalize_homeserver_url("matrix.org///"),
            "https://matrix.org"
        );
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(
            normalize_homeserver_url("  matrix.org  "),
            "https://matrix.org"
        );
    }

    #[test]
    fn normalize_preserves_port() {
        assert_eq!(
            normalize_homeserver_url("matrix.org:8448"),
            "https://matrix.org:8448"
        );
    }

    #[test]
    fn normalize_preserves_path() {
        assert_eq!(
            normalize_homeserver_url("https://matrix.org/_matrix"),
            "https://matrix.org/_matrix"
        );
    }

    #[test]
    fn normalize_empty_string() {
        // Documents current behavior: empty input produces bare scheme
        assert_eq!(normalize_homeserver_url(""), "https://");
    }
}
