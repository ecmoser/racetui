use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use crate::live::event::{
    LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes, TireInfo, WeatherInfo,
};
use crate::live::LiveProvider;
use crate::scraper::fetcher::create_http_client;

const WRC_LIVE_TIMING_URL: &str = "https://api.wrc.com/contelive/results/live-timing.json";

/// WRC live timing feed root structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WrcLiveFeed {
    #[serde(alias = "rally", alias = "event", alias = "session", default)]
    pub rally: Option<WrcRallyInfo>,
    #[serde(alias = "stage", alias = "current_stage", default)]
    pub stage: Option<WrcStageInfo>,
    #[serde(alias = "weather", alias = "conditions", default)]
    pub weather: Option<WrcWeatherInfo>,
    #[serde(alias = "entries", alias = "drivers", alias = "cars", alias = "classification", default)]
    pub entries: Vec<WrcDriverEntry>,
    #[serde(alias = "status", alias = "stage_status", default)]
    pub status: Option<String>,
}

/// Rally event info in WRC feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WrcRallyInfo {
    #[serde(alias = "name", alias = "rally_name", alias = "event_name", default)]
    pub name: Option<String>,
    #[serde(alias = "country", alias = "location", default)]
    pub location: Option<String>,
    #[serde(alias = "total_stages", alias = "stages_count", default)]
    pub total_stages: Option<u32>,
    #[serde(alias = "current_stage_number", alias = "stage_number", default)]
    pub current_stage_number: Option<u32>,
}

/// Active stage info in WRC feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WrcStageInfo {
    #[serde(alias = "name", alias = "stage_name", alias = "title", default)]
    pub name: Option<String>,
    #[serde(alias = "distance", alias = "distance_km", alias = "length", default)]
    pub distance_km: Option<f64>,
    #[serde(alias = "status", default)]
    pub status: Option<String>,
    #[serde(alias = "surface", default)]
    pub surface: Option<String>,
}

/// Weather info in WRC feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WrcWeatherInfo {
    #[serde(alias = "air_temp", alias = "temp", default)]
    pub air_temp: Option<f64>,
    #[serde(alias = "condition", alias = "weather", default)]
    pub condition: Option<String>,
    #[serde(alias = "rain", alias = "rainfall", default)]
    pub rain: Option<bool>,
}

/// Driver / Car timing entry in WRC feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WrcDriverEntry {
    #[serde(alias = "pos", alias = "position", alias = "rank", alias = "overall_pos", default)]
    pub pos: Option<u32>,
    #[serde(alias = "car_number", alias = "number", alias = "no", default)]
    pub number: Option<String>,
    #[serde(alias = "driver", alias = "driver_name", alias = "name", default)]
    pub driver: Option<String>,
    #[serde(alias = "first_name", default)]
    pub first_name: Option<String>,
    #[serde(alias = "last_name", default)]
    pub last_name: Option<String>,
    #[serde(alias = "codriver", alias = "co_driver", alias = "codriver_name", default)]
    pub codriver: Option<String>,
    #[serde(alias = "team", alias = "team_name", alias = "entrant", default)]
    pub team: Option<String>,
    #[serde(alias = "car", alias = "vehicle", alias = "model", default)]
    pub car: Option<String>,
    #[serde(alias = "class", alias = "group", alias = "category", default)]
    pub class_name: Option<String>,
    #[serde(alias = "gap", alias = "gap_leader", alias = "diff", default)]
    pub gap: Option<serde_json::Value>,
    #[serde(alias = "interval", alias = "int", alias = "diff_prev", default)]
    pub interval: Option<serde_json::Value>,
    #[serde(alias = "stage_time", alias = "time", alias = "last_lap", default)]
    pub stage_time: Option<serde_json::Value>,
    #[serde(alias = "total_time", alias = "best_lap", alias = "overall_time", default)]
    pub total_time: Option<serde_json::Value>,
    #[serde(alias = "stages_completed", alias = "laps", default)]
    pub stages_completed: Option<u32>,
    #[serde(alias = "status", alias = "state", default)]
    pub status: Option<String>,
    #[serde(alias = "tire", alias = "tires", alias = "compound", default)]
    pub tire: Option<String>,
    #[serde(alias = "split1", alias = "s1", default)]
    pub split1: Option<String>,
    #[serde(alias = "split2", alias = "s2", default)]
    pub split2: Option<String>,
    #[serde(alias = "split3", alias = "s3", default)]
    pub split3: Option<String>,
}

/// Live timing provider for WRC and ERC.
pub struct WrcLiveProvider {
    client: reqwest::Client,
    feed_url: String,
    series_id: &'static str,
}

impl WrcLiveProvider {
    /// Create a new provider for WRC.
    pub fn new() -> Self {
        Self::new_with_series("wrc")
    }

    /// Create a new provider for a specific series.
    pub fn new_with_series(series_id: &'static str) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            feed_url: WRC_LIVE_TIMING_URL.to_string(),
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

impl Default for WrcLiveProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveProvider for WrcLiveProvider {
    async fn is_session_live(&self) -> Result<bool> {
        let feed: WrcLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to query WRC live feed")?
            .json()
            .await
            .context("Failed to parse WRC live feed JSON")?;

        if feed.entries.is_empty() {
            return Ok(false);
        }

        let flag = feed
            .stage
            .as_ref()
            .and_then(|s| s.status.as_deref())
            .or(feed.status.as_deref())
            .unwrap_or("Active");

        let is_finished = flag.eq_ignore_ascii_case("finished")
            || flag.eq_ignore_ascii_case("final")
            || flag.eq_ignore_ascii_case("complete");

        Ok(!is_finished)
    }

    async fn fetch_timing(&self) -> Result<LiveTimingData> {
        let feed: WrcLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to fetch WRC live feed")?
            .json()
            .await
            .context("Failed to parse WRC live feed JSON")?;

        Ok(parse_wrc_live_feed(&feed, self.series_id))
    }
}

/// Parse WRC live timing feed into standard LiveTimingData.
pub fn parse_wrc_live_feed(feed: &WrcLiveFeed, series_id: &str) -> LiveTimingData {
    let mut driver_entries = Vec::new();

    for e in &feed.entries {
        let pos = e.pos.unwrap_or(99);

        let driver_name = if let Some(ref d) = e.driver {
            if let Some(ref co) = e.codriver {
                format!("{} / {}", d, co)
            } else {
                d.clone()
            }
        } else {
            match (&e.first_name, &e.last_name) {
                (Some(f), Some(l)) => format!("{} {}", f, l),
                (Some(f), None) => f.clone(),
                (None, Some(l)) => l.clone(),
                (None, None) => "Unknown Driver".to_string(),
            }
        };

        let driver_code = if let Some(ref l) = e.last_name {
            Some(
                l.chars()
                    .filter(|ch| ch.is_alphabetic())
                    .take(3)
                    .collect::<String>()
                    .to_uppercase(),
            )
        } else if let Some(ref d) = e.driver {
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

        let driver_number = e
            .number
            .as_ref()
            .and_then(|n| n.parse::<u32>().ok());

        let team = if let Some(ref t) = e.team {
            if let Some(ref c) = e.car {
                format!("{} ({})", t, c)
            } else {
                t.clone()
            }
        } else {
            e.car.clone().unwrap_or_else(|| "WRC Team".to_string())
        };

        let gap = format_json_val_or_default(e.gap.as_ref(), if pos == 1 { "LEADER" } else { "-" });
        let interval = format_json_val_or_default(e.interval.as_ref(), if pos == 1 { "-" } else { "-" });

        let last_lap_str = format_json_val_opt(e.stage_time.as_ref());
        let best_lap_str = format_json_val_opt(e.total_time.as_ref());

        let tire = e.tire.as_ref().map(|comp| {
            let normalized = match comp.to_lowercase().as_str() {
                "soft" | "s" => "Soft",
                "medium" | "m" => "Medium",
                "hard" | "h" => "Hard",
                "wet" | "w" | "rain" => "Wet",
                "snow" | "studded" => "Snow",
                _ => comp.as_str(),
            };
            TireInfo {
                compound: normalized.to_string(),
                laps: e.stages_completed.unwrap_or(0),
                is_new: false,
            }
        });

        let pit_info = PitInfo {
            stops_count: 0,
            last_stop_lap: None,
            last_stop_duration_secs: None,
            in_pit: false,
        };

        let status = if let Some(ref st) = e.status {
            st.clone()
        } else {
            "On Stage".to_string()
        };

        let sectors = SectorTimes {
            s1_ms: None,
            s2_ms: None,
            s3_ms: None,
            s1_str: e.split1.clone(),
            s2_str: e.split2.clone(),
            s3_str: e.split3.clone(),
            s1_fastest: false,
            s2_fastest: false,
            s3_fastest: false,
        };

        driver_entries.push(LiveDriverEntry {
            position: pos,
            driver_number,
            driver_name,
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
            laps_completed: e.stages_completed.unwrap_or(0),
            status,
            current_position: None,
            fastest_lap: false,
        });
    }

    driver_entries.sort_by_key(|d| d.position);

    // Fastest stage time
    let mut min_stage_time: Option<String> = None;
    for d in &driver_entries {
        if let Some(ref st) = d.last_lap_time {
            if st != "-" && !st.is_empty() {
                if min_stage_time.as_ref().map_or(true, |m| st < m) {
                    min_stage_time = Some(st.clone());
                }
            }
        }
    }

    if let Some(ref min_stage) = min_stage_time {
        for d in &mut driver_entries {
            if d.last_lap_time.as_ref() == Some(min_stage) {
                d.fastest_lap = true;
                break;
            }
        }
    }

    let session_name = feed
        .stage
        .as_ref()
        .and_then(|s| s.name.clone())
        .unwrap_or_else(|| "Special Stage".to_string());

    let circuit_name = feed
        .rally
        .as_ref()
        .and_then(|r| r.name.clone())
        .unwrap_or_else(|| "Rally Event".to_string());

    let session_status = feed
        .stage
        .as_ref()
        .and_then(|s| s.status.clone())
        .or_else(|| feed.status.clone())
        .unwrap_or_else(|| "Live".to_string());

    let current_lap = feed
        .rally
        .as_ref()
        .and_then(|r| r.current_stage_number);

    let total_laps = feed
        .rally
        .as_ref()
        .and_then(|r| r.total_stages);

    let weather = feed.weather.as_ref().map(|w| WeatherInfo {
        air_temp_c: w.air_temp,
        track_temp_c: None,
        humidity_pct: None,
        wind_speed_kmh: None,
        wind_direction_deg: None,
        rainfall: w.rain.unwrap_or(false),
        rain_intensity: None,
        description: w.condition.clone(),
    });

    LiveTimingData {
        series_id: series_id.to_string(),
        session_name,
        event_name: circuit_name.clone(),
        circuit_name,
        total_laps,
        current_lap,
        time_remaining: None,
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
                    format!("+{:.1}s", f)
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
                let ms = (total_ms % 1000) / 100;
                if mins > 0 {
                    Some(format!("{}:{:02}.{}", mins, secs, ms))
                } else {
                    Some(format!("{}.{}", secs, ms))
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
    fn test_parse_wrc_live_feed_sample() {
        let sample_json = r#"{
            "rally": {
                "name": "Rally Sweden",
                "location": "Umea",
                "total_stages": 18,
                "current_stage_number": 7
            },
            "stage": {
                "name": "SS7 - Norrby 2",
                "distance_km": 12.54,
                "status": "Live",
                "surface": "Snow"
            },
            "weather": {
                "air_temp": -4.5,
                "condition": "Snow",
                "rain": false
            },
            "entries": [
                {
                    "pos": 1,
                    "number": "11",
                    "driver": "Thierry Neuville",
                    "codriver": "Martijn Wydaeghe",
                    "team": "Hyundai Shell Mobis WRT",
                    "car": "Hyundai i20 N Rally1",
                    "gap": "LEADER",
                    "interval": "-",
                    "stage_time": "5:43.2",
                    "total_time": "45:12.8",
                    "stages_completed": 7,
                    "tire": "Snow",
                    "split1": "1:45.2",
                    "split2": "3:30.1",
                    "split3": "4:55.4"
                },
                {
                    "pos": 2,
                    "number": "33",
                    "driver": "Elfyn Evans",
                    "codriver": "Scott Martin",
                    "team": "Toyota Gazoo Racing WRT",
                    "car": "Toyota GR Yaris Rally1",
                    "gap": "+2.4",
                    "interval": "+2.4",
                    "stage_time": "5:45.6",
                    "total_time": "45:15.2",
                    "stages_completed": 7,
                    "tire": "Snow",
                    "split1": "1:45.9",
                    "split2": "3:31.2",
                    "split3": "4:57.1"
                }
            ]
        }"#;

        let feed: WrcLiveFeed = serde_json::from_str(sample_json).expect("parse wrc feed");
        let timing = parse_wrc_live_feed(&feed, "wrc");

        assert_eq!(timing.series_id, "wrc");
        assert_eq!(timing.session_name, "SS7 - Norrby 2");
        assert_eq!(timing.circuit_name, "Rally Sweden");
        assert_eq!(timing.session_status, "Live");
        assert_eq!(timing.current_lap, Some(7));
        assert_eq!(timing.total_laps, Some(18));

        let weather = timing.weather.as_ref().expect("weather data");
        assert_eq!(weather.air_temp_c, Some(-4.5));
        assert_eq!(weather.description.as_deref(), Some("Snow"));

        assert_eq!(timing.drivers.len(), 2);
        let d1 = &timing.drivers[0];
        assert_eq!(d1.position, 1);
        assert_eq!(d1.driver_name, "Thierry Neuville / Martijn Wydaeghe");
        assert_eq!(d1.driver_number, Some(11));
        assert_eq!(d1.team_name, "Hyundai Shell Mobis WRT (Hyundai i20 N Rally1)");
        assert_eq!(d1.tire.as_ref().unwrap().compound, "Snow");
        assert!(d1.fastest_lap);
        assert_eq!(d1.sectors.s1_str.as_deref(), Some("1:45.2"));

        let d2 = &timing.drivers[1];
        assert_eq!(d2.position, 2);
        assert_eq!(d2.driver_name, "Elfyn Evans / Scott Martin");
        assert_eq!(d2.driver_code.as_deref(), Some("EVA"));
        assert_eq!(d2.driver_number, Some(33));
        assert_eq!(d2.gap_to_leader, "+2.4");
        assert!(!d2.fastest_lap);
    }
}
