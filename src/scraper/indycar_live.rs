use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use crate::live::event::{LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes, TireInfo};
use crate::live::LiveProvider;
use crate::scraper::fetcher::create_http_client;

const INDYCAR_LIVE_TIMINGS_URL: &str = "https://racecontrol.indycar.com/xml/timings.json";

/// IndyCar live timings root structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IndyCarLiveFeed {
    #[serde(alias = "session", alias = "session_info", default)]
    pub session: Option<IndyCarSessionInfo>,
    #[serde(
        alias = "drivers",
        alias = "cars",
        alias = "timing_data",
        alias = "records",
        default
    )]
    pub drivers: Vec<IndyCarDriverTiming>,
    #[serde(alias = "flag_status", alias = "flag", alias = "track_status", default)]
    pub flag_status: Option<String>,
    #[serde(alias = "lap_number", alias = "current_lap", default)]
    pub lap_number: Option<u32>,
    #[serde(alias = "total_laps", alias = "laps_in_race", default)]
    pub total_laps: Option<u32>,
}

/// Session metadata in IndyCar live feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IndyCarSessionInfo {
    #[serde(alias = "session_name", alias = "name", alias = "run_name", default)]
    pub session_name: Option<String>,
    #[serde(alias = "track_name", alias = "circuit_name", default)]
    pub track_name: Option<String>,
    #[serde(alias = "flag_status", alias = "flag", default)]
    pub flag_status: Option<String>,
    #[serde(alias = "lap_number", alias = "current_lap", default)]
    pub lap_number: Option<u32>,
    #[serde(alias = "total_laps", default)]
    pub total_laps: Option<u32>,
    #[serde(alias = "time_remaining", default)]
    pub time_remaining: Option<String>,
}

/// Single driver entry in IndyCar live timings.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IndyCarDriverTiming {
    #[serde(alias = "rank", alias = "position", alias = "pos", default)]
    pub rank: Option<u32>,
    #[serde(alias = "car_number", alias = "number", alias = "car_no", default)]
    pub car_number: Option<String>,
    #[serde(alias = "full_name", alias = "driver_name", alias = "name", default)]
    pub driver_name: Option<String>,
    #[serde(alias = "first_name", default)]
    pub first_name: Option<String>,
    #[serde(alias = "last_name", default)]
    pub last_name: Option<String>,
    #[serde(alias = "team_name", alias = "team", alias = "entrant", default)]
    pub team_name: Option<String>,
    #[serde(alias = "gap", alias = "gap_to_leader", alias = "diff", default)]
    pub gap: Option<serde_json::Value>,
    #[serde(alias = "interval", alias = "int", default)]
    pub interval: Option<serde_json::Value>,
    #[serde(
        alias = "last_lap_time",
        alias = "last_lap",
        alias = "last_time",
        default
    )]
    pub last_lap_time: Option<serde_json::Value>,
    #[serde(
        alias = "best_lap_time",
        alias = "best_lap",
        alias = "fastest_lap_time",
        default
    )]
    pub best_lap_time: Option<serde_json::Value>,
    #[serde(alias = "laps_completed", alias = "laps", default)]
    pub laps_completed: Option<u32>,
    #[serde(alias = "pit_stops", alias = "pits", alias = "num_pits", default)]
    pub pit_stops: Option<u32>,
    #[serde(alias = "last_pit_lap", default)]
    pub last_pit_lap: Option<u32>,
    #[serde(alias = "in_pit", default)]
    pub in_pit: Option<bool>,
    #[serde(alias = "status", default)]
    pub status: Option<String>,
    #[serde(alias = "tire_compound", alias = "tire", alias = "compound", default)]
    pub tire_compound: Option<String>,
    #[serde(
        alias = "tires_laps",
        alias = "tire_laps",
        alias = "stint_laps",
        default
    )]
    pub tires_laps: Option<u32>,
    #[serde(alias = "s1", alias = "sector1", default)]
    pub s1: Option<String>,
    #[serde(alias = "s2", alias = "sector2", default)]
    pub s2: Option<String>,
    #[serde(alias = "s3", alias = "sector3", default)]
    pub s3: Option<String>,
}

/// Live timing provider for IndyCar and Indy NXT.
pub struct IndyCarLiveProvider {
    client: reqwest::Client,
    feed_url: String,
    series_id: &'static str,
}

impl IndyCarLiveProvider {
    /// Create a new IndyCar live provider.
    pub fn new() -> Self {
        Self::new_with_series("indycar")
    }

    /// Create a new provider for a specific series (e.g. "indycar", "indy_nxt").
    pub fn new_with_series(series_id: &'static str) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            feed_url: INDYCAR_LIVE_TIMINGS_URL.to_string(),
            series_id,
        }
    }

    /// Create with a custom feed URL (useful for testing or fallback mirrors).
    pub fn with_feed_url(feed_url: String, series_id: &'static str) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            feed_url,
            series_id,
        }
    }
}

impl Default for IndyCarLiveProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveProvider for IndyCarLiveProvider {
    async fn is_session_live(&self) -> Result<bool> {
        let feed: IndyCarLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to query IndyCar live feed")?
            .json()
            .await
            .context("Failed to parse IndyCar live feed JSON")?;

        if feed.drivers.is_empty() {
            return Ok(false);
        }

        let flag = feed
            .session
            .as_ref()
            .and_then(|s| s.flag_status.as_deref())
            .or(feed.flag_status.as_deref())
            .unwrap_or("Active");

        let is_finished = flag.eq_ignore_ascii_case("checkered")
            || flag.eq_ignore_ascii_case("finished")
            || flag.eq_ignore_ascii_case("final");

        Ok(!is_finished)
    }

    async fn fetch_timing(&self) -> Result<LiveTimingData> {
        let feed: IndyCarLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to fetch IndyCar live feed")?
            .json()
            .await
            .context("Failed to parse IndyCar live feed JSON")?;

        Ok(parse_indycar_live_feed(&feed, self.series_id))
    }
}

/// Parse IndyCar live feed into standard LiveTimingData.
pub fn parse_indycar_live_feed(feed: &IndyCarLiveFeed, series_id: &str) -> LiveTimingData {
    let mut driver_entries = Vec::new();

    for d in &feed.drivers {
        let pos = d.rank.unwrap_or(99);

        let name = if let Some(ref n) = d.driver_name {
            n.clone()
        } else {
            match (&d.first_name, &d.last_name) {
                (Some(f), Some(l)) => format!("{} {}", f, l),
                (Some(f), None) => f.clone(),
                (None, Some(l)) => l.clone(),
                (None, None) => "Unknown Driver".to_string(),
            }
        };

        let driver_code = d.last_name.as_ref().or(d.driver_name.as_ref()).map(|l| {
            l.chars()
                .filter(|c| c.is_alphabetic())
                .take(3)
                .collect::<String>()
                .to_uppercase()
        });

        let driver_number = d
            .car_number
            .as_ref()
            .and_then(|num_str| num_str.parse::<u32>().ok());

        let team = d
            .team_name
            .clone()
            .unwrap_or_else(|| "IndyCar Team".to_string());

        let gap = format_json_val_or_default(d.gap.as_ref(), if pos == 1 { "LEADER" } else { "-" });
        let interval =
            format_json_val_or_default(d.interval.as_ref(), if pos == 1 { "-" } else { "-" });

        let last_lap_str = format_json_val_opt(d.last_lap_time.as_ref());
        let best_lap_str = format_json_val_opt(d.best_lap_time.as_ref());

        let tire = d.tire_compound.as_ref().map(|comp| {
            let normalized_comp = match comp.to_lowercase().as_str() {
                "alternate" | "alt" | "soft" | "red" => "Alternate",
                "primary" | "prim" | "hard" | "black" => "Primary",
                "rain" | "wet" => "Rain",
                _ => comp.as_str(),
            };
            TireInfo {
                compound: normalized_comp.to_string(),
                laps: d.tires_laps.unwrap_or(0),
                is_new: false,
            }
        });

        let in_pit = d.in_pit == Some(true)
            || d.status
                .as_deref()
                .map_or(false, |s| s.eq_ignore_ascii_case("pit"));

        let pit_info = PitInfo {
            stops_count: d.pit_stops.unwrap_or(0),
            last_stop_lap: d.last_pit_lap,
            last_stop_duration_secs: None,
            in_pit,
        };

        let status = if in_pit {
            "In Pit".to_string()
        } else if let Some(ref st) = d.status {
            st.clone()
        } else {
            "On Track".to_string()
        };

        let sectors = SectorTimes {
            s1_ms: None,
            s2_ms: None,
            s3_ms: None,
            s1_str: d.s1.clone(),
            s2_str: d.s2.clone(),
            s3_str: d.s3.clone(),
            s1_fastest: false,
            s2_fastest: false,
            s3_fastest: false,
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
            last_lap_time: last_lap_str,
            best_lap_time: best_lap_str,
            sectors,
            tire,
            pits: pit_info,
            laps_completed: d.laps_completed.unwrap_or(0),
            status,
            current_position: None,
            fastest_lap: false,
        });
    }

    driver_entries.sort_by_key(|d| d.position);

    // Identify fastest lap across the field
    let mut min_best_str: Option<String> = None;
    for d in &driver_entries {
        if let Some(ref best) = d.best_lap_time {
            if best != "-" && !best.is_empty() {
                if min_best_str.as_ref().map_or(true, |m| best < m) {
                    min_best_str = Some(best.clone());
                }
            }
        }
    }

    if let Some(ref min_best) = min_best_str {
        for d in &mut driver_entries {
            if d.best_lap_time.as_ref() == Some(min_best) {
                d.fastest_lap = true;
                break;
            }
        }
    }

    let session_name = feed
        .session
        .as_ref()
        .and_then(|s| s.session_name.clone())
        .unwrap_or_else(|| "Race".to_string());

    let circuit_name = feed
        .session
        .as_ref()
        .and_then(|s| s.track_name.clone())
        .unwrap_or_else(|| "IndyCar Circuit".to_string());

    let session_status = feed
        .session
        .as_ref()
        .and_then(|s| s.flag_status.clone())
        .or_else(|| feed.flag_status.clone())
        .unwrap_or_else(|| "Green".to_string());

    let current_lap = feed
        .session
        .as_ref()
        .and_then(|s| s.lap_number)
        .or(feed.lap_number);

    let total_laps = feed
        .session
        .as_ref()
        .and_then(|s| s.total_laps)
        .or(feed.total_laps);

    let time_remaining = feed.session.as_ref().and_then(|s| s.time_remaining.clone());

    LiveTimingData {
        series_id: series_id.to_string(),
        session_name,
        event_name: format!("IndyCar @ {}", circuit_name),
        circuit_name,
        total_laps,
        current_lap,
        time_remaining,
        session_status,
        drivers: driver_entries,
        weather: None,
        updated_at: Utc::now(),
    }
}

fn format_json_val_or_default(val: Option<&serde_json::Value>, default: &str) -> String {
    match val {
        Some(serde_json::Value::String(s)) => {
            if s.is_empty() {
                default.to_string()
            } else {
                s.clone()
            }
        }
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                if f == 0.0 {
                    "LEADER".to_string()
                } else {
                    format!("+{:.3}s", f)
                }
            } else {
                n.to_string()
            }
        }
        _ => default.to_string(),
    }
}

fn format_json_val_opt(val: Option<&serde_json::Value>) -> Option<String> {
    match val {
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                let total_ms = (f * 1000.0).round() as u64;
                let mins = total_ms / 60000;
                let secs = (total_ms % 60000) / 1000;
                let ms = total_ms % 1000;
                if mins > 0 {
                    Some(format!("{}:{:02}.{:03}", mins, secs, ms))
                } else {
                    Some(format!("{}.{:03}", secs, ms))
                }
            } else {
                Some(n.to_string())
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_indycar_live_feed_sample() {
        let sample_json = r#"{
            "session": {
                "session_name": "Race",
                "track_name": "Indianapolis Motor Speedway",
                "flag_status": "Green",
                "lap_number": 85,
                "total_laps": 200
            },
            "drivers": [
                {
                    "rank": 1,
                    "car_number": "10",
                    "driver_name": "Alex Palou",
                    "team_name": "Chip Ganassi Racing",
                    "gap": "LEADER",
                    "interval": "-",
                    "last_lap_time": "00:40.1234",
                    "best_lap_time": "00:39.8765",
                    "laps_completed": 85,
                    "pit_stops": 2,
                    "last_pit_lap": 60,
                    "in_pit": false,
                    "tire_compound": "Primary",
                    "tires_laps": 25,
                    "s1": "13.234",
                    "s2": "14.123",
                    "s3": "12.519"
                },
                {
                    "rank": 2,
                    "car_number": "2",
                    "first_name": "Josef",
                    "last_name": "Newgarden",
                    "team_name": "Team Penske",
                    "gap": "+0.450",
                    "interval": "+0.450",
                    "last_lap_time": "00:40.3000",
                    "best_lap_time": "00:40.0100",
                    "laps_completed": 85,
                    "pit_stops": 2,
                    "last_pit_lap": 58,
                    "in_pit": false,
                    "tire_compound": "Alternate",
                    "tires_laps": 27,
                    "s1": "13.300",
                    "s2": "14.200",
                    "s3": "12.800"
                }
            ]
        }"#;

        let feed: IndyCarLiveFeed =
            serde_json::from_str(sample_json).expect("parse sample indycar live feed");
        let timing = parse_indycar_live_feed(&feed, "indycar");

        assert_eq!(timing.series_id, "indycar");
        assert_eq!(timing.session_name, "Race");
        assert_eq!(timing.circuit_name, "Indianapolis Motor Speedway");
        assert_eq!(timing.session_status, "Green");
        assert_eq!(timing.current_lap, Some(85));
        assert_eq!(timing.total_laps, Some(200));
        assert_eq!(timing.drivers.len(), 2);

        let d1 = &timing.drivers[0];
        assert_eq!(d1.position, 1);
        assert_eq!(d1.driver_name, "Alex Palou");
        assert_eq!(d1.driver_number, Some(10));
        assert_eq!(d1.team_name, "Chip Ganassi Racing");
        assert_eq!(d1.gap_to_leader, "LEADER");
        assert_eq!(d1.tire.as_ref().unwrap().compound, "Primary");
        assert_eq!(d1.tire.as_ref().unwrap().laps, 25);
        assert!(d1.fastest_lap);
        assert_eq!(d1.sectors.s1_str.as_deref(), Some("13.234"));

        let d2 = &timing.drivers[1];
        assert_eq!(d2.position, 2);
        assert_eq!(d2.driver_name, "Josef Newgarden");
        assert_eq!(d2.driver_code.as_deref(), Some("NEW"));
        assert_eq!(d2.driver_number, Some(2));
        assert_eq!(d2.tire.as_ref().unwrap().compound, "Alternate");
        assert_eq!(d2.gap_to_leader, "+0.450");
        assert!(!d2.fastest_lap);
    }
}
