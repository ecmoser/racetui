use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct TcrScraper;

impl SeriesScraper for TcrScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/tcr",
            &series.id,
            &tcr_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_tcr_schedule(&series.id, 2026);
            events.extend(get_official_tcr_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_tcr_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn tcr_stream_links() -> Vec<StreamLink> {
    vec![StreamLink {
        platform: "YouTube (TCR TV)".to_string(),
        url: "https://www.youtube.com/@TCRTV".to_string(),
        access: StreamAccess::Free,
    }]
}

pub fn get_official_tcr_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "TCR Vallelunga",
            "Autodromo Vallelunga Piero Taruffi",
            "Italy",
            (4, 18),
            (4, 19),
        ),
        (
            "TCR Marrakech",
            "Circuit International Automobile Moulay El Hassan",
            "Morocco",
            (5, 2),
            (5, 3),
        ),
        (
            "TCR Mid-Ohio",
            "Mid-Ohio Sports Car Course",
            "USA",
            (6, 6),
            (6, 7),
        ),
        (
            "TCR Interlagos",
            "Autódromo José Carlos Pace",
            "Brazil",
            (7, 18),
            (7, 19),
        ),
        (
            "TCR El Pinar",
            "Autódromo Víctor Borrat Fabini",
            "Uruguay",
            (8, 1),
            (8, 2),
        ),
        (
            "TCR Zhuzhou",
            "Zhuzhou International Circuit",
            "China",
            (10, 17),
            (10, 18),
        ),
        (
            "TCR Macau Guia Race",
            "Guia Circuit",
            "Macau",
            (11, 14),
            (11, 15),
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
                        .and_hms_opt(11, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 1".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date
                        .and_hms_opt(15, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 2".to_string(),
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
                circuit_name: circuit.to_string(),
                location: country.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: tcr_stream_links(),
                status,
            }
        })
        .collect()
}
