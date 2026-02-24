use std::path::PathBuf;

use anyhow::Result;
use tracing::info;

pub const APP_NAME: &str = "walrust";
pub const TICK_RATE_MS: u64 = 250;
pub const RENDER_RATE_MS: u64 = 50;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct WalrustConfig {
    #[serde(default)]
    pub network: NetworkConfig,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct NetworkConfig {
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join(APP_NAME);
    Ok(dir)
}

pub fn load_config() -> WalrustConfig {
    let path = match config_dir() {
        Ok(dir) => dir.join("config.toml"),
        Err(_) => return WalrustConfig::default(),
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => {
                info!("Loaded config from {}", path.display());
                config
            }
            Err(e) => {
                info!("Failed to parse config at {}: {}", path.display(), e);
                WalrustConfig::default()
            }
        },
        Err(_) => {
            let config = WalrustConfig::default();
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    info!("Could not create config dir {}: {}", parent.display(), e);
                    return config;
                }
            }
            match toml::to_string_pretty(&config) {
                Ok(contents) => {
                    if let Err(e) = std::fs::write(&path, &contents) {
                        info!("Could not write default config to {}: {}", path.display(), e);
                    } else {
                        info!("Created default config at {}", path.display());
                    }
                }
                Err(e) => {
                    info!("Could not serialize default config: {}", e);
                }
            }
            config
        }
    }
}

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
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("log");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}
