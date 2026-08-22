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

    /// Live timing poll interval in seconds. Default: 2.
    #[serde(default = "default_live_poll_interval_secs")]
    pub live_poll_interval_secs: u64,

    /// Daemon and background notification configuration.
    #[serde(default)]
    pub daemon: DaemonConfig,
}

/// Configuration for racetui daemon mode and background notifications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonConfig {
    /// How many minutes before a session start to trigger a notification. Default: 30.
    #[serde(default = "default_daemon_notify_minutes_before")]
    pub notify_minutes_before: u64,

    /// Session types to notify for (e.g. ["race", "qualifying"] or ["all"]). Default: ["all"].
    #[serde(default = "default_daemon_notify_session_types")]
    pub notify_session_types: Vec<String>,

    /// Series filter mode: "favorites" (only favorited series), "all" (all non-hidden series), or specific series ID. Default: "favorites".
    #[serde(default = "default_daemon_notify_series_filter")]
    pub notify_series_filter: String,

    /// Whether to play a notification sound / audio alert. Default: true.
    #[serde(default = "default_daemon_notify_sound")]
    pub notify_sound: bool,

    /// Polling / check interval in seconds for the daemon loop. Default: 300 (5 minutes).
    #[serde(default = "default_daemon_poll_interval_secs")]
    pub poll_interval_secs: u64,
}

fn default_daemon_notify_minutes_before() -> u64 {
    30
}

fn default_daemon_notify_session_types() -> Vec<String> {
    vec!["all".to_string()]
}

fn default_daemon_notify_series_filter() -> String {
    "favorites".to_string()
}

fn default_daemon_notify_sound() -> bool {
    true
}

fn default_daemon_poll_interval_secs() -> u64 {
    300
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            notify_minutes_before: default_daemon_notify_minutes_before(),
            notify_session_types: default_daemon_notify_session_types(),
            notify_series_filter: default_daemon_notify_series_filter(),
            notify_sound: default_daemon_notify_sound(),
            poll_interval_secs: default_daemon_poll_interval_secs(),
        }
    }
}

impl DaemonConfig {
    /// Check whether a session of the given name should trigger a notification based on `notify_session_types`.
    pub fn should_notify_session(&self, session_name: &str) -> bool {
        if self.notify_session_types.iter().any(|t| t == "all") {
            return true;
        }
        let lower = session_name.to_lowercase();
        self.notify_session_types
            .iter()
            .any(|t| lower.contains(&t.to_lowercase()))
    }

    /// Check whether an event from the given series should trigger a notification based on `notify_series_filter`.
    pub fn should_notify_series(
        &self,
        series_id: &str,
        favorites: &HashSet<String>,
        hidden: &HashSet<String>,
    ) -> bool {
        if hidden.contains(series_id) {
            return false;
        }
        match self.notify_series_filter.as_str() {
            "all" => true,
            "favorites" => favorites.contains(series_id),
            specific => specific.eq_ignore_ascii_case(series_id),
        }
    }
}

fn default_cache_ttl_hours() -> u64 {
    24
}

fn default_notification_threshold_hours() -> u64 {
    2
}

fn default_live_poll_interval_secs() -> u64 {
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
            live_poll_interval_secs: default_live_poll_interval_secs(),
            daemon: DaemonConfig::default(),
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
        assert_eq!(config.live_poll_interval_secs, 2);
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
        config.live_poll_interval_secs = 10;

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: UserConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.cache_ttl_hours, 12);
        assert_eq!(parsed.live_poll_interval_secs, 10);
        assert!(parsed.favorites.contains("f1"));
        assert!(parsed.hidden_series.contains("dtm"));
        assert_eq!(parsed.default_view, "list");
        assert_eq!(parsed.daemon.notify_minutes_before, 30);
        assert_eq!(parsed.daemon.poll_interval_secs, 300);
    }

    #[test]
    fn test_daemon_config_toml_parsing() {
        let toml_sample = r#"
            favorites = ["f1", "indycar"]
            [daemon]
            notify_minutes_before = 15
            notify_session_types = ["race", "qualifying"]
            notify_series_filter = "favorites"
            notify_sound = false
            poll_interval_secs = 60
        "#;

        let config: UserConfig = toml::from_str(toml_sample).unwrap();
        assert_eq!(config.daemon.notify_minutes_before, 15);
        assert_eq!(
            config.daemon.notify_session_types,
            vec!["race".to_string(), "qualifying".to_string()]
        );
        assert_eq!(config.daemon.notify_series_filter, "favorites");
        assert!(!config.daemon.notify_sound);
        assert_eq!(config.daemon.poll_interval_secs, 60);

        assert!(config.daemon.should_notify_session("Grand Prix - Race"));
        assert!(config.daemon.should_notify_session("Qualifying 1"));
        assert!(!config.daemon.should_notify_session("Practice 1"));

        let mut favorites = HashSet::new();
        favorites.insert("f1".to_string());
        let mut hidden = HashSet::new();
        hidden.insert("wec".to_string());

        assert!(config
            .daemon
            .should_notify_series("f1", &favorites, &hidden));
        assert!(!config
            .daemon
            .should_notify_series("motogp", &favorites, &hidden));
        assert!(!config
            .daemon
            .should_notify_series("wec", &favorites, &hidden));
    }
}
