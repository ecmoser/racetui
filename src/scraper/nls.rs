use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct NlsScraper;

impl SeriesScraper for NlsScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/nls",
            &series.id,
            &nls_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_nls_schedule(&series.id, 2026);
            events.extend(get_official_nls_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_nls_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn nls_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "YouTube (Nürburgring Nordschleife)".to_string(),
            url: "https://www.youtube.com/@VLNOFFICIAL".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "NLS Official".to_string(),
            url: "https://www.nuerburgring-langstrecken-serie.de/en/live/".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_nls_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        ("65. ADAC Westfalenfahrt (NLS 1)", (3, 21)),
        ("49. DMV 4-Stunden-Rennen (NLS 2)", (4, 4)),
        ("57. ADAC Barbarossapreis (NLS 3)", (6, 20)),
        ("48. RCM DMV Grenzlandrennen (NLS 4)", (7, 11)),
        ("6 Hour ADAC Ruhr-Pokal-Rennen (NLS 5)", (8, 1)),
        ("56. ADAC Reinoldus-Langstreckenrennen (NLS 6)", (9, 12)),
        ("57. ADAC Barbarossapreis (NLS 7)", (9, 26)),
        ("49. DMV Münsterlandpokal (NLS 8)", (10, 10)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, (m, d)))| {
            let race_date = NaiveDate::from_ymd_opt(year, m, d).unwrap();

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
                    start_time: race_date
                        .and_hms_opt(6, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race (4h / 6h)".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_date
                        .and_hms_opt(10, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: "Nürburgring Nordschleife".to_string(),
                location: "Nürburg".to_string(),
                country: "Germany".to_string(),
                start_date: race_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: nls_stream_links(),
                status,
            }
        })
        .collect()
}
