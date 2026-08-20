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

pub struct FormulaEScraper;

impl StandingsFetcher for FormulaEScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "formula_e".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for FormulaEScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "Formula E race results for season {} round {} not yet available",
            season,
            round
        )
    }
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
}
