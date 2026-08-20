use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct LmemScraper {
    pub category: &'static str, // "elms", "aslms"
}

impl SeriesScraper for LmemScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let slug = match self.category {
            "elms" => "elms",
            "aslms" => "aslms",
            _ => "elms",
        };
        let mut events = fetch_json_ld_calendar(
            &client,
            &format!("https://raceweek.io/{}", slug),
            &series.id,
            &lmem_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_lmem_schedule(&series.id, self.category, 2026);
            events.extend(get_official_lmem_schedule(&series.id, self.category, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_lmem_schedule(&series.id, self.category, 2027));
        }

        Ok(events)
    }
}

fn lmem_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "YouTube (European Le Mans Series)".to_string(),
            url: "https://www.youtube.com/@EuropeanLeMansSeriesOfficial".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "ELMS TV".to_string(),
            url: "https://www.europeanlemansseries.com/live".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_lmem_schedule(series_id: &str, category: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events: Vec<(&str, &str, &str, (u32, u32), (u32, u32))> = match category {
        "elms" => vec![
            (
                "4 Hours of Barcelona",
                "Circuit de Barcelona-Catalunya",
                "Spain",
                (4, 11),
                (4, 12),
            ),
            (
                "4 Hours of Le Castellet",
                "Circuit Paul Ricard",
                "France",
                (5, 2),
                (5, 3),
            ),
            (
                "4 Hours of Imola",
                "Autodromo Enzo e Dino Ferrari",
                "Italy",
                (7, 4),
                (7, 5),
            ),
            (
                "4 Hours of Spa-Francorchamps",
                "Circuit de Spa-Francorchamps",
                "Belgium",
                (8, 22),
                (8, 23),
            ),
            (
                "4 Hours of Silverstone",
                "Silverstone Circuit",
                "UK",
                (9, 12),
                (9, 13),
            ),
            (
                "4 Hours of Portimão",
                "Autódromo Internacional do Algarve",
                "Portugal",
                (10, 17),
                (10, 18),
            ),
        ],
        "aslms" => vec![
            (
                "4 Hours of Sepang (Race 1)",
                "Sepang International Circuit",
                "Malaysia",
                (12, 5),
                (12, 6),
            ),
            (
                "4 Hours of Sepang (Race 2)",
                "Sepang International Circuit",
                "Malaysia",
                (12, 6),
                (12, 7),
            ),
            (
                "4 Hours of Dubai (Race 1)",
                "Dubai Autodrome",
                "UAE",
                (1, 30),
                (1, 31),
            ),
            (
                "4 Hours of Dubai (Race 2)",
                "Dubai Autodrome",
                "UAE",
                (1, 31),
                (2, 1),
            ),
            (
                "4 Hours of Abu Dhabi (Race 1)",
                "Yas Marina Circuit",
                "UAE",
                (2, 6),
                (2, 7),
            ),
            (
                "4 Hours of Abu Dhabi (Race 2)",
                "Yas Marina Circuit",
                "UAE",
                (2, 7),
                (2, 8),
            ),
        ],
        _ => vec![],
    };

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
                        .and_hms_opt(13, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(11, 30, 0)
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
                stream_links: lmem_stream_links(),
                status,
            }
        })
        .collect()
}
