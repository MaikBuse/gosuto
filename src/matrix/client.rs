use std::time::Duration;

use anyhow::{Context, Result};
use matrix_sdk::encryption::EncryptionSettings;
use matrix_sdk::{Client, config::SyncSettings};
use tracing::{debug, info, warn};

fn encryption_settings() -> EncryptionSettings {
    EncryptionSettings {
        auto_enable_cross_signing: true,
        ..Default::default()
    }
}

use crate::config;
use crate::event::{AppEvent, EventSender};
use crate::matrix::session::{self, StoredSession};

pub async fn try_restore_session(
    tx: &EventSender,
    accept_invalid_certs: bool,
) -> Result<Option<Client>> {
    let session_path = config::session_path()?;
    if !session_path.exists() {
        return Ok(None);
    }

    let stored = session::load_session(&session_path)?;
    debug!("Restoring session for {}", stored.user_id);

    let store_path = config::store_path_for_homeserver(&stored.homeserver)?;
    debug!("Using per-server store at {}", store_path.display());
    let mut builder = Client::builder()
        .homeserver_url(&stored.homeserver)
        .sqlite_store(&store_path, None)
        .with_encryption_settings(encryption_settings());

    if accept_invalid_certs {
        builder = builder.disable_ssl_verification();
    }

    let client = match builder.build().await {
        Ok(c) => c,
        Err(e) => {
            // Server unreachable (DNS, network, etc.) — session is likely still valid.
            // Return Ok(None) so main.rs does NOT delete session.json.
            info!(
                "Cannot reach homeserver during session restore, keeping session: {}",
                e
            );
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

    // Fresh login — clear any stale store data (crypto keys from old devices)
    if store_path.exists() {
        debug!("Clearing stale store at {}", store_path.display());
        std::fs::remove_dir_all(&store_path)?;
        std::fs::create_dir_all(&store_path)?;
    }

    debug!("Using per-server store at {}", store_path.display());

    let client = {
        // Try with server discovery first
        let mut builder = Client::builder()
            .server_name_or_homeserver_url(&homeserver)
            .sqlite_store(&store_path, None)
            .with_encryption_settings(encryption_settings());
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
                    .sqlite_store(&store_path, None)
                    .with_encryption_settings(encryption_settings());
                if accept_invalid_certs {
                    builder = builder.disable_ssl_verification();
                }
                builder.build().await?
            }
        }
    };

    debug!(
        "Resolved homeserver URL: {} (input was: {})",
        client.homeserver(),
        homeserver
    );

    tokio::time::timeout(
        Duration::from_secs(30),
        client
            .matrix_auth()
            .login_username(username, password)
            .initial_device_display_name("gosuto"),
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

pub async fn register(
    homeserver: &str,
    username: &str,
    password: &str,
    registration_token: &str,
    tx: &EventSender,
    accept_invalid_certs: bool,
) -> Result<Client> {
    let homeserver = normalize_homeserver_url(homeserver);
    info!(
        "Registration requested for homeserver input: {}",
        homeserver
    );

    let store_path = config::store_path_for_homeserver(&homeserver)?;

    // Fresh registration — clear any stale store data
    if store_path.exists() {
        debug!("Clearing stale store at {}", store_path.display());
        std::fs::remove_dir_all(&store_path)?;
        std::fs::create_dir_all(&store_path)?;
    }

    let client = {
        let mut builder = Client::builder()
            .server_name_or_homeserver_url(&homeserver)
            .sqlite_store(&store_path, None)
            .with_encryption_settings(encryption_settings());
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
                    .sqlite_store(&store_path, None)
                    .with_encryption_settings(encryption_settings());
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

    let response = tokio::time::timeout(
        Duration::from_secs(30),
        attempt_register(&client, username, password, registration_token),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Registration timed out — server may be rate limiting. Wait a moment and try again."
        )
    })??;

    info!("Registration succeeded for {}", response.user_id);

    // The SDK's register() internally calls set_session() when the server
    // returns an access_token, so the client is already authenticated.
    // Only fall back to explicit login for the rare inhibit_login case.
    if client.user_id().is_none() {
        info!("Server used inhibit_login — logging in explicitly");
        client
            .matrix_auth()
            .login_username(username, password)
            .initial_device_display_name("gosuto")
            .await
            .context("Login after registration failed")?;
    }

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

async fn attempt_register(
    client: &Client,
    username: &str,
    password: &str,
    registration_token: &str,
) -> Result<matrix_sdk::ruma::api::client::account::register::v3::Response> {
    use matrix_sdk::ruma::api::client::account::register::v3::Request as RegisterRequest;

    // Build initial request (no auth)
    let mut request = RegisterRequest::new();
    request.username = Some(username.to_owned());
    request.password = Some(password.to_owned());
    request.initial_device_display_name = Some("gosuto".to_owned());

    match client.matrix_auth().register(request).await {
        Ok(response) => Ok(response),
        Err(err) => {
            let Some(uiaa) = err.as_uiaa_response() else {
                return Err(anyhow::anyhow!("Registration failed: {}", err));
            };

            complete_uia_flow(client, username, password, registration_token, uiaa).await
        }
    }
}

async fn complete_uia_flow(
    client: &Client,
    username: &str,
    password: &str,
    registration_token: &str,
    initial_uiaa: &matrix_sdk::ruma::api::client::uiaa::UiaaInfo,
) -> Result<matrix_sdk::ruma::api::client::account::register::v3::Response> {
    use matrix_sdk::ruma::api::client::{
        account::register::v3::Request as RegisterRequest,
        uiaa::{AuthData, AuthType, Dummy, RegistrationToken},
    };

    let session_id = initial_uiaa.session.clone();

    // Find a flow where all stages are Dummy or RegistrationToken
    let flow = initial_uiaa
        .flows
        .iter()
        .find(|f| {
            f.stages
                .iter()
                .all(|s| matches!(s, AuthType::Dummy | AuthType::RegistrationToken))
        })
        .ok_or_else(|| {
            let types: Vec<String> = initial_uiaa
                .flows
                .iter()
                .flat_map(|f| f.stages.iter().map(|s| format!("{:?}", s)))
                .collect();
            anyhow::anyhow!(
                "No supported registration flow. Server requires: {}",
                types.join(", ")
            )
        })?;

    let mut completed: Vec<AuthType> = initial_uiaa.completed.clone();

    for stage in &flow.stages {
        if completed.contains(stage) {
            continue;
        }

        let auth_data: AuthData = match stage {
            AuthType::Dummy => {
                let mut dummy = Dummy::new();
                dummy.session = session_id.clone();
                AuthData::Dummy(dummy)
            }
            AuthType::RegistrationToken => {
                if registration_token.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Server requires a registration token but none was provided"
                    ));
                }
                let mut token = RegistrationToken::new(registration_token.to_owned());
                token.session = session_id.clone();
                AuthData::RegistrationToken(token)
            }
            other => {
                return Err(anyhow::anyhow!("Unsupported auth stage: {:?}", other));
            }
        };

        // Rebuild request each time (Request is not Clone)
        let mut request = RegisterRequest::new();
        request.username = Some(username.to_owned());
        request.password = Some(password.to_owned());
        request.initial_device_display_name = Some("gosuto".to_owned());
        request.auth = Some(auth_data);

        match client.matrix_auth().register(request).await {
            Ok(response) => return Ok(response),
            Err(err) => {
                let Some(uiaa) = err.as_uiaa_response() else {
                    return Err(anyhow::anyhow!("Registration failed: {}", err));
                };
                // Update completed stages from server response
                completed = uiaa.completed.clone();
            }
        }
    }

    Err(anyhow::anyhow!(
        "Registration UIA flow completed all stages but server did not return success"
    ))
}

pub async fn logout(client: &Client) -> Result<()> {
    let session_path = config::session_path()?;
    session::delete_session(&session_path)?;

    // Try to log out from the server, but don't fail if it doesn't work
    let _ = client.matrix_auth().logout().await;

    // Clean up only this server's store
    let store_path = config::store_path_for_homeserver_unchecked(client.homeserver().as_str())?;
    debug!("Removing per-server store at {}", store_path.display());
    if let Err(e) = std::fs::remove_dir_all(&store_path) {
        warn!("Could not remove store: {}", e);
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
        assert_eq!(
            normalize_homeserver_url("matrix.org/"),
            "https://matrix.org"
        );
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
