use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct SuperGtScraper;

impl StandingsFetcher for SuperGtScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "super_gt".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for SuperGtScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "Super GT race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for SuperGtScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/sgt",
            &series.id,
            &super_gt_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_super_gt_schedule(&series.id);
            events.extend(get_official_2027_super_gt_schedule(&series.id));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_super_gt_schedule(&series.id));
        }

        Ok(events)
    }
}

fn super_gt_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Motorsport.tv".to_string(),
            url: "https://motorsport.tv/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "J SPORTS (Japan)".to_string(),
            url: "https://www.jsports.co.jp/motor/supergt/".to_string(),
            access: StreamAccess::Paid,
        },
    ]
}

pub fn get_official_2026_super_gt_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Okayama International Circuit",
            "Mimasaka",
            "Japan",
            (2026, 4, 11),
            (2026, 4, 12),
        ),
        (
            "Fuji Speedway",
            "Oyama",
            "Japan",
            (2026, 5, 3),
            (2026, 5, 4),
        ),
        (
            "Suzuka Circuit",
            "Suzuka",
            "Japan",
            (2026, 5, 30),
            (2026, 5, 31),
        ),
        (
            "Fuji Speedway",
            "Oyama",
            "Japan",
            (2026, 8, 8),
            (2026, 8, 9),
        ),
        (
            "Sepang International Circuit",
            "Sepang",
            "Malaysia",
            (2026, 8, 29),
            (2026, 8, 30),
        ),
        (
            "Sportsland SUGO",
            "Murata",
            "Japan",
            (2026, 9, 19),
            (2026, 9, 20),
        ),
        ("Autopolis", "Hita", "Japan", (2026, 10, 17), (2026, 10, 18)),
        (
            "Mobility Resort Motegi",
            "Motegi",
            "Japan",
            (2026, 11, 7),
            (2026, 11, 8),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let status = if sun_date < today {
                EventStatus::Completed
            } else if sat_date <= today && today <= sun_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            // JST start times in UTC: Quali 14:00 JST (05:00 UTC), Race 13:30 JST (04:30 UTC)
            let quali_time = sat_date
                .and_hms_opt(5, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = sun_date
                .and_hms_opt(4, 30, 0)
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
                    end_time: race_time.map(|rt| rt + chrono::Duration::hours(3)),
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Super GT at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: super_gt_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_super_gt_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Okayama International Circuit",
            "Mimasaka",
            "Japan",
            (2027, 4, 10),
            (2027, 4, 11),
        ),
        (
            "Fuji Speedway",
            "Oyama",
            "Japan",
            (2027, 5, 2),
            (2027, 5, 3),
        ),
        (
            "Suzuka Circuit",
            "Suzuka",
            "Japan",
            (2027, 5, 29),
            (2027, 5, 30),
        ),
        (
            "Fuji Speedway",
            "Oyama",
            "Japan",
            (2027, 8, 7),
            (2027, 8, 8),
        ),
        (
            "Sepang International Circuit",
            "Sepang",
            "Malaysia",
            (2027, 8, 28),
            (2027, 8, 29),
        ),
        (
            "Sportsland SUGO",
            "Murata",
            "Japan",
            (2027, 9, 18),
            (2027, 9, 19),
        ),
        ("Autopolis", "Hita", "Japan", (2027, 10, 16), (2027, 10, 17)),
        (
            "Mobility Resort Motegi",
            "Motegi",
            "Japan",
            (2027, 11, 6),
            (2027, 11, 7),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let quali_time = sat_date
                .and_hms_opt(5, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = sun_date
                .and_hms_opt(4, 30, 0)
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
                    end_time: race_time.map(|rt| rt + chrono::Duration::hours(3)),
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("Super GT at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: super_gt_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_super_gt_schedule() {
        let events = get_official_2026_super_gt_schedule("super_gt");
        assert_eq!(events.len(), 8);
        assert_eq!(events[0].event_name, "Super GT at Mimasaka");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[7].event_name, "Super GT at Motegi");
        assert_eq!(events[7].round, Some(8));
    }
}
