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

pub struct BsbScraper;

impl StandingsFetcher for BsbScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "bsb".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for BsbScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "BSB race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for BsbScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/bsb",
            &series.id,
            &bsb_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_bsb_schedule(&series.id, 2026);
            events.extend(get_official_bsb_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_bsb_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn bsb_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Discovery+ / TNT Sports".to_string(),
            url: "https://www.discoveryplus.com/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "Eurosport".to_string(),
            url: "https://www.eurosport.com/".to_string(),
            access: StreamAccess::Paid,
        },
    ]
}

pub fn get_official_bsb_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        ("Circuito de Navarra", "Navarra", "Spain", (4, 18), (4, 19)),
        ("Oulton Park (Sprint)", "Oulton Park", "UK", (5, 2), (5, 4)),
        (
            "Donington Park (National)",
            "Donington Park",
            "UK",
            (5, 16),
            (5, 17),
        ),
        ("Snetterton (300)", "Snetterton", "UK", (6, 20), (6, 21)),
        ("Knockhill", "Knockhill", "UK", (7, 4), (7, 5)),
        ("Brands Hatch (GP)", "Brands Hatch", "UK", (7, 25), (7, 26)),
        ("Thruxton", "Thruxton", "UK", (8, 8), (8, 9)),
        ("Cadwell Park", "Cadwell Park", "UK", (8, 29), (8, 31)),
        (
            "Oulton Park (Showdown)",
            "Oulton Park",
            "UK",
            (9, 12),
            (9, 13),
        ),
        (
            "Donington Park (GP Showdown)",
            "Donington Park",
            "UK",
            (10, 3),
            (10, 4),
        ),
        (
            "Brands Hatch (GP Finale)",
            "Brands Hatch",
            "UK",
            (10, 17),
            (10, 18),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, start, end))| {
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
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Sprint Race".to_string(),
                    session_type: SessionType::Sprint,
                    start_time: sat_date
                        .and_hms_opt(15, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 2".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(12, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 3".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(15, 45, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("BSB at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: bsb_stream_links(),
                status,
            }
        })
        .collect()
}
