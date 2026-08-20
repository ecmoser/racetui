use chrono::NaiveDate;
use racetui::data::results::{
    read_qualifying_cache, read_results_cache, write_qualifying_cache, write_results_cache,
    DriverResult, QualifyingDriverResult, QualifyingResults, RaceResults,
};
use racetui::data::standings::{
    read_standings_cache, write_standings_cache, ConstructorStanding, DriverStanding,
    SeasonStandings,
};
use racetui::scraper::f1::{
    parse_jolpica_constructor_standings, parse_jolpica_driver_standings,
    parse_jolpica_qualifying_results, parse_jolpica_race_results,
};
use racetui::scraper::nascar::{parse_nascar_results, parse_nascar_standings};
use std::fs;

#[test]
fn test_mock_f1_race_results_parsing() {
    let json_data = fs::read_to_string("tests/fixtures/f1_race_results_2026_1.json")
        .expect("Failed to read f1_race_results fixture");
    let results =
        parse_jolpica_race_results(&json_data).expect("Failed to parse f1 race results fixture");

    assert_eq!(results.series_id, "f1");
    assert_eq!(results.round, 1);
    assert_eq!(results.event_name, "Bahrain Grand Prix");
    assert_eq!(results.results.len(), 2);

    let winner = &results.results[0];
    assert_eq!(winner.position, Some(1));
    assert_eq!(winner.driver_name, "George Russell");
    assert_eq!(winner.driver_code.as_deref(), Some("RUS"));
    assert_eq!(winner.driver_number, Some(63));
    assert_eq!(winner.team, "Mercedes");
    assert_eq!(winner.points, 25.0);
    assert!(winner.fastest_lap);
    assert_eq!(winner.gap_to_leader, "1:31:44.742");

    let p2 = &results.results[1];
    assert_eq!(p2.position, Some(2));
    assert_eq!(p2.driver_name, "Max Verstappen");
    assert_eq!(p2.team, "Red Bull Racing");
    assert_eq!(p2.points, 18.0);
    assert!(!p2.fastest_lap);
}

#[test]
fn test_mock_f1_qualifying_results_parsing() {
    let json_data = fs::read_to_string("tests/fixtures/f1_qualifying_results_2026_1.json")
        .expect("Failed to read f1_qualifying_results fixture");
    let results = parse_jolpica_qualifying_results(&json_data)
        .expect("Failed to parse f1 qualifying results fixture");

    assert_eq!(results.series_id, "f1");
    assert_eq!(results.round, 1);
    assert_eq!(results.results.len(), 2);

    let pole = &results.results[0];
    assert_eq!(pole.position, 1);
    assert_eq!(pole.driver_name, "George Russell");
    assert_eq!(pole.q1.as_deref(), Some("1:30.191"));
    assert_eq!(pole.q2.as_deref(), Some("1:29.479"));
    assert_eq!(pole.q3.as_deref(), Some("1:28.974"));

    let p2 = &results.results[1];
    assert_eq!(p2.position, 2);
    assert_eq!(p2.driver_name, "Lando Norris");
    assert_eq!(p2.q3.as_deref(), Some("1:29.041"));
}

#[test]
fn test_mock_f1_standings_parsing() {
    let driver_json = fs::read_to_string("tests/fixtures/f1_driver_standings_2026.json")
        .expect("Failed to read driver standings fixture");
    let constructor_json = fs::read_to_string("tests/fixtures/f1_constructor_standings_2026.json")
        .expect("Failed to read constructor standings fixture");

    let driver_standings = parse_jolpica_driver_standings(&driver_json)
        .expect("Failed to parse f1 driver standings fixture");
    let constructor_standings = parse_jolpica_constructor_standings(&constructor_json)
        .expect("Failed to parse f1 constructor standings fixture");

    assert_eq!(driver_standings.len(), 2);
    assert_eq!(driver_standings[0].driver_name, "George Russell");
    assert_eq!(driver_standings[0].points, 25.0);
    assert_eq!(driver_standings[0].wins, 1);

    assert_eq!(constructor_standings.len(), 2);
    assert_eq!(constructor_standings[0].name, "Mercedes");
    assert_eq!(constructor_standings[0].points, 40.0);
    assert_eq!(constructor_standings[0].wins, 1);
}

#[test]
fn test_mock_nascar_results_and_standings_parsing() {
    let results_json = fs::read_to_string("tests/fixtures/nascar_results.json")
        .expect("Failed to read nascar results fixture");
    let standings_json = fs::read_to_string("tests/fixtures/nascar_standings.json")
        .expect("Failed to read nascar standings fixture");

    let results = parse_nascar_results(&results_json, "nascar_cup", 1)
        .expect("Failed to parse nascar results");
    assert_eq!(results.results.len(), 2);
    assert_eq!(results.results[0].driver_name, "Kyle Larson");
    assert_eq!(results.results[0].position, Some(1));
    assert_eq!(results.results[0].grid_position, Some(4));
    assert_eq!(results.results[0].points, 50.0);

    let standings = parse_nascar_standings(&standings_json, "nascar_cup", 2026)
        .expect("Failed to parse nascar standings");
    assert_eq!(standings.drivers.len(), 2);
    assert_eq!(standings.drivers[0].driver_name, "Kyle Larson");
    assert_eq!(standings.drivers[0].points, 850.0);
    assert_eq!(standings.drivers[0].wins, 3);
}

#[test]
fn test_mock_serialization_roundtrips_and_caching() {
    // 1. Race results cache roundtrip
    let race_results = RaceResults {
        series_id: "mock_series".to_string(),
        round: 99,
        event_name: "Mock Grand Prix".to_string(),
        circuit_name: "Mock Circuit".to_string(),
        race_date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        results: vec![DriverResult {
            position: Some(1),
            driver_name: "Mock Racer".to_string(),
            driver_code: Some("MCK".to_string()),
            driver_number: Some(99),
            team: "Mock Racing".to_string(),
            grid_position: Some(2),
            gap_to_leader: "Leader".to_string(),
            gap_to_ahead: "".to_string(),
            penalty: None,
            status: "Finished".to_string(),
            points: 25.0,
            fastest_lap: true,
        }],
        fetched_at: chrono::Utc::now(),
    };

    let serialized = serde_json::to_string(&race_results).unwrap();
    let deserialized: RaceResults = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.event_name, "Mock Grand Prix");
    assert_eq!(deserialized.results[0].driver_name, "Mock Racer");

    assert!(write_results_cache(&race_results).is_ok());
    let cached_results = read_results_cache("mock_series", 99).unwrap();
    assert!(cached_results.is_some());
    assert_eq!(cached_results.unwrap().event_name, race_results.event_name);

    // 2. Qualifying results cache roundtrip
    let qual_results = QualifyingResults {
        series_id: "mock_series".to_string(),
        round: 99,
        event_name: "Mock Grand Prix".to_string(),
        circuit_name: "Mock Circuit".to_string(),
        race_date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        results: vec![QualifyingDriverResult {
            position: 1,
            driver_name: "Mock Racer".to_string(),
            driver_code: Some("MCK".to_string()),
            driver_number: Some(99),
            team: "Mock Racing".to_string(),
            q1: Some("1:15.000".to_string()),
            q2: Some("1:14.500".to_string()),
            q3: Some("1:14.000".to_string()),
        }],
        fetched_at: chrono::Utc::now(),
    };

    assert!(write_qualifying_cache(&qual_results).is_ok());
    let cached_qual = read_qualifying_cache("mock_series", 99).unwrap();
    assert!(cached_qual.is_some());
    assert_eq!(
        cached_qual.unwrap().results[0].q3.as_deref(),
        Some("1:14.000")
    );

    // 3. Season standings cache roundtrip
    let season_standings = SeasonStandings {
        series_id: "mock_series".to_string(),
        season: 2026,
        drivers: vec![DriverStanding {
            position: 1,
            driver_name: "Mock Racer".to_string(),
            driver_code: Some("MCK".to_string()),
            driver_number: Some(99),
            team: "Mock Racing".to_string(),
            points: 150.0,
            wins: 4,
        }],
        constructors: vec![ConstructorStanding {
            position: 1,
            name: "Mock Racing".to_string(),
            points: 200.0,
            wins: 5,
        }],
        fetched_at: chrono::Utc::now(),
    };

    assert!(write_standings_cache(&season_standings).is_ok());
    let cached_standings = read_standings_cache("mock_series", 2026).unwrap();
    assert!(cached_standings.is_some());
    let cs = cached_standings.unwrap();
    assert_eq!(cs.drivers[0].driver_name, "Mock Racer");
    assert_eq!(cs.constructors[0].name, "Mock Racing");
}
