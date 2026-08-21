use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use crate::live::event::{
    LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes,
};
use crate::live::LiveProvider;
use crate::scraper::fetcher::create_http_client;

const NASCAR_LIVE_FEED_URL: &str = "https://cf.nascar.com/live/feeds/live-feed.json";

/// NASCAR live feed JSON root structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NascarLiveFeed {
    #[serde(alias = "lap_number", alias = "current_lap", default)]
    pub lap_number: Option<u32>,
    #[serde(alias = "laps_in_race", alias = "total_laps", default)]
    pub laps_in_race: Option<u32>,
    #[serde(alias = "laps_to_go", default)]
    pub laps_to_go: Option<u32>,
    #[serde(alias = "flag_state", default)]
    pub flag_state: Option<u32>,
    #[serde(alias = "race_id", default)]
    pub race_id: Option<u64>,
    #[serde(alias = "run_name", alias = "session_name", default)]
    pub run_name: Option<String>,
    #[serde(alias = "series_id", default)]
    pub series_id: Option<u32>,
    #[serde(alias = "run_type", default)]
    pub run_type: Option<u32>,
    #[serde(alias = "track_name", default)]
    pub track_name: Option<String>,
    #[serde(alias = "race_name", alias = "event_name", default)]
    pub race_name: Option<String>,
    #[serde(alias = "vehicles", alias = "drivers", default)]
    pub vehicles: Vec<NascarLiveVehicle>,
}

/// Single vehicle / driver entry in NASCAR live feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NascarLiveVehicle {
    #[serde(alias = "running_position", alias = "position", alias = "pos", default)]
    pub running_position: Option<u32>,
    #[serde(alias = "vehicle_number", alias = "car_number", alias = "number", default)]
    pub vehicle_number: Option<String>,
    #[serde(default)]
    pub driver: Option<NascarLiveDriverInfo>,
    #[serde(alias = "driver_name", default)]
    pub driver_name: Option<String>,
    #[serde(alias = "delta", alias = "time_delta", alias = "gap", default)]
    pub delta: Option<serde_json::Value>,
    #[serde(alias = "sponsor_name", alias = "sponsor", default)]
    pub sponsor_name: Option<String>,
    #[serde(alias = "team_name", default)]
    pub team_name: Option<String>,
    #[serde(alias = "vehicle_manufacturer", alias = "manufacturer", default)]
    pub vehicle_manufacturer: Option<String>,
    #[serde(alias = "laps_completed", default)]
    pub laps_completed: Option<u32>,
    #[serde(alias = "last_lap_time", default)]
    pub last_lap_time: Option<f64>,
    #[serde(alias = "best_lap_time", default)]
    pub best_lap_time: Option<f64>,
    #[serde(alias = "is_on_track", default)]
    pub is_on_track: Option<bool>,
    #[serde(alias = "status", default)]
    pub status: Option<serde_json::Value>,
    #[serde(alias = "pit_stops", default)]
    pub pit_stops: Vec<NascarLivePitStop>,
}

/// Driver identity in NASCAR live vehicle.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NascarLiveDriverInfo {
    #[serde(alias = "full_name", default)]
    pub full_name: Option<String>,
    #[serde(alias = "first_name", default)]
    pub first_name: Option<String>,
    #[serde(alias = "last_name", default)]
    pub last_name: Option<String>,
    #[serde(alias = "driver_id", default)]
    pub driver_id: Option<u64>,
}

/// Pit stop record in NASCAR live vehicle.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NascarLivePitStop {
    #[serde(alias = "pit_in_lap", default)]
    pub pit_in_lap: Option<u32>,
    #[serde(alias = "pit_out_lap", default)]
    pub pit_out_lap: Option<u32>,
    #[serde(alias = "pit_duration", alias = "duration", default)]
    pub pit_duration: Option<f64>,
}

/// Live timing provider for NASCAR series (Cup, Xfinity, Trucks, ARCA).
pub struct NascarLiveProvider {
    pub nascar_series_id: u32,
    pub racetui_series_id: &'static str,
    client: reqwest::Client,
    feed_url: String,
}

impl NascarLiveProvider {
    /// Create a new provider for a specific NASCAR series.
    pub fn new(nascar_series_id: u32, racetui_series_id: &'static str) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            nascar_series_id,
            racetui_series_id,
            client,
            feed_url: NASCAR_LIVE_FEED_URL.to_string(),
        }
    }

    /// Create a provider with custom feed URL (useful for testing or series-specific endpoints).
    pub fn with_feed_url(
        nascar_series_id: u32,
        racetui_series_id: &'static str,
        feed_url: String,
    ) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            nascar_series_id,
            racetui_series_id,
            client,
            feed_url,
        }
    }
}

impl LiveProvider for NascarLiveProvider {
    async fn is_session_live(&self) -> Result<bool> {
        let feed: NascarLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to query NASCAR live feed")?
            .json()
            .await
            .context("Failed to parse NASCAR live feed JSON")?;

        // Verify if feed is for this series (if series_id is present)
        if let Some(sid) = feed.series_id {
            if sid != self.nascar_series_id && sid != 0 {
                return Ok(false);
            }
        }

        // Flag states: 1 = Green, 2 = Yellow, 3 = Red, 4 = Checkered, 8 = Warmup, 9 = Finished
        let is_live_flag = match feed.flag_state {
            Some(4) | Some(9) => false, // Checkered or Finished
            Some(_) => true,
            None => !feed.vehicles.is_empty(),
        };

        Ok(is_live_flag && !feed.vehicles.is_empty())
    }

    async fn fetch_timing(&self) -> Result<LiveTimingData> {
        let feed: NascarLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to fetch NASCAR live feed")?
            .json()
            .await
            .context("Failed to parse NASCAR live feed JSON")?;

        Ok(parse_nascar_live_feed(&feed, self.racetui_series_id))
    }
}

/// Parse NASCAR live feed into a standard LiveTimingData structure.
pub fn parse_nascar_live_feed(feed: &NascarLiveFeed, racetui_series_id: &str) -> LiveTimingData {
    let mut best_lap_overall: Option<f64> = None;

    for v in &feed.vehicles {
        if let Some(best) = v.best_lap_time {
            if best > 0.0 && best_lap_overall.map_or(true, |b| best < b) {
                best_lap_overall = Some(best);
            }
        }
    }

    let mut driver_entries = Vec::new();

    for v in &feed.vehicles {
        let pos = v.running_position.unwrap_or(99);

        // Driver name
        let name = if let Some(ref d) = v.driver {
            d.full_name.clone().unwrap_or_else(|| {
                match (&d.first_name, &d.last_name) {
                    (Some(f), Some(l)) => format!("{} {}", f, l),
                    (Some(f), None) => f.clone(),
                    (None, Some(l)) => l.clone(),
                    (None, None) => "Unknown Driver".to_string(),
                }
            })
        } else if let Some(ref n) = v.driver_name {
            n.clone()
        } else {
            "Unknown Driver".to_string()
        };

        let driver_code = if let Some(ref d) = v.driver {
            d.last_name.as_ref().map(|l| {
                l.chars()
                    .take(3)
                    .collect::<String>()
                    .to_uppercase()
            })
        } else {
            None
        };

        let driver_number = v
            .vehicle_number
            .as_ref()
            .and_then(|num_str| num_str.parse::<u32>().ok());

        let team = v
            .sponsor_name
            .clone()
            .or_else(|| v.team_name.clone())
            .or_else(|| v.vehicle_manufacturer.clone())
            .unwrap_or_else(|| "NASCAR Team".to_string());

        let gap = format_nascar_gap(v.delta.as_ref(), pos == 1);
        let interval = if pos == 1 { "-".to_string() } else { gap.clone() };

        let is_fastest = match (v.best_lap_time, best_lap_overall) {
            (Some(b), Some(overall)) => (b - overall).abs() < 0.0001,
            _ => false,
        };

        let stops_count = v.pit_stops.len() as u32;
        let last_pit = v.pit_stops.last();
        let pit_info = PitInfo {
            stops_count,
            last_stop_lap: last_pit.and_then(|p| p.pit_in_lap),
            last_stop_duration_secs: last_pit.and_then(|p| p.pit_duration),
            in_pit: v.is_on_track == Some(false),
        };

        let status = if v.is_on_track == Some(false) {
            "In Pit".to_string()
        } else {
            "On Track".to_string()
        };

        driver_entries.push(LiveDriverEntry {
            position: pos,
            driver_number,
            driver_name: name,
            driver_code,
            team_name: team,
            team_color: None,
            gap_to_leader: gap,
            interval,
            last_lap_time: v.last_lap_time.map(format_nascar_lap_time),
            best_lap_time: v.best_lap_time.map(format_nascar_lap_time),
            sectors: SectorTimes::default(),
            tire: None,
            pits: pit_info,
            laps_completed: v.laps_completed.unwrap_or(0),
            status,
            current_position: None,
            fastest_lap: is_fastest,
        });
    }

    driver_entries.sort_by_key(|d| d.position);

    let session_status = match feed.flag_state {
        Some(1) => "Green".to_string(),
        Some(2) => "Caution".to_string(),
        Some(3) => "Red".to_string(),
        Some(4) => "Checkered".to_string(),
        Some(8) => "Warmup".to_string(),
        Some(9) => "Finished".to_string(),
        _ => "Active".to_string(),
    };

    let session_name = feed.run_name.clone().unwrap_or_else(|| "Race".to_string());
    let event_name = feed.race_name.clone().unwrap_or_else(|| "NASCAR Race".to_string());
    let circuit_name = feed.track_name.clone().unwrap_or_else(|| "Track".to_string());

    LiveTimingData {
        series_id: racetui_series_id.to_string(),
        session_name,
        event_name,
        circuit_name,
        total_laps: feed.laps_in_race,
        current_lap: feed.lap_number,
        time_remaining: None,
        session_status,
        drivers: driver_entries,
        weather: None,
        updated_at: Utc::now(),
    }
}

fn format_nascar_gap(val: Option<&serde_json::Value>, is_p1: bool) -> String {
    if is_p1 {
        return "LEADER".to_string();
    }
    match val {
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                if f == 0.0 {
                    "LEADER".to_string()
                } else {
                    format!("+{:.3}s", f)
                }
            } else {
                "-".to_string()
            }
        }
        Some(serde_json::Value::String(s)) => {
            if s.eq_ignore_ascii_case("leader") || s == "0" || s == "0.0" {
                "LEADER".to_string()
            } else if !s.starts_with('+') && !s.contains("LAP") {
                format!("+{}", s)
            } else {
                s.clone()
            }
        }
        _ => "-".to_string(),
    }
}

fn format_nascar_lap_time(duration_secs: f64) -> String {
    if duration_secs <= 0.0 {
        return "-".to_string();
    }
    let total_ms = (duration_secs * 1000.0).round() as u64;
    let mins = total_ms / 60000;
    let secs = (total_ms % 60000) / 1000;
    let ms = total_ms % 1000;
    if mins > 0 {
        format!("{}:{:02}.{:03}", mins, secs, ms)
    } else {
        format!("{}.{:03}", secs, ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nascar_live_feed_sample() {
        let sample_json = r#"{
            "lap_number": 55,
            "laps_in_race": 200,
            "laps_to_go": 145,
            "flag_state": 1,
            "race_id": 5400,
            "run_name": "Race",
            "series_id": 1,
            "track_name": "Daytona International Speedway",
            "race_name": "Daytona 500",
            "vehicles": [
                {
                    "running_position": 1,
                    "vehicle_number": "5",
                    "driver": {
                        "full_name": "Kyle Larson",
                        "first_name": "Kyle",
                        "last_name": "Larson",
                        "driver_id": 1234
                    },
                    "delta": 0.0,
                    "sponsor_name": "HendrickCars.com",
                    "laps_completed": 55,
                    "last_lap_time": 45.123,
                    "best_lap_time": 44.890,
                    "is_on_track": true,
                    "pit_stops": [
                        {
                            "pit_in_lap": 30,
                            "pit_out_lap": 31,
                            "pit_duration": 12.4
                        }
                    ]
                },
                {
                    "running_position": 2,
                    "vehicle_number": "9",
                    "driver": {
                        "full_name": "Chase Elliott",
                        "first_name": "Chase",
                        "last_name": "Elliott",
                        "driver_id": 5678
                    },
                    "delta": 0.150,
                    "sponsor_name": "NAPA Auto Parts",
                    "laps_completed": 55,
                    "last_lap_time": 45.200,
                    "best_lap_time": 45.050,
                    "is_on_track": true,
                    "pit_stops": []
                }
            ]
        }"#;

        let feed: NascarLiveFeed = serde_json::from_str(sample_json).expect("parse sample nascar feed");
        let timing = parse_nascar_live_feed(&feed, "nascar_cup");

        assert_eq!(timing.series_id, "nascar_cup");
        assert_eq!(timing.session_name, "Race");
        assert_eq!(timing.event_name, "Daytona 500");
        assert_eq!(timing.circuit_name, "Daytona International Speedway");
        assert_eq!(timing.current_lap, Some(55));
        assert_eq!(timing.total_laps, Some(200));
        assert_eq!(timing.session_status, "Green");
        assert_eq!(timing.drivers.len(), 2);

        let d1 = &timing.drivers[0];
        assert_eq!(d1.position, 1);
        assert_eq!(d1.driver_name, "Kyle Larson");
        assert_eq!(d1.driver_code.as_deref(), Some("LAR"));
        assert_eq!(d1.driver_number, Some(5));
        assert_eq!(d1.gap_to_leader, "LEADER");
        assert_eq!(d1.best_lap_time.as_deref(), Some("44.890"));
        assert!(d1.fastest_lap);
        assert_eq!(d1.pits.stops_count, 1);
        assert_eq!(d1.pits.last_stop_lap, Some(30));

        let d2 = &timing.drivers[1];
        assert_eq!(d2.position, 2);
        assert_eq!(d2.driver_name, "Chase Elliott");
        assert_eq!(d2.gap_to_leader, "+0.150s");
        assert!(!d2.fastest_lap);
        assert_eq!(d2.pits.stops_count, 0);
    }
}
