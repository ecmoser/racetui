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

pub struct ExtremeEScraper;

impl StandingsFetcher for ExtremeEScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "extreme_e".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for ExtremeEScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "Extreme E race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for ExtremeEScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/extreme-e",
            &series.id,
            &extreme_e_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_extreme_e_schedule(&series.id, 2026);
            events.extend(get_official_extreme_e_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_extreme_e_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn extreme_e_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "YouTube (Extreme E)".to_string(),
            url: "https://www.youtube.com/@ExtremeELive".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "ITVX / Discovery+".to_string(),
            url: "https://www.extreme-e.com/en/broadcast-info".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_extreme_e_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        ("Desert X-Prix", "Jeddah", "Saudi Arabia", (2, 14), (2, 15)),
        (
            "Hydro X-Prix",
            "Dumfries and Galloway",
            "UK",
            (5, 9),
            (5, 10),
        ),
        (
            "Island X-Prix (Round 1)",
            "Sardinia",
            "Italy",
            (9, 12),
            (9, 13),
        ),
        (
            "Island X-Prix (Round 2)",
            "Sardinia",
            "Italy",
            (9, 19),
            (9, 20),
        ),
        (
            "Valley X-Prix",
            "Phoenix, Arizona",
            "USA",
            (11, 21),
            (11, 22),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, loc, country, start, end))| {
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
                    name: "Qualifying 1 & 2".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(9, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Round 1 Grand Final".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Round 2 Grand Final".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: format!("{} Off-Road Course", loc),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: extreme_e_stream_links(),
                status,
            }
        })
        .collect()
}
