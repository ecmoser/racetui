use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single entry in a driver championship standings table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverStanding {
    /// Current championship position (1-based)
    pub position: u32,
    /// Driver's full name (e.g., "Max Verstappen")
    pub driver_name: String,
    /// Driver's short abbreviation (e.g., "VER"), if available
    pub driver_code: Option<String>,
    /// Driver's permanent number (e.g., 1), if applicable
    pub driver_number: Option<u32>,
    /// Team/constructor name (e.g., "Red Bull Racing")
    pub team: String,
    /// Total championship points
    pub points: f64,
    /// Number of wins this season
    pub wins: u32,
}

/// A single entry in a constructor/manufacturer/team championship standings table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorStanding {
    /// Current championship position (1-based)
    pub position: u32,
    /// Constructor/team/manufacturer name (e.g., "Red Bull Racing")
    pub name: String,
    /// Total championship points
    pub points: f64,
    /// Number of wins this season
    pub wins: u32,
}

/// Complete championship standings for a single series in a single season.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonStandings {
    /// Series ID (e.g., "f1", "nascar_cup")
    pub series_id: String,
    /// Season year (e.g., 2026)
    pub season: u32,
    /// Driver championship standings (always present)
    pub drivers: Vec<DriverStanding>,
    /// Constructor/manufacturer standings (empty if the series doesn't have them)
    pub constructors: Vec<ConstructorStanding>,
    /// When this data was last fetched (UTC timestamp)
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

impl SeasonStandings {
    /// Returns true if this series has constructor/manufacturer standings.
    pub fn has_constructor_standings(&self) -> bool {
        !self.constructors.is_empty()
    }
}

/// Get the standings cache directory: ~/.local/share/racetui/standings/
fn standings_cache_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("racetui")
        .join("standings");
    Ok(data_dir)
}

/// Build the cache file path for a specific series and season standings.
/// Format: ~/.local/share/racetui/standings/{series_id}_{season}.json
fn standings_cache_path(series_id: &str, season: u32) -> Result<PathBuf> {
    let dir = standings_cache_dir()?;
    Ok(dir.join(format!("{}_{}.json", series_id, season)))
}

/// Write season standings to the cache file.
pub fn write_standings_cache(standings: &SeasonStandings) -> Result<()> {
    let path = standings_cache_path(&standings.series_id, standings.season)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create standings cache dir {}", parent.display())
        })?;
    }
    let json = serde_json::to_string_pretty(standings).context("Failed to serialize standings")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write standings cache to {}", path.display()))?;
    tracing::debug!(
        "Cached standings for {} season {} at {}",
        standings.series_id,
        standings.season,
        path.display()
    );
    Ok(())
}

/// Read season standings from the cache file. Returns None if no cache exists.
pub fn read_standings_cache(series_id: &str, season: u32) -> Result<Option<SeasonStandings>> {
    let path = standings_cache_path(series_id, season)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read standings cache at {}", path.display()))?;
    let standings: SeasonStandings = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse standings cache at {}", path.display()))?;
    Ok(Some(standings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_driver_standing_serialization_roundtrip() {
        let standing = DriverStanding {
            position: 1,
            driver_name: "Max Verstappen".to_string(),
            driver_code: Some("VER".to_string()),
            driver_number: Some(1),
            team: "Red Bull Racing".to_string(),
            points: 575.0,
            wins: 19,
        };

        let json = serde_json::to_string(&standing).unwrap();
        let deserialized: DriverStanding = serde_json::from_str(&json).unwrap();
        assert_eq!(standing, deserialized);
    }

    #[test]
    fn test_constructor_standing_serialization_roundtrip() {
        let standing = ConstructorStanding {
            position: 1,
            name: "Red Bull Racing".to_string(),
            points: 860.0,
            wins: 21,
        };

        let json = serde_json::to_string(&standing).unwrap();
        let deserialized: ConstructorStanding = serde_json::from_str(&json).unwrap();
        assert_eq!(standing, deserialized);
    }

    #[test]
    fn test_season_standings_has_constructor_standings() {
        let with_constructors = SeasonStandings {
            series_id: "f1".to_string(),
            season: 2026,
            drivers: vec![DriverStanding {
                position: 1,
                driver_name: "Max Verstappen".to_string(),
                driver_code: Some("VER".to_string()),
                driver_number: Some(1),
                team: "Red Bull Racing".to_string(),
                points: 25.0,
                wins: 1,
            }],
            constructors: vec![ConstructorStanding {
                position: 1,
                name: "Red Bull Racing".to_string(),
                points: 25.0,
                wins: 1,
            }],
            fetched_at: Utc::now(),
        };
        assert!(with_constructors.has_constructor_standings());

        let without_constructors = SeasonStandings {
            series_id: "indycar".to_string(),
            season: 2026,
            drivers: vec![DriverStanding {
                position: 1,
                driver_name: "Alex Palou".to_string(),
                driver_code: None,
                driver_number: Some(10),
                team: "Chip Ganassi Racing".to_string(),
                points: 50.0,
                wins: 1,
            }],
            constructors: vec![],
            fetched_at: Utc::now(),
        };
        assert!(!without_constructors.has_constructor_standings());
    }

    #[test]
    fn test_season_standings_json_roundtrip() {
        let standings = SeasonStandings {
            series_id: "f1".to_string(),
            season: 2026,
            drivers: vec![DriverStanding {
                position: 1,
                driver_name: "Lando Norris".to_string(),
                driver_code: Some("NOR".to_string()),
                driver_number: Some(4),
                team: "McLaren".to_string(),
                points: 26.0,
                wins: 1,
            }],
            constructors: vec![ConstructorStanding {
                position: 1,
                name: "McLaren".to_string(),
                points: 40.0,
                wins: 1,
            }],
            fetched_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&standings).unwrap();
        let deserialized: SeasonStandings = serde_json::from_str(&json).unwrap();
        assert_eq!(standings.series_id, deserialized.series_id);
        assert_eq!(standings.season, deserialized.season);
        assert_eq!(standings.drivers, deserialized.drivers);
        assert_eq!(standings.constructors, deserialized.constructors);
    }

    #[test]
    fn test_standings_cache_read_write() {
        let standings = SeasonStandings {
            series_id: "test_standings_cache_series".to_string(),
            season: 2026,
            drivers: vec![DriverStanding {
                position: 1,
                driver_name: "Test Driver".to_string(),
                driver_code: Some("TST".to_string()),
                driver_number: Some(99),
                team: "Test Team".to_string(),
                points: 100.0,
                wins: 5,
            }],
            constructors: vec![],
            fetched_at: Utc::now(),
        };

        write_standings_cache(&standings).unwrap();
        let read = read_standings_cache("test_standings_cache_series", 2026)
            .unwrap()
            .expect("should read cached standings");
        assert_eq!(read.series_id, "test_standings_cache_series");
        assert_eq!(read.season, 2026);
        assert_eq!(read.drivers.len(), 1);
        assert_eq!(read.drivers[0].driver_name, "Test Driver");

        // Clean up
        if let Ok(path) = standings_cache_path("test_standings_cache_series", 2026) {
            let _ = std::fs::remove_file(path);
        }
    }
}
