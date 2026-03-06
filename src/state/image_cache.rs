use std::collections::{HashMap, HashSet};
use std::fmt;

use ratatui_image::protocol::StatefulProtocol;

pub struct CachedImage {
    pub protocol: Option<StatefulProtocol>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub last_encoded_rect: Option<ratatui::layout::Rect>,
}

impl fmt::Debug for CachedImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedImage").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct ImageCache {
    images: HashMap<String, CachedImage>,
    failed: HashSet<String>,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            failed: HashSet::new(),
        }
    }

    pub fn get_mut(&mut self, event_id: &str) -> Option<&mut CachedImage> {
        self.images.get_mut(event_id)
    }

    pub fn insert(&mut self, event_id: String, image: CachedImage) {
        self.failed.remove(&event_id);
        self.images.insert(event_id, image);
    }

    pub fn is_loaded(&self, event_id: &str) -> bool {
        self.images.contains_key(event_id)
    }

    pub fn mark_failed(&mut self, event_id: &str) {
        self.failed.insert(event_id.to_string());
    }

    pub fn is_failed(&self, event_id: &str) -> bool {
        self.failed.contains(event_id)
    }

    pub fn clear(&mut self) {
        self.images.clear();
        self.failed.clear();
    }
}
