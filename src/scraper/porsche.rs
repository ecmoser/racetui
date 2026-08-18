use anyhow::Result;
use chrono::{Datelike, DateTime, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::sportstimes::fetch_sportstimes_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct PorscheScraper;

impl SeriesScraper for PorscheScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_sportstimes_calendar(
            &client,
            "https://f1calendar.com",
            &series.id,
            &porsche_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_porsche_schedule(&series.id, 2026);
            events.extend(get_official_porsche_schedule(&series.id, 2027));
        } else if !events.iter().any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027) {
            events.extend(get_official_porsche_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn porsche_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "F1TV".to_string(),
            url: "https://f1tv.formula1.com/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "Porsche Motorsport".to_string(),
            url: "https://motorsports.porsche.com/".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_porsche_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        ("Autodromo Enzo e Dino Ferrari", "Imola", "Italy", (5, 15), (5, 17)),
        ("Circuit de Monaco", "Monte Carlo", "Monaco", (5, 22), (5, 24)),
        ("Red Bull Ring", "Spielberg", "Austria", (6, 26), (6, 28)),
        ("Silverstone Circuit", "Silverstone", "UK", (7, 3), (7, 5)),
        ("Circuit de Spa-Francorchamps", "Spa-Francorchamps", "Belgium", (7, 24), (7, 26)),
        ("Hungaroring", "Budapest", "Hungary", (7, 31), (8, 2)),
        ("Circuit Zandvoort", "Zandvoort", "Netherlands", (8, 28), (8, 30)),
        ("Autodromo Nazionale Monza", "Monza", "Italy", (9, 4), (9, 6)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, start, end))| {
            let fri_date = NaiveDate::from_ymd_opt(year, start.0, start.1).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(year, end.0, end.1).unwrap();
            let sat_date = sun_date.pred_opt().unwrap_or(sun_date);

            let status = if sun_date < today {
                EventStatus::Completed
            } else if fri_date <= today && today <= sun_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let sessions = vec![
                Session {
                    name: "Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fri_date.and_hms_opt(16, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date.and_hms_opt(10, 20, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date.and_hms_opt(10, 45, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Porsche Supercup at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: fri_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: porsche_stream_links(),
                status,
            }
        })
        .collect()
}
