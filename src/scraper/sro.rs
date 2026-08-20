use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::SeriesScraper;
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct SroScraper {
    pub category: &'static str, // "gtwc_eu", "gtwc_am", "igtc"
}

impl SeriesScraper for SroScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let slug = match self.category {
            "gtwc_eu" => "gtwce",
            "gtwc_am" => "gtwca",
            "igtc" => "igtc",
            _ => "gtwce",
        };
        let mut events = fetch_json_ld_calendar(
            &client,
            &format!("https://raceweek.io/{}", slug),
            &series.id,
            &sro_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_sro_schedule(&series.id, self.category, 2026);
            events.extend(get_official_sro_schedule(&series.id, self.category, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_sro_schedule(&series.id, self.category, 2027));
        }

        Ok(events)
    }
}

fn sro_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "YouTube (GTWorld)".to_string(),
            url: "https://www.youtube.com/@GTWorld".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "SRO Motorsports".to_string(),
            url: "https://www.gt-world-challenge-europe.com/watch-live".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_sro_schedule(series_id: &str, category: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events: Vec<(&str, &str, &str, (u32, u32), (u32, u32))> = match category {
        "gtwc_eu" => vec![
            (
                "Circuit Paul Ricard (Endurance)",
                "Circuit Paul Ricard",
                "France",
                (4, 11),
                (4, 12),
            ),
            (
                "Brands Hatch (Sprint)",
                "Brands Hatch",
                "UK",
                (5, 2),
                (5, 3),
            ),
            (
                "Misano (Sprint)",
                "Misano World Circuit",
                "Italy",
                (5, 30),
                (5, 31),
            ),
            (
                "Monza (Endurance)",
                "Autodromo Nazionale Monza",
                "Italy",
                (6, 20),
                (6, 21),
            ),
            (
                "CrowdStrike 24 Hours of Spa",
                "Circuit de Spa-Francorchamps",
                "Belgium",
                (6, 27),
                (6, 28),
            ),
            (
                "Hockenheim (Sprint)",
                "Hockenheimring",
                "Germany",
                (7, 18),
                (7, 19),
            ),
            (
                "Magny-Cours (Sprint)",
                "Circuit de Nevers Magny-Cours",
                "France",
                (8, 29),
                (8, 30),
            ),
            (
                "Nürburgring (Endurance)",
                "Nürburgring",
                "Germany",
                (9, 12),
                (9, 13),
            ),
            (
                "Valencia (Sprint)",
                "Circuit Ricardo Tormo",
                "Spain",
                (9, 26),
                (9, 27),
            ),
            (
                "Barcelona (Endurance)",
                "Circuit de Barcelona-Catalunya",
                "Spain",
                (10, 10),
                (10, 11),
            ),
        ],
        "gtwc_am" => vec![
            ("Sonoma Raceway", "Sonoma Raceway", "USA", (3, 28), (3, 29)),
            (
                "Long Beach",
                "Long Beach Street Circuit",
                "USA",
                (4, 18),
                (4, 19),
            ),
            (
                "Sebring International Raceway",
                "Sebring International Raceway",
                "USA",
                (5, 9),
                (5, 10),
            ),
            (
                "Circuit of the Americas",
                "Circuit of the Americas",
                "USA",
                (5, 30),
                (5, 31),
            ),
            (
                "VIRginia International Raceway",
                "VIRginia International Raceway",
                "USA",
                (7, 18),
                (7, 19),
            ),
            ("Road America", "Road America", "USA", (8, 15), (8, 16)),
            (
                "Barber Motorsports Park",
                "Barber Motorsports Park",
                "USA",
                (9, 12),
                (9, 13),
            ),
            (
                "Indianapolis Motor Speedway",
                "Indianapolis Motor Speedway",
                "USA",
                (10, 3),
                (10, 4),
            ),
        ],
        "igtc" => vec![
            (
                "Bathurst 12 Hour",
                "Mount Panorama Circuit",
                "Australia",
                (2, 14),
                (2, 15),
            ),
            (
                "Nürburgring 24 Hours",
                "Nürburgring Nordschleife",
                "Germany",
                (5, 23),
                (5, 24),
            ),
            (
                "CrowdStrike 24 Hours of Spa",
                "Circuit de Spa-Francorchamps",
                "Belgium",
                (6, 27),
                (6, 28),
            ),
            (
                "Suzuka 1000km",
                "Suzuka International Racing Course",
                "Japan",
                (9, 12),
                (9, 13),
            ),
            (
                "Indianapolis 8 Hour",
                "Indianapolis Motor Speedway",
                "USA",
                (10, 3),
                (10, 4),
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
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("{} — {}", series_id.to_uppercase(), name),
                circuit_name: circuit.to_string(),
                location: country.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: sro_stream_links(),
                status,
            }
        })
        .collect()
}
