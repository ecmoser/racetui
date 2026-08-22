use serde::{Deserialize, Serialize};

use crate::live::track_map::TrackPoint;

/// Tire compound and stint information for a driver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TireInfo {
    /// Compound name (e.g., "Soft", "Medium", "Hard", "Intermediate", "Wet")
    pub compound: String,
    /// Total laps run on this set of tires
    pub laps: u32,
    /// Whether the tire set was brand new at stint start
    pub is_new: bool,
}

/// Pit stop information for a driver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PitInfo {
    /// Total number of pit stops made in the session
    pub stops_count: u32,
    /// Lap number of the most recent pit stop
    pub last_stop_lap: Option<u32>,
    /// Duration of the most recent pit stop in seconds (stationary or lane)
    pub last_stop_duration_secs: Option<f64>,
    /// Whether the driver is currently in the pit lane
    pub in_pit: bool,
}

/// Sector times for a driver's current or most recent lap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SectorTimes {
    /// Sector 1 duration in milliseconds
    pub s1_ms: Option<u32>,
    /// Sector 2 duration in milliseconds
    pub s2_ms: Option<u32>,
    /// Sector 3 duration in milliseconds
    pub s3_ms: Option<u32>,
    /// Sector 1 formatted display string (e.g. "28.123")
    pub s1_str: Option<String>,
    /// Sector 2 formatted display string (e.g. "32.456")
    pub s2_str: Option<String>,
    /// Sector 3 formatted display string (e.g. "25.789")
    pub s3_str: Option<String>,
    /// Whether sector 1 was personal best or session fastest
    pub s1_fastest: bool,
    /// Whether sector 2 was personal best or session fastest
    pub s2_fastest: bool,
    /// Whether sector 3 was personal best or session fastest
    pub s3_fastest: bool,
}

/// Driver coordinate on track (for track map positions).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverPosition {
    pub driver_number: Option<u32>,
    pub driver_code: Option<String>,
    pub x: f64,
    pub y: f64,
}

/// Live timing entry for a single driver on the leaderboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveDriverEntry {
    /// Current position on track / leaderboard (1-based)
    pub position: u32,
    /// Permanent or event driver number
    pub driver_number: Option<u32>,
    /// Full driver name
    pub driver_name: String,
    /// Driver code / abbreviation (e.g. "VER", "HAM")
    pub driver_code: Option<String>,
    /// Team name
    pub team_name: String,
    /// Team color hex code (e.g. "#3671C6")
    pub team_color: Option<String>,
    /// Gap to session leader (e.g. "LEADER", "+1.234", "+1 Lap")
    pub gap_to_leader: String,
    /// Interval to car ahead (e.g. "-", "+0.456")
    pub interval: String,
    /// Time of the most recent completed lap (e.g. "1:23.456")
    pub last_lap_time: Option<String>,
    /// Best lap time in the session (e.g. "1:22.987")
    pub best_lap_time: Option<String>,
    /// Sector times for recent lap
    pub sectors: SectorTimes,
    /// Current tire info
    pub tire: Option<TireInfo>,
    /// Pit stop info
    pub pits: PitInfo,
    /// Laps completed by this driver
    pub laps_completed: u32,
    /// Current status (e.g., "On Track", "In Pit", "OUT", "DNF", "Retired")
    pub status: String,
    /// Live GPS/coordinate position on circuit (if available)
    pub current_position: Option<DriverPosition>,
    /// Whether this driver currently holds the session fastest lap
    pub fastest_lap: bool,
}

/// Track and weather conditions for the live session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WeatherInfo {
    /// Ambient air temperature in Celsius
    pub air_temp_c: Option<f64>,
    /// Track surface temperature in Celsius
    pub track_temp_c: Option<f64>,
    /// Relative humidity percentage (0.0 to 100.0)
    pub humidity_pct: Option<f64>,
    /// Wind speed in km/h
    pub wind_speed_kmh: Option<f64>,
    /// Wind direction in degrees (0..=360)
    pub wind_direction_deg: Option<f64>,
    /// True if rainfall is detected
    pub rainfall: bool,
    /// Rain intensity description (e.g. "None", "Light", "Heavy")
    pub rain_intensity: Option<String>,
    /// Track condition description (e.g. "Dry", "Damp", "Wet")
    pub description: Option<String>,
}

/// Complete live timing data snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveTimingData {
    /// Series identifier (e.g. "f1", "nascar_cup")
    pub series_id: String,
    /// Session name (e.g. "Race", "Qualifying", "Practice 1")
    pub session_name: String,
    /// Event name (e.g. "Monaco Grand Prix")
    pub event_name: String,
    /// Circuit / Track name
    pub circuit_name: String,
    /// Total scheduled laps (if lap-based race)
    pub total_laps: Option<u32>,
    /// Current race leader lap (if lap-based race)
    pub current_lap: Option<u32>,
    /// Time remaining in timed session (e.g. "00:45:12")
    pub time_remaining: Option<String>,
    /// Session track status (e.g. "Green", "Yellow", "Red", "SC", "VSC", "Finished")
    pub session_status: String,
    /// Driver leaderboard ordered by position
    pub drivers: Vec<LiveDriverEntry>,
    /// Current weather info
    pub weather: Option<WeatherInfo>,
    /// Timestamp when this snapshot was fetched (UTC)
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Events dispatched by live timing background tasks.
#[derive(Debug, Clone)]
pub enum LiveEvent {
    /// Periodic live timing update for an active session
    TimingUpdate {
        series_id: String,
        data: Box<LiveTimingData>,
    },
    /// Circuit track map geometry loaded
    TrackGeometryLoaded {
        series_id: String,
        points: Vec<TrackPoint>,
    },
    /// Live session started or detected
    SessionStarted {
        series_id: String,
        session_name: String,
    },
    /// Live session ended
    SessionEnded { series_id: String },
    /// Error occurred during live timing fetch
    LiveError { series_id: String, error: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_live_timing_data_serialization_roundtrip() {
        let driver = LiveDriverEntry {
            position: 1,
            driver_number: Some(1),
            driver_name: "Max Verstappen".to_string(),
            driver_code: Some("VER".to_string()),
            team_name: "Red Bull Racing".to_string(),
            team_color: Some("#3671C6".to_string()),
            gap_to_leader: "LEADER".to_string(),
            interval: "-".to_string(),
            last_lap_time: Some("1:20.123".to_string()),
            best_lap_time: Some("1:19.876".to_string()),
            sectors: SectorTimes {
                s1_ms: Some(28120),
                s2_ms: Some(30450),
                s3_ms: Some(21553),
                s1_str: Some("28.120".to_string()),
                s2_str: Some("30.450".to_string()),
                s3_str: Some("21.553".to_string()),
                s1_fastest: true,
                s2_fastest: false,
                s3_fastest: true,
            },
            tire: Some(TireInfo {
                compound: "Soft".to_string(),
                laps: 12,
                is_new: true,
            }),
            pits: PitInfo {
                stops_count: 1,
                last_stop_lap: Some(18),
                last_stop_duration_secs: Some(2.4),
                in_pit: false,
            },
            laps_completed: 25,
            status: "On Track".to_string(),
            current_position: Some(DriverPosition {
                driver_number: Some(1),
                driver_code: Some("VER".to_string()),
                x: 100.0,
                y: 200.0,
            }),
            fastest_lap: true,
        };

        let weather = WeatherInfo {
            air_temp_c: Some(24.5),
            track_temp_c: Some(38.0),
            humidity_pct: Some(45.0),
            wind_speed_kmh: Some(10.5),
            wind_direction_deg: Some(180.0),
            rainfall: false,
            rain_intensity: None,
            description: Some("Sunny".to_string()),
        };

        let timing_data = LiveTimingData {
            series_id: "f1".to_string(),
            session_name: "Race".to_string(),
            event_name: "Monaco Grand Prix".to_string(),
            circuit_name: "Circuit de Monaco".to_string(),
            total_laps: Some(78),
            current_lap: Some(25),
            time_remaining: None,
            session_status: "Green".to_string(),
            drivers: vec![driver],
            weather: Some(weather),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&timing_data).expect("serialize live timing data");
        let deserialized: LiveTimingData =
            serde_json::from_str(&json).expect("deserialize live timing data");

        assert_eq!(timing_data.series_id, deserialized.series_id);
        assert_eq!(timing_data.session_name, deserialized.session_name);
        assert_eq!(timing_data.drivers.len(), deserialized.drivers.len());
        assert_eq!(
            timing_data.drivers[0].driver_name,
            deserialized.drivers[0].driver_name
        );
        assert_eq!(timing_data.drivers[0].tire, deserialized.drivers[0].tire);
        assert_eq!(timing_data.weather, deserialized.weather);
    }

    #[test]
    fn test_live_event_debug_and_clone() {
        let event = LiveEvent::SessionStarted {
            series_id: "f1".to_string(),
            session_name: "Qualifying".to_string(),
        };
        let cloned = event.clone();
        match cloned {
            LiveEvent::SessionStarted {
                series_id,
                session_name,
            } => {
                assert_eq!(series_id, "f1");
                assert_eq!(session_name, "Qualifying");
            }
            _ => panic!("unexpected event variant"),
        }
    }
}
