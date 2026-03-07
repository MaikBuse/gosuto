use futures::StreamExt;
use matrix_sdk::Client;
use matrix_sdk::encryption::verification::{
    SasState, SasVerification, Verification, VerificationRequest, VerificationRequestState,
};
use tracing::{info, warn};

use crate::event::{AppEvent, EventSender};

/// Drive a SAS verification flow once we have a SasVerification handle.
async fn drive_sas(
    sas: SasVerification,
    flow_id: String,
    tx: &EventSender,
    confirm_rx: tokio::sync::oneshot::Receiver<bool>,
) {
    let sender = sas.other_user_id().to_string();
    let mut stream = sas.changes();

    // If we didn't start the SAS, we need to accept the start message
    if !sas.we_started()
        && let Err(e) = sas.accept().await
    {
        let _ = tx.send(AppEvent::VerificationError(format!(
            "Failed to accept SAS: {e}"
        )));
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

                let _ = tx.send(AppEvent::VerificationSasEmoji {
                    emojis: emoji_data,
                    flow_id: flow_id.clone(),
                    sender: sender.clone(),
                });

                // Wait for user confirmation
                if let Some(rx) = confirm_rx.take() {
                    match rx.await {
                        Ok(true) => {
                            if let Err(e) = sas.confirm().await {
                                let _ = tx.send(AppEvent::VerificationError(format!(
                                    "Failed to confirm: {e}"
                                )));
                                return;
                            }
                        }
                        Ok(false) => {
                            let _ = sas.mismatch().await;
                            let _ = tx.send(AppEvent::VerificationCancelled {
                                reason: "Emoji mismatch".to_string(),
                            });
                            return;
                        }
                        Err(_) => {
                            // Channel dropped (modal closed)
                            let _ = sas.cancel().await;
                            let _ = tx.send(AppEvent::VerificationCancelled {
                                reason: "Cancelled".to_string(),
                            });
                            return;
                        }
                    }
                }
            }
            SasState::Done { .. } => {
                info!("SAS verification completed with {}", sender);
                let _ = tx.send(AppEvent::VerificationCompleted {
                    sender: sender.clone(),
                });
                return;
            }
            SasState::Cancelled(info) => {
                let reason = info.reason().to_string();
                warn!("SAS verification cancelled: {}", reason);
                let _ = tx.send(AppEvent::VerificationCancelled { reason });
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
    let flow_id = request.flow_id().to_owned();

    // If we received the request (didn't initiate), accept it
    if !we_initiated && let Err(e) = request.accept().await {
        let _ = tx.send(AppEvent::VerificationError(format!(
            "Failed to accept verification: {e}"
        )));
        return;
    }

    let _ = tx.send(AppEvent::VerificationRequestReceived {
        sender: sender.clone(),
        flow_id: flow_id.clone(),
    });

    let mut stream = request.changes();

    while let Some(state) = stream.next().await {
        match state {
            VerificationRequestState::Ready { .. } => {
                // We're ready — if we initiated, start SAS
                if we_initiated {
                    match request.start_sas().await {
                        Ok(Some(sas)) => {
                            drive_sas(sas, flow_id, tx, confirm_rx).await;
                            return;
                        }
                        Ok(None) => {
                            let _ = tx.send(AppEvent::VerificationError(
                                "Failed to start SAS verification".to_string(),
                            ));
                            return;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(AppEvent::VerificationError(format!("SAS start error: {e}")));
                            return;
                        }
                    }
                }
                // If we're responding, wait for Transitioned (other side starts SAS)
            }
            VerificationRequestState::Transitioned { verification } => match verification {
                Verification::SasV1(sas) => {
                    drive_sas(sas, flow_id, tx, confirm_rx).await;
                    return;
                }
                _ => {
                    let _ = tx.send(AppEvent::VerificationError(
                        "Unsupported verification method".to_string(),
                    ));
                    return;
                }
            },
            VerificationRequestState::Done => {
                let _ = tx.send(AppEvent::VerificationCompleted {
                    sender: sender.clone(),
                });
                return;
            }
            VerificationRequestState::Cancelled(info) => {
                let reason = info.reason().to_string();
                let _ = tx.send(AppEvent::VerificationCancelled { reason });
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
            let _ = tx.send(AppEvent::VerificationError("Not logged in".to_string()));
            return;
        }
    };

    let identity = match client.encryption().get_user_identity(&user_id).await {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            let _ = tx.send(AppEvent::VerificationError(
                "No cross-signing identity found. Try logging out and back in.".to_string(),
            ));
            return;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::VerificationError(format!(
                "Failed to get identity: {e}"
            )));
            return;
        }
    };

    let request = match identity.request_verification().await {
        Ok(req) => req,
        Err(e) => {
            let _ = tx.send(AppEvent::VerificationError(format!(
                "Failed to request verification: {e}"
            )));
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
            let _ = tx.send(AppEvent::VerificationError(format!("Invalid user ID: {e}")));
            return;
        }
    };

    let identity = match client.encryption().request_user_identity(&user_id).await {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            let _ = tx.send(AppEvent::VerificationError(format!(
                "No cross-signing identity found for {user_id}. They may not have set up cross-signing."
            )));
            return;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::VerificationError(format!(
                "Failed to get identity: {e}"
            )));
            return;
        }
    };

    let request = match identity.request_verification().await {
        Ok(req) => req,
        Err(e) => {
            let _ = tx.send(AppEvent::VerificationError(format!(
                "Failed to request verification: {e}"
            )));
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
