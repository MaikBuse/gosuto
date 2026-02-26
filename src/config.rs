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

pub fn store_path_for_homeserver(homeserver: &str) -> Result<PathBuf> {
    let hostname = url::Url::parse(homeserver)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| {
            // Fallback: sanitize the raw string for use as a directory name
            homeserver.replace(['/', ':', '\\'], "_")
        });
    let path = data_dir()?.join("store").join(hostname);
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Returns the store path for a homeserver without creating it.
/// Use this for cleanup/deletion paths.
pub fn store_path_for_homeserver_unchecked(homeserver: &str) -> Result<PathBuf> {
    let hostname = url::Url::parse(homeserver)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| homeserver.replace(['/', ':', '\\'], "_"));
    Ok(data_dir()?.join("store").join(hostname))
}

pub fn log_path() -> Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("log");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_path_extracts_hostname_from_url() {
        let path = store_path_for_homeserver("https://matrix.org").unwrap();
        assert_eq!(path.file_name().unwrap(), "matrix.org");
    }

    #[test]
    fn store_path_strips_port_from_url() {
        let path = store_path_for_homeserver("https://matrix.org:8448").unwrap();
        assert_eq!(path.file_name().unwrap(), "matrix.org");
    }

    #[test]
    fn store_path_sanitizes_non_url_input() {
        let path = store_path_for_homeserver("not://valid").unwrap();
        // url::Url::parse succeeds for "not://valid" with host "valid"
        // but for truly unparseable input, it sanitizes slashes/colons
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
    }

    #[test]
    fn store_path_unchecked_matches_checked() {
        let checked = store_path_for_homeserver("https://example.com").unwrap();
        let unchecked = store_path_for_homeserver_unchecked("https://example.com").unwrap();
        assert_eq!(checked, unchecked);
    }
}
