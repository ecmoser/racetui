use racetui::live::event::{LiveEvent, LiveTimingData};
use racetui::live::get_live_provider;
use racetui::scraper::alkamelsystems_live::{parse_alkamel_live_feed, AlKamelLiveFeed};
use racetui::scraper::indycar_live::{parse_indycar_live_feed, IndyCarLiveFeed};
use racetui::scraper::motogp_live::{parse_motogp_live_feed, MotoGPLiveFeed};
use racetui::scraper::nascar_live::{parse_nascar_live_feed, NascarLiveFeed};
use racetui::scraper::wrc_live::{parse_wrc_live_feed, WrcLiveFeed};
use std::fs;
use tokio::sync::mpsc;

#[test]
fn test_mock_nascar_live_timing_parsing() {
    let json_data = fs::read_to_string("tests/fixtures/nascar_live_timing.json")
        .expect("Failed to read nascar_live_timing fixture");
    let feed: NascarLiveFeed = serde_json::from_str(&json_data).expect("deserialize nascar live feed");
    let timing = parse_nascar_live_feed(&feed, "nascar_cup");

    assert_eq!(timing.series_id, "nascar_cup");
    assert_eq!(timing.session_name, "Daytona 500");
    assert_eq!(timing.circuit_name, "Daytona International Speedway");
    assert_eq!(timing.session_status, "Green");
    assert_eq!(timing.current_lap, Some(150));
    assert_eq!(timing.drivers.len(), 2);

    let leader = &timing.drivers[0];
    assert_eq!(leader.position, 1);
    assert_eq!(leader.driver_name, "Kyle Larson");
    assert_eq!(leader.driver_number, Some(5));
    assert_eq!(leader.gap_to_leader, "LEADER");
    assert_eq!(leader.team_name, "Chevrolet");
    assert_eq!(leader.pits.stops_count, 3);
    assert_eq!(leader.pits.last_stop_lap, Some(110));
    assert!(leader.fastest_lap);

    let p2 = &timing.drivers[1];
    assert_eq!(p2.position, 2);
    assert_eq!(p2.driver_name, "Chase Elliott");
    assert_eq!(p2.driver_number, Some(9));
    assert_eq!(p2.gap_to_leader, "+0.245s");
    assert!(!p2.fastest_lap);
}

#[test]
fn test_mock_indycar_live_timing_parsing() {
    let json_data = fs::read_to_string("tests/fixtures/indycar_live_timing.json")
        .expect("Failed to read indycar_live_timing fixture");
    let feed: IndyCarLiveFeed = serde_json::from_str(&json_data).expect("deserialize indycar live feed");
    let timing = parse_indycar_live_feed(&feed, "indycar");

    assert_eq!(timing.series_id, "indycar");
    assert_eq!(timing.session_name, "Indianapolis 500 - Race");
    assert_eq!(timing.circuit_name, "Indianapolis Motor Speedway");
    assert_eq!(timing.session_status, "Green");
    assert_eq!(timing.current_lap, Some(160));
    assert_eq!(timing.total_laps, Some(200));
    assert_eq!(timing.time_remaining.as_deref(), Some("00:35:00"));
    assert_eq!(timing.drivers.len(), 2);

    let leader = &timing.drivers[0];
    assert_eq!(leader.position, 1);
    assert_eq!(leader.driver_name, "Alex Palou");
    assert_eq!(leader.driver_number, Some(10));
    assert_eq!(leader.team_name, "Chip Ganassi Racing");
    assert_eq!(leader.gap_to_leader, "LEADER");
    assert_eq!(leader.tire.as_ref().unwrap().compound, "Primary");
    assert_eq!(leader.tire.as_ref().unwrap().laps, 30);
    assert_eq!(leader.sectors.s1_str.as_deref(), Some("12.800"));
    assert!(leader.fastest_lap);

    let p2 = &timing.drivers[1];
    assert_eq!(p2.position, 2);
    assert_eq!(p2.driver_name, "Pato O'Ward");
    assert_eq!(p2.driver_number, Some(5));
    assert_eq!(p2.team_name, "Arrow McLaren");
    assert_eq!(p2.gap_to_leader, "+0.185");
    assert_eq!(p2.tire.as_ref().unwrap().compound, "Alternate");
    assert_eq!(p2.tire.as_ref().unwrap().laps, 32);
    assert!(!p2.fastest_lap);
}

#[test]
fn test_mock_alkamel_wec_live_timing_parsing() {
    let json_data = fs::read_to_string("tests/fixtures/wec_live_timing.json")
        .expect("Failed to read wec_live_timing fixture");
    let feed: AlKamelLiveFeed = serde_json::from_str(&json_data).expect("deserialize alkamel wec live feed");
    let timing = parse_alkamel_live_feed(&feed, "wec");

    assert_eq!(timing.series_id, "wec");
    assert_eq!(timing.session_name, "24 Hours of Le Mans - Race");
    assert_eq!(timing.circuit_name, "Circuit de la Sarthe");
    assert_eq!(timing.session_status, "Green");
    assert_eq!(timing.current_lap, Some(250));
    assert_eq!(timing.total_laps, Some(380));
    assert_eq!(timing.time_remaining.as_deref(), Some("08:12:30"));

    let weather = timing.weather.as_ref().expect("weather data");
    assert_eq!(weather.air_temp_c, Some(19.5));
    assert_eq!(weather.track_temp_c, Some(26.0));
    assert_eq!(weather.humidity_pct, Some(65.0));
    assert_eq!(weather.wind_speed_kmh, Some(5.5));
    assert!(!weather.rainfall);

    assert_eq!(timing.drivers.len(), 2);
    let d1 = &timing.drivers[0];
    assert_eq!(d1.position, 1);
    assert_eq!(d1.driver_name, "Antonio Fuoco");
    assert_eq!(d1.driver_number, Some(50));
    assert_eq!(d1.team_name, "Ferrari AF Corse (Hypercar)");
    assert_eq!(d1.gap_to_leader, "LEADER");
    assert_eq!(d1.tire.as_ref().unwrap().compound, "Hard");
    assert_eq!(d1.tire.as_ref().unwrap().laps, 12);
    assert_eq!(d1.pits.stops_count, 18);
    assert_eq!(d1.pits.last_stop_lap, Some(238));
    assert!(d1.fastest_lap);

    let d2 = &timing.drivers[1];
    assert_eq!(d2.position, 2);
    assert_eq!(d2.driver_name, "Kevin Estre");
    assert_eq!(d2.driver_number, Some(6));
    assert_eq!(d2.team_name, "Porsche Penske Motorsport (Hypercar)");
    assert_eq!(d2.gap_to_leader, "+12.450");
    assert_eq!(d2.tire.as_ref().unwrap().compound, "Medium");
    assert_eq!(d2.tire.as_ref().unwrap().laps, 14);
    assert!(!d2.fastest_lap);
}

#[test]
fn test_mock_motogp_live_timing_parsing() {
    let json_data = fs::read_to_string("tests/fixtures/motogp_live_timing.json")
        .expect("Failed to read motogp_live_timing fixture");
    let feed: MotoGPLiveFeed = serde_json::from_str(&json_data).expect("deserialize motogp live feed");
    let timing = parse_motogp_live_feed(&feed, "motogp");

    assert_eq!(timing.series_id, "motogp");
    assert_eq!(timing.session_name, "MotoGP Grand Prix of the Americas - Race");
    assert_eq!(timing.circuit_name, "Circuit of the Americas");
    assert_eq!(timing.session_status, "Green");
    assert_eq!(timing.current_lap, Some(18));
    assert_eq!(timing.total_laps, Some(20));

    let weather = timing.weather.as_ref().expect("weather data");
    assert_eq!(weather.air_temp_c, Some(24.0));
    assert_eq!(weather.track_temp_c, Some(36.5));
    assert_eq!(weather.humidity_pct, Some(52.0));
    assert_eq!(weather.wind_speed_kmh, Some(6.5));
    assert!(!weather.rainfall);
    assert_eq!(weather.description.as_deref(), Some("Dry"));

    assert_eq!(timing.drivers.len(), 2);
    let r1 = &timing.drivers[0];
    assert_eq!(r1.position, 1);
    assert_eq!(r1.driver_name, "Maverick Vinales");
    assert_eq!(r1.driver_number, Some(12));
    assert_eq!(r1.team_name, "Aprilia Racing (Aprilia)");
    assert_eq!(r1.gap_to_leader, "LEADER");
    assert_eq!(r1.tire.as_ref().unwrap().compound, "Medium");
    assert_eq!(r1.sectors.s1_str.as_deref(), Some("30.120"));
    assert!(r1.fastest_lap);

    let r2 = &timing.drivers[1];
    assert_eq!(r2.position, 2);
    assert_eq!(r2.driver_name, "Marc Marquez");
    assert_eq!(r2.driver_code.as_deref(), Some("MAR"));
    assert_eq!(r2.driver_number, Some(93));
    assert_eq!(r2.team_name, "Gresini Racing MotoGP (Ducati)");
    assert_eq!(r2.tire.as_ref().unwrap().compound, "Soft");
    assert_eq!(r2.gap_to_leader, "+1.728");
    assert!(!r2.fastest_lap);
}

#[test]
fn test_mock_wrc_live_timing_parsing() {
    let json_data = fs::read_to_string("tests/fixtures/wrc_live_timing.json")
        .expect("Failed to read wrc_live_timing fixture");
    let feed: WrcLiveFeed = serde_json::from_str(&json_data).expect("deserialize wrc live feed");
    let timing = parse_wrc_live_feed(&feed, "wrc");

    assert_eq!(timing.series_id, "wrc");
    assert_eq!(timing.session_name, "SS12 - Monte Lerno 2");
    assert_eq!(timing.circuit_name, "Rally Italia Sardegna");
    assert_eq!(timing.session_status, "Live");
    assert_eq!(timing.current_lap, Some(12));
    assert_eq!(timing.total_laps, Some(16));

    let weather = timing.weather.as_ref().expect("weather data");
    assert_eq!(weather.air_temp_c, Some(28.5));
    assert_eq!(weather.description.as_deref(), Some("Sunny"));
    assert!(!weather.rainfall);

    assert_eq!(timing.drivers.len(), 2);
    let d1 = &timing.drivers[0];
    assert_eq!(d1.position, 1);
    assert_eq!(d1.driver_name, "Sebastien Ogier / Vincent Landais");
    assert_eq!(d1.driver_number, Some(17));
    assert_eq!(d1.team_name, "Toyota Gazoo Racing WRT (Toyota GR Yaris Rally1)");
    assert_eq!(d1.gap_to_leader, "LEADER");
    assert_eq!(d1.last_lap_time.as_deref(), Some("14:52.3"));
    assert_eq!(d1.best_lap_time.as_deref(), Some("2:10:45.0"));
    assert_eq!(d1.tire.as_ref().unwrap().compound, "Hard");
    assert_eq!(d1.sectors.s1_str.as_deref(), Some("4:12.1"));
    assert!(d1.fastest_lap);

    let d2 = &timing.drivers[1];
    assert_eq!(d2.position, 2);
    assert_eq!(d2.driver_name, "Ott Tanak / Martin Jarveoja");
    assert_eq!(d2.driver_number, Some(8));
    assert_eq!(d2.team_name, "Hyundai Shell Mobis WRT (Hyundai i20 N Rally1)");
    assert_eq!(d2.gap_to_leader, "+3.1");
    assert_eq!(d2.last_lap_time.as_deref(), Some("14:55.4"));
    assert_eq!(d2.best_lap_time.as_deref(), Some("2:10:48.1"));
    assert!(!d2.fastest_lap);
}

#[tokio::test]
async fn test_mock_live_timing_serialization_and_event_channel() {
    let json_data = fs::read_to_string("tests/fixtures/indycar_live_timing.json")
        .expect("Failed to read indycar fixture");
    let feed: IndyCarLiveFeed = serde_json::from_str(&json_data).unwrap();
    let timing = parse_indycar_live_feed(&feed, "indycar");

    // Test serialization roundtrip
    let serialized = serde_json::to_string(&timing).expect("serialize LiveTimingData");
    let deserialized: LiveTimingData = serde_json::from_str(&serialized).expect("deserialize LiveTimingData");
    assert_eq!(deserialized.series_id, "indycar");
    assert_eq!(deserialized.drivers.len(), 2);
    assert_eq!(deserialized.drivers[0].driver_name, "Alex Palou");

    // Test live event channel dispatch
    let (tx, mut rx) = mpsc::unbounded_channel::<LiveEvent>();
    let event = LiveEvent::TimingUpdate {
        series_id: "indycar".to_string(),
        data: Box::new(deserialized),
    };

    assert!(tx.send(event).is_ok());

    let received = rx.recv().await.expect("receive live event");
    match received {
        LiveEvent::TimingUpdate { series_id, data } => {
            assert_eq!(series_id, "indycar");
            assert_eq!(data.circuit_name, "Indianapolis Motor Speedway");
            assert_eq!(data.drivers[0].position, 1);
        }
        _ => panic!("Expected TimingUpdate event"),
    }

    // Verify all Phase 2 providers are registered in get_live_provider
    for series in &[
        "f1", "nascar_cup", "nascar_xfinity", "nascar_trucks", "arca",
        "indycar", "indy_nxt", "imsa", "wec", "formula_e", "elms", "24h_series",
        "motogp", "moto2", "moto3", "worldsbk", "wrc", "erc",
    ] {
        assert!(
            get_live_provider(series).is_some(),
            "Expected live provider for series '{}'",
            series
        );
    }
}
