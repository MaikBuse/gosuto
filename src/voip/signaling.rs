use anyhow::Result;
use matrix_sdk::Client;
use serde_json::json;
use tracing::info;

/// Send m.call.invite to a room
pub async fn send_call_invite(
    client: &Client,
    room_id: &str,
    call_id: &str,
    sdp: &str,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let content_json = json!({
        "call_id": call_id,
        "version": 1,
        "lifetime": 60000,
        "offer": {
            "type": "offer",
            "sdp": sdp
        }
    });

    let raw = serde_json::value::RawValue::from_string(serde_json::to_string(&content_json)?)?;
    room.send_raw("m.call.invite", raw).await?;
    info!("Sent m.call.invite for call {}", call_id);

    Ok(())
}

/// Send m.call.answer to a room
pub async fn send_call_answer(
    client: &Client,
    room_id: &str,
    call_id: &str,
    sdp: &str,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let content_json = json!({
        "call_id": call_id,
        "version": 1,
        "answer": {
            "type": "answer",
            "sdp": sdp
        }
    });

    let raw = serde_json::value::RawValue::from_string(serde_json::to_string(&content_json)?)?;
    room.send_raw("m.call.answer", raw).await?;
    info!("Sent m.call.answer for call {}", call_id);

    Ok(())
}

/// Send m.call.candidates to a room
pub async fn send_call_candidates(
    client: &Client,
    room_id: &str,
    call_id: &str,
    candidates: &[serde_json::Value],
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let content_json = json!({
        "call_id": call_id,
        "version": 1,
        "candidates": candidates
    });

    let raw = serde_json::value::RawValue::from_string(serde_json::to_string(&content_json)?)?;
    room.send_raw("m.call.candidates", raw).await?;
    info!(
        "Sent m.call.candidates for call {} ({} candidates)",
        call_id,
        candidates.len()
    );

    Ok(())
}

/// Send m.call.hangup to a room
pub async fn send_call_hangup(
    client: &Client,
    room_id: &str,
    call_id: &str,
    reason: &str,
) -> Result<()> {
    let room_id_parsed: matrix_sdk::ruma::OwnedRoomId = room_id.try_into()?;
    let room = client
        .get_room(&room_id_parsed)
        .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

    let content_json = json!({
        "call_id": call_id,
        "version": 1,
        "reason": reason
    });

    let raw = serde_json::value::RawValue::from_string(serde_json::to_string(&content_json)?)?;
    room.send_raw("m.call.hangup", raw).await?;
    info!(
        "Sent m.call.hangup for call {} (reason: {})",
        call_id, reason
    );

    Ok(())
}
