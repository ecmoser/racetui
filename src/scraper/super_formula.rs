use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct SuperFormulaScraper;

impl SeriesScraper for SuperFormulaScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/sf",
            &series.id,
            &super_formula_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_super_formula_schedule(&series.id);
            events.extend(get_official_2027_super_formula_schedule(&series.id));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_super_formula_schedule(&series.id));
        }

        Ok(events)
    }
}

fn super_formula_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Motorsport.tv / SFgo".to_string(),
            url: "https://motorsport.tv/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "YouTube (SF Official)".to_string(),
            url: "https://www.youtube.com/@superformulavideo".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_2026_super_formula_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Mobility Resort Motegi",
            "Motegi",
            (2026, 4, 4),
            (2026, 4, 5),
        ),
        ("Autopolis", "Hita", (2026, 5, 16), (2026, 5, 17)),
        ("Sportsland SUGO", "Murata", (2026, 6, 20), (2026, 6, 21)),
        ("Fuji Speedway", "Oyama", (2026, 7, 18), (2026, 7, 19)),
        (
            "Mobility Resort Motegi",
            "Motegi",
            (2026, 8, 22),
            (2026, 8, 23),
        ),
        ("Fuji Speedway", "Oyama", (2026, 10, 10), (2026, 10, 11)),
        ("Suzuka Circuit", "Suzuka", (2026, 11, 21), (2026, 11, 22)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let status = if sun_date < today {
                EventStatus::Completed
            } else if sat_date <= today && today <= sun_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            // JST start times in UTC: Quali 09:00 JST (00:00 UTC), Race 14:30 JST (05:30 UTC)
            let quali_time = sat_date
                .and_hms_opt(0, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = sun_date
                .and_hms_opt(5, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_time,
                    end_time: race_time.map(|rt| rt + chrono::Duration::hours(2)),
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Super Formula at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: "Japan".to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: super_formula_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_super_formula_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Mobility Resort Motegi",
            "Motegi",
            (2027, 4, 3),
            (2027, 4, 4),
        ),
        ("Autopolis", "Hita", (2027, 5, 15), (2027, 5, 16)),
        ("Sportsland SUGO", "Murata", (2027, 6, 19), (2027, 6, 20)),
        ("Fuji Speedway", "Oyama", (2027, 7, 17), (2027, 7, 18)),
        (
            "Mobility Resort Motegi",
            "Motegi",
            (2027, 8, 21),
            (2027, 8, 22),
        ),
        ("Fuji Speedway", "Oyama", (2027, 10, 9), (2027, 10, 10)),
        ("Suzuka Circuit", "Suzuka", (2027, 11, 20), (2027, 11, 21)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let quali_time = sat_date
                .and_hms_opt(0, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = sun_date
                .and_hms_opt(5, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_time,
                    end_time: race_time.map(|rt| rt + chrono::Duration::hours(2)),
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Super Formula at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: "Japan".to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: super_formula_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_super_formula_schedule() {
        let events = get_official_2026_super_formula_schedule("super_formula");
        assert_eq!(events.len(), 7);
        assert_eq!(events[0].event_name, "Super Formula at Motegi");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[6].event_name, "Super Formula at Suzuka");
        assert_eq!(events[6].round, Some(7));
    }
}
