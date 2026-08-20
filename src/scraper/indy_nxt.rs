use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::sportstimes::fetch_sportstimes_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct IndyNxtScraper;

impl SeriesScraper for IndyNxtScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_sportstimes_calendar(
            &client,
            "https://indycarcalendar.com",
            &series.id,
            &indy_nxt_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_indy_nxt_schedule(&series.id, 2026);
            events.extend(get_official_indy_nxt_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_indy_nxt_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn indy_nxt_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Peacock / INDYCAR LIVE".to_string(),
            url: "https://www.peacocktv.com/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "INDY NXT".to_string(),
            url: "https://www.indynxt.com/".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_indy_nxt_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        ("Streets of St. Petersburg", "St. Petersburg", "USA", (3, 1)),
        ("Barber Motorsports Park", "Birmingham", "USA", (5, 3)),
        (
            "Indianapolis Motor Speedway (Road Course)",
            "Indianapolis",
            "USA",
            (5, 15),
        ),
        ("Streets of Detroit", "Detroit", "USA", (6, 7)),
        ("Road America", "Elkhart Lake", "USA", (6, 28)),
        ("Mid-Ohio Sports Car Course", "Lexington", "USA", (7, 12)),
        ("Iowa Speedway", "Newton", "USA", (7, 18)),
        ("Exhibition Place", "Toronto", "Canada", (7, 26)),
        ("World Wide Technology Raceway", "Madison", "USA", (8, 9)),
        ("Portland International Raceway", "Portland", "USA", (8, 16)),
        ("Milwaukee Mile", "West Allis", "USA", (8, 30)),
        ("Nashville Superspeedway", "Lebanon", "USA", (9, 13)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, (m, d)))| {
            let race_date = NaiveDate::from_ymd_opt(year, m, d).unwrap();
            let sat_date = race_date.pred_opt().unwrap_or(race_date);

            let status = if race_date < today {
                EventStatus::Completed
            } else if sat_date <= today && today <= race_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let sessions = vec![
                Session {
                    name: "Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: sat_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(17, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_date
                        .and_hms_opt(16, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("INDY NXT at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: indy_nxt_stream_links(),
                status,
            }
        })
        .collect()
}
