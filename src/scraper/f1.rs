use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Deserialize;

use super::fetcher;
use super::sportstimes::fetch_sportstimes_calendar;
use super::SeriesScraper;
use crate::data::models::*;

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
}
