use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct SupercarsScraper;

impl StandingsFetcher for SupercarsScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "supercars".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for SupercarsScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "Supercars race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for SupercarsScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/supercars",
            &series.id,
            &supercars_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_supercars_schedule(&series.id, 2026);
            events.extend(get_official_supercars_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_supercars_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn supercars_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "SuperView (International)".to_string(),
            url: "https://www.supercars.com/superview/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "Fox Sports / Kayo (Australia)".to_string(),
            url: "https://kayosports.com.au/".to_string(),
            access: StreamAccess::Paid,
        },
    ]
}

pub fn get_official_supercars_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Sydney 500",
            "Sydney Motorsport Park",
            "Australia",
            (2, 21),
            (2, 22),
        ),
        (
            "Melbourne SuperSprint (F1 Support)",
            "Albert Park Circuit",
            "Australia",
            (3, 7),
            (3, 8),
        ),
        (
            "Taupo Super400",
            "Taupo International Motorsport Park",
            "New Zealand",
            (4, 11),
            (4, 12),
        ),
        (
            "Tasmania SuperSprint",
            "Symmons Plains Raceway",
            "Australia",
            (5, 9),
            (5, 10),
        ),
        (
            "Perth SuperSprint",
            "Carco.com.au Raceway (Wanneroo)",
            "Australia",
            (6, 6),
            (6, 7),
        ),
        (
            "Darwin Triple Crown",
            "Hidden Valley Raceway",
            "Australia",
            (6, 20),
            (6, 21),
        ),
        (
            "Townsville 500",
            "Reid Park Street Circuit",
            "Australia",
            (7, 11),
            (7, 12),
        ),
        (
            "Ipswich SuperSprint",
            "Queensland Raceway",
            "Australia",
            (8, 8),
            (8, 9),
        ),
        (
            "The Bend 500 (Enduro)",
            "Shell V-Power Motorsport Park",
            "Australia",
            (9, 12),
            (9, 13),
        ),
        (
            "Repco Bathurst 1000",
            "Mount Panorama Circuit",
            "Australia",
            (10, 10),
            (10, 11),
        ),
        (
            "Boost Mobile Gold Coast 500",
            "Surfers Paradise Street Circuit",
            "Australia",
            (10, 24),
            (10, 25),
        ),
        (
            "VAILO Adelaide 500",
            "Adelaide Street Circuit",
            "Australia",
            (11, 28),
            (11, 29),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, country, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(year, start.0, start.1).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(year, end.0, end.1).unwrap();

            let status = if sun_date < today {
                EventStatus::Completed
            } else if sat_date <= today && today <= sun_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(3, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 1".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date
                        .and_hms_opt(5, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 2".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(5, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Supercars — {}", name),
                circuit_name: circuit.to_string(),
                location: country.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: supercars_stream_links(),
                status,
            }
        })
        .collect()
}
