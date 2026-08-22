use chrono::{Duration, Utc};
use racetui::config::{DaemonConfig, UserConfig};
use racetui::data::models::{EventStatus, RaceEvent, Session, SessionType};
use racetui::data::results::{DriverResult, RaceResults};
use racetui::data::standings::{ConstructorStanding, DriverStanding, SeasonStandings};
use racetui::live::event::{DriverPosition, LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes};
use racetui::live::track_map::{parse_multiviewer_circuit_json, TrackMapData, TrackPoint};
use racetui::notify::scheduler::NotificationTracker;
use racetui::notify::terminal::{format_osc777, format_osc9};
use racetui::notify::NotificationPayload;
use std::collections::HashSet;

#[test]
fn test_track_map_geometry_and_bounds() {
    let raw_json = r#"{
        "circuit_key": 63,
        "circuit_name": "Bahrain International Circuit",
        "x": [0.0, 100.0, 200.0, 100.0],
        "y": [0.0, 50.0, 0.0, -50.0],
        "rotation": 0.0
    }"#;

    let points = parse_multiviewer_circuit_json(raw_json).expect("parsed points");
    assert_eq!(points.len(), 4);

    let map_data = TrackMapData::new("Bahrain", points, vec![]);
    let (x_bounds, y_bounds) = map_data.bounds().expect("valid bounds");
    assert!(x_bounds[0] < 0.0);
    assert!(x_bounds[1] > 200.0);
    assert!(y_bounds[0] < -50.0);
    assert!(y_bounds[1] > 50.0);
}

#[test]
fn test_track_map_from_live_timing_positions() {
    let circuit_points = vec![
        TrackPoint { x: 0.0, y: 0.0 },
        TrackPoint { x: 100.0, y: 100.0 },
    ];

    let driver = LiveDriverEntry {
        position: 1,
        driver_number: Some(44),
        driver_name: "Lewis Hamilton".to_string(),
        driver_code: Some("HAM".to_string()),
        team_name: "Ferrari".to_string(),
        team_color: Some("#E8002D".to_string()),
        gap_to_leader: "LEADER".to_string(),
        interval: "-".to_string(),
        last_lap_time: None,
        best_lap_time: None,
        sectors: SectorTimes::default(),
        tire: None,
        pits: PitInfo::default(),
        laps_completed: 25,
        status: "On Track".to_string(),
        current_position: Some(DriverPosition {
            driver_number: Some(44),
            driver_code: Some("HAM".to_string()),
            x: 50.0,
            y: 50.0,
        }),
        fastest_lap: true,
    };

    let live_data = LiveTimingData {
        series_id: "f1".to_string(),
        session_name: "Race".to_string(),
        event_name: "British Grand Prix".to_string(),
        circuit_name: "Silverstone Circuit".to_string(),
        total_laps: Some(52),
        current_lap: Some(25),
        time_remaining: None,
        session_status: "Green".to_string(),
        drivers: vec![driver],
        weather: None,
        updated_at: Utc::now(),
    };

    let map_data =
        TrackMapData::from_live_timing("Silverstone Circuit", &circuit_points, &live_data);
    assert_eq!(map_data.circuit_name, "Silverstone Circuit");
    assert_eq!(map_data.points.len(), 2);
    assert_eq!(map_data.drivers.len(), 1);
    assert_eq!(map_data.drivers[0].driver_name, "Lewis Hamilton");
    assert_eq!(map_data.drivers[0].driver_code.as_deref(), Some("HAM"));
    assert_eq!(map_data.drivers[0].x, 50.0);
    assert_eq!(map_data.drivers[0].y, 50.0);
}

#[test]
fn test_terminal_notification_osc_formatting() {
    let osc777 = format_osc777("Race Alert", "Safety Car Deployed in Monaco Grand Prix");
    assert_eq!(
        osc777,
        "\x1b]777;notify;Race Alert;Safety Car Deployed in Monaco Grand Prix\x1b\\"
    );

    let osc9 = format_osc9("Race Alert", "Safety Car Deployed in Monaco Grand Prix");
    assert_eq!(
        osc9,
        "\x1b]9;Race Alert: Safety Car Deployed in Monaco Grand Prix\x1b\\"
    );
}

#[test]
fn test_notification_tracker_session_deduplication() {
    let mut tracker = NotificationTracker::new();
    let start_time = Utc::now() + Duration::minutes(15);

    let key = NotificationTracker::session_key("f1", Some(1), "Race", &start_time);
    assert!(key.contains("f1:1:Race"));

    assert!(!tracker.is_notified("f1", Some(1), "Race", &start_time));
    tracker.record("f1", Some(1), "Race", &start_time);
    assert!(tracker.is_notified("f1", Some(1), "Race", &start_time));
    assert_eq!(tracker.len(), 1);

    tracker.clear();
    assert_eq!(tracker.len(), 0);
    assert!(!tracker.is_notified("f1", Some(1), "Race", &start_time));
}

#[test]
fn test_daemon_config_parsing_and_defaults() {
    let toml_content = r#"
        notify_minutes_before = 15
        notify_session_types = ["race", "qualifying"]
        notify_series_filter = "favorites"
        notify_sound = true
        poll_interval_secs = 60
    "#;

    let config: DaemonConfig = toml::from_str(toml_content).expect("parse DaemonConfig");
    assert_eq!(config.notify_minutes_before, 15);
    assert_eq!(config.notify_session_types, vec!["race", "qualifying"]);
    assert_eq!(config.notify_series_filter, "favorites");
    assert!(config.notify_sound);
    assert_eq!(config.poll_interval_secs, 60);
}

#[test]
fn test_results_and_standings_serialization_roundtrips() {
    let result = DriverResult {
        position: Some(1),
        driver_name: "Marc Marquez".to_string(),
        driver_code: None,
        driver_number: Some(93),
        team: "Ducati Lenovo Team".to_string(),
        gap_to_leader: "".to_string(),
        gap_to_ahead: "".to_string(),
        grid_position: Some(2),
        points: 25.0,
        fastest_lap: true,
        penalty: None,
        status: "Finished".to_string(),
    };

    assert_eq!(result.positions_gained(), Some(1));

    let race_results = RaceResults {
        series_id: "motogp".to_string(),
        round: 1,
        event_name: "Qatar Grand Prix".to_string(),
        circuit_name: "Lusail International Circuit".to_string(),
        race_date: Utc::now().date_naive(),
        results: vec![result],
        fetched_at: Utc::now(),
    };

    let serialized_results = serde_json::to_string(&race_results).expect("serialize RaceResults");
    let deserialized_results: RaceResults =
        serde_json::from_str(&serialized_results).expect("deserialize RaceResults");
    assert_eq!(race_results, deserialized_results);

    let standings = SeasonStandings {
        series_id: "motogp".to_string(),
        season: 2026,
        drivers: vec![DriverStanding {
            position: 1,
            driver_name: "Marc Marquez".to_string(),
            driver_code: None,
            driver_number: Some(93),
            team: "Ducati Lenovo Team".to_string(),
            points: 545.0,
            wins: 11,
        }],
        constructors: vec![ConstructorStanding {
            position: 1,
            name: "Ducati".to_string(),
            points: 700.0,
            wins: 16,
        }],
        fetched_at: Utc::now(),
    };

    assert!(standings.has_constructor_standings());

    let serialized_standings = serde_json::to_string(&standings).expect("serialize Standings");
    let deserialized_standings: SeasonStandings =
        serde_json::from_str(&serialized_standings).expect("deserialize Standings");
    assert_eq!(standings, deserialized_standings);
}
