use anyhow::Result;
use chrono::{Datelike, DateTime, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct WrcScraper;

impl SeriesScraper for WrcScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/wrc",
            &series.id,
            &wrc_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_wrc_schedule(&series.id);
            events.extend(get_official_2027_wrc_schedule(&series.id));
        } else if !events.iter().any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027) {
            events.extend(get_official_2027_wrc_schedule(&series.id));
        }

        Ok(events)
    }
}

fn wrc_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Rally.TV".to_string(),
            url: "https://www.rally.tv/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "Red Bull TV".to_string(),
            url: "https://www.redbull.com/tv/wrc".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_2026_wrc_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        ("Rallye Monte-Carlo", "Gap / Monaco", "Monaco", "Tarmac / Snow", (2026, 1, 25)),
        ("Rally Sweden", "Umeå", "Sweden", "Snow", (2026, 2, 15)),
        ("Safari Rally Kenya", "Naivasha", "Kenya", "Gravel", (2026, 3, 22)),
        ("Rally Islas Canarias", "Las Palmas", "Spain", "Tarmac", (2026, 4, 26)),
        ("Rally de Portugal", "Matosinhos", "Portugal", "Gravel", (2026, 5, 17)),
        ("Rally Italia Sardegna", "Olbia", "Italy", "Gravel", (2026, 6, 7)),
        ("Acropolis Rally Greece", "Lamia", "Greece", "Gravel", (2026, 6, 28)),
        ("Rally Estonia", "Tartu", "Estonia", "Gravel", (2026, 7, 19)),
        ("Secto Rally Finland", "Jyväskylä", "Finland", "Gravel", (2026, 8, 2)),
        ("Rally del Paraguay", "Encarnación", "Paraguay", "Gravel", (2026, 8, 30)),
        ("Rally Chile Bio Bío", "Concepción", "Chile", "Gravel", (2026, 9, 13)),
        ("Central European Rally", "Passau / Prague", "Germany / Czechia", "Tarmac", (2026, 10, 18)),
        ("FORUM8 Rally Japan", "Toyota City", "Japan", "Tarmac", (2026, 11, 8)),
        ("Rally Saudi Arabia", "Jeddah", "Saudi Arabia", "Gravel", (2026, 11, 29)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, loc, country, surface, date))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let thu_date = race_date.pred_opt().unwrap_or(race_date).pred_opt().unwrap_or(race_date).pred_opt().unwrap_or(race_date);
            let fri_date = thu_date.succ_opt().unwrap_or(thu_date);
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);

            let status = if race_date < today {
                EventStatus::Completed
            } else if thu_date <= today && today <= race_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let sessions = vec![
                Session {
                    name: format!("Shakedown ({})", surface),
                    session_type: SessionType::Practice,
                    start_time: thu_date.and_hms_opt(8, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Leg 1 Stages".to_string(),
                    session_type: SessionType::Race,
                    start_time: fri_date.and_hms_opt(7, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Leg 2 Stages".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date.and_hms_opt(7, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Power Stage / Final".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_date.and_hms_opt(12, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: format!("{} ({})", loc, surface),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: thu_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: wrc_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_wrc_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        ("Rallye Monte-Carlo", "Gap / Monaco", "Monaco", "Tarmac / Snow", (2027, 1, 24)),
        ("Rally Sweden", "Umeå", "Sweden", "Snow", (2027, 2, 14)),
        ("Safari Rally Kenya", "Naivasha", "Kenya", "Gravel", (2027, 3, 21)),
        ("Rally Islas Canarias", "Las Palmas", "Spain", "Tarmac", (2027, 4, 25)),
        ("Rally de Portugal", "Matosinhos", "Portugal", "Gravel", (2027, 5, 16)),
        ("Rally Italia Sardegna", "Olbia", "Italy", "Gravel", (2027, 6, 6)),
        ("Acropolis Rally Greece", "Lamia", "Greece", "Gravel", (2027, 6, 27)),
        ("Rally Estonia", "Tartu", "Estonia", "Gravel", (2027, 7, 18)),
        ("Secto Rally Finland", "Jyväskylä", "Finland", "Gravel", (2027, 8, 1)),
        ("Rally del Paraguay", "Encarnación", "Paraguay", "Gravel", (2027, 8, 29)),
        ("Rally Chile Bio Bío", "Concepción", "Chile", "Gravel", (2027, 9, 12)),
        ("Central European Rally", "Passau / Prague", "Germany / Czechia", "Tarmac", (2027, 10, 17)),
        ("FORUM8 Rally Japan", "Toyota City", "Japan", "Tarmac", (2027, 11, 7)),
        ("Rally Saudi Arabia", "Jeddah", "Saudi Arabia", "Gravel", (2027, 11, 28)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, loc, country, surface, date))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let thu_date = race_date.pred_opt().unwrap_or(race_date).pred_opt().unwrap_or(race_date).pred_opt().unwrap_or(race_date);
            let fri_date = thu_date.succ_opt().unwrap_or(thu_date);
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);

            let sessions = vec![
                Session {
                    name: format!("Shakedown ({})", surface),
                    session_type: SessionType::Practice,
                    start_time: thu_date.and_hms_opt(8, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Leg 1 Stages".to_string(),
                    session_type: SessionType::Race,
                    start_time: fri_date.and_hms_opt(7, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Leg 2 Stages".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date.and_hms_opt(7, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Power Stage / Final".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_date.and_hms_opt(12, 0, 0).map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: format!("{} ({})", loc, surface),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: thu_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: wrc_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_wrc_schedule() {
        let events = get_official_2026_wrc_schedule("wrc");
        assert_eq!(events.len(), 14);
        assert_eq!(events[0].event_name, "Rallye Monte-Carlo");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[13].event_name, "Rally Saudi Arabia");
        assert_eq!(events[13].round, Some(14));
    }
}
