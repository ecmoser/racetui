use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::sportstimes::fetch_sportstimes_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

use serde::Deserialize;

pub struct FormulaEScraper;

impl StandingsFetcher for FormulaEScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = create_http_client()?;

        // 1. Fetch championships
        let champ_url = "https://api.formula-e.pulselive.com/formula-e/v1/championships";
        let champ_text = client.get(champ_url).send().await?.text().await?;
        let champ_resp: FormulaEChampionshipsResponse = serde_json::from_str(&champ_text)?;

        let season_str = format!("{}", season);
        let target_champ = champ_resp
            .championships
            .iter()
            .find(|c| c.name.contains(&season_str))
            .or_else(|| {
                champ_resp
                    .championships
                    .iter()
                    .find(|c| c.status == "Present")
            })
            .or_else(|| champ_resp.championships.last())
            .ok_or_else(|| {
                anyhow::anyhow!("No Formula E championship found for season {}", season)
            })?;

        // 2. Fetch driver standings
        let driver_url = format!(
            "https://api.formula-e.pulselive.com/formula-e/v1/standings/drivers?championshipId={}",
            target_champ.id
        );
        let driver_text = client.get(&driver_url).send().await?.text().await?;
        let drivers = parse_formula_e_driver_standings(&driver_text)?;

        // 3. Fetch team standings
        let team_url = format!(
            "https://api.formula-e.pulselive.com/formula-e/v1/standings/teams?championshipId={}",
            target_champ.id
        );
        let mut constructors = Vec::new();
        if let Ok(team_resp) = client.get(&team_url).send().await {
            if let Ok(team_text) = team_resp.text().await {
                constructors = parse_formula_e_team_standings(&team_text).unwrap_or_default();
            }
        }

        Ok(SeasonStandings {
            series_id: "formula_e".to_string(),
            season,
            drivers,
            constructors,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse Formula E driver standings JSON.
pub fn parse_formula_e_driver_standings(
    json: &str,
) -> Result<Vec<crate::data::standings::DriverStanding>> {
    let raw: Vec<FormulaEDriverStandingEntry> = serde_json::from_str(json)?;
    let list = raw
        .into_iter()
        .map(|d| {
            let full_name = format!("{} {}", d.driver_first_name, d.driver_last_name)
                .trim()
                .to_string();
            let wins = d
                .driver_race_standings
                .as_ref()
                .map(|rs| rs.iter().filter(|r| r.race_position == Some(1)).count() as u32)
                .unwrap_or(0);
            crate::data::standings::DriverStanding {
                position: d.driver_position,
                driver_name: full_name,
                driver_code: d.driver_tla,
                driver_number: None,
                team: d.driver_team_name.unwrap_or_default(),
                points: d.driver_points as f64,
                wins,
            }
        })
        .collect();
    Ok(list)
}

/// Parse Formula E team standings JSON.
pub fn parse_formula_e_team_standings(
    json: &str,
) -> Result<Vec<crate::data::standings::ConstructorStanding>> {
    let raw: Vec<FormulaETeamStandingEntry> = serde_json::from_str(json)?;
    let list = raw
        .into_iter()
        .map(|t| crate::data::standings::ConstructorStanding {
            position: t.team_position,
            name: t.team_name,
            points: t.team_points as f64,
            wins: 0,
        })
        .collect();
    Ok(list)
}

#[derive(Debug, Deserialize)]
struct FormulaEChampionshipsResponse {
    championships: Vec<FormulaEChampionship>,
}

#[derive(Debug, Deserialize)]
struct FormulaEChampionship {
    id: String,
    name: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct FormulaEDriverStandingEntry {
    #[serde(rename = "driverPosition")]
    driver_position: u32,
    #[serde(rename = "driverFirstName", default)]
    driver_first_name: String,
    #[serde(rename = "driverLastName", default)]
    driver_last_name: String,
    #[serde(rename = "driverTLA")]
    driver_tla: Option<String>,
    #[serde(rename = "driverTeamName")]
    driver_team_name: Option<String>,
    #[serde(rename = "driverPoints", default)]
    driver_points: f64,
    #[serde(rename = "driverRaceStandings")]
    driver_race_standings: Option<Vec<FormulaERaceStandingEntry>>,
}

#[derive(Debug, Deserialize)]
struct FormulaERaceStandingEntry {
    #[serde(rename = "racePosition")]
    race_position: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FormulaETeamStandingEntry {
    #[serde(rename = "teamPosition")]
    team_position: u32,
    #[serde(rename = "teamName")]
    team_name: String,
    #[serde(rename = "teamPoints", default)]
    team_points: f64,
}

impl SeriesScraper for FormulaEScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_sportstimes_calendar(
            &client,
            "https://formulaecal.com",
            &series.id,
            &formula_e_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_formula_e_schedule(&series.id);
            events.extend(get_official_2027_formula_e_schedule(&series.id));
        } else if !events.iter().any(|e| e.start_date.year() == 2027) {
            events.extend(get_official_2027_formula_e_schedule(&series.id));
        }

        Ok(events)
    }
}

fn formula_e_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "YouTube (Practice)".to_string(),
            url: "https://www.youtube.com/@FIAFormulaE".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "Roku / TNT Sports".to_string(),
            url: "https://www.fiaformulae.com/ways-to-watch".to_string(),
            access: StreamAccess::Mixed,
        },
    ]
}

pub fn get_official_2026_formula_e_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "São Paulo E-Prix",
            "São Paulo Street Circuit",
            "São Paulo",
            "Brazil",
            (2025, 12, 6),
        ),
        (
            "Mexico City E-Prix",
            "Autódromo Hermanos Rodríguez",
            "Mexico City",
            "Mexico",
            (2026, 1, 10),
        ),
        (
            "Diriyah E-Prix (Race 1)",
            "Jeddah Corniche Circuit",
            "Jeddah",
            "Saudi Arabia",
            (2026, 2, 13),
        ),
        (
            "Diriyah E-Prix (Race 2)",
            "Jeddah Corniche Circuit",
            "Jeddah",
            "Saudi Arabia",
            (2026, 2, 14),
        ),
        (
            "Miami E-Prix",
            "Homestead-Miami Speedway",
            "Miami, Florida",
            "USA",
            (2026, 4, 11),
        ),
        (
            "Monaco E-Prix (Race 1)",
            "Circuit de Monaco",
            "Monte Carlo",
            "Monaco",
            (2026, 5, 2),
        ),
        (
            "Monaco E-Prix (Race 2)",
            "Circuit de Monaco",
            "Monte Carlo",
            "Monaco",
            (2026, 5, 3),
        ),
        (
            "Tokyo E-Prix (Race 1)",
            "Tokyo Street Circuit",
            "Tokyo",
            "Japan",
            (2026, 5, 16),
        ),
        (
            "Tokyo E-Prix (Race 2)",
            "Tokyo Street Circuit",
            "Tokyo",
            "Japan",
            (2026, 5, 17),
        ),
        (
            "Shanghai E-Prix (Race 1)",
            "Shanghai International Circuit",
            "Shanghai",
            "China",
            (2026, 5, 30),
        ),
        (
            "Shanghai E-Prix (Race 2)",
            "Shanghai International Circuit",
            "Shanghai",
            "China",
            (2026, 5, 31),
        ),
        (
            "Jakarta E-Prix",
            "Jakarta International E-Prix Circuit",
            "Jakarta",
            "Indonesia",
            (2026, 6, 20),
        ),
        (
            "Berlin E-Prix (Race 1)",
            "Tempelhof Airport Street Circuit",
            "Berlin",
            "Germany",
            (2026, 7, 11),
        ),
        (
            "Berlin E-Prix (Race 2)",
            "Tempelhof Airport Street Circuit",
            "Berlin",
            "Germany",
            (2026, 7, 12),
        ),
        (
            "London E-Prix (Race 1)",
            "ExCeL London",
            "London",
            "UK",
            (2026, 7, 25),
        ),
        (
            "London E-Prix (Race 2)",
            "ExCeL London",
            "London",
            "UK",
            (2026, 7, 26),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, country, date))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let quali_time = race_date
                .and_hms_opt(8, 20, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = race_date
                .and_hms_opt(13, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let status = if race_date < today {
                EventStatus::Completed
            } else if race_date == today {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
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
                start_date: race_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: formula_e_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_formula_e_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Mexico City E-Prix",
            "Autódromo Hermanos Rodríguez",
            "Mexico City",
            "Mexico",
            (2027, 1, 9),
            2,
        ),
        (
            "Miami E-Prix",
            "Homestead-Miami Speedway",
            "Miami",
            "USA",
            (2027, 1, 30),
            3,
        ),
        (
            "Diriyah E-Prix (Race 1)",
            "Riyadh Street Circuit",
            "Diriyah",
            "Saudi Arabia",
            (2027, 2, 12),
            4,
        ),
        (
            "Diriyah E-Prix (Race 2)",
            "Riyadh Street Circuit",
            "Diriyah",
            "Saudi Arabia",
            (2027, 2, 13),
            5,
        ),
        (
            "Monaco E-Prix (Race 1)",
            "Circuit de Monaco",
            "Monte Carlo",
            "Monaco",
            (2027, 5, 1),
            6,
        ),
        (
            "Monaco E-Prix (Race 2)",
            "Circuit de Monaco",
            "Monte Carlo",
            "Monaco",
            (2027, 5, 2),
            7,
        ),
        (
            "Tokyo E-Prix (Race 1)",
            "Tokyo Street Circuit",
            "Tokyo",
            "Japan",
            (2027, 5, 15),
            8,
        ),
        (
            "Tokyo E-Prix (Race 2)",
            "Tokyo Street Circuit",
            "Tokyo",
            "Japan",
            (2027, 5, 16),
            9,
        ),
        (
            "Shanghai E-Prix (Race 1)",
            "Shanghai International Circuit",
            "Shanghai",
            "China",
            (2027, 5, 29),
            10,
        ),
        (
            "Shanghai E-Prix (Race 2)",
            "Shanghai International Circuit",
            "Shanghai",
            "China",
            (2027, 5, 30),
            11,
        ),
        (
            "Jakarta E-Prix",
            "Jakarta International E-Prix Circuit",
            "Jakarta",
            "Indonesia",
            (2027, 6, 19),
            12,
        ),
        (
            "Berlin E-Prix (Race 1)",
            "Tempelhof Airport Street Circuit",
            "Berlin",
            "Germany",
            (2027, 7, 10),
            13,
        ),
        (
            "Berlin E-Prix (Race 2)",
            "Tempelhof Airport Street Circuit",
            "Berlin",
            "Germany",
            (2027, 7, 11),
            14,
        ),
        (
            "London E-Prix (Race 1)",
            "ExCeL London",
            "London",
            "UK",
            (2027, 7, 24),
            15,
        ),
        (
            "London E-Prix (Race 2)",
            "ExCeL London",
            "London",
            "UK",
            (2027, 7, 25),
            16,
        ),
    ];

    raw_events
        .into_iter()
        .map(|(name, circuit, loc, country, date, round)| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let quali_time = race_date
                .and_hms_opt(10, 30, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = race_date
                .and_hms_opt(15, 0, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
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
                start_date: race_date,
                end_date: race_date,
                round: Some(round),
                sessions,
                stream_links: formula_e_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_formula_e_schedule() {
        let events = get_official_2026_formula_e_schedule("formula_e");
        assert_eq!(events.len(), 16);
        assert_eq!(events[0].event_name, "São Paulo E-Prix");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[15].event_name, "London E-Prix (Race 2)");
        assert_eq!(events[15].round, Some(16));
    }

    #[test]
    fn test_parse_formula_e_standings() {
        let driver_json = r#"[
            {
                "driverPosition": 1,
                "driverFirstName": "Oliver",
                "driverLastName": "Rowland",
                "driverTLA": "ROW",
                "driverTeamName": "NISSAN FORMULA E TEAM",
                "driverPoints": 184.0,
                "driverRaceStandings": [
                    { "racePosition": 1 },
                    { "racePosition": 3 }
                ]
            }
        ]"#;

        let team_json = r#"[
            {
                "teamPosition": 1,
                "teamName": "TAG HEUER PORSCHE FORMULA E TEAM",
                "teamPoints": 230.0
            }
        ]"#;

        let drivers = parse_formula_e_driver_standings(driver_json).unwrap();
        assert_eq!(drivers.len(), 1);
        assert_eq!(drivers[0].driver_name, "Oliver Rowland");
        assert_eq!(drivers[0].driver_code.as_deref(), Some("ROW"));
        assert_eq!(drivers[0].points, 184.0);
        assert_eq!(drivers[0].wins, 1);

        let teams = parse_formula_e_team_standings(team_json).unwrap();
        assert_eq!(teams.len(), 1);
        assert_eq!(teams[0].name, "TAG HEUER PORSCHE FORMULA E TEAM");
        assert_eq!(teams[0].points, 230.0);
    }

    #[test]
    fn test_parse_formula_e_race_results() {
        let sample = r#"[
            {
                "position": 1,
                "driverFirstName": "Pascal",
                "driverLastName": "Wehrlein",
                "driverTLA": "WEH",
                "driverTeamName": "TAG Heuer Porsche",
                "points": 25.0,
                "gap": "",
                "status": "CLASSIFIED"
            }
        ]"#;

        let results = parse_formula_e_race_results(sample, 1, "Mexico City E-Prix").unwrap();
        assert_eq!(results.series_id, "formula_e");
        assert_eq!(results.round, 1);
        assert_eq!(results.event_name, "Mexico City E-Prix");
        assert_eq!(results.results.len(), 1);
        assert_eq!(results.results[0].position, Some(1));
        assert_eq!(results.results[0].driver_name, "Pascal Wehrlein");
        assert_eq!(results.results[0].driver_code.as_deref(), Some("WEH"));
        assert_eq!(results.results[0].points, 25.0);
    }
}

impl ResultsFetcher for FormulaEScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        let client = create_http_client()?;

        // 1. Fetch championships
        let champ_url = "https://api.formula-e.pulselive.com/formula-e/v1/championships";
        let champ_text = client.get(champ_url).send().await?.text().await?;
        let champ_resp: FormulaEChampionshipsResponse = serde_json::from_str(&champ_text)?;

        let season_str = format!("{}", season);
        let target_champ = champ_resp
            .championships
            .iter()
            .find(|c| c.name.contains(&season_str))
            .or_else(|| {
                champ_resp
                    .championships
                    .iter()
                    .find(|c| c.status == "Present")
            })
            .or_else(|| champ_resp.championships.last())
            .ok_or_else(|| {
                anyhow::anyhow!("No Formula E championship found for season {}", season)
            })?;

        // 2. Fetch races
        let races_url = format!(
            "https://api.formula-e.pulselive.com/formula-e/v1/races?championshipId={}",
            target_champ.id
        );
        let races_text = client.get(&races_url).send().await?.text().await?;
        let races_resp: FormulaERacesResponse = serde_json::from_str(&races_text)?;
        let target_race = races_resp
            .races
            .into_iter()
            .find(|r| r.round == Some(round))
            .ok_or_else(|| anyhow::anyhow!("Formula E race round {} not found", round))?;

        let session_id = target_race
            .sessions
            .iter()
            .find(|s| {
                s.session_type.eq_ignore_ascii_case("race")
                    || s.session_type.eq_ignore_ascii_case("RAC")
            })
            .or_else(|| target_race.sessions.last())
            .map(|s| s.id.clone())
            .ok_or_else(|| anyhow::anyhow!("No race session found for round {}", round))?;

        // 3. Fetch session results
        let res_url = format!(
            "https://api.formula-e.pulselive.com/formula-e/v1/sessions/{}/results",
            session_id
        );
        let res_text = client.get(&res_url).send().await?.text().await?;
        parse_formula_e_race_results(&res_text, round, &target_race.name)
    }
}

/// Parse Formula E session results JSON into RaceResults.
pub fn parse_formula_e_race_results(
    json: &str,
    round: u32,
    event_name: &str,
) -> Result<RaceResults> {
    let raw: Vec<FormulaESessionResultEntry> = serde_json::from_str(json)?;
    let mut results = Vec::new();

    for entry in raw {
        let first = entry.driver_first_name.unwrap_or_default();
        let last = entry.driver_last_name.unwrap_or_default();
        let full_name = format!("{} {}", first, last).trim().to_string();

        results.push(crate::data::results::DriverResult {
            position: Some(entry.position),
            driver_name: if full_name.is_empty() {
                "Unknown".to_string()
            } else {
                full_name
            },
            driver_code: entry.driver_tla,
            driver_number: None,
            team: entry.driver_team_name.unwrap_or_default(),
            gap_to_leader: entry.gap.unwrap_or_default(),
            gap_to_ahead: "".to_string(),
            grid_position: None,
            points: entry.points.unwrap_or(0.0),
            fastest_lap: false,
            penalty: None,
            status: entry.status.unwrap_or_else(|| "Finished".to_string()),
        });
    }

    Ok(RaceResults {
        series_id: "formula_e".to_string(),
        round,
        event_name: event_name.to_string(),
        circuit_name: "".to_string(),
        race_date: Utc::now().date_naive(),
        results,
        fetched_at: Utc::now(),
    })
}

#[derive(Debug, Deserialize)]
struct FormulaERacesResponse {
    #[serde(default)]
    races: Vec<FormulaERaceSummary>,
}

#[derive(Debug, Deserialize)]
struct FormulaERaceSummary {
    id: String,
    name: String,
    round: Option<u32>,
    #[serde(default)]
    sessions: Vec<FormulaESessionSummary>,
}

#[derive(Debug, Deserialize)]
struct FormulaESessionSummary {
    id: String,
    #[serde(rename = "type", default)]
    session_type: String,
}

#[derive(Debug, Deserialize)]
struct FormulaESessionResultEntry {
    position: u32,
    #[serde(rename = "driverFirstName")]
    driver_first_name: Option<String>,
    #[serde(rename = "driverLastName")]
    driver_last_name: Option<String>,
    #[serde(rename = "driverTLA")]
    driver_tla: Option<String>,
    #[serde(rename = "driverTeamName")]
    driver_team_name: Option<String>,
    points: Option<f64>,
    gap: Option<String>,
    status: Option<String>,
}
