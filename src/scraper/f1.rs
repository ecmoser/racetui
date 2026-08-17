use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Deserialize;

use super::fetcher;
use super::SeriesScraper;
use crate::data::models::*;

/// F1 scraper using the Jolpica API (successor to Ergast).
/// API endpoint: https://api.jolpi.ca/ergast/f1/current.json
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
fn convert_session(
    name: &str,
    session_type: SessionType,
    jolpica: &JolpicaSession,
) -> Session {
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
        let response: JolpicaResponse =
            fetcher::fetch_json(&client, &series.calendar_url)
                .await
                .context("Failed to fetch F1 calendar from Jolpica API")?;

        let mut events = Vec::new();

        for race in response.mr_data.race_table.races {
            let round: u32 = race.round.parse().unwrap_or(0);

            // Parse the race date
            let start_date = NaiveDate::parse_from_str(&race.date, "%Y-%m-%d")
                .with_context(|| format!("Failed to parse F1 race date: {}", race.date))?;

            // Build sessions list
            let mut sessions = Vec::new();

            if let Some(ref fp1) = race.first_practice {
                sessions.push(convert_session("Free Practice 1", SessionType::Practice, fp1));
            }
            if let Some(ref fp2) = race.second_practice {
                sessions.push(convert_session("Free Practice 2", SessionType::Practice, fp2));
            }
            if let Some(ref fp3) = race.third_practice {
                sessions.push(convert_session("Free Practice 3", SessionType::Practice, fp3));
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
                sessions.push(convert_session("Qualifying", SessionType::Qualifying, quali));
            }

            // The race itself
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: parse_datetime(&race.date, race.time.as_deref()),
                end_time: None,
            });

            // Determine event date range (earliest session to race day)
            let earliest_date = sessions
                .iter()
                .filter_map(|s| s.start_time)
                .min()
                .map(|dt| dt.date_naive())
                .unwrap_or(start_date);

            // Determine status based on dates
            let now = Utc::now();
            let status = if sessions.iter().any(|s| {
                s.start_time
                    .map_or(false, |t| {
                        t <= now && s.end_time.map_or(
                            t + chrono::Duration::hours(2) > now,
                            |e| e > now,
                        )
                    })
            }) {
                EventStatus::Live
            } else if start_date >= now.date_naive() {
                EventStatus::Upcoming
            } else {
                EventStatus::Completed
            };

            // F1TV stream link
            let stream_links = vec![StreamLink {
                platform: "F1TV".to_string(),
                url: "https://f1tv.formula1.com".to_string(),
                access: StreamAccess::Paid,
            }];

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
                stream_links,
                status,
            });
        }

        Ok(events)
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
}
