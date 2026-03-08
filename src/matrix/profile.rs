use matrix_sdk::Client;
use tracing::error;

use crate::event::{AppEvent, EventSender};

pub async fn fetch_user_config(client: &Client, tx: &EventSender) {
    let display_name = match client.account().get_display_name().await {
        Ok(name) => name,
        Err(e) => {
            error!("Failed to fetch display name: {}", e);
            let _ = tx.send(AppEvent::UserConfigError(format!(
                "Failed to fetch display name: {}",
                e
            )));
            return;
        }
    };

    client
        .encryption()
        .wait_for_e2ee_initialization_tasks()
        .await;

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

    let recovery_state = client.encryption().recovery().state();
    // Incomplete means recovery is set up on the server but private
    // keys aren't cached locally — treat as enabled.
    let recovery_enabled = matches!(
        recovery_state,
        matrix_sdk::encryption::recovery::RecoveryState::Enabled
            | matrix_sdk::encryption::recovery::RecoveryState::Incomplete
    );

    let _ = tx.send(AppEvent::UserConfigLoaded {
        display_name,
        verified,
        recovery_enabled,
    });
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
            let _ = tx.send(AppEvent::UserConfigError("No user ID".to_string()));
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
            let _ = tx.send(AppEvent::UserConfigUpdated);
        }
        Err(e) => {
            let _ = tx.send(AppEvent::UserConfigError(format!(
                "Failed to change password: {e}"
            )));
        }
    }
}

pub async fn set_user_display_name(client: &Client, name: &str, tx: &EventSender) {
    match client.account().set_display_name(Some(name)).await {
        Ok(_) => {
            let _ = tx.send(AppEvent::UserConfigUpdated);
        }
        Err(e) => {
            let _ = tx.send(AppEvent::UserConfigError(format!(
                "Failed to set display name: {}",
                e
            )));
        }
    }
}
