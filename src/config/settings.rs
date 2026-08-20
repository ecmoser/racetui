use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// User configuration, persisted to ~/.config/racetui/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Series IDs that the user has favorited (e.g. ["f1", "wec"])
    #[serde(default)]
    pub favorites: HashSet<String>,

    /// Cache time-to-live in hours. Default: 24.
    #[serde(default = "default_cache_ttl_hours")]
    pub cache_ttl_hours: u64,

    /// How many hours before an event to highlight it as "upcoming soon".
    /// Default: 2 hours.
    #[serde(default = "default_notification_threshold_hours")]
    pub notification_threshold_hours: u64,

    /// Command to open livestream URLs.
    /// Default: "xdg-open" on Linux, "open" on macOS, "start" on Windows.
    /// Can be set to e.g. "mpv" for direct streams.
    #[serde(default = "default_open_command")]
    pub open_command: String,

    /// Default view mode: "list" or "calendar".
    #[serde(default = "default_view_mode")]
    pub default_view: String,

    /// Hidden series IDs (series the user has chosen to hide).
    #[serde(default)]
    pub hidden_series: HashSet<String>,
}

fn default_cache_ttl_hours() -> u64 {
    24
}

fn default_notification_threshold_hours() -> u64 {
    2
}

fn default_open_command() -> String {
    if cfg!(target_os = "macos") {
        "open".to_string()
    } else if cfg!(target_os = "windows") {
        "start".to_string()
    } else {
        "xdg-open".to_string()
    }
}

fn default_view_mode() -> String {
    "list".to_string()
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            favorites: HashSet::new(),
            cache_ttl_hours: default_cache_ttl_hours(),
            notification_threshold_hours: default_notification_threshold_hours(),
            open_command: default_open_command(),
            default_view: default_view_mode(),
            hidden_series: HashSet::new(),
        }
    }
}

impl UserConfig {
    /// Get the config file path: ~/.config/racetui/config.toml
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("racetui");
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from disk. If the file doesn't exist, return defaults.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Self =
            toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;
        Ok(config)
    }

    /// Save config to disk. Creates parent directories if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory {}", parent.display())
            })?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }

    /// Toggle a series as favorite. Returns true if it's now a favorite.
    pub fn toggle_favorite(&mut self, series_id: &str) -> bool {
        if self.favorites.contains(series_id) {
            self.favorites.remove(series_id);
            false
        } else {
            self.favorites.insert(series_id.to_string());
            true
        }
    }

    /// Toggle a series as hidden. Returns true if it's now hidden.
    pub fn toggle_hidden(&mut self, series_id: &str) -> bool {
        if self.hidden_series.contains(series_id) {
            self.hidden_series.remove(series_id);
            false
        } else {
            self.hidden_series.insert(series_id.to_string());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = UserConfig::default();
        assert_eq!(config.cache_ttl_hours, 24);
        assert_eq!(config.notification_threshold_hours, 2);
        assert_eq!(config.default_view, "list");
        assert!(config.favorites.is_empty());
        assert!(config.hidden_series.is_empty());
    }

    #[test]
    fn test_toggle_favorite() {
        let mut config = UserConfig::default();
        assert!(config.toggle_favorite("f1"));
        assert!(config.favorites.contains("f1"));
        assert!(!config.toggle_favorite("f1"));
        assert!(!config.favorites.contains("f1"));
    }

    #[test]
    fn test_toggle_hidden() {
        let mut config = UserConfig::default();
        assert!(config.toggle_hidden("nascar_cup"));
        assert!(config.hidden_series.contains("nascar_cup"));
        assert!(!config.toggle_hidden("nascar_cup"));
        assert!(!config.hidden_series.contains("nascar_cup"));
    }

    #[test]
    fn test_toml_roundtrip() {
        let mut config = UserConfig::default();
        config.favorites.insert("f1".to_string());
        config.hidden_series.insert("dtm".to_string());
        config.cache_ttl_hours = 12;

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: UserConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.cache_ttl_hours, 12);
        assert!(parsed.favorites.contains("f1"));
        assert!(parsed.hidden_series.contains("dtm"));
        assert_eq!(parsed.default_view, "list");
    }
}
