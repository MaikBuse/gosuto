use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::fs_utils::write_private_file;

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredSession {
    pub homeserver: String,
    pub user_id: String,
    pub device_id: String,
    pub access_token: String,
}

pub fn save_session(path: &Path, session: &StoredSession) -> Result<()> {
    let json = serde_json::to_string_pretty(session)?;
    write_private_file(path, &json)?;
    Ok(())
}

pub fn load_session(path: &Path) -> Result<StoredSession> {
    let json = std::fs::read_to_string(path)?;
    let session: StoredSession = serde_json::from_str(&json)?;
    Ok(session)
}

pub fn delete_session(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
