use std::time::Duration;

use matrix_sdk::Client;
use tracing::{error, warn};

use crate::event::{AppEvent, EventSender, RecoveryStatus, WarnClosed};

pub async fn fetch_user_config(client: &Client, tx: &EventSender) {
    let display_name = match client.account().get_display_name().await {
        Ok(name) => name,
        Err(e) => {
            error!("Failed to fetch display name: {}", e);
            tx.send(AppEvent::UserConfigError(format!(
                "Failed to fetch display name: {}",
                e
            )))
            .warn_closed("UserConfigError");
            return;
        }
    };

    if tokio::time::timeout(
        Duration::from_secs(5),
        client.encryption().wait_for_e2ee_initialization_tasks(),
    )
    .await
    .is_err()
    {
        warn!("Timed out waiting for e2ee initialization tasks, continuing anyway");
    }

    let cross_signing = client.encryption().cross_signing_status().await;

    let verified = match cross_signing {
        Some(ref status) => status.is_complete(),
        None => false,
    };

    // cross_signing_status checks private key availability, which may not
    // persist across restarts. Fall back to the SDK's verification_state
    // which checks if our device is cross-signed (persisted in crypto store).
    let vs = client.encryption().verification_state().get();

    let verified = verified || {
        use matrix_sdk::encryption::VerificationState;
        matches!(vs, VerificationState::Verified)
    };

    let recovery_status = match client.encryption().recovery().state() {
        matrix_sdk::encryption::recovery::RecoveryState::Enabled => RecoveryStatus::Enabled,
        matrix_sdk::encryption::recovery::RecoveryState::Incomplete => RecoveryStatus::Incomplete,
        _ => RecoveryStatus::Disabled,
    };

    tx.send(AppEvent::UserConfigLoaded {
        display_name,
        verified,
        recovery_status,
    })
    .warn_closed("UserConfigLoaded");
}

pub async fn change_user_password(
    client: &Client,
    current_password: &str,
    new_password: &str,
    tx: &EventSender,
) {
    let user_id = match client.user_id() {
        Some(id) => id.to_string(),
        None => {
            tx.send(AppEvent::UserConfigError("No user ID".to_string()))
                .warn_closed("UserConfigError");
            return;
        }
    };
    let identifier =
        matrix_sdk::ruma::api::client::uiaa::UserIdentifier::UserIdOrLocalpart(user_id);
    let password =
        matrix_sdk::ruma::api::client::uiaa::Password::new(identifier, current_password.to_owned());
    let auth = matrix_sdk::ruma::api::client::uiaa::AuthData::Password(password);

    match client
        .account()
        .change_password(new_password, Some(auth))
        .await
    {
        Ok(_) => {
            tx.send(AppEvent::UserConfigUpdated)
                .warn_closed("UserConfigUpdated");
        }
        Err(e) => {
            tx.send(AppEvent::UserConfigError(format!(
                "Failed to change password: {e}"
            )))
            .warn_closed("UserConfigError");
        }
    }
}

pub async fn set_user_display_name(client: &Client, name: &str, tx: &EventSender) {
    match client.account().set_display_name(Some(name)).await {
        Ok(_) => {
            tx.send(AppEvent::UserConfigUpdated)
                .warn_closed("UserConfigUpdated");
        }
        Err(e) => {
            tx.send(AppEvent::UserConfigError(format!(
                "Failed to set display name: {}",
                e
            )))
            .warn_closed("UserConfigError");
        }
    }
}
