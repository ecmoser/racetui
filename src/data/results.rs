use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single driver's result in a completed race.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverResult {
    /// Finishing position (1-based). None if DNF/DNS/DSQ.
    pub position: Option<u32>,
    /// Driver's full name
    pub driver_name: String,
    /// Driver's short abbreviation (e.g., "VER"), if available
    pub driver_code: Option<String>,
    /// Driver's permanent number, if applicable
    pub driver_number: Option<u32>,
    /// Team/constructor name
    pub team: String,
    /// Gap to race leader as a display string (e.g., "+5.123s", "+1 Lap", "DNF")
    pub gap_to_leader: String,
    /// Gap to car ahead as a display string (e.g., "+1.456s")
    pub gap_to_ahead: String,
    /// Starting grid position (for calculating positions gained/lost)
    pub grid_position: Option<u32>,
    /// Points earned in this race
    pub points: f64,
    /// Whether this driver set the fastest lap
    pub fastest_lap: bool,
    /// Penalty information, if any (e.g., "+5s Time Penalty")
    pub penalty: Option<String>,
    /// Classification status (e.g., "Finished", "DNF", "DNS", "DSQ")
    pub status: String,
}

impl DriverResult {
    /// Calculate positions gained or lost from grid to finish.
    /// Positive = gained, negative = lost. None if grid or finish position unknown.
    pub fn positions_gained(&self) -> Option<i32> {
        match (self.grid_position, self.position) {
            (Some(grid), Some(finish)) => Some(grid as i32 - finish as i32),
            _ => None,
        }
    }
}

/// Complete race results for a single event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RaceResults {
    /// Series ID (e.g., "f1")
    pub series_id: String,
    /// Round number within the season
    pub round: u32,
    /// Event name (e.g., "Monaco Grand Prix")
    pub event_name: String,
    /// Circuit name
    pub circuit_name: String,
    /// Race date
    pub race_date: chrono::NaiveDate,
    /// Individual driver results, ordered by finishing position
    pub results: Vec<DriverResult>,
    /// When this data was last fetched (UTC timestamp)
    pub fetched_at: DateTime<Utc>,
}

/// A single driver's result in a qualifying session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualifyingDriverResult {
    /// Qualifying position (1-based)
    pub position: u32,
    /// Driver's full name
    pub driver_name: String,
    /// Driver's short abbreviation (e.g., "RUS"), if available
    pub driver_code: Option<String>,
    /// Driver's permanent number, if applicable
    pub driver_number: Option<u32>,
    /// Team/constructor name
    pub team: String,
    /// Q1 lap time string (e.g., "1:19.507")
    pub q1: Option<String>,
    /// Q2 lap time string (e.g., "1:18.934")
    pub q2: Option<String>,
    /// Q3 lap time string (e.g., "1:18.518")
    pub q3: Option<String>,
}

/// Complete qualifying results for a single event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualifyingResults {
    /// Series ID (e.g., "f1")
    pub series_id: String,
    /// Round number within the season
    pub round: u32,
    /// Event name (e.g., "Monaco Grand Prix")
    pub event_name: String,
    /// Circuit name
    pub circuit_name: String,
    /// Race / qualifying date
    pub race_date: chrono::NaiveDate,
    /// Individual driver qualifying results, ordered by position
    pub results: Vec<QualifyingDriverResult>,
    /// When this data was last fetched (UTC timestamp)
    pub fetched_at: DateTime<Utc>,
}

/// Get the results cache directory: ~/.local/share/racetui/results/
fn results_cache_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("racetui")
        .join("results");
    Ok(data_dir)
}

/// Build the cache file path for a specific event's results.
/// Format: ~/.local/share/racetui/results/{series_id}_round_{round}.json
fn results_cache_path(series_id: &str, round: u32) -> Result<PathBuf> {
    let dir = results_cache_dir()?;
    Ok(dir.join(format!("{}_round_{}.json", series_id, round)))
}

/// Write race results to the cache file.
pub fn write_results_cache(results: &RaceResults) -> Result<()> {
    let path = results_cache_path(&results.series_id, results.round)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create results cache dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(results).context("Failed to serialize race results")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write results cache to {}", path.display()))?;
    tracing::debug!(
        "Cached results for {} round {} at {}",
        results.series_id,
        results.round,
        path.display()
    );
    Ok(())
}

/// Read race results from the cache file. Returns None if no cache exists.
pub fn read_results_cache(series_id: &str, round: u32) -> Result<Option<RaceResults>> {
    let path = results_cache_path(series_id, round)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read results cache at {}", path.display()))?;
    let results: RaceResults = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse results cache at {}", path.display()))?;
    Ok(Some(results))
}

/// Build the cache file path for qualifying results.
/// Format: ~/.local/share/racetui/results/{series_id}_qualifying_round_{round}.json
fn qualifying_cache_path(series_id: &str, round: u32) -> Result<PathBuf> {
    let dir = results_cache_dir()?;
    Ok(dir.join(format!("{}_qualifying_round_{}.json", series_id, round)))
}

/// Write qualifying results to the cache file.
pub fn write_qualifying_cache(results: &QualifyingResults) -> Result<()> {
    let path = qualifying_cache_path(&results.series_id, results.round)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create results cache dir {}", parent.display()))?;
    }
    let json =
        serde_json::to_string_pretty(results).context("Failed to serialize qualifying results")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write qualifying cache to {}", path.display()))?;
    tracing::debug!(
        "Cached qualifying results for {} round {} at {}",
        results.series_id,
        results.round,
        path.display()
    );
    Ok(())
}

/// Read qualifying results from the cache file. Returns None if no cache exists.
pub fn read_qualifying_cache(series_id: &str, round: u32) -> Result<Option<QualifyingResults>> {
    let path = qualifying_cache_path(series_id, round)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read qualifying cache at {}", path.display()))?;
    let results: QualifyingResults = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse qualifying cache at {}", path.display()))?;
    Ok(Some(results))
}

/// List all cached result files for a series. Returns (round, path) pairs.
pub fn list_cached_results(series_id: &str) -> Result<Vec<(u32, PathBuf)>> {
    let dir = results_cache_dir()?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let prefix = format!("{}_round_", series_id);
    let mut results = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.starts_with(&prefix) && filename.ends_with(".json") {
            let round_str = filename
                .strip_prefix(&prefix)
                .and_then(|s| s.strip_suffix(".json"));
            if let Some(round_str) = round_str {
                if let Ok(round) = round_str.parse::<u32>() {
                    results.push((round, entry.path()));
                }
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_positions_gained() {
        let gained = DriverResult {
            position: Some(1),
            driver_name: "Max Verstappen".to_string(),
            driver_code: Some("VER".to_string()),
            driver_number: Some(1),
            team: "Red Bull Racing".to_string(),
            gap_to_leader: "Leader".to_string(),
            gap_to_ahead: "Leader".to_string(),
            grid_position: Some(4),
            points: 25.0,
            fastest_lap: true,
            penalty: None,
            status: "Finished".to_string(),
        };
        assert_eq!(gained.positions_gained(), Some(3));

        let lost = DriverResult {
            position: Some(5),
            grid_position: Some(2),
            ..gained.clone()
        };
        assert_eq!(lost.positions_gained(), Some(-3));

        let no_change = DriverResult {
            position: Some(3),
            grid_position: Some(3),
            ..gained.clone()
        };
        assert_eq!(no_change.positions_gained(), Some(0));

        let dnf = DriverResult {
            position: None,
            grid_position: Some(1),
            ..gained.clone()
        };
        assert_eq!(dnf.positions_gained(), None);

        let no_grid = DriverResult {
            position: Some(1),
            grid_position: None,
            ..gained
        };
        assert_eq!(no_grid.positions_gained(), None);
    }

    #[test]
    fn test_race_results_serialization_roundtrip() {
        let race_results = RaceResults {
            series_id: "f1".to_string(),
            round: 1,
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            race_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            results: vec![DriverResult {
                position: Some(1),
                driver_name: "Max Verstappen".to_string(),
                driver_code: Some("VER".to_string()),
                driver_number: Some(1),
                team: "Red Bull Racing".to_string(),
                gap_to_leader: "Leader".to_string(),
                gap_to_ahead: "Leader".to_string(),
                grid_position: Some(1),
                points: 26.0,
                fastest_lap: true,
                penalty: None,
                status: "Finished".to_string(),
            }],
            fetched_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&race_results).unwrap();
        let deserialized: RaceResults = serde_json::from_str(&json).unwrap();
        assert_eq!(race_results.series_id, deserialized.series_id);
        assert_eq!(race_results.round, deserialized.round);
        assert_eq!(race_results.event_name, deserialized.event_name);
        assert_eq!(race_results.circuit_name, deserialized.circuit_name);
        assert_eq!(race_results.race_date, deserialized.race_date);
        assert_eq!(race_results.results, deserialized.results);
    }

    #[test]
    fn test_results_cache_read_write() {
        let test_dir =
            std::env::temp_dir().join(format!("racetui_test_cache_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&test_dir);

        let race_results = RaceResults {
            series_id: "f1_test_cache".to_string(),
            round: 99,
            event_name: "Test Grand Prix".to_string(),
            circuit_name: "Test Circuit".to_string(),
            race_date: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            results: vec![],
            fetched_at: Utc::now(),
        };

        let file_path = test_dir.join("f1_test_cache_round_99.json");
        let json = serde_json::to_string_pretty(&race_results).unwrap();
        std::fs::write(&file_path, &json).unwrap();

        let read_back: RaceResults =
            serde_json::from_str(&std::fs::read_to_string(&file_path).unwrap()).unwrap();
        assert_eq!(read_back.event_name, "Test Grand Prix");

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_qualifying_results_serialization_and_cache() {
        let q_results = QualifyingResults {
            series_id: "f1".to_string(),
            round: 1,
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            race_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            results: vec![QualifyingDriverResult {
                position: 1,
                driver_name: "Max Verstappen".to_string(),
                driver_code: Some("VER".to_string()),
                driver_number: Some(1),
                team: "Red Bull Racing".to_string(),
                q1: Some("1:29.421".to_string()),
                q2: Some("1:28.740".to_string()),
                q3: Some("1:28.197".to_string()),
            }],
            fetched_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&q_results).unwrap();
        let deserialized: QualifyingResults = serde_json::from_str(&json).unwrap();
        assert_eq!(q_results.series_id, deserialized.series_id);
        assert_eq!(q_results.round, deserialized.round);
        assert_eq!(q_results.results, deserialized.results);
    }
}
