use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use crate::live::event::{
    LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes, TireInfo, WeatherInfo,
};
use crate::live::LiveProvider;
use crate::scraper::fetcher::create_http_client;

/// Al Kamel Systems live timing feed root structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AlKamelLiveFeed {
    #[serde(alias = "session", alias = "session_info", alias = "head", default)]
    pub session: Option<AlKamelSessionInfo>,
    #[serde(alias = "weather", alias = "weather_info", default)]
    pub weather: Option<AlKamelWeatherInfo>,
    #[serde(alias = "cars", alias = "drivers", alias = "entries", alias = "records", default)]
    pub cars: Vec<AlKamelCarEntry>,
    #[serde(alias = "status", alias = "flag", alias = "session_status", default)]
    pub status: Option<String>,
    #[serde(alias = "track", alias = "circuit", default)]
    pub track: Option<String>,
}

/// Session metadata in Al Kamel live feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AlKamelSessionInfo {
    #[serde(alias = "name", alias = "session_name", default)]
    pub name: Option<String>,
    #[serde(alias = "track", alias = "track_name", alias = "circuit", default)]
    pub track: Option<String>,
    #[serde(alias = "status", alias = "flag", default)]
    pub status: Option<String>,
    #[serde(alias = "laps", alias = "lap_number", alias = "current_lap", default)]
    pub laps: Option<u32>,
    #[serde(alias = "total_laps", default)]
    pub total_laps: Option<u32>,
    #[serde(alias = "time_remaining", alias = "remaining", default)]
    pub time_remaining: Option<String>,
}

/// Weather data in Al Kamel live feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AlKamelWeatherInfo {
    #[serde(alias = "air_temp", alias = "air_temperature", alias = "temp_air", default)]
    pub air_temp: Option<f64>,
    #[serde(alias = "track_temp", alias = "track_temperature", alias = "temp_track", default)]
    pub track_temp: Option<f64>,
    #[serde(alias = "humidity", default)]
    pub humidity: Option<f64>,
    #[serde(alias = "wind_speed", default)]
    pub wind_speed: Option<f64>,
    #[serde(alias = "wind_direction", alias = "wind_dir", default)]
    pub wind_direction: Option<u32>,
    #[serde(alias = "rainfall", alias = "rain", default)]
    pub rainfall: Option<bool>,
}

/// Single car/driver entry in Al Kamel live timing feed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AlKamelCarEntry {
    #[serde(alias = "pos", alias = "position", alias = "rank", default)]
    pub pos: Option<u32>,
    #[serde(alias = "number", alias = "car_number", alias = "no", alias = "num", default)]
    pub number: Option<String>,
    #[serde(alias = "driver", alias = "driver_name", alias = "name", default)]
    pub driver: Option<String>,
    #[serde(alias = "first_name", default)]
    pub first_name: Option<String>,
    #[serde(alias = "last_name", default)]
    pub last_name: Option<String>,
    #[serde(alias = "team", alias = "team_name", alias = "competitor", default)]
    pub team: Option<String>,
    #[serde(alias = "class", alias = "category", alias = "group", default)]
    pub class_name: Option<String>,
    #[serde(alias = "gap", alias = "gap_leader", alias = "diff", default)]
    pub gap: Option<serde_json::Value>,
    #[serde(alias = "interval", alias = "int", alias = "diff_prev", default)]
    pub interval: Option<serde_json::Value>,
    #[serde(alias = "last_lap", alias = "last_lap_time", alias = "last_time", default)]
    pub last_lap: Option<serde_json::Value>,
    #[serde(alias = "best_lap", alias = "best_lap_time", alias = "fastest_lap", default)]
    pub best_lap: Option<serde_json::Value>,
    #[serde(alias = "laps", alias = "laps_completed", alias = "num_laps", default)]
    pub laps: Option<u32>,
    #[serde(alias = "pits", alias = "pit_stops", alias = "stops", default)]
    pub pits: Option<u32>,
    #[serde(alias = "last_pit", alias = "last_pit_lap", default)]
    pub last_pit: Option<u32>,
    #[serde(alias = "in_pit", alias = "pit", default)]
    pub in_pit: Option<bool>,
    #[serde(alias = "state", alias = "status", default)]
    pub state: Option<String>,
    #[serde(alias = "tire", alias = "tire_compound", alias = "compound", default)]
    pub tire: Option<String>,
    #[serde(alias = "tire_laps", alias = "stint_laps", default)]
    pub tire_laps: Option<u32>,
    #[serde(alias = "s1", alias = "sector1", default)]
    pub s1: Option<String>,
    #[serde(alias = "s2", alias = "sector2", default)]
    pub s2: Option<String>,
    #[serde(alias = "s3", alias = "sector3", default)]
    pub s3: Option<String>,
}

/// Shared live timing provider for Al Kamel Systems timed series (WEC, IMSA, Formula E, ELMS, 24H Series).
pub struct AlKamelLiveProvider {
    client: reqwest::Client,
    feed_url: String,
    series_id: &'static str,
}

impl AlKamelLiveProvider {
    /// Create a new provider for a specific series using its standard Al Kamel endpoint.
    pub fn new(series_id: &'static str) -> Self {
        let feed_url = match series_id {
            "imsa" => "https://livetiming.alkamelsystems.com/imsa/data.json".to_string(),
            "wec" => "https://fiawec.alkamelsystems.com/live/data.json".to_string(),
            "formula_e" => "https://fiaformulae.alkamelsystems.com/live/data.json".to_string(),
            "elms" => "https://elms.alkamelsystems.com/live/data.json".to_string(),
            "24h_series" => "https://24hseries.alkamelsystems.com/live/data.json".to_string(),
            other => format!("https://{}.alkamelsystems.com/live/data.json", other),
        };

        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            feed_url,
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

impl LiveProvider for AlKamelLiveProvider {
    async fn is_session_live(&self) -> Result<bool> {
        let feed: AlKamelLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to query Al Kamel live feed")?
            .json()
            .await
            .context("Failed to parse Al Kamel live feed JSON")?;

        if feed.cars.is_empty() {
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
            || flag.eq_ignore_ascii_case("complete");

        Ok(!is_finished)
    }

    async fn fetch_timing(&self) -> Result<LiveTimingData> {
        let feed: AlKamelLiveFeed = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to fetch Al Kamel live feed")?
            .json()
            .await
            .context("Failed to parse Al Kamel live feed JSON")?;

        Ok(parse_alkamel_live_feed(&feed, self.series_id))
    }
}

/// Parse Al Kamel live feed into standard LiveTimingData.
pub fn parse_alkamel_live_feed(feed: &AlKamelLiveFeed, series_id: &str) -> LiveTimingData {
    let mut driver_entries = Vec::new();

    for c in &feed.cars {
        let pos = c.pos.unwrap_or(99);

        let name = if let Some(ref d) = c.driver {
            d.clone()
        } else {
            match (&c.first_name, &c.last_name) {
                (Some(f), Some(l)) => format!("{} {}", f, l),
                (Some(f), None) => f.clone(),
                (None, Some(l)) => l.clone(),
                (None, None) => "Unknown Driver".to_string(),
            }
        };

        let driver_code = c.last_name.as_ref().or(c.driver.as_ref()).map(|l| {
            l.chars()
                .filter(|ch| ch.is_alphabetic())
                .take(3)
                .collect::<String>()
                .to_uppercase()
        });

        let driver_number = c
            .number
            .as_ref()
            .and_then(|n| n.parse::<u32>().ok());

        let team = if let Some(ref t) = c.team {
            if let Some(ref cls) = c.class_name {
                format!("{} ({})", t, cls)
            } else {
                t.clone()
            }
        } else {
            c.class_name.clone().unwrap_or_else(|| "Racing Team".to_string())
        };

        let gap = format_json_val_or_default(c.gap.as_ref(), if pos == 1 { "LEADER" } else { "-" });
        let interval = format_json_val_or_default(c.interval.as_ref(), if pos == 1 { "-" } else { "-" });

        let last_lap_str = format_json_val_opt(c.last_lap.as_ref());
        let best_lap_str = format_json_val_opt(c.best_lap.as_ref());

        let tire = c.tire.as_ref().map(|comp| {
            let normalized_comp = match comp.to_lowercase().as_str() {
                "soft" | "s" | "option" | "red" => "Soft",
                "medium" | "m" | "prime" | "yellow" => "Medium",
                "hard" | "h" | "white" => "Hard",
                "wet" | "w" | "rain" => "Wet",
                "intermediate" | "inter" | "i" => "Intermediate",
                _ => comp.as_str(),
            };
            TireInfo {
                compound: normalized_comp.to_string(),
                laps: c.tire_laps.unwrap_or(0),
                is_new: false,
            }
        });

        let in_pit = c.in_pit == Some(true) || c.state.as_deref().map_or(false, |s| s.eq_ignore_ascii_case("pit"));

        let pit_info = PitInfo {
            stops_count: c.pits.unwrap_or(0),
            last_stop_lap: c.last_pit,
            last_stop_duration_secs: None,
            in_pit,
        };

        let status = if in_pit {
            "In Pit".to_string()
        } else if let Some(ref st) = c.state {
            st.clone()
        } else {
            "On Track".to_string()
        };

        let sectors = SectorTimes {
            s1_ms: None,
            s2_ms: None,
            s3_ms: None,
            s1_str: c.s1.clone(),
            s2_str: c.s2.clone(),
            s3_str: c.s3.clone(),
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
            laps_completed: c.laps.unwrap_or(0),
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
        .and_then(|s| s.track.clone())
        .or_else(|| feed.track.clone())
        .unwrap_or_else(|| "Circuit".to_string());

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
        rainfall: w.rainfall.unwrap_or(false),
        rain_intensity: None,
        description: None,
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
    fn test_parse_alkamel_live_feed_sample() {
        let sample_json = r#"{
            "session": {
                "name": "6 Hours of Spa-Francorchamps - Race",
                "track": "Circuit de Spa-Francorchamps",
                "status": "Green",
                "laps": 110,
                "total_laps": 160,
                "time_remaining": "01:45:00"
            },
            "weather": {
                "air_temp": 18.5,
                "track_temp": 24.2,
                "humidity": 62.0,
                "wind_speed": 4.1,
                "rainfall": false
            },
            "cars": [
                {
                    "pos": 1,
                    "number": "6",
                    "driver": "Kevin Estre",
                    "team": "Porsche Penske Motorsport",
                    "class": "Hypercar",
                    "gap": "LEADER",
                    "interval": "-",
                    "last_lap": "2:04.123",
                    "best_lap": "2:02.890",
                    "laps": 110,
                    "pits": 4,
                    "last_pit": 90,
                    "in_pit": false,
                    "tire": "Hard",
                    "tire_laps": 20,
                    "s1": "38.123",
                    "s2": "54.234",
                    "s3": "30.533"
                },
                {
                    "pos": 2,
                    "number": "7",
                    "first_name": "Kamui",
                    "last_name": "Kobayashi",
                    "team": "Toyota Gazoo Racing",
                    "class": "Hypercar",
                    "gap": "+4.321",
                    "interval": "+4.321",
                    "last_lap": "2:04.300",
                    "best_lap": "2:03.100",
                    "laps": 110,
                    "pits": 4,
                    "last_pit": 88,
                    "in_pit": false,
                    "tire": "Medium",
                    "tire_laps": 22,
                    "s1": "38.200",
                    "s2": "54.400",
                    "s3": "30.500"
                }
            ]
        }"#;

        let feed: AlKamelLiveFeed = serde_json::from_str(sample_json).expect("parse alkamel feed");
        let timing = parse_alkamel_live_feed(&feed, "wec");

        assert_eq!(timing.series_id, "wec");
        assert_eq!(timing.session_name, "6 Hours of Spa-Francorchamps - Race");
        assert_eq!(timing.circuit_name, "Circuit de Spa-Francorchamps");
        assert_eq!(timing.session_status, "Green");
        assert_eq!(timing.current_lap, Some(110));
        assert_eq!(timing.total_laps, Some(160));
        assert_eq!(timing.time_remaining.as_deref(), Some("01:45:00"));

        let weather = timing.weather.as_ref().expect("weather data");
        assert_eq!(weather.air_temp_c, Some(18.5));
        assert_eq!(weather.track_temp_c, Some(24.2));
        assert_eq!(weather.humidity_pct, Some(62.0));
        assert!(!weather.rainfall);

        assert_eq!(timing.drivers.len(), 2);
        let d1 = &timing.drivers[0];
        assert_eq!(d1.position, 1);
        assert_eq!(d1.driver_name, "Kevin Estre");
        assert_eq!(d1.driver_number, Some(6));
        assert_eq!(d1.team_name, "Porsche Penske Motorsport (Hypercar)");
        assert_eq!(d1.tire.as_ref().unwrap().compound, "Hard");
        assert_eq!(d1.tire.as_ref().unwrap().laps, 20);
        assert!(d1.fastest_lap);

        let d2 = &timing.drivers[1];
        assert_eq!(d2.position, 2);
        assert_eq!(d2.driver_name, "Kamui Kobayashi");
        assert_eq!(d2.driver_code.as_deref(), Some("KOB"));
        assert_eq!(d2.driver_number, Some(7));
        assert_eq!(d2.team_name, "Toyota Gazoo Racing (Hypercar)");
        assert_eq!(d2.tire.as_ref().unwrap().compound, "Medium");
        assert_eq!(d2.gap_to_leader, "+4.321");
        assert!(!d2.fastest_lap);
    }
}
