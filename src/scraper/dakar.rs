use anyhow::Result;
use chrono::{Datelike, DateTime, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct DakarScraper;

impl SeriesScraper for DakarScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/dakar",
            &series.id,
            &dakar_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_dakar_schedule(&series.id, 2026);
            events.extend(get_official_dakar_schedule(&series.id, 2027));
        } else if !events.iter().any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027) {
            events.extend(get_official_dakar_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn dakar_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Red Bull TV".to_string(),
            url: "https://www.redbull.com/tv/dakar".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "Eurosport / Discovery+".to_string(),
            url: "https://www.discoveryplus.com/".to_string(),
            access: StreamAccess::Paid,
        },
    ]
}

pub fn get_official_dakar_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();
    let start_date = NaiveDate::from_ymd_opt(year, 1, 3).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 1, 16).unwrap();

    let status = if end_date < today {
        EventStatus::Completed
    } else if start_date <= today && today <= end_date {
        EventStatus::Live
    } else {
        EventStatus::Upcoming
    };

    let sessions = vec![
        Session {
            name: "Prologue / Stage 1".to_string(),
            session_type: SessionType::Practice,
            start_time: start_date.and_hms_opt(5, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            end_time: None,
        },
        Session {
            name: "Marathon Stages (1-12)".to_string(),
            session_type: SessionType::Other("Rally Raid".to_string()),
            start_time: start_date.succ_opt().unwrap_or(start_date).and_hms_opt(4, 30, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            end_time: None,
        },
        Session {
            name: "Final Stage & Podium".to_string(),
            session_type: SessionType::Race,
            start_time: end_date.and_hms_opt(6, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            end_time: None,
        },
    ];

    vec![RaceEvent {
        series_id: series_id.to_string(),
        event_name: format!("Dakar Rally {}", year),
        circuit_name: "Saudi Arabia Desert Trail".to_string(),
        location: "Bisha to Shubaytah".to_string(),
        country: "Saudi Arabia".to_string(),
        start_date,
        end_date,
        round: Some(1),
        sessions,
        stream_links: dakar_stream_links(),
        status,
    }]
}
