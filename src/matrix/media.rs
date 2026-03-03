use matrix_sdk::Client;
use matrix_sdk::media::MediaThumbnailSettings;
use matrix_sdk::ruma::events::room::message::ImageMessageEventContent;
use matrix_sdk::ruma::uint;
use tracing::{debug, error};

use crate::event::{AppEvent, EventSender};

pub async fn fetch_image(
    client: &Client,
    event_id: String,
    content: &ImageMessageEventContent,
    tx: &EventSender,
) {
    let settings = MediaThumbnailSettings::new(uint!(400), uint!(300));

    // Try thumbnail first, fall back to full file
    let data = match client.media().get_thumbnail(content, settings, true).await {
        Ok(Some(data)) => data,
        _ => match client.media().get_file(content, true).await {
            Ok(Some(data)) => data,
            Ok(None) => {
                let _ = tx.send(AppEvent::ImageFailed {
                    event_id,
                    error: "No media source".to_string(),
                });
                return;
            }
            Err(e) => {
                error!("Failed to download image {}: {}", event_id, e);
                let _ = tx.send(AppEvent::ImageFailed {
                    event_id,
                    error: e.to_string(),
                });
                return;
            }
        },
    };

    debug!("Downloaded image {} ({} bytes)", event_id, data.len());
    let _ = tx.send(AppEvent::ImageLoaded {
        event_id,
        image_data: data,
    });
}
