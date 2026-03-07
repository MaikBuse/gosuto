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
    tracing::info!("cross_signing_status: {:?}", cross_signing);

    let verified = match cross_signing {
        Some(ref status) => status.is_complete(),
        None => false,
    };

    // cross_signing_status checks private key availability, which may not
    // persist across restarts. Fall back to the SDK's verification_state
    // which checks if our device is cross-signed (persisted in crypto store).
    let vs = client.encryption().verification_state().get();
    tracing::info!("verification_state: {:?}", vs);

    let verified = verified || {
        use matrix_sdk::encryption::VerificationState;
        matches!(vs, VerificationState::Verified)
    };
    tracing::info!("final verified: {}", verified);

    let recovery_state = client.encryption().recovery().state();
    tracing::info!("recovery_state: {:?}", recovery_state);

    let recovery_enabled = matches!(
        recovery_state,
        matrix_sdk::encryption::recovery::RecoveryState::Enabled
    ) || (matches!(
        recovery_state,
        matrix_sdk::encryption::recovery::RecoveryState::Incomplete
    ) && verified);
    tracing::info!("recovery_enabled: {}", recovery_enabled);

    let _ = tx.send(AppEvent::UserConfigLoaded {
        display_name,
        verified,
        recovery_enabled,
    });
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
