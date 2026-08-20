use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Deserialize;

use super::fetcher;
use super::sportstimes::fetch_sportstimes_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::*;
use crate::data::results::{DriverResult, QualifyingDriverResult, QualifyingResults, RaceResults};
use crate::data::standings::{ConstructorStanding, DriverStanding, SeasonStandings};

pub fn f1_stream_links() -> Vec<StreamLink> {
    vec![StreamLink {
        platform: "F1TV".to_string(),
        url: "https://f1tv.formula1.com".to_string(),
        access: StreamAccess::Paid,
    }]
}

/// F1 scraper using sportstimes API / Jolpica API.
pub struct F1Scraper;

// --- Jolpica API response structs ---
// These match the JSON structure returned by the API.

#[derive(Debug, Deserialize)]
struct JolpicaResponse {
    #[serde(rename = "MRData")]
    mr_data: MRData,
}

#[derive(Debug, Deserialize)]
struct MRData {
    #[serde(rename = "RaceTable")]
    race_table: RaceTable,
}

#[derive(Debug, Deserialize)]
struct RaceTable {
    #[serde(rename = "Races")]
    races: Vec<JolpicaRace>,
}

#[derive(Debug, Deserialize)]
struct JolpicaRace {
    round: String,
    #[serde(rename = "raceName")]
    race_name: String,
    #[serde(rename = "Circuit")]
    circuit: JolpicaCircuit,
    date: String,
    time: Option<String>,
    #[serde(rename = "FirstPractice")]
    first_practice: Option<JolpicaSession>,
    #[serde(rename = "SecondPractice")]
    second_practice: Option<JolpicaSession>,
    #[serde(rename = "ThirdPractice")]
    third_practice: Option<JolpicaSession>,
    #[serde(rename = "Qualifying")]
    qualifying: Option<JolpicaSession>,
    #[serde(rename = "Sprint")]
    sprint: Option<JolpicaSession>,
    #[serde(rename = "SprintQualifying")]
    sprint_qualifying: Option<JolpicaSession>,
    // Newer API versions may use "SprintShootout" instead
    #[serde(rename = "SprintShootout")]
    sprint_shootout: Option<JolpicaSession>,
}

#[derive(Debug, Deserialize)]
struct JolpicaCircuit {
    #[serde(rename = "circuitName")]
    circuit_name: String,
    #[serde(rename = "Location")]
    location: JolpicaLocation,
}

#[derive(Debug, Deserialize)]
struct JolpicaLocation {
    locality: String,
    country: String,
}

#[derive(Debug, Deserialize)]
struct JolpicaSession {
    date: String,
    time: Option<String>,
}

// --- Jolpica Standings API response structs ---

#[derive(Debug, Deserialize)]
pub struct JolpicaStandingsResponse {
    #[serde(rename = "MRData")]
    pub mr_data: StandingsMRData,
}

#[derive(Debug, Deserialize)]
pub struct StandingsMRData {
    #[serde(rename = "StandingsTable")]
    pub standings_table: StandingsTable,
}

#[derive(Debug, Deserialize)]
pub struct StandingsTable {
    pub season: Option<String>,
    #[serde(rename = "StandingsLists", default)]
    pub standings_lists: Vec<StandingsList>,
}

#[derive(Debug, Deserialize)]
pub struct StandingsList {
    pub season: Option<String>,
    pub round: Option<String>,
    #[serde(rename = "DriverStandings", default)]
    pub driver_standings: Vec<JolpicaDriverStanding>,
    #[serde(rename = "ConstructorStandings", default)]
    pub constructor_standings: Vec<JolpicaConstructorStanding>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaDriverStanding {
    pub position: String,
    #[serde(rename = "positionText")]
    pub position_text: Option<String>,
    pub points: String,
    pub wins: String,
    #[serde(rename = "Driver")]
    pub driver: JolpicaDriver,
    #[serde(rename = "Constructors", default)]
    pub constructors: Vec<JolpicaConstructor>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaDriver {
    #[serde(rename = "driverId")]
    pub driver_id: String,
    #[serde(rename = "permanentNumber")]
    pub permanent_number: Option<String>,
    pub code: Option<String>,
    #[serde(rename = "givenName")]
    pub given_name: String,
    #[serde(rename = "familyName")]
    pub family_name: String,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaConstructor {
    #[serde(rename = "constructorId")]
    pub constructor_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaConstructorStanding {
    pub position: String,
    #[serde(rename = "positionText")]
    pub position_text: Option<String>,
    pub points: String,
    pub wins: String,
    #[serde(rename = "Constructor")]
    pub constructor: JolpicaConstructor,
}

// --- Jolpica Results API response structs ---

#[derive(Debug, Deserialize)]
pub struct JolpicaResultsResponse {
    #[serde(rename = "MRData")]
    pub mr_data: ResultsMRData,
}

#[derive(Debug, Deserialize)]
pub struct ResultsMRData {
    #[serde(rename = "RaceTable")]
    pub race_table: ResultsRaceTable,
}

#[derive(Debug, Deserialize)]
pub struct ResultsRaceTable {
    pub season: Option<String>,
    pub round: Option<String>,
    #[serde(rename = "Races", default)]
    pub races: Vec<JolpicaResultsRace>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaResultsRace {
    pub season: String,
    pub round: String,
    #[serde(rename = "raceName")]
    pub race_name: String,
    #[serde(rename = "Circuit")]
    pub circuit: JolpicaCircuit,
    pub date: String,
    pub time: Option<String>,
    #[serde(rename = "Results", alias = "SprintResults", default)]
    pub results: Vec<JolpicaRaceResultEntry>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaRaceResultEntry {
    pub number: String,
    pub position: String,
    #[serde(rename = "positionText")]
    pub position_text: Option<String>,
    pub points: String,
    #[serde(rename = "Driver")]
    pub driver: JolpicaDriver,
    #[serde(rename = "Constructor")]
    pub constructor: JolpicaConstructor,
    pub grid: String,
    pub laps: String,
    pub status: String,
    #[serde(rename = "Time")]
    pub time: Option<JolpicaTime>,
    #[serde(rename = "FastestLap")]
    pub fastest_lap: Option<JolpicaFastestLap>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaTime {
    pub millis: Option<String>,
    pub time: String,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaFastestLap {
    pub rank: Option<String>,
    pub lap: Option<String>,
    #[serde(rename = "Time")]
    pub time: Option<JolpicaFastestLapTime>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaFastestLapTime {
    pub time: String,
}

// --- Jolpica Qualifying API structs ---

#[derive(Debug, Deserialize)]
pub struct JolpicaQualifyingResponse {
    #[serde(rename = "MRData")]
    pub mr_data: JolpicaQualifyingMrData,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaQualifyingMrData {
    #[serde(rename = "RaceTable")]
    pub race_table: JolpicaQualifyingRaceTable,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaQualifyingRaceTable {
    pub season: Option<String>,
    pub round: Option<String>,
    #[serde(rename = "Races", default)]
    pub races: Vec<JolpicaQualifyingRace>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaQualifyingRace {
    pub season: String,
    pub round: String,
    #[serde(rename = "raceName")]
    pub race_name: String,
    #[serde(rename = "Circuit")]
    pub circuit: JolpicaCircuit,
    pub date: String,
    #[serde(rename = "QualifyingResults", default)]
    pub qualifying_results: Vec<JolpicaQualifyingResultEntry>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaQualifyingResultEntry {
    pub number: Option<String>,
    pub position: String,
    #[serde(rename = "Driver")]
    pub driver: JolpicaDriver,
    #[serde(rename = "Constructor")]
    pub constructor: JolpicaConstructor,
    #[serde(rename = "Q1")]
    pub q1: Option<String>,
    #[serde(rename = "Q2")]
    pub q2: Option<String>,
    #[serde(rename = "Q3")]
    pub q3: Option<String>,
}

// --- Helper functions ---

/// Parse a date string "YYYY-MM-DD" and optional time "HH:MM:SSZ" into a UTC DateTime.
fn parse_datetime(date: &str, time: Option<&str>) -> Option<chrono::DateTime<Utc>> {
    let naive_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    if let Some(time_str) = time {
        // Time format from Jolpica: "14:00:00Z"
        let cleaned = time_str.trim_end_matches('Z');
        let naive_time = NaiveTime::parse_from_str(cleaned, "%H:%M:%S").ok()?;
        let naive_dt = naive_date.and_time(naive_time);
        Some(Utc.from_utc_datetime(&naive_dt))
    } else {
        None
    }
}

/// Convert a JolpicaSession to a Session.
fn convert_session(name: &str, session_type: SessionType, jolpica: &JolpicaSession) -> Session {
    Session {
        name: name.to_string(),
        session_type,
        start_time: parse_datetime(&jolpica.date, jolpica.time.as_deref()),
        end_time: None,
    }
}

// --- Trait implementation ---

impl SeriesScraper for F1Scraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = fetcher::create_http_client()?;

        // Try live sportstimes API first (provides all sessions & full multi-year support)
        let mut events = if let Ok(fetched) = fetch_sportstimes_calendar(
            &client,
            "https://f1calendar.com",
            &series.id,
            &f1_stream_links(),
        )
        .await
        {
            fetched
        } else {
            Vec::new()
        };

        if events.is_empty() {
            if let Ok(response) =
                fetcher::fetch_json::<JolpicaResponse>(&client, &series.calendar_url).await
            {
                for race in response.mr_data.race_table.races {
                    let round: u32 = race.round.parse().unwrap_or(0);
                    let start_date = match NaiveDate::parse_from_str(&race.date, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(_) => continue,
                    };

                    let mut sessions = Vec::new();
                    if let Some(ref fp1) = race.first_practice {
                        sessions.push(convert_session(
                            "Free Practice 1",
                            SessionType::Practice,
                            fp1,
                        ));
                    }
                    if let Some(ref fp2) = race.second_practice {
                        sessions.push(convert_session(
                            "Free Practice 2",
                            SessionType::Practice,
                            fp2,
                        ));
                    }
                    if let Some(ref fp3) = race.third_practice {
                        sessions.push(convert_session(
                            "Free Practice 3",
                            SessionType::Practice,
                            fp3,
                        ));
                    }
                    if let Some(ref sq) = race.sprint_qualifying {
                        sessions.push(convert_session(
                            "Sprint Qualifying",
                            SessionType::SprintQualifying,
                            sq,
                        ));
                    }
                    if let Some(ref ss) = race.sprint_shootout {
                        sessions.push(convert_session(
                            "Sprint Shootout",
                            SessionType::SprintQualifying,
                            ss,
                        ));
                    }
                    if let Some(ref sprint) = race.sprint {
                        sessions.push(convert_session("Sprint", SessionType::Sprint, sprint));
                    }
                    if let Some(ref quali) = race.qualifying {
                        sessions.push(convert_session(
                            "Qualifying",
                            SessionType::Qualifying,
                            quali,
                        ));
                    }
                    sessions.push(Session {
                        name: "Race".to_string(),
                        session_type: SessionType::Race,
                        start_time: parse_datetime(&race.date, race.time.as_deref()),
                        end_time: None,
                    });

                    let earliest_date = sessions
                        .iter()
                        .filter_map(|s| s.start_time)
                        .min()
                        .map(|dt| dt.date_naive())
                        .unwrap_or(start_date);

                    events.push(RaceEvent {
                        series_id: "f1".to_string(),
                        event_name: race.race_name,
                        circuit_name: race.circuit.circuit_name,
                        location: race.circuit.location.locality,
                        country: race.circuit.location.country,
                        start_date: earliest_date,
                        end_date: start_date,
                        round: Some(round),
                        sessions,
                        stream_links: f1_stream_links(),
                        status: EventStatus::Upcoming,
                    });
                }
            }
        }

        // If 2027 calendar not yet present, append the 2027 official season calendar
        if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_f1_schedule("f1"));
        }

        Ok(events)
    }
}

pub fn get_official_2027_f1_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Australian Grand Prix",
            "Albert Park Grand Prix Circuit",
            "Melbourne",
            "Australia",
            (2027, 3, 12),
            (2027, 3, 14),
        ),
        (
            "Chinese Grand Prix",
            "Shanghai International Circuit",
            "Shanghai",
            "China",
            (2027, 3, 19),
            (2027, 3, 21),
        ),
        (
            "Japanese Grand Prix",
            "Suzuka International Racing Course",
            "Suzuka",
            "Japan",
            (2027, 4, 2),
            (2027, 4, 4),
        ),
        (
            "Bahrain Grand Prix",
            "Bahrain International Circuit",
            "Sakhir",
            "Bahrain",
            (2027, 4, 9),
            (2027, 4, 11),
        ),
        (
            "Saudi Arabian Grand Prix",
            "Jeddah Corniche Circuit",
            "Jeddah",
            "Saudi Arabia",
            (2027, 4, 16),
            (2027, 4, 18),
        ),
        (
            "Miami Grand Prix",
            "Miami International Autodrome",
            "Miami",
            "USA",
            (2027, 4, 30),
            (2027, 5, 2),
        ),
        (
            "Emilia Romagna Grand Prix",
            "Autodromo Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2027, 5, 14),
            (2027, 5, 16),
        ),
        (
            "Monaco Grand Prix",
            "Circuit de Monaco",
            "Monte Carlo",
            "Monaco",
            (2027, 5, 21),
            (2027, 5, 23),
        ),
        (
            "Spanish Grand Prix",
            "Circuit de Barcelona-Catalunya",
            "Barcelona",
            "Spain",
            (2027, 5, 28),
            (2027, 5, 30),
        ),
        (
            "Canadian Grand Prix",
            "Circuit Gilles Villeneuve",
            "Montreal",
            "Canada",
            (2027, 6, 11),
            (2027, 6, 13),
        ),
        (
            "Austrian Grand Prix",
            "Red Bull Ring",
            "Spielberg",
            "Austria",
            (2027, 6, 25),
            (2027, 6, 27),
        ),
        (
            "British Grand Prix",
            "Silverstone Circuit",
            "Silverstone",
            "UK",
            (2027, 7, 2),
            (2027, 7, 4),
        ),
        (
            "Belgian Grand Prix",
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2027, 7, 23),
            (2027, 7, 25),
        ),
        (
            "Hungarian Grand Prix",
            "Hungaroring",
            "Budapest",
            "Hungary",
            (2027, 7, 30),
            (2027, 8, 1),
        ),
        (
            "Dutch Grand Prix",
            "Circuit Zandvoort",
            "Zandvoort",
            "Netherlands",
            (2027, 8, 27),
            (2027, 8, 29),
        ),
        (
            "Italian Grand Prix",
            "Autodromo Nazionale Monza",
            "Monza",
            "Italy",
            (2027, 9, 3),
            (2027, 9, 5),
        ),
        (
            "Azerbaijan Grand Prix",
            "Baku City Circuit",
            "Baku",
            "Azerbaijan",
            (2027, 9, 17),
            (2027, 9, 19),
        ),
        (
            "Singapore Grand Prix",
            "Marina Bay Street Circuit",
            "Singapore",
            "Singapore",
            (2027, 10, 1),
            (2027, 10, 3),
        ),
        (
            "United States Grand Prix",
            "Circuit of the Americas",
            "Austin",
            "USA",
            (2027, 10, 15),
            (2027, 10, 17),
        ),
        (
            "Mexico City Grand Prix",
            "Autódromo Hermanos Rodríguez",
            "Mexico City",
            "Mexico",
            (2027, 10, 22),
            (2027, 10, 24),
        ),
        (
            "São Paulo Grand Prix",
            "Autódromo José Carlos Pace",
            "São Paulo",
            "Brazil",
            (2027, 11, 5),
            (2027, 11, 7),
        ),
        (
            "Las Vegas Grand Prix",
            "Las Vegas Strip Circuit",
            "Las Vegas",
            "USA",
            (2027, 11, 18),
            (2027, 11, 20),
        ),
        (
            "Qatar Grand Prix",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2027, 11, 26),
            (2027, 11, 28),
        ),
        (
            "Abu Dhabi Grand Prix",
            "Yas Marina Circuit",
            "Abu Dhabi",
            "UAE",
            (2027, 12, 3),
            (2027, 12, 5),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, country, start, end))| {
            let fri_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let fp1_time = fri_date
                .and_hms_opt(11, 30, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let fp2_time = fri_date
                .and_hms_opt(15, 0, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let fp3_time = sat_date
                .and_hms_opt(10, 30, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let quali_time = sat_date
                .and_hms_opt(14, 0, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = sun_date
                .and_hms_opt(13, 0, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Free Practice 1".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fp1_time,
                    end_time: None,
                },
                Session {
                    name: "Free Practice 2".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fp2_time,
                    end_time: None,
                },
                Session {
                    name: "Free Practice 3".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fp3_time,
                    end_time: None,
                },
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Grand Prix".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: fri_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: f1_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

/// Parse Jolpica driver standings JSON response into a Vec of DriverStanding.
pub fn parse_jolpica_driver_standings(json: &str) -> Result<Vec<DriverStanding>> {
    let data: JolpicaStandingsResponse = serde_json::from_str(json)?;
    let mut drivers = Vec::new();
    if let Some(list) = data.mr_data.standings_table.standings_lists.first() {
        for d in &list.driver_standings {
            drivers.push(DriverStanding {
                position: d.position.parse().unwrap_or(0),
                driver_name: format!("{} {}", d.driver.given_name, d.driver.family_name),
                driver_code: d.driver.code.clone(),
                driver_number: d
                    .driver
                    .permanent_number
                    .as_deref()
                    .and_then(|n| n.parse().ok()),
                team: d
                    .constructors
                    .first()
                    .map(|c| c.name.clone())
                    .unwrap_or_default(),
                points: d.points.parse().unwrap_or(0.0),
                wins: d.wins.parse().unwrap_or(0),
            });
        }
    }
    Ok(drivers)
}

/// Parse Jolpica constructor standings JSON response into a Vec of ConstructorStanding.
pub fn parse_jolpica_constructor_standings(json: &str) -> Result<Vec<ConstructorStanding>> {
    let data: JolpicaStandingsResponse = serde_json::from_str(json)?;
    let mut constructors = Vec::new();
    if let Some(list) = data.mr_data.standings_table.standings_lists.first() {
        for c in &list.constructor_standings {
            constructors.push(ConstructorStanding {
                position: c.position.parse().unwrap_or(0),
                name: c.constructor.name.clone(),
                points: c.points.parse().unwrap_or(0.0),
                wins: c.wins.parse().unwrap_or(0),
            });
        }
    }
    Ok(constructors)
}

impl StandingsFetcher for F1Scraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = fetcher::create_http_client()?;
        let driver_url = format!(
            "https://api.jolpi.ca/ergast/f1/{}/driverStandings.json",
            season
        );
        let constr_url = format!(
            "https://api.jolpi.ca/ergast/f1/{}/constructorStandings.json",
            season
        );

        let driver_text = client.get(&driver_url).send().await?.text().await?;
        let drivers = parse_jolpica_driver_standings(&driver_text).unwrap_or_default();

        let mut constructors = Vec::new();
        if let Ok(constr_resp) = client.get(&constr_url).send().await {
            if let Ok(text) = constr_resp.text().await {
                constructors = parse_jolpica_constructor_standings(&text).unwrap_or_default();
            }
        }

        Ok(SeasonStandings {
            series_id: "f1".to_string(),
            season,
            drivers,
            constructors,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse Jolpica race results JSON response into RaceResults.
pub fn parse_jolpica_race_results(json: &str) -> Result<RaceResults> {
    let data: JolpicaResultsResponse = serde_json::from_str(json)?;
    let race = data
        .mr_data
        .race_table
        .races
        .into_iter()
        .next()
        .context("No race results in Jolpica response")?;

    let round: u32 = race.round.parse().unwrap_or(0);
    let race_date = NaiveDate::parse_from_str(&race.date, "%Y-%m-%d")
        .unwrap_or_else(|_| Utc::now().date_naive());

    let results: Vec<DriverResult> = race
        .results
        .into_iter()
        .map(|r| {
            let position = r.position.parse::<u32>().ok();
            let driver_name = format!("{} {}", r.driver.given_name, r.driver.family_name);
            let driver_code = r.driver.code;
            let driver_number = r
                .driver
                .permanent_number
                .as_deref()
                .or(Some(&r.number))
                .and_then(|n| n.parse::<u32>().ok());
            let team = r.constructor.name;
            let gap_to_leader = r
                .time
                .as_ref()
                .map(|t| t.time.clone())
                .unwrap_or_else(|| r.status.clone());
            let grid_position = r.grid.parse::<u32>().ok();
            let points = r.points.parse::<f64>().unwrap_or(0.0);
            let fastest_lap = r
                .fastest_lap
                .as_ref()
                .map_or(false, |fl| fl.rank.as_deref() == Some("1"));
            let status = r.status;

            DriverResult {
                position,
                driver_name,
                driver_code,
                driver_number,
                team,
                gap_to_leader,
                gap_to_ahead: String::new(),
                grid_position,
                points,
                fastest_lap,
                penalty: None,
                status,
            }
        })
        .collect();

    Ok(RaceResults {
        series_id: "f1".to_string(),
        round,
        event_name: race.race_name,
        circuit_name: race.circuit.circuit_name,
        race_date,
        results,
        fetched_at: Utc::now(),
    })
}

/// Parse Jolpica Ergast qualifying JSON into `QualifyingResults`.
pub fn parse_jolpica_qualifying_results(json_str: &str) -> Result<QualifyingResults> {
    let resp: JolpicaQualifyingResponse = serde_json::from_str(json_str)
        .context("Failed to parse Jolpica qualifying results JSON")?;

    let race = resp
        .mr_data
        .race_table
        .races
        .into_iter()
        .next()
        .context("No qualifying race data in Jolpica response")?;

    let round = race.round.parse::<u32>().unwrap_or(0);
    let race_date = NaiveDate::parse_from_str(&race.date, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());

    let results = race
        .qualifying_results
        .into_iter()
        .map(|entry| {
            let position = entry.position.parse::<u32>().unwrap_or(0);
            let driver_number = entry
                .number
                .and_then(|n| n.parse::<u32>().ok())
                .or_else(|| {
                    entry
                        .driver
                        .permanent_number
                        .as_deref()
                        .and_then(|n| n.parse::<u32>().ok())
                });
            let driver_name = format!("{} {}", entry.driver.given_name, entry.driver.family_name);
            let driver_code = entry.driver.code;
            let team = entry.constructor.name;

            QualifyingDriverResult {
                position,
                driver_name,
                driver_code,
                driver_number,
                team,
                q1: entry.q1,
                q2: entry.q2,
                q3: entry.q3,
            }
        })
        .collect();

    Ok(QualifyingResults {
        series_id: "f1".to_string(),
        round,
        event_name: race.race_name,
        circuit_name: race.circuit.circuit_name,
        race_date,
        results,
        fetched_at: Utc::now(),
    })
}

impl ResultsFetcher for F1Scraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        let client = fetcher::create_http_client()?;
        let url = format!(
            "https://api.jolpi.ca/ergast/f1/{}/{}/results.json",
            season, round
        );
        let resp_text = client.get(&url).send().await?.text().await?;
        parse_jolpica_race_results(&resp_text)
    }

    async fn fetch_qualifying(&self, season: u32, round: u32) -> Result<QualifyingResults> {
        let client = fetcher::create_http_client()?;
        let url = format!(
            "https://api.jolpi.ca/ergast/f1/{}/{}/qualifying.json",
            season, round
        );
        let resp_text = client.get(&url).send().await?.text().await?;
        parse_jolpica_qualifying_results(&resp_text)
    }

    async fn fetch_sprint(&self, season: u32, round: u32) -> Result<RaceResults> {
        let client = fetcher::create_http_client()?;
        let url = format!(
            "https://api.jolpi.ca/ergast/f1/{}/{}/sprint.json",
            season, round
        );
        let resp_text = client.get(&url).send().await?.text().await?;
        parse_jolpica_race_results(&resp_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        let dt = parse_datetime("2026-03-01", Some("15:00:00Z"));
        assert!(dt.is_some());
        let val = dt.unwrap();
        assert_eq!(val.to_rfc3339(), "2026-03-01T15:00:00+00:00");

        let dt_notime = parse_datetime("2026-03-01", None);
        assert!(dt_notime.is_none());
    }

    #[test]
    fn test_jolpica_json_parsing() {
        let json_data = r#"{
            "MRData": {
                "RaceTable": {
                    "Races": [
                        {
                            "round": "1",
                            "raceName": "Bahrain Grand Prix",
                            "Circuit": {
                                "circuitName": "Bahrain International Circuit",
                                "Location": {
                                    "locality": "Sakhir",
                                    "country": "Bahrain"
                                }
                            },
                            "date": "2026-03-01",
                            "time": "15:00:00Z",
                            "FirstPractice": {
                                "date": "2026-02-27",
                                "time": "11:30:00Z"
                            },
                            "Qualifying": {
                                "date": "2026-02-28",
                                "time": "15:00:00Z"
                            }
                        }
                    ]
                }
            }
        }"#;

        let parsed: JolpicaResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(parsed.mr_data.race_table.races.len(), 1);
        let race = &parsed.mr_data.race_table.races[0];
        assert_eq!(race.race_name, "Bahrain Grand Prix");
        assert_eq!(race.circuit.location.country, "Bahrain");
        assert!(race.first_practice.is_some());
        assert!(race.qualifying.is_some());
    }

    #[test]
    fn test_parse_jolpica_driver_standings() {
        let json_data = r#"{
            "MRData": {
                "xmlns": "http://ergast.com/mrd/1.5",
                "series": "f1",
                "url": "http://api.jolpica.com/ergast/f1/2026/driverstandings.json",
                "limit": "30",
                "offset": "0",
                "total": "2",
                "StandingsTable": {
                    "season": "2026",
                    "StandingsLists": [
                        {
                            "season": "2026",
                            "round": "1",
                            "DriverStandings": [
                                {
                                    "position": "1",
                                    "positionText": "1",
                                    "points": "25",
                                    "wins": "1",
                                    "Driver": {
                                        "driverId": "max_verstappen",
                                        "permanentNumber": "1",
                                        "code": "VER",
                                        "givenName": "Max",
                                        "familyName": "Verstappen"
                                    },
                                    "Constructors": [
                                        {
                                            "constructorId": "red_bull",
                                            "name": "Red Bull"
                                        }
                                    ]
                                },
                                {
                                    "position": "2",
                                    "positionText": "2",
                                    "points": "18",
                                    "wins": "0",
                                    "Driver": {
                                        "driverId": "norris",
                                        "permanentNumber": "4",
                                        "code": "NOR",
                                        "givenName": "Lando",
                                        "familyName": "Norris"
                                    },
                                    "Constructors": [
                                        {
                                            "constructorId": "mclaren",
                                            "name": "McLaren"
                                        }
                                    ]
                                }
                            ]
                        }
                    ]
                }
            }
        }"#;

        let drivers = parse_jolpica_driver_standings(json_data).unwrap();
        assert_eq!(drivers.len(), 2);
        assert_eq!(drivers[0].position, 1);
        assert_eq!(drivers[0].driver_name, "Max Verstappen");
        assert_eq!(drivers[0].driver_code.as_deref(), Some("VER"));
        assert_eq!(drivers[0].driver_number, Some(1));
        assert_eq!(drivers[0].team, "Red Bull");
        assert_eq!(drivers[0].points, 25.0);
        assert_eq!(drivers[0].wins, 1);

        assert_eq!(drivers[1].position, 2);
        assert_eq!(drivers[1].driver_name, "Lando Norris");
        assert_eq!(drivers[1].driver_code.as_deref(), Some("NOR"));
        assert_eq!(drivers[1].driver_number, Some(4));
        assert_eq!(drivers[1].team, "McLaren");
        assert_eq!(drivers[1].points, 18.0);
        assert_eq!(drivers[1].wins, 0);
    }

    #[test]
    fn test_parse_jolpica_constructor_standings() {
        let json_data = r#"{
            "MRData": {
                "xmlns": "http://ergast.com/mrd/1.5",
                "series": "f1",
                "url": "http://api.jolpica.com/ergast/f1/2026/constructorstandings.json",
                "limit": "30",
                "offset": "0",
                "total": "2",
                "StandingsTable": {
                    "season": "2026",
                    "StandingsLists": [
                        {
                            "season": "2026",
                            "round": "1",
                            "ConstructorStandings": [
                                {
                                    "position": "1",
                                    "positionText": "1",
                                    "points": "40",
                                    "wins": "1",
                                    "Constructor": {
                                        "constructorId": "red_bull",
                                        "name": "Red Bull"
                                    }
                                },
                                {
                                    "position": "2",
                                    "positionText": "2",
                                    "points": "28",
                                    "wins": "0",
                                    "Constructor": {
                                        "constructorId": "mclaren",
                                        "name": "McLaren"
                                    }
                                }
                            ]
                        }
                    ]
                }
            }
        }"#;

        let constructors = parse_jolpica_constructor_standings(json_data).unwrap();
        assert_eq!(constructors.len(), 2);
        assert_eq!(constructors[0].position, 1);
        assert_eq!(constructors[0].name, "Red Bull");
        assert_eq!(constructors[0].points, 40.0);
        assert_eq!(constructors[0].wins, 1);

        assert_eq!(constructors[1].position, 2);
        assert_eq!(constructors[1].name, "McLaren");
        assert_eq!(constructors[1].points, 28.0);
        assert_eq!(constructors[1].wins, 0);
    }

    #[test]
    fn test_parse_jolpica_race_results() {
        let json_data = r#"{
            "MRData": {
                "xmlns": "http://ergast.com/mrd/1.5",
                "series": "f1",
                "url": "http://api.jolpica.com/ergast/f1/2026/1/results.json",
                "limit": "30",
                "offset": "0",
                "total": "2",
                "RaceTable": {
                    "season": "2026",
                    "round": "1",
                    "Races": [
                        {
                            "season": "2026",
                            "round": "1",
                            "raceName": "Bahrain Grand Prix",
                            "Circuit": {
                                "circuitName": "Bahrain International Circuit",
                                "Location": {
                                    "locality": "Sakhir",
                                    "country": "Bahrain"
                                }
                            },
                            "date": "2026-03-01",
                            "time": "15:00:00Z",
                            "Results": [
                                {
                                    "number": "1",
                                    "position": "1",
                                    "positionText": "1",
                                    "points": "26",
                                    "Driver": {
                                        "driverId": "max_verstappen",
                                        "permanentNumber": "1",
                                        "code": "VER",
                                        "givenName": "Max",
                                        "familyName": "Verstappen"
                                    },
                                    "Constructor": {
                                        "constructorId": "red_bull",
                                        "name": "Red Bull"
                                    },
                                    "grid": "1",
                                    "laps": "57",
                                    "status": "Finished",
                                    "Time": {
                                        "millis": "5504742",
                                        "time": "1:31:44.742"
                                    },
                                    "FastestLap": {
                                        "rank": "1",
                                        "lap": "39",
                                        "Time": {
                                            "time": "1:32.608"
                                        }
                                    }
                                },
                                {
                                    "number": "4",
                                    "position": "2",
                                    "positionText": "2",
                                    "points": "18",
                                    "Driver": {
                                        "driverId": "norris",
                                        "permanentNumber": "4",
                                        "code": "NOR",
                                        "givenName": "Lando",
                                        "familyName": "Norris"
                                    },
                                    "Constructor": {
                                        "constructorId": "mclaren",
                                        "name": "McLaren"
                                    },
                                    "grid": "3",
                                    "laps": "57",
                                    "status": "Finished",
                                    "Time": {
                                        "millis": "5527199",
                                        "time": "+22.457"
                                    },
                                    "FastestLap": {
                                        "rank": "2",
                                        "lap": "42",
                                        "Time": {
                                            "time": "1:33.123"
                                        }
                                    }
                                }
                            ]
                        }
                    ]
                }
            }
        }"#;

        let results = parse_jolpica_race_results(json_data).unwrap();
        assert_eq!(results.series_id, "f1");
        assert_eq!(results.round, 1);
        assert_eq!(results.event_name, "Bahrain Grand Prix");
        assert_eq!(results.circuit_name, "Bahrain International Circuit");
        assert_eq!(results.results.len(), 2);

        let p1 = &results.results[0];
        assert_eq!(p1.position, Some(1));
        assert_eq!(p1.driver_name, "Max Verstappen");
        assert_eq!(p1.driver_code.as_deref(), Some("VER"));
        assert_eq!(p1.driver_number, Some(1));
        assert_eq!(p1.team, "Red Bull");
        assert_eq!(p1.grid_position, Some(1));
        assert_eq!(p1.positions_gained(), Some(0));
        assert_eq!(p1.points, 26.0);
        assert!(p1.fastest_lap);
        assert_eq!(p1.gap_to_leader, "1:31:44.742");

        let p2 = &results.results[1];
        assert_eq!(p2.position, Some(2));
        assert_eq!(p2.driver_name, "Lando Norris");
        assert_eq!(p2.driver_code.as_deref(), Some("NOR"));
        assert_eq!(p2.driver_number, Some(4));
        assert_eq!(p2.team, "McLaren");
        assert_eq!(p2.grid_position, Some(3));
        assert_eq!(p2.positions_gained(), Some(1));
        assert_eq!(p2.points, 18.0);
        assert!(!p2.fastest_lap);
        assert_eq!(p2.gap_to_leader, "+22.457");
    }

    #[tokio::test]
    async fn test_f1_live_standings_fetch() {
        let scraper = F1Scraper;
        let standings = scraper.fetch_standings(2026).await;
        assert!(
            standings.is_ok(),
            "F1 standings fetch failed: {:?}",
            standings.err()
        );
        let s = standings.unwrap();
        assert_eq!(s.series_id, "f1");
        assert_eq!(s.season, 2026);
        assert!(
            !s.drivers.is_empty(),
            "Expected drivers in F1 2026 standings"
        );
        assert!(
            !s.constructors.is_empty(),
            "Expected constructors in F1 2026 standings"
        );
    }

    #[test]
    fn test_parse_jolpica_qualifying_results() {
        let json_data = r#"{
            "MRData": {
                "xmlns": "",
                "series": "f1",
                "url": "https://api.jolpi.ca/ergast/f1/2026/1/qualifying.json",
                "limit": "30",
                "offset": "0",
                "total": "2",
                "RaceTable": {
                    "season": "2026",
                    "round": "1",
                    "Races": [
                        {
                            "season": "2026",
                            "round": "1",
                            "raceName": "Australian Grand Prix",
                            "Circuit": {
                                "circuitId": "albert_park",
                                "circuitName": "Albert Park Grand Prix Circuit",
                                "Location": {
                                    "locality": "Melbourne",
                                    "country": "Australia"
                                }
                            },
                            "date": "2026-03-08",
                            "QualifyingResults": [
                                {
                                    "number": "63",
                                    "position": "1",
                                    "Driver": {
                                        "driverId": "russell",
                                        "permanentNumber": "63",
                                        "code": "RUS",
                                        "givenName": "George",
                                        "familyName": "Russell"
                                    },
                                    "Constructor": {
                                        "constructorId": "mercedes",
                                        "name": "Mercedes"
                                    },
                                    "Q1": "1:19.507",
                                    "Q2": "1:18.934",
                                    "Q3": "1:18.518"
                                }
                            ]
                        }
                    ]
                }
            }
        }"#;

        let q = parse_jolpica_qualifying_results(json_data).unwrap();
        assert_eq!(q.series_id, "f1");
        assert_eq!(q.round, 1);
        assert_eq!(q.event_name, "Australian Grand Prix");
        assert_eq!(q.results.len(), 1);
        assert_eq!(q.results[0].position, 1);
        assert_eq!(q.results[0].driver_name, "George Russell");
        assert_eq!(q.results[0].q3.as_deref(), Some("1:18.518"));
    }

    #[tokio::test]
    async fn test_f1_live_qualifying_fetch() {
        let scraper = F1Scraper;
        let res = scraper.fetch_qualifying(2026, 1).await;
        assert!(res.is_ok(), "F1 qualifying fetch failed: {:?}", res.err());
        let q = res.unwrap();
        assert_eq!(q.series_id, "f1");
        assert_eq!(q.round, 1);
        assert!(!q.results.is_empty(), "Expected qualifying results entries");
    }
}
