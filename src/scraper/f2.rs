use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::sportstimes::fetch_sportstimes_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

use serde::Deserialize;

pub struct F2Scraper;

impl StandingsFetcher for F2Scraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = create_http_client()?;
        let driver_url = "https://www.fiaformula2.com/Standings/Driver";
        let mut drivers = Vec::new();
        let mut constructors = Vec::new();

        if let Ok(resp) = client.get(driver_url).send().await {
            if let Ok(html) = resp.text().await {
                drivers = parse_f2_standings_html(&html).unwrap_or_default();
            }
        }

        let team_url = "https://www.fiaformula2.com/Standings/Team";
        if let Ok(resp) = client.get(team_url).send().await {
            if let Ok(html) = resp.text().await {
                constructors = parse_f2_team_standings_html(&html).unwrap_or_default();
            }
        }

        Ok(SeasonStandings {
            series_id: "f2".to_string(),
            season,
            drivers,
            constructors,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse F2 driver standings from page content / embedded JSON.
pub fn parse_f2_standings_html(html: &str) -> Result<Vec<crate::data::standings::DriverStanding>> {
    let unescaped = html.replace("\\\"", "\"").replace("\\\\", "\\");
    if let Some(start_idx) = unescaped.find("\"driverFirstName\"") {
        if let Some(arr_start) = unescaped[..start_idx].rfind('[') {
            if let Some(arr_end) = unescaped[arr_start..].find(']') {
                let json_slice = &unescaped[arr_start..=arr_start + arr_end];
                if let Ok(entries) = serde_json::from_str::<Vec<F2RawDriverEntry>>(json_slice) {
                    let drivers = entries
                        .into_iter()
                        .enumerate()
                        .map(|(i, d)| {
                            let first = d.driver_first_name.unwrap_or_default();
                            let last = d.driver_last_name.unwrap_or_default();
                            let full_name = format!("{} {}", first, last).trim().to_string();
                            let pos = d
                                .display_position
                                .as_deref()
                                .and_then(|p| p.parse::<u32>().ok())
                                .unwrap_or((i + 1) as u32);
                            crate::data::standings::DriverStanding {
                                position: pos,
                                driver_name: if full_name.is_empty() {
                                    d.driver_short_name.unwrap_or_else(|| "Unknown".to_string())
                                } else {
                                    full_name
                                },
                                driver_code: d.driver_tla,
                                driver_number: None,
                                team: "".to_string(),
                                points: d.championship_points.unwrap_or(0.0),
                                wins: 0,
                            }
                        })
                        .collect();
                    return Ok(drivers);
                }
            }
        }
    }

    Ok(Vec::new())
}

/// Parse F2 team standings from page content / embedded JSON.
pub fn parse_f2_team_standings_html(
    html: &str,
) -> Result<Vec<crate::data::standings::ConstructorStanding>> {
    let unescaped = html.replace("\\\"", "\"").replace("\\\\", "\\");
    if let Some(start_idx) = unescaped.find("\"teamName\"") {
        if let Some(arr_start) = unescaped[..start_idx].rfind('[') {
            if let Some(arr_end) = unescaped[arr_start..].find(']') {
                let json_slice = &unescaped[arr_start..=arr_start + arr_end];
                if let Ok(entries) = serde_json::from_str::<Vec<F2RawTeamEntry>>(json_slice) {
                    let teams = entries
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| {
                            let pos = t
                                .display_position
                                .as_deref()
                                .and_then(|p| p.parse::<u32>().ok())
                                .unwrap_or((i + 1) as u32);
                            crate::data::standings::ConstructorStanding {
                                position: pos,
                                name: t.team_name.unwrap_or_else(|| "Unknown".to_string()),
                                points: t.championship_points.unwrap_or(0.0),
                                wins: 0,
                            }
                        })
                        .collect();
                    return Ok(teams);
                }
            }
        }
    }

    Ok(Vec::new())
}

#[derive(Debug, Deserialize)]
struct F2RawDriverEntry {
    #[serde(rename = "displayPosition")]
    display_position: Option<String>,
    #[serde(rename = "championshipPoints")]
    championship_points: Option<f64>,
    #[serde(rename = "driverTLA")]
    driver_tla: Option<String>,
    #[serde(rename = "driverFirstName")]
    driver_first_name: Option<String>,
    #[serde(rename = "driverLastName")]
    driver_last_name: Option<String>,
    #[serde(rename = "driverShortName")]
    driver_short_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct F2RawTeamEntry {
    #[serde(rename = "displayPosition")]
    display_position: Option<String>,
    #[serde(rename = "championshipPoints")]
    championship_points: Option<f64>,
    #[serde(rename = "teamName")]
    team_name: Option<String>,
}

impl ResultsFetcher for F2Scraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "F2 race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for F2Scraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_sportstimes_calendar(
            &client,
            "https://f2calendar.com",
            &series.id,
            &f2_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_f2_schedule(&series.id);
            events.extend(get_official_2027_f2_schedule(&series.id));
        } else if !events.iter().any(|e| e.start_date.year() == 2027) {
            events.extend(get_official_2027_f2_schedule(&series.id));
        }

        Ok(events)
    }
}

fn f2_stream_links() -> Vec<StreamLink> {
    vec![StreamLink {
        platform: "F1TV".to_string(),
        url: "https://f1tv.formula1.com/".to_string(),
        access: StreamAccess::Paid,
    }]
}

pub fn get_official_2026_f2_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Albert Park Circuit",
            "Melbourne",
            "Australia",
            (2026, 3, 13),
            (2026, 3, 15),
        ),
        (
            "Bahrain International Circuit",
            "Sakhir",
            "Bahrain",
            (2026, 3, 27),
            (2026, 3, 29),
        ),
        (
            "Jeddah Corniche Circuit",
            "Jeddah",
            "Saudi Arabia",
            (2026, 4, 10),
            (2026, 4, 12),
        ),
        (
            "Autodromo Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2026, 5, 15),
            (2026, 5, 17),
        ),
        (
            "Circuit de Monaco",
            "Monte Carlo",
            "Monaco",
            (2026, 5, 22),
            (2026, 5, 24),
        ),
        (
            "Circuit de Barcelona-Catalunya",
            "Barcelona",
            "Spain",
            (2026, 6, 5),
            (2026, 6, 7),
        ),
        (
            "Red Bull Ring",
            "Spielberg",
            "Austria",
            (2026, 6, 26),
            (2026, 6, 28),
        ),
        (
            "Silverstone Circuit",
            "Silverstone",
            "UK",
            (2026, 7, 3),
            (2026, 7, 5),
        ),
        (
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2026, 7, 24),
            (2026, 7, 26),
        ),
        (
            "Hungaroring",
            "Budapest",
            "Hungary",
            (2026, 7, 31),
            (2026, 8, 2),
        ),
        (
            "Autodromo Nazionale Monza",
            "Monza",
            "Italy",
            (2026, 9, 4),
            (2026, 9, 6),
        ),
        (
            "Baku City Circuit",
            "Baku",
            "Azerbaijan",
            (2026, 9, 18),
            (2026, 9, 20),
        ),
        (
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2026, 11, 27),
            (2026, 11, 29),
        ),
        (
            "Yas Marina Circuit",
            "Abu Dhabi",
            "UAE",
            (2026, 12, 4),
            (2026, 12, 6),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, start, end))| {
            let fri_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();
            let sat_date = sun_date.pred_opt().unwrap_or(sun_date);

            let status = if sun_date < today {
                EventStatus::Completed
            } else if fri_date <= today && today <= sun_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let quali_time = fri_date
                .and_hms_opt(14, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let sprint_time = sat_date
                .and_hms_opt(13, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let feature_time = sun_date
                .and_hms_opt(9, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Sprint Race".to_string(),
                    session_type: SessionType::Sprint,
                    start_time: sprint_time,
                    end_time: None,
                },
                Session {
                    name: "Feature Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: feature_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Formula 2 at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sun_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: f2_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_f2_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Albert Park Circuit",
            "Melbourne",
            "Australia",
            (2027, 3, 12),
            (2027, 3, 14),
        ),
        (
            "Bahrain International Circuit",
            "Sakhir",
            "Bahrain",
            (2027, 4, 9),
            (2027, 4, 11),
        ),
        (
            "Jeddah Corniche Circuit",
            "Jeddah",
            "Saudi Arabia",
            (2027, 4, 16),
            (2027, 4, 18),
        ),
        (
            "Autodromo Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2027, 5, 14),
            (2027, 5, 16),
        ),
        (
            "Circuit de Monaco",
            "Monte Carlo",
            "Monaco",
            (2027, 5, 21),
            (2027, 5, 23),
        ),
        (
            "Circuit de Barcelona-Catalunya",
            "Barcelona",
            "Spain",
            (2027, 5, 28),
            (2027, 5, 30),
        ),
        (
            "Red Bull Ring",
            "Spielberg",
            "Austria",
            (2027, 6, 25),
            (2027, 6, 27),
        ),
        (
            "Silverstone Circuit",
            "Silverstone",
            "UK",
            (2027, 7, 2),
            (2027, 7, 4),
        ),
        (
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2027, 7, 23),
            (2027, 7, 25),
        ),
        (
            "Hungaroring",
            "Budapest",
            "Hungary",
            (2027, 7, 30),
            (2027, 8, 1),
        ),
        (
            "Autodromo Nazionale Monza",
            "Monza",
            "Italy",
            (2027, 9, 3),
            (2027, 9, 5),
        ),
        (
            "Baku City Circuit",
            "Baku",
            "Azerbaijan",
            (2027, 9, 17),
            (2027, 9, 19),
        ),
        (
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2027, 11, 26),
            (2027, 11, 28),
        ),
        (
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
        .map(|(i, (circuit, loc, country, start, end))| {
            let fri_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();
            let sat_date = sun_date.pred_opt().unwrap_or(sun_date);

            let quali_time = fri_date
                .and_hms_opt(14, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let sprint_time = sat_date
                .and_hms_opt(13, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let feature_time = sun_date
                .and_hms_opt(9, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Sprint Race".to_string(),
                    session_type: SessionType::Sprint,
                    start_time: sprint_time,
                    end_time: None,
                },
                Session {
                    name: "Feature Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: feature_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Formula 2 at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: fri_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: f2_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_f2_schedule() {
        let events = get_official_2026_f2_schedule("f2");
        assert_eq!(events.len(), 14);
        assert_eq!(events[0].event_name, "Formula 2 at Melbourne");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[13].event_name, "Formula 2 at Abu Dhabi");
        assert_eq!(events[13].round, Some(14));
    }

    #[test]
    fn test_parse_f2_standings_html() {
        let sample = r#"
            self.__next_f.push([1,"[{\"position\":\"1st\",\"displayPosition\":\"1\",\"championshipPoints\":140,\"driverFirstName\":\"Rafael\",\"driverLastName\":\"Camara\",\"driverTLA\":\"CAM\"}],\"category\":\"Driver\""]);
        "#;
        let drivers = parse_f2_standings_html(sample).unwrap();
        assert_eq!(drivers.len(), 1);
        assert_eq!(drivers[0].driver_name, "Rafael Camara");
        assert_eq!(drivers[0].driver_code.as_deref(), Some("CAM"));
        assert_eq!(drivers[0].points, 140.0);

        let team_sample = r#"
            self.__next_f.push([1,"[{\"position\":\"1st\",\"displayPosition\":\"1\",\"championshipPoints\":210,\"teamName\":\"PREMA Racing\"}],\"category\":\"Team\""]);
        "#;
        let teams = parse_f2_team_standings_html(team_sample).unwrap();
        assert_eq!(teams.len(), 1);
        assert_eq!(teams[0].name, "PREMA Racing");
        assert_eq!(teams[0].points, 210.0);
    }
}
