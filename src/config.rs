use std::path::PathBuf;

use anyhow::Result;

pub const APP_NAME: &str = "walrust";
pub const TICK_RATE_MS: u64 = 250;
pub const RENDER_RATE_MS: u64 = 50;

pub fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?
        .join(APP_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn session_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("session.json"))
}

pub fn store_path() -> Result<PathBuf> {
    let path = data_dir()?.join("store");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn log_path() -> Result<PathBuf> {
    let path = data_dir()?.join("logs");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}
