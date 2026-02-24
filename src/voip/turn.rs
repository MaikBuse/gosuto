use matrix_sdk::Client;
use tracing::{info, warn};
use webrtc::ice_transport::ice_server::RTCIceServer;

/// Fetch TURN/STUN server configuration from the Matrix homeserver.
/// Falls back to a public STUN server if the homeserver doesn't provide one.
pub async fn get_ice_servers(client: &Client) -> Vec<RTCIceServer> {
    match fetch_turn_servers(client).await {
        Ok(servers) if !servers.is_empty() => {
            info!("Got {} ICE servers from homeserver", servers.len());
            servers
        }
        Ok(_) => {
            warn!("Homeserver returned no TURN servers, using public STUN");
            fallback_stun()
        }
        Err(e) => {
            warn!("Failed to fetch TURN servers: {}, using public STUN", e);
            fallback_stun()
        }
    }
}

async fn fetch_turn_servers(client: &Client) -> anyhow::Result<Vec<RTCIceServer>> {
    let request = matrix_sdk::ruma::api::client::voip::get_turn_server_info::v3::Request::new();
    let response = client.send(request).await?;

    let mut servers = Vec::new();

    let urls: Vec<String> = response.uris.iter().map(|u| u.to_string()).collect();

    if !urls.is_empty() {
        servers.push(RTCIceServer {
            urls,
            username: response.username.clone(),
            credential: response.password.clone(),
            ..Default::default()
        });
    }

    Ok(servers)
}

fn fallback_stun() -> Vec<RTCIceServer> {
    vec![RTCIceServer {
        urls: vec!["stun:stun.l.google.com:19302".to_string()],
        ..Default::default()
    }]
}
