use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use crate::live::event::{
    LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes, TireInfo, WeatherInfo,
};
use crate::live::LiveProvider;
use crate::scraper::fetcher::create_http_client;

const MOTOGP_LIVE_TIMING_URL: &str = "https://api.motogp.pulselive.com/motogp/v1/live-timing/session";

/// MotoGP live timing feed root structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MotoGPLiveFeed {
    #[serde(alias = "session", alias = "session_info", alias = "head", default)]
    pub session: Option<MotoGPSessionInfo>,
    #[serde(alias = "weather", alias = "track_conditions", default)]
    pub weather: Option<MotoGPWeatherInfo>,
    #[serde(alias = "riders", alias = "drivers", alias = "classification", alias = "records", default)]
    pub riders: Vec<MotoGPRiderEntry>,
    #[serde(alias = "status", alias = "flag", alias = "session_status", default)]
    pub status: Option<String>,
    #[serde(alias = "circuit", alias = "track", default)]
    pub circuit: Option<String>,
}

/// Session metadata in MotoGP live timing.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MotoGPSessionInfo {
    #[serde(alias = "name", alias = "session_name", alias = "type", default)]
    pub name: Option<String>,
    #[serde(alias = "circuit", alias = "track", alias = "circuit_name", default)]
    pub circuit: Option<String>,
    #[serde(alias = "status", alias = "flag", alias = "condition", default)]
    pub status: Option<String>,
    #[serde(alias = "laps", alias = "current_lap", default)]
    pub laps: Option<u32>,
    #[serde(alias = "total_laps", default)]
    pub total_laps: Option<u32>,
    #[serde(alias = "time_remaining", alias = "remaining", default)]
    pub time_remaining: Option<String>,
}

/// Weather conditions in MotoGP live timing.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MotoGPWeatherInfo {
    #[serde(alias = "air_temp", alias = "temp_air", default)]
    pub air_temp: Option<f64>,
    #[serde(alias = "track_temp", alias = "temp_track", default)]
    pub track_temp: Option<f64>,
    #[serde(alias = "humidity", default)]
    pub humidity: Option<f64>,
    #[serde(alias = "wind_speed", default)]
    pub wind_speed: Option<f64>,
    #[serde(alias = "wind_direction", alias = "wind_dir", default)]
    pub wind_direction: Option<u32>,
    #[serde(alias = "wet", alias = "rainfall", alias = "rain", default)]
    pub wet: Option<bool>,
    #[serde(alias = "condition", alias = "description", default)]
    pub condition: Option<String>,
}

/// Single rider timing entry in MotoGP feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MotoGPRiderEntry {
    #[serde(alias = "pos", alias = "position", alias = "rank", default)]
    pub pos: Option<u32>,
    #[serde(alias = "number", alias = "rider_number", alias = "num", alias = "bib", default)]
    pub number: Option<String>,
    #[serde(alias = "rider", alias = "rider_name", alias = "name", alias = "full_name", default)]
    pub rider: Option<String>,
    #[serde(alias = "first_name", default)]
    pub first_name: Option<String>,
    #[serde(alias = "last_name", default)]
    pub last_name: Option<String>,
    #[serde(alias = "team", alias = "team_name", default)]
    pub team: Option<String>,
    #[serde(alias = "bike", alias = "constructor", alias = "manufacturer", default)]
    pub bike: Option<String>,
    #[serde(alias = "gap", alias = "gap_leader", alias = "diff", default)]
    pub gap: Option<serde_json::Value>,
    #[serde(alias = "interval", alias = "int", alias = "diff_prev", default)]
    pub interval: Option<serde_json::Value>,
    #[serde(alias = "last_lap", alias = "last_lap_time", default)]
    pub last_lap: Option<serde_json::Value>,
    #[serde(alias = "best_lap", alias = "best_lap_time", alias = "fastest_lap", default)]
    pub best_lap: Option<serde_json::Value>,
    #[serde(alias = "laps", alias = "laps_completed", default)]
    pub laps: Option<u32>,
    #[serde(alias = "in_pit", alias = "pit", alias = "on_pit", default)]
    pub in_pit: Option<bool>,
    #[serde(alias = "state", alias = "status", default)]
    pub state: Option<String>,
    #[serde(alias = "tire_front", alias = "front_tire", alias = "tire", alias = "compound", default)]
    pub tire: Option<String>,
    #[serde(alias = "s1", alias = "sector1", default)]
    pub s1: Option<String>,
    #[serde(alias = "s2", alias = "sector2", default)]
    pub s2: Option<String>,
    #[serde(alias = "s3", alias = "sector3", default)]
    pub s3: Option<String>,
}

/// Live timing provider for MotoGP, Moto2, Moto3, WorldSBK.
pub struct MotoGPLiveProvider {
    client: reqwest::Client,
    feed_url: String,
    series_id: &'static str,
}

impl MotoGPLiveProvider {
    /// Create a new provider for MotoGP.
    pub fn new() -> Self {
        Self::new_with_series("motogp")
    }

    /// Create a new provider for a specific series.
    pub fn new_with_series(series_id: &'static str) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            feed_url: MOTOGP_LIVE_TIMING_URL.to_string(),
            series_id,
        }
    }

    /// Create with a custom feed URL.
    pub fn with_url(feed_url: String, series_id: &'static str) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            feed_url,
            series_id,
        }
    }
}

impl Default for MotoGPLiveProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveProvider for MotoGPLiveProvider {
    async fn is_session_live(&self) -> Result<bool> {
        let feed: MotoGPLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to query MotoGP live feed")?
            .json()
            .await
            .context("Failed to parse MotoGP live feed JSON")?;

        if feed.riders.is_empty() {
            return Ok(false);
        }

        let flag = feed
            .session
            .as_ref()
            .and_then(|s| s.status.as_deref())
            .or(feed.status.as_deref())
            .unwrap_or("Active");

        let is_finished = flag.eq_ignore_ascii_case("checkered")
            || flag.eq_ignore_ascii_case("finished")
            || flag.eq_ignore_ascii_case("final")
            || flag.eq_ignore_ascii_case("ended");

        Ok(!is_finished)
    }

    async fn fetch_timing(&self) -> Result<LiveTimingData> {
        let feed: MotoGPLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to fetch MotoGP live feed")?
            .json()
            .await
            .context("Failed to parse MotoGP live feed JSON")?;

        Ok(parse_motogp_live_feed(&feed, self.series_id))
    }
}

/// Parse MotoGP live feed into standard LiveTimingData.
pub fn parse_motogp_live_feed(feed: &MotoGPLiveFeed, series_id: &str) -> LiveTimingData {
    let mut driver_entries = Vec::new();

    for r in &feed.riders {
        let pos = r.pos.unwrap_or(99);

        let name = if let Some(ref d) = r.rider {
            d.clone()
        } else {
            match (&r.first_name, &r.last_name) {
                (Some(f), Some(l)) => format!("{} {}", f, l),
                (Some(f), None) => f.clone(),
                (None, Some(l)) => l.clone(),
                (None, None) => "Unknown Rider".to_string(),
            }
        };

        let driver_code = if let Some(ref l) = r.last_name {
            Some(
                l.chars()
                    .filter(|ch| ch.is_alphabetic())
                    .take(3)
                    .collect::<String>()
                    .to_uppercase(),
            )
        } else if let Some(ref d) = r.rider {
            let surname = d.split_whitespace().last().unwrap_or(d.as_str());
            Some(
                surname
                    .chars()
                    .filter(|ch| ch.is_alphabetic())
                    .take(3)
                    .collect::<String>()
                    .to_uppercase(),
            )
        } else {
            None
        };

        let driver_number = r
            .number
            .as_ref()
            .and_then(|n| n.parse::<u32>().ok());

        let team = if let Some(ref t) = r.team {
            if let Some(ref b) = r.bike {
                format!("{} ({})", t, b)
            } else {
                t.clone()
            }
        } else {
            r.bike.clone().unwrap_or_else(|| "MotoGP Team".to_string())
        };

        let gap = format_json_val_or_default(r.gap.as_ref(), if pos == 1 { "LEADER" } else { "-" });
        let interval = format_json_val_or_default(r.interval.as_ref(), if pos == 1 { "-" } else { "-" });

        let last_lap_str = format_json_val_opt(r.last_lap.as_ref());
        let best_lap_str = format_json_val_opt(r.best_lap.as_ref());

        let tire = r.tire.as_ref().map(|comp| {
            let normalized = match comp.to_lowercase().as_str() {
                "soft" | "s" => "Soft",
                "medium" | "m" => "Medium",
                "hard" | "h" => "Hard",
                "wet" | "w" | "rain" => "Wet",
                _ => comp.as_str(),
            };
            TireInfo {
                compound: normalized.to_string(),
                laps: r.laps.unwrap_or(0),
                is_new: false,
            }
        });

        let in_pit = r.in_pit == Some(true) || r.state.as_deref().map_or(false, |s| s.eq_ignore_ascii_case("pit"));

        let pit_info = PitInfo {
            stops_count: 0,
            last_stop_lap: None,
            last_stop_duration_secs: None,
            in_pit,
        };

        let status = if in_pit {
            "In Pit".to_string()
        } else if let Some(ref st) = r.state {
            st.clone()
        } else {
            "On Track".to_string()
        };

        let sectors = SectorTimes {
            s1_ms: None,
            s2_ms: None,
            s3_ms: None,
            s1_str: r.s1.clone(),
            s2_str: r.s2.clone(),
            s3_str: r.s3.clone(),
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
            laps_completed: r.laps.unwrap_or(0),
            status,
            current_position: None,
            fastest_lap: false,
        });
    }

    driver_entries.sort_by_key(|d| d.position);

    // Identify fastest lap
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
        .and_then(|s| s.name.clone())
        .unwrap_or_else(|| "Race".to_string());

    let circuit_name = feed
        .session
        .as_ref()
        .and_then(|s| s.circuit.clone())
        .or_else(|| feed.circuit.clone())
        .unwrap_or_else(|| "MotoGP Circuit".to_string());

    let session_status = feed
        .session
        .as_ref()
        .and_then(|s| s.status.clone())
        .or_else(|| feed.status.clone())
        .unwrap_or_else(|| "Green".to_string());

    let current_lap = feed
        .session
        .as_ref()
        .and_then(|s| s.laps);

    let total_laps = feed
        .session
        .as_ref()
        .and_then(|s| s.total_laps);

    let time_remaining = feed
        .session
        .as_ref()
        .and_then(|s| s.time_remaining.clone());

    let weather = feed.weather.as_ref().map(|w| WeatherInfo {
        air_temp_c: w.air_temp,
        track_temp_c: w.track_temp,
        humidity_pct: w.humidity,
        wind_speed_kmh: w.wind_speed,
        wind_direction_deg: w.wind_direction.map(|d| d as f64),
        rainfall: w.wet.unwrap_or(false),
        rain_intensity: None,
        description: w.condition.clone(),
    });

    LiveTimingData {
        series_id: series_id.to_string(),
        session_name,
        event_name: format!("{} @ {}", series_id.to_uppercase(), circuit_name),
        circuit_name,
        total_laps,
        current_lap,
        time_remaining,
        session_status,
        drivers: driver_entries,
        weather,
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
    fn test_parse_motogp_live_feed_sample() {
        let sample_json = r#"{
            "session": {
                "name": "MotoGP Race",
                "circuit": "Autodromo Termas de Rio Hondo",
                "status": "Green",
                "laps": 15,
                "total_laps": 25
            },
            "weather": {
                "air_temp": 26.0,
                "track_temp": 39.5,
                "humidity": 48.0,
                "wind_speed": 8.0,
                "wet": false,
                "condition": "Dry"
            },
            "riders": [
                {
                    "pos": 1,
                    "number": "1",
                    "rider": "Francesco Bagnaia",
                    "team": "Ducati Lenovo Team",
                    "bike": "Ducati",
                    "gap": "LEADER",
                    "interval": "-",
                    "last_lap": "1:39.123",
                    "best_lap": "1:38.890",
                    "laps": 15,
                    "in_pit": false,
                    "tire_front": "Hard",
                    "s1": "24.123",
                    "s2": "30.234",
                    "s3": "22.533"
                },
                {
                    "pos": 2,
                    "number": "89",
                    "first_name": "Jorge",
                    "last_name": "Martin",
                    "team": "Prima Pramac Racing",
                    "bike": "Ducati",
                    "gap": "+0.312",
                    "interval": "+0.312",
                    "last_lap": "1:39.250",
                    "best_lap": "1:38.910",
                    "laps": 15,
                    "in_pit": false,
                    "tire_front": "Medium",
                    "s1": "24.200",
                    "s2": "30.300",
                    "s3": "22.600"
                }
            ]
        }"#;

        let feed: MotoGPLiveFeed = serde_json::from_str(sample_json).expect("parse motogp feed");
        let timing = parse_motogp_live_feed(&feed, "motogp");

        assert_eq!(timing.series_id, "motogp");
        assert_eq!(timing.session_name, "MotoGP Race");
        assert_eq!(timing.circuit_name, "Autodromo Termas de Rio Hondo");
        assert_eq!(timing.session_status, "Green");
        assert_eq!(timing.current_lap, Some(15));
        assert_eq!(timing.total_laps, Some(25));

        let weather = timing.weather.as_ref().expect("weather data");
        assert_eq!(weather.air_temp_c, Some(26.0));
        assert_eq!(weather.track_temp_c, Some(39.5));
        assert_eq!(weather.humidity_pct, Some(48.0));
        assert!(!weather.rainfall);

        assert_eq!(timing.drivers.len(), 2);
        let r1 = &timing.drivers[0];
        assert_eq!(r1.position, 1);
        assert_eq!(r1.driver_name, "Francesco Bagnaia");
        assert_eq!(r1.driver_number, Some(1));
        assert_eq!(r1.team_name, "Ducati Lenovo Team (Ducati)");
        assert_eq!(r1.tire.as_ref().unwrap().compound, "Hard");
        assert!(r1.fastest_lap);

        let r2 = &timing.drivers[1];
        assert_eq!(r2.position, 2);
        assert_eq!(r2.driver_name, "Jorge Martin");
        assert_eq!(r2.driver_code.as_deref(), Some("MAR"));
        assert_eq!(r2.driver_number, Some(89));
        assert_eq!(r2.tire.as_ref().unwrap().compound, "Medium");
        assert_eq!(r2.gap_to_leader, "+0.312");
        assert!(!r2.fastest_lap);
    }
}
