use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

use crate::live::event::{
    DriverPosition, LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes, TireInfo, WeatherInfo,
};
use crate::live::LiveProvider;
use crate::scraper::fetcher::create_http_client;

const OPENF1_BASE_URL: &str = "https://api.openf1.org/v1";

/// OpenF1 API session response item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Session {
    pub session_key: u64,
    pub session_name: Option<String>,
    pub session_type: Option<String>,
    pub circuit_key: Option<u64>,
    pub circuit_short_name: Option<String>,
    pub country_name: Option<String>,
    pub location: Option<String>,
    pub date_start: Option<DateTime<Utc>>,
    pub date_end: Option<DateTime<Utc>>,
    pub year: Option<u32>,
}

/// OpenF1 API driver response item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Driver {
    pub driver_number: u32,
    pub broadcast_name: Option<String>,
    pub full_name: Option<String>,
    pub name_acronym: Option<String>,
    pub team_name: Option<String>,
    pub team_colour: Option<String>,
}

/// OpenF1 API position update item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Position {
    pub date: Option<DateTime<Utc>>,
    pub driver_number: u32,
    pub position: u32,
}

/// OpenF1 API interval update item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Interval {
    pub date: Option<DateTime<Utc>>,
    pub driver_number: u32,
    pub gap_to_leader: Option<serde_json::Value>,
    pub interval: Option<serde_json::Value>,
}

/// OpenF1 API lap item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Lap {
    pub date_start: Option<DateTime<Utc>>,
    pub driver_number: u32,
    pub lap_number: u32,
    pub lap_duration: Option<f64>,
    pub duration_sector_1: Option<f64>,
    pub duration_sector_2: Option<f64>,
    pub duration_sector_3: Option<f64>,
    pub is_pit_out_lap: Option<bool>,
}

/// OpenF1 API tire stint item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Stint {
    pub driver_number: u32,
    pub stint_number: Option<u32>,
    pub compound: Option<String>,
    pub lap_start: Option<u32>,
    pub lap_end: Option<u32>,
    pub tyre_age_at_start: Option<u32>,
}

/// OpenF1 API pit stop item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Pit {
    pub date: Option<DateTime<Utc>>,
    pub driver_number: u32,
    pub lap_number: Option<u32>,
    pub pit_duration: Option<f64>,
}

/// OpenF1 API weather observation item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Weather {
    pub date: Option<DateTime<Utc>>,
    pub air_temperature: Option<f64>,
    pub track_temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub pressure: Option<f64>,
    pub rainfall: Option<serde_json::Value>,
    pub wind_direction: Option<f64>,
    pub wind_speed: Option<f64>,
}

/// OpenF1 API car location item.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenF1Location {
    pub date: Option<DateTime<Utc>>,
    pub driver_number: u32,
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

/// Live timing provider for Formula 1 using the OpenF1 REST API.
pub struct F1LiveProvider {
    client: reqwest::Client,
    base_url: String,
}

impl F1LiveProvider {
    /// Create a new F1LiveProvider with default HTTP client.
    pub fn new() -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: OPENF1_BASE_URL.to_string(),
        }
    }

    /// Create an F1LiveProvider with custom base URL (useful for testing).
    pub fn with_base_url(base_url: String) -> Self {
        let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
        Self { client, base_url }
    }
}

impl Default for F1LiveProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveProvider for F1LiveProvider {
    async fn is_session_live(&self) -> Result<bool> {
        let url = format!("{}/sessions?session_key=latest", self.base_url);
        let sessions: Vec<OpenF1Session> = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to query OpenF1 sessions")?
            .json()
            .await
            .context("Failed to parse OpenF1 session response")?;

        let session = match sessions.into_iter().next() {
            Some(s) => s,
            None => return Ok(false),
        };

        let now = Utc::now();
        if let (Some(start), Some(end)) = (session.date_start, session.date_end) {
            // Consider session live from start until end + 15 minutes grace period
            let grace_end = end + chrono::Duration::minutes(15);
            Ok(now >= start && now <= grace_end)
        } else {
            Ok(false)
        }
    }

    async fn fetch_timing(&self) -> Result<LiveTimingData> {
        // 1. Fetch latest session metadata
        let session_url = format!("{}/sessions?session_key=latest", self.base_url);
        let sessions: Vec<OpenF1Session> = self
            .client
            .get(&session_url)
            .send()
            .await
            .context("Failed to query OpenF1 latest session")?
            .json()
            .await
            .context("Failed to parse OpenF1 latest session response")?;

        let session = sessions
            .into_iter()
            .next()
            .context("No session data returned by OpenF1")?;

        let session_key = session.session_key;

        // 2. Fetch all session data in parallel
        let drivers_fut = fetch_endpoint_opt::<OpenF1Driver>(self.client.clone(), format!("{}/drivers?session_key={}", self.base_url, session_key));
        let positions_fut = fetch_endpoint_opt::<OpenF1Position>(self.client.clone(), format!("{}/position?session_key={}", self.base_url, session_key));
        let intervals_fut = fetch_endpoint_opt::<OpenF1Interval>(self.client.clone(), format!("{}/intervals?session_key={}", self.base_url, session_key));
        let laps_fut = fetch_endpoint_opt::<OpenF1Lap>(self.client.clone(), format!("{}/laps?session_key={}", self.base_url, session_key));
        let stints_fut = fetch_endpoint_opt::<OpenF1Stint>(self.client.clone(), format!("{}/stints?session_key={}", self.base_url, session_key));
        let pits_fut = fetch_endpoint_opt::<OpenF1Pit>(self.client.clone(), format!("{}/pit?session_key={}", self.base_url, session_key));
        let weather_fut = fetch_endpoint_opt::<OpenF1Weather>(self.client.clone(), format!("{}/weather?session_key={}", self.base_url, session_key));
        let locations_fut = fetch_endpoint_opt::<OpenF1Location>(self.client.clone(), format!("{}/location?session_key={}", self.base_url, session_key));

        let (drivers, positions, intervals, laps, stints, pits, weather, locations) = tokio::join!(
            drivers_fut,
            positions_fut,
            intervals_fut,
            laps_fut,
            stints_fut,
            pits_fut,
            weather_fut,
            locations_fut
        );

        Ok(parse_openf1_data(
            &session,
            drivers,
            positions,
            intervals,
            laps,
            stints,
            pits,
            weather,
            locations,
        ))
    }
}

async fn fetch_endpoint_opt<T: serde::de::DeserializeOwned>(
    client: reqwest::Client,
    url: String,
) -> Vec<T> {
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json::<Vec<T>>().await.unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Parse and assemble OpenF1 data feeds into a unified LiveTimingData snapshot.
pub fn parse_openf1_data(
    session: &OpenF1Session,
    drivers: Vec<OpenF1Driver>,
    positions: Vec<OpenF1Position>,
    intervals: Vec<OpenF1Interval>,
    laps: Vec<OpenF1Lap>,
    stints: Vec<OpenF1Stint>,
    pits: Vec<OpenF1Pit>,
    weather: Vec<OpenF1Weather>,
    locations: Vec<OpenF1Location>,
) -> LiveTimingData {
    // 1. Group latest position per driver
    let mut latest_position: HashMap<u32, u32> = HashMap::new();
    let mut pos_sorted = positions;
    pos_sorted.sort_by_key(|p| p.date);
    for p in pos_sorted {
        latest_position.insert(p.driver_number, p.position);
    }

    // 2. Group latest interval per driver
    let mut latest_interval: HashMap<u32, OpenF1Interval> = HashMap::new();
    let mut int_sorted = intervals;
    int_sorted.sort_by_key(|i| i.date);
    for i in int_sorted {
        latest_interval.insert(i.driver_number, i);
    }

    // 3. Group laps per driver and find fastest lap / sectors
    let mut driver_laps: HashMap<u32, Vec<OpenF1Lap>> = HashMap::new();
    let mut best_lap_time_overall: Option<f64> = None;
    let mut best_s1_overall: Option<f64> = None;
    let mut best_s2_overall: Option<f64> = None;
    let mut best_s3_overall: Option<f64> = None;

    for lap in laps {
        if let Some(dur) = lap.lap_duration {
            if dur > 0.0 && best_lap_time_overall.map_or(true, |b| dur < b) {
                best_lap_time_overall = Some(dur);
            }
        }
        if let Some(s1) = lap.duration_sector_1 {
            if s1 > 0.0 && best_s1_overall.map_or(true, |b| s1 < b) {
                best_s1_overall = Some(s1);
            }
        }
        if let Some(s2) = lap.duration_sector_2 {
            if s2 > 0.0 && best_s2_overall.map_or(true, |b| s2 < b) {
                best_s2_overall = Some(s2);
            }
        }
        if let Some(s3) = lap.duration_sector_3 {
            if s3 > 0.0 && best_s3_overall.map_or(true, |b| s3 < b) {
                best_s3_overall = Some(s3);
            }
        }
        driver_laps.entry(lap.driver_number).or_default().push(lap);
    }

    // 4. Group stints per driver
    let mut latest_stint: HashMap<u32, OpenF1Stint> = HashMap::new();
    for stint in stints {
        let entry = latest_stint.entry(stint.driver_number).or_insert_with(|| stint.clone());
        if stint.stint_number.unwrap_or(0) >= entry.stint_number.unwrap_or(0) {
            *entry = stint;
        }
    }

    // 5. Group pits per driver
    let mut driver_pits: HashMap<u32, Vec<OpenF1Pit>> = HashMap::new();
    for pit in pits {
        driver_pits.entry(pit.driver_number).or_default().push(pit);
    }

    // 6. Group latest location per driver
    let mut latest_loc: HashMap<u32, OpenF1Location> = HashMap::new();
    let mut loc_sorted = locations;
    loc_sorted.sort_by_key(|l| l.date);
    for l in loc_sorted {
        latest_loc.insert(l.driver_number, l);
    }

    // 7. Assemble driver entries
    let mut driver_entries = Vec::new();
    let mut leader_laps = 0;

    for driver in drivers {
        let num = driver.driver_number;
        let pos = latest_position.get(&num).copied().unwrap_or(99);

        // Laps data
        let laps_list = driver_laps.get(&num);
        let completed_laps = laps_list.map(|l| l.len() as u32).unwrap_or(0);
        if pos == 1 && completed_laps > leader_laps {
            leader_laps = completed_laps;
        }

        let last_lap = laps_list.and_then(|l| l.iter().max_by_key(|lap| lap.lap_number));
        let best_lap_dur = laps_list.and_then(|l| {
            l.iter()
                .filter_map(|lap| lap.lap_duration)
                .filter(|&d| d > 0.0)
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        });

        let is_fastest_lap = match (best_lap_dur, best_lap_time_overall) {
            (Some(d), Some(overall)) => (d - overall).abs() < 0.0001,
            _ => false,
        };

        let sectors = if let Some(ll) = last_lap {
            SectorTimes {
                s1_ms: ll.duration_sector_1.map(|s| (s * 1000.0) as u32),
                s2_ms: ll.duration_sector_2.map(|s| (s * 1000.0) as u32),
                s3_ms: ll.duration_sector_3.map(|s| (s * 1000.0) as u32),
                s1_str: ll.duration_sector_1.map(format_sector_time),
                s2_str: ll.duration_sector_2.map(format_sector_time),
                s3_str: ll.duration_sector_3.map(format_sector_time),
                s1_fastest: ll.duration_sector_1.map_or(false, |s| {
                    best_s1_overall.map_or(false, |b| (s - b).abs() < 0.0001)
                }),
                s2_fastest: ll.duration_sector_2.map_or(false, |s| {
                    best_s2_overall.map_or(false, |b| (s - b).abs() < 0.0001)
                }),
                s3_fastest: ll.duration_sector_3.map_or(false, |s| {
                    best_s3_overall.map_or(false, |b| (s - b).abs() < 0.0001)
                }),
            }
        } else {
            SectorTimes::default()
        };

        // Gaps / Intervals
        let interval_item = latest_interval.get(&num);
        let gap_str = format_openf1_gap(interval_item.and_then(|i| i.gap_to_leader.as_ref()), pos == 1);
        let int_str = format_openf1_interval(interval_item.and_then(|i| i.interval.as_ref()), pos == 1);

        // Stint / Tire info
        let tire = latest_stint.get(&num).map(|s| {
            let compound = s.compound.as_deref().map(normalize_compound).unwrap_or_else(|| "Unknown".to_string());
            let stint_start_lap = s.lap_start.unwrap_or(1);
            let age_at_start = s.tyre_age_at_start.unwrap_or(0);
            let tire_laps = completed_laps.saturating_sub(stint_start_lap) + age_at_start + 1;
            TireInfo {
                compound,
                laps: tire_laps,
                is_new: age_at_start == 0,
            }
        });

        // Pit info
        let pits_list = driver_pits.get(&num);
        let stops_count = pits_list.map(|p| p.len() as u32).unwrap_or(0);
        let last_pit = pits_list.and_then(|p| p.iter().max_by_key(|pit| pit.lap_number));
        let in_pit = last_lap.map_or(false, |l| l.is_pit_out_lap == Some(true));

        let pit_info = PitInfo {
            stops_count,
            last_stop_lap: last_pit.and_then(|p| p.lap_number),
            last_stop_duration_secs: last_pit.and_then(|p| p.pit_duration),
            in_pit,
        };

        // GPS position on track
        let current_pos = latest_loc.get(&num).map(|loc| DriverPosition {
            driver_number: Some(num),
            driver_code: driver.name_acronym.clone(),
            x: loc.x,
            y: loc.y,
        });

        let status = if in_pit {
            "In Pit".to_string()
        } else {
            "On Track".to_string()
        };

        let team_color = driver.team_colour.map(|c| {
            if c.starts_with('#') {
                c
            } else {
                format!("#{}", c)
            }
        });

        driver_entries.push(LiveDriverEntry {
            position: pos,
            driver_number: Some(num),
            driver_name: driver.full_name.or(driver.broadcast_name).unwrap_or_else(|| format!("Driver {}", num)),
            driver_code: driver.name_acronym,
            team_name: driver.team_name.unwrap_or_else(|| "Unknown".to_string()),
            team_color,
            gap_to_leader: gap_str,
            interval: int_str,
            last_lap_time: last_lap.and_then(|l| l.lap_duration).map(format_lap_time),
            best_lap_time: best_lap_dur.map(format_lap_time),
            sectors,
            tire,
            pits: pit_info,
            laps_completed: completed_laps,
            status,
            current_position: current_pos,
            fastest_lap: is_fastest_lap,
        });
    }

    // Sort leaderboard by position
    driver_entries.sort_by_key(|d| d.position);

    // Weather
    let weather_info = weather.into_iter().max_by_key(|w| w.date).map(|w| {
        let is_rain = match &w.rainfall {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::Number(n)) => n.as_f64().map_or(false, |v| v > 0.0),
            _ => false,
        };
        let wind_kmh = w.wind_speed.map(|m_s| m_s * 3.6);
        let desc = if is_rain { "Rain" } else { "Dry" };
        WeatherInfo {
            air_temp_c: w.air_temperature,
            track_temp_c: w.track_temperature,
            humidity_pct: w.humidity,
            wind_speed_kmh: wind_kmh,
            wind_direction_deg: w.wind_direction,
            rainfall: is_rain,
            rain_intensity: None,
            description: Some(desc.to_string()),
        }
    });

    let circuit = session
        .circuit_short_name
        .clone()
        .or_else(|| session.location.clone())
        .unwrap_or_else(|| "Circuit".to_string());

    let event_name = session
        .country_name
        .as_ref()
        .map(|c| format!("{} Grand Prix", c))
        .unwrap_or_else(|| "Grand Prix".to_string());

    LiveTimingData {
        series_id: "f1".to_string(),
        session_name: session.session_name.clone().unwrap_or_else(|| "Session".to_string()),
        event_name,
        circuit_name: circuit,
        total_laps: None,
        current_lap: if leader_laps > 0 { Some(leader_laps) } else { None },
        time_remaining: None,
        session_status: "Green".to_string(),
        drivers: driver_entries,
        weather: weather_info,
        updated_at: Utc::now(),
    }
}

fn format_openf1_gap(val: Option<&serde_json::Value>, is_p1: bool) -> String {
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

fn format_openf1_interval(val: Option<&serde_json::Value>, is_p1: bool) -> String {
    if is_p1 {
        return "-".to_string();
    }
    match val {
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                if f == 0.0 {
                    "-".to_string()
                } else {
                    format!("+{:.3}s", f)
                }
            } else {
                "-".to_string()
            }
        }
        Some(serde_json::Value::String(s)) => {
            if s == "0" || s == "0.0" || s == "-" {
                "-".to_string()
            } else if !s.starts_with('+') && !s.contains("LAP") {
                format!("+{}", s)
            } else {
                s.clone()
            }
        }
        _ => "-".to_string(),
    }
}

fn format_lap_time(duration_secs: f64) -> String {
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

fn format_sector_time(sec: f64) -> String {
    if sec <= 0.0 {
        "-".to_string()
    } else {
        format!("{:.3}", sec)
    }
}

fn normalize_compound(compound: &str) -> String {
    match compound.trim().to_uppercase().as_str() {
        "SOFT" => "Soft".to_string(),
        "MEDIUM" => "Medium".to_string(),
        "HARD" => "Hard".to_string(),
        "INTERMEDIATE" => "Intermediate".to_string(),
        "WET" => "Wet".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_lap_time() {
        assert_eq!(format_lap_time(83.456), "1:23.456");
        assert_eq!(format_lap_time(59.123), "59.123");
        assert_eq!(format_lap_time(125.789), "2:05.789");
        assert_eq!(format_lap_time(0.0), "-");
    }

    #[test]
    fn test_normalize_compound() {
        assert_eq!(normalize_compound("SOFT"), "Soft");
        assert_eq!(normalize_compound("MEDIUM"), "Medium");
        assert_eq!(normalize_compound("HARD"), "Hard");
        assert_eq!(normalize_compound("INTERMEDIATE"), "Intermediate");
        assert_eq!(normalize_compound("WET"), "Wet");
        assert_eq!(normalize_compound("TEST"), "Test");
    }

    #[test]
    fn test_parse_openf1_data_assembly() {
        let session = OpenF1Session {
            session_key: 9472,
            session_name: Some("Race".to_string()),
            session_type: Some("Race".to_string()),
            circuit_key: Some(63),
            circuit_short_name: Some("Bahrain".to_string()),
            country_name: Some("Bahrain".to_string()),
            location: Some("Sakhir".to_string()),
            date_start: None,
            date_end: None,
            year: Some(2026),
        };

        let drivers = vec![
            OpenF1Driver {
                driver_number: 1,
                broadcast_name: Some("M VERSTAPPEN".to_string()),
                full_name: Some("Max Verstappen".to_string()),
                name_acronym: Some("VER".to_string()),
                team_name: Some("Red Bull Racing".to_string()),
                team_colour: Some("3671C6".to_string()),
            },
            OpenF1Driver {
                driver_number: 44,
                broadcast_name: Some("L HAMILTON".to_string()),
                full_name: Some("Lewis Hamilton".to_string()),
                name_acronym: Some("HAM".to_string()),
                team_name: Some("Ferrari".to_string()),
                team_colour: Some("E8002D".to_string()),
            },
        ];

        let positions = vec![
            OpenF1Position {
                date: None,
                driver_number: 1,
                position: 1,
            },
            OpenF1Position {
                date: None,
                driver_number: 44,
                position: 2,
            },
        ];

        let intervals = vec![
            OpenF1Interval {
                date: None,
                driver_number: 1,
                gap_to_leader: Some(serde_json::json!(0.0)),
                interval: Some(serde_json::json!(0.0)),
            },
            OpenF1Interval {
                date: None,
                driver_number: 44,
                gap_to_leader: Some(serde_json::json!(2.456)),
                interval: Some(serde_json::json!(2.456)),
            },
        ];

        let laps = vec![
            OpenF1Lap {
                date_start: None,
                driver_number: 1,
                lap_number: 1,
                lap_duration: Some(92.5),
                duration_sector_1: Some(29.1),
                duration_sector_2: Some(38.2),
                duration_sector_3: Some(25.2),
                is_pit_out_lap: Some(false),
            },
            OpenF1Lap {
                date_start: None,
                driver_number: 1,
                lap_number: 2,
                lap_duration: Some(90.1),
                duration_sector_1: Some(28.5),
                duration_sector_2: Some(37.1),
                duration_sector_3: Some(24.5),
                is_pit_out_lap: Some(false),
            },
            OpenF1Lap {
                date_start: None,
                driver_number: 44,
                lap_number: 1,
                lap_duration: Some(93.0),
                duration_sector_1: Some(29.5),
                duration_sector_2: Some(38.0),
                duration_sector_3: Some(25.5),
                is_pit_out_lap: Some(false),
            },
        ];

        let stints = vec![
            OpenF1Stint {
                driver_number: 1,
                stint_number: Some(1),
                compound: Some("MEDIUM".to_string()),
                lap_start: Some(1),
                lap_end: None,
                tyre_age_at_start: Some(0),
            },
            OpenF1Stint {
                driver_number: 44,
                stint_number: Some(1),
                compound: Some("SOFT".to_string()),
                lap_start: Some(1),
                lap_end: None,
                tyre_age_at_start: Some(3),
            },
        ];

        let pits = vec![];
        let weather = vec![OpenF1Weather {
            date: None,
            air_temperature: Some(26.0),
            track_temperature: Some(35.0),
            humidity: Some(40.0),
            pressure: Some(1013.0),
            rainfall: Some(serde_json::json!(0)),
            wind_direction: Some(90.0),
            wind_speed: Some(4.0),
        }];
        let locations = vec![];

        let timing = parse_openf1_data(
            &session, drivers, positions, intervals, laps, stints, pits, weather, locations,
        );

        assert_eq!(timing.series_id, "f1");
        assert_eq!(timing.event_name, "Bahrain Grand Prix");
        assert_eq!(timing.session_name, "Race");
        assert_eq!(timing.drivers.len(), 2);

        // Leader
        let p1 = &timing.drivers[0];
        assert_eq!(p1.position, 1);
        assert_eq!(p1.driver_name, "Max Verstappen");
        assert_eq!(p1.driver_code.as_deref(), Some("VER"));
        assert_eq!(p1.gap_to_leader, "LEADER");
        assert_eq!(p1.interval, "-");
        assert_eq!(p1.team_color.as_deref(), Some("#3671C6"));
        assert_eq!(p1.best_lap_time.as_deref(), Some("1:30.100"));
        assert!(p1.fastest_lap);
        assert_eq!(p1.tire.as_ref().unwrap().compound, "Medium");
        assert_eq!(p1.tire.as_ref().unwrap().laps, 2); // 2 completed - 1 start + 0 age + 1 = 2 laps on tire

        // P2
        let p2 = &timing.drivers[1];
        assert_eq!(p2.position, 2);
        assert_eq!(p2.driver_name, "Lewis Hamilton");
        assert_eq!(p2.gap_to_leader, "+2.456s");
        assert_eq!(p2.interval, "+2.456s");
        assert!(!p2.fastest_lap);
        assert_eq!(p2.tire.as_ref().unwrap().compound, "Soft");

        // Weather
        assert!(timing.weather.is_some());
        let w = timing.weather.unwrap();
        assert_eq!(w.air_temp_c, Some(26.0));
        assert_eq!(w.track_temp_c, Some(35.0));
        assert_eq!(w.rainfall, false);
    }
}
