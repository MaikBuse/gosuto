use futures::StreamExt;
use matrix_sdk::Client;
use matrix_sdk::encryption::verification::{
    SasState, SasVerification, Verification, VerificationRequest, VerificationRequestState,
};
use tracing::{info, warn};

use crate::event::{AppEvent, EventSender, WarnClosed};

/// Drive a SAS verification flow once we have a SasVerification handle.
async fn drive_sas(
    sas: SasVerification,
    tx: &EventSender,
    confirm_rx: tokio::sync::oneshot::Receiver<bool>,
) {
    let sender = sas.other_user_id().to_string();
    let mut stream = sas.changes();

    // If we didn't start the SAS, we need to accept the start message
    if !sas.we_started()
        && let Err(e) = sas.accept().await
    {
        tx.send(AppEvent::VerificationError(format!(
            "Failed to accept SAS: {e}"
        )))
        .warn_closed("VerificationError");
        return;
    }

    let mut confirm_rx = Some(confirm_rx);

    while let Some(state) = stream.next().await {
        match state {
            SasState::KeysExchanged { emojis, .. } => {
                let emoji_data: Vec<(String, String)> = if let Some(emojis) = emojis {
                    emojis
                        .emojis
                        .iter()
                        .map(|e| (e.symbol.to_string(), e.description.to_string()))
                        .collect()
                } else {
                    vec![]
                };

                tx.send(AppEvent::VerificationSasEmoji {
                    emojis: emoji_data,
                    sender: sender.clone(),
                })
                .warn_closed("VerificationSasEmoji");

                // Wait for user confirmation
                if let Some(rx) = confirm_rx.take() {
                    match rx.await {
                        Ok(true) => {
                            if let Err(e) = sas.confirm().await {
                                tx.send(AppEvent::VerificationError(format!(
                                    "Failed to confirm: {e}"
                                )))
                                .warn_closed("VerificationError");
                                return;
                            }
                        }
                        Ok(false) => {
                            if let Err(e) = sas.mismatch().await {
                                tracing::warn!("sas.mismatch() failed: {e}");
                            }
                            tx.send(AppEvent::VerificationCancelled {
                                reason: "Emoji mismatch".to_string(),
                            })
                            .warn_closed("VerificationCancelled");
                            return;
                        }
                        Err(_) => {
                            // Channel dropped (modal closed)
                            if let Err(e) = sas.cancel().await {
                                tracing::warn!("sas.cancel() failed: {e}");
                            }
                            tx.send(AppEvent::VerificationCancelled {
                                reason: "Cancelled".to_string(),
                            })
                            .warn_closed("VerificationCancelled");
                            return;
                        }
                    }
                }
            }
            SasState::Done { .. } => {
                info!("SAS verification completed with {}", sender);
                tx.send(AppEvent::VerificationCompleted)
                    .warn_closed("VerificationCompleted");
                return;
            }
            SasState::Cancelled(info) => {
                let reason = info.reason().to_string();
                warn!("SAS verification cancelled: {}", reason);
                tx.send(AppEvent::VerificationCancelled { reason })
                    .warn_closed("VerificationCancelled");
                return;
            }
            _ => {}
        }
    }
}

/// Drive a VerificationRequest through its state changes until SAS completes.
async fn drive_request(
    request: VerificationRequest,
    tx: &EventSender,
    confirm_rx: tokio::sync::oneshot::Receiver<bool>,
    we_initiated: bool,
) {
    let sender = request.other_user_id().to_string();

    // If we received the request (didn't initiate), accept it
    if !we_initiated && let Err(e) = request.accept().await {
        tx.send(AppEvent::VerificationError(format!(
            "Failed to accept verification: {e}"
        )))
        .warn_closed("VerificationError");
        return;
    }

    tx.send(AppEvent::VerificationRequestReceived {
        sender: sender.clone(),
    })
    .warn_closed("VerificationRequestReceived");

    let mut stream = request.changes();

    while let Some(state) = stream.next().await {
        match state {
            VerificationRequestState::Ready { .. } => {
                // We're ready — if we initiated, start SAS
                if we_initiated {
                    match request.start_sas().await {
                        Ok(Some(sas)) => {
                            drive_sas(sas, tx, confirm_rx).await;
                            return;
                        }
                        Ok(None) => {
                            tx.send(AppEvent::VerificationError(
                                "Failed to start SAS verification".to_string(),
                            ))
                            .warn_closed("VerificationError");
                            return;
                        }
                        Err(e) => {
                            tx.send(AppEvent::VerificationError(format!("SAS start error: {e}")))
                                .warn_closed("VerificationError");
                            return;
                        }
                    }
                }
                // If we're responding, wait for Transitioned (other side starts SAS)
            }
            VerificationRequestState::Transitioned { verification } => match verification {
                Verification::SasV1(sas) => {
                    drive_sas(sas, tx, confirm_rx).await;
                    return;
                }
                _ => {
                    tx.send(AppEvent::VerificationError(
                        "Unsupported verification method".to_string(),
                    ))
                    .warn_closed("VerificationError");
                    return;
                }
            },
            VerificationRequestState::Done => {
                tx.send(AppEvent::VerificationCompleted)
                    .warn_closed("VerificationCompleted");
                return;
            }
            VerificationRequestState::Cancelled(info) => {
                let reason = info.reason().to_string();
                tx.send(AppEvent::VerificationCancelled { reason })
                    .warn_closed("VerificationCancelled");
                return;
            }
            _ => {}
        }
    }
}

/// Start self-verification (verify this device against another of our devices).
pub async fn start_self_verification(
    client: Client,
    tx: EventSender,
    confirm_rx: tokio::sync::oneshot::Receiver<bool>,
) {
    let user_id = match client.user_id() {
        Some(id) => id.to_owned(),
        None => {
            tx.send(AppEvent::VerificationError("Not logged in".to_string()))
                .warn_closed("VerificationError");
            return;
        }
    };

    let identity = match client.encryption().get_user_identity(&user_id).await {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            tx.send(AppEvent::VerificationError(
                "No cross-signing identity found. Try logging out and back in.".to_string(),
            ))
            .warn_closed("VerificationError");
            return;
        }
        Err(e) => {
            tx.send(AppEvent::VerificationError(format!(
                "Failed to get identity: {e}"
            )))
            .warn_closed("VerificationError");
            return;
        }
    };

    let request = match identity.request_verification().await {
        Ok(req) => req,
        Err(e) => {
            tx.send(AppEvent::VerificationError(format!(
                "Failed to request verification: {e}"
            )))
            .warn_closed("VerificationError");
            return;
        }
    };

    drive_request(request, &tx, confirm_rx, true).await;
}

/// Start verification of another user.
pub async fn start_user_verification(
    client: Client,
    user_id_str: &str,
    tx: EventSender,
    confirm_rx: tokio::sync::oneshot::Receiver<bool>,
) {
    let user_id: matrix_sdk::ruma::OwnedUserId = match user_id_str.try_into() {
        Ok(id) => id,
        Err(e) => {
            tx.send(AppEvent::VerificationError(format!("Invalid user ID: {e}")))
                .warn_closed("VerificationError");
            return;
        }
    };

    let identity = match client.encryption().request_user_identity(&user_id).await {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            tx.send(AppEvent::VerificationError(format!(
                "No cross-signing identity found for {user_id}. They may not have set up cross-signing."
            )))
            .warn_closed("VerificationError");
            return;
        }
        Err(e) => {
            tx.send(AppEvent::VerificationError(format!(
                "Failed to get identity: {e}"
            )))
            .warn_closed("VerificationError");
            return;
        }
    };

    let request = match identity.request_verification().await {
        Ok(req) => req,
        Err(e) => {
            tx.send(AppEvent::VerificationError(format!(
                "Failed to request verification: {e}"
            )))
            .warn_closed("VerificationError");
            return;
        }
    };

    drive_request(request, &tx, confirm_rx, true).await;
}

/// Handle an incoming verification request from another device/user.
pub async fn handle_incoming_verification(
    request: VerificationRequest,
    tx: EventSender,
    confirm_rx: tokio::sync::oneshot::Receiver<bool>,
) {
    drive_request(request, &tx, confirm_rx, false).await;
}
