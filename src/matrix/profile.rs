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

    let _ = tx.send(AppEvent::UserConfigLoaded { display_name });
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
