use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::sportstimes::fetch_sportstimes_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct F1AcademyScraper;

impl SeriesScraper for F1AcademyScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_sportstimes_calendar(
            &client,
            "https://f1calendar.com",
            &series.id,
            &f1_academy_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_f1_academy_schedule(&series.id, 2026);
            events.extend(get_official_f1_academy_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_f1_academy_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn f1_academy_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "YouTube (F1 Academy)".to_string(),
            url: "https://www.youtube.com/@f1academy".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "F1TV".to_string(),
            url: "https://f1tv.formula1.com/".to_string(),
            access: StreamAccess::Paid,
        },
    ]
}

pub fn get_official_f1_academy_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Shanghai International Circuit",
            "Shanghai",
            "China",
            (3, 20),
            (3, 22),
        ),
        (
            "Jeddah Corniche Circuit",
            "Jeddah",
            "Saudi Arabia",
            (4, 17),
            (4, 19),
        ),
        (
            "Miami International Autodrome",
            "Miami",
            "USA",
            (5, 1),
            (5, 3),
        ),
        (
            "Circuit Gilles Villeneuve",
            "Montreal",
            "Canada",
            (6, 12),
            (6, 14),
        ),
        (
            "Circuit Zandvoort",
            "Zandvoort",
            "Netherlands",
            (8, 28),
            (8, 30),
        ),
        (
            "Marina Bay Street Circuit",
            "Singapore",
            "Singapore",
            (10, 2),
            (10, 4),
        ),
        (
            "Las Vegas Strip Circuit",
            "Las Vegas",
            "USA",
            (11, 19),
            (11, 21),
        ),
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
                    start_time: fri_date
                        .and_hms_opt(10, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: fri_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 1".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date
                        .and_hms_opt(12, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 2".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(10, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("F1 Academy at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: fri_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: f1_academy_stream_links(),
                status,
            }
        })
        .collect()
}
