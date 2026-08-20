use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::models::RaceEvent;

const CURRENT_CACHE_VERSION: u32 = 2;

/// Metadata stored alongside cached events for a series.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// Schema/data version
    #[serde(default)]
    version: u32,
    /// When this data was fetched
    fetched_at: DateTime<Utc>,
    /// The race events
    events: Vec<RaceEvent>,
}

/// Get the cache directory: ~/.local/share/racetui/cache/
fn cache_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("racetui")
        .join("cache");
    Ok(data_dir)
}

/// Get the cache file path for a specific series.
/// e.g., ~/.local/share/racetui/cache/f1.json
fn cache_file_path(series_id: &str) -> Result<PathBuf> {
    Ok(cache_dir()?.join(format!("{}.json", series_id)))
}

/// Read cached events for a series.
/// Returns None if no cache file exists or if cache version is outdated.
/// Returns Some((events, fetched_at)) if cache exists and is current.
pub fn read_cache(series_id: &str) -> Result<Option<(Vec<RaceEvent>, DateTime<Utc>)>> {
    let path = cache_file_path(series_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read cache file {}", path.display()))?;
    let entry: CacheEntry = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse cache file {}", path.display()))?;

    // Invalidate if from an older cache schema version
    if entry.version < CURRENT_CACHE_VERSION {
        return Ok(None);
    }

    Ok(Some((entry.events, entry.fetched_at)))
}

/// Write events to the cache for a series.
pub fn write_cache(series_id: &str, events: &[RaceEvent]) -> Result<()> {
    let path = cache_file_path(series_id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory {}", parent.display()))?;
    }
    let entry = CacheEntry {
        version: CURRENT_CACHE_VERSION,
        fetched_at: Utc::now(),
        events: events.to_vec(),
    };
    let content = serde_json::to_string_pretty(&entry).context("Failed to serialize cache")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write cache to {}", path.display()))?;
    Ok(())
}

/// Check if the cache for a series is still valid (not expired).
/// Returns true if the cache is fresh (within TTL), false otherwise.
pub fn is_cache_fresh(series_id: &str, ttl_hours: u64) -> Result<bool> {
    match read_cache(series_id)? {
        None => Ok(false),
        Some((_, fetched_at)) => {
            let age = Utc::now().signed_duration_since(fetched_at);
            Ok(age.num_hours() < ttl_hours as i64)
        }
    }
}

/// Get the age of the cache in hours. Returns None if no cache exists.
pub fn cache_age_hours(series_id: &str) -> Result<Option<u64>> {
    match read_cache(series_id)? {
        None => Ok(None),
        Some((_, fetched_at)) => {
            let age = Utc::now().signed_duration_since(fetched_at);
            Ok(Some(age.num_hours() as u64))
        }
    }
}

/// Clear the cache for a specific series.
pub fn clear_cache(series_id: &str) -> Result<()> {
    let path = cache_file_path(series_id)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove cache file {}", path.display()))?;
    }
    Ok(())
}

/// Clear all cached data.
pub fn clear_all_cache() -> Result<()> {
    let dir = cache_dir()?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("Failed to remove cache directory {}", dir.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::models::EventStatus;
    use chrono::NaiveDate;

    #[test]
    fn test_cache_entry_serialization() {
        let event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain GP".to_string(),
            circuit_name: "Sakhir".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 3, 3).unwrap(),
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };

        let entry = CacheEntry {
            version: CURRENT_CACHE_VERSION,
            fetched_at: Utc::now(),
            events: vec![event],
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: CacheEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.events.len(), 1);
        assert_eq!(deserialized.events[0].event_name, "Bahrain GP");
    }
}
