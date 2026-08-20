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

pub struct WorldSbkScraper;

impl StandingsFetcher for WorldSbkScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "worldsbk".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for WorldSbkScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "WorldSBK race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for WorldSbkScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/wsbk",
            &series.id,
            &worldsbk_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_worldsbk_schedule(&series.id, 2026);
            events.extend(get_official_worldsbk_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_worldsbk_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn worldsbk_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "WorldSBK VideoPass".to_string(),
            url: "https://www.worldsbk.com/en/videopass".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "Eurosport / Discovery+".to_string(),
            url: "https://www.discoveryplus.com/".to_string(),
            access: StreamAccess::Paid,
        },
    ]
}

pub fn get_official_worldsbk_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Australian Round",
            "Phillip Island Grand Prix Circuit",
            "Australia",
            (2, 21),
            (2, 22),
        ),
        (
            "Portuguese Round",
            "Autódromo Internacional do Algarve",
            "Portugal",
            (3, 28),
            (3, 29),
        ),
        (
            "Dutch Round",
            "TT Circuit Assen",
            "Netherlands",
            (4, 18),
            (4, 19),
        ),
        (
            "Italian Round",
            "Misano World Circuit",
            "Italy",
            (6, 13),
            (6, 14),
        ),
        ("UK Round", "Donington Park", "UK", (7, 11), (7, 12)),
        (
            "Czech Round",
            "Autodrom Most",
            "Czech Republic",
            (7, 25),
            (7, 26),
        ),
        (
            "Hungarian Round",
            "Balaton Park Circuit",
            "Hungary",
            (8, 8),
            (8, 9),
        ),
        (
            "French Round",
            "Circuit de Nevers Magny-Cours",
            "France",
            (9, 5),
            (9, 6),
        ),
        (
            "Emilia-Romagna Round",
            "Autodromo Enzo e Dino Ferrari",
            "Italy",
            (9, 19),
            (9, 20),
        ),
        (
            "Spanish Round",
            "MotorLand Aragón",
            "Spain",
            (9, 26),
            (9, 27),
        ),
        (
            "Portuguese Round 2",
            "Circuito do Estoril",
            "Portugal",
            (10, 10),
            (10, 11),
        ),
        (
            "Spanish Round Finale",
            "Circuito de Jerez",
            "Spain",
            (10, 17),
            (10, 18),
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
                    name: "Superpole (Qualifying)".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(10, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 1".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Superpole Race".to_string(),
                    session_type: SessionType::Sprint,
                    start_time: sun_date
                        .and_hms_opt(10, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 2".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("WorldSBK — {}", name),
                circuit_name: circuit.to_string(),
                location: country.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: worldsbk_stream_links(),
                status,
            }
        })
        .collect()
}
