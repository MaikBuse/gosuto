use std::path::PathBuf;

use anyhow::Result;
use tracing::{info, warn};

pub const APP_NAME: &str = "gosuto";
pub const TICK_RATE_MS: u64 = 250;
pub const RENDER_RATE_MS: u64 = 50;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct GosutoConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub effects: EffectsConfig,
    #[serde(default)]
    pub audio: AudioConfig,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AudioConfig {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    #[serde(default = "default_volume")]
    pub input_volume: f32,
    #[serde(default = "default_volume")]
    pub output_volume: f32,
    #[serde(default)]
    pub voice_activity: bool,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,
    #[serde(default)]
    pub push_to_talk: bool,
    #[serde(default)]
    pub push_to_talk_key: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            input_device: None,
            output_device: None,
            input_volume: 1.0,
            output_volume: 1.0,
            voice_activity: false,
            sensitivity: 0.15,
            push_to_talk: false,
            push_to_talk_key: None,
        }
    }
}

fn default_volume() -> f32 {
    1.0
}

fn default_sensitivity() -> f32 {
    0.15
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct EffectsConfig {
    #[serde(default = "default_true")]
    pub rain: bool,
    #[serde(default = "default_true")]
    pub glitch: bool,
}

fn default_true() -> bool {
    true
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self {
            rain: true,
            glitch: true,
        }
    }
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

pub fn load_config() -> GosutoConfig {
    let path = match config_dir() {
        Ok(dir) => dir.join("config.toml"),
        Err(_) => return GosutoConfig::default(),
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => {
                info!("Loaded config from {}", path.display());
                config
            }
            Err(e) => {
                warn!("Failed to parse config at {}: {}", path.display(), e);
                GosutoConfig::default()
            }
        },
        Err(_) => {
            let config = GosutoConfig::default();
            if let Some(parent) = path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                warn!("Could not create config dir {}: {}", parent.display(), e);
                return config;
            }
            match toml::to_string_pretty(&config) {
                Ok(contents) => {
                    if let Err(e) = std::fs::write(&path, &contents) {
                        warn!(
                            "Could not write default config to {}: {}",
                            path.display(),
                            e
                        );
                    } else {
                        info!("Created default config at {}", path.display());
                    }
                }
                Err(e) => {
                    warn!("Could not serialize default config: {}", e);
                }
            }
            config
        }
    }
}

pub fn save_config(config: &GosutoConfig) {
    let path = match config_dir() {
        Ok(dir) => dir.join("config.toml"),
        Err(_) => return,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match toml::to_string_pretty(config) {
        Ok(contents) => {
            if let Err(e) = std::fs::write(&path, &contents) {
                warn!("Could not write config to {}: {}", path.display(), e);
            }
        }
        Err(e) => warn!("Could not serialize config: {}", e),
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
    let path = data_dir()?.join("logs");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Delete log files older than `max_days`. Best-effort — errors are silently ignored.
pub fn cleanup_old_logs(path: &std::path::Path, max_days: u64) {
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(max_days * 24 * 60 * 60);
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_file()
            && let Ok(modified) = meta.modified()
            && modified < cutoff
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
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
