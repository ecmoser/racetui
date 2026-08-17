use anyhow::Result;
use chrono::{NaiveDate, Utc};
use scraper::{Html, Selector};

use super::{fetcher, SeriesScraper};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct BtccScraper;

impl SeriesScraper for BtccScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        if let Ok(client) = fetcher::create_http_client() {
            if let Ok(response) = client.get(&series.calendar_url).send().await {
                if let Ok(html_text) = response.text().await {
                    if !html_text.contains("Vercel Security Checkpoint") && !html_text.contains("Enable JavaScript to continue") {
                        if let Ok(events) = parse_btcc_html(&html_text, &series.id) {
                            if !events.is_empty() {
                                return Ok(events);
                            }
                        }
                    }
                }
            }
        }

        // Return the official 2026 BTCC calendar
        Ok(get_official_2026_btcc_schedule(&series.id))
    }
}

pub fn parse_btcc_html(html_text: &str, series_id: &str) -> Result<Vec<RaceEvent>> {
    let document = Html::parse_document(html_text);
    let row_selector = Selector::parse(".calendar-item, .event-row, .race-card")
        .map_err(|e| anyhow::anyhow!("Invalid row selector: {:?}", e))?;
    let title_selector = Selector::parse(".circuit-name, .event-title, h3")
        .map_err(|e| anyhow::anyhow!("Invalid title selector: {:?}", e))?;

    let mut events = Vec::new();
    let today = Utc::now().date_naive();

    for row in document.select(&row_selector) {
        if let Some(title_elem) = row.select(&title_selector).next() {
            let title = title_elem.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                events.push(RaceEvent {
                    series_id: series_id.to_string(),
                    event_name: format!("BTCC at {}", title),
                    circuit_name: title,
                    location: "UK".to_string(),
                    country: "UK".to_string(),
                    start_date: today,
                    end_date: today,
                    round: None,
                    sessions: vec![],
                    stream_links: btcc_stream_links(),
                    status: EventStatus::Upcoming,
                });
            }
        }
    }

    Ok(events)
}

fn btcc_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "ITVX".to_string(),
            url: "https://www.itv.com".to_string(),
            access: StreamAccess::Mixed,
        },
        StreamLink {
            platform: "TikTok (@ITVSport)".to_string(),
            url: "https://www.tiktok.com/@itvsport".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_2026_btcc_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Donington Park (National)",
            "Donington Park (National Circuit)",
            "Castle Donington, Leicestershire",
            (2026, 4, 25),
            (2026, 4, 26),
        ),
        (
            "Brands Hatch (Indy)",
            "Brands Hatch (Indy Circuit)",
            "West Kingsdown, Kent",
            (2026, 5, 9),
            (2026, 5, 10),
        ),
        (
            "Snetterton (300)",
            "Snetterton Circuit (300)",
            "Norwich, Norfolk",
            (2026, 5, 23),
            (2026, 5, 24),
        ),
        (
            "Thruxton",
            "Thruxton Circuit",
            "Andover, Hampshire",
            (2026, 6, 6),
            (2026, 6, 7),
        ),
        (
            "Oulton Park (Island)",
            "Oulton Park (Island Circuit)",
            "Tarporley, Cheshire",
            (2026, 6, 20),
            (2026, 6, 21),
        ),
        (
            "Croft",
            "Croft Circuit",
            "Dalton-on-Tees, North Yorkshire",
            (2026, 7, 25),
            (2026, 7, 26),
        ),
        (
            "Knockhill",
            "Knockhill Racing Circuit",
            "Fife, Scotland",
            (2026, 8, 15),
            (2026, 8, 16),
        ),
        (
            "Donington Park (GP)",
            "Donington Park (Grand Prix Circuit)",
            "Castle Donington, Leicestershire",
            (2026, 8, 29),
            (2026, 8, 30),
        ),
        (
            "Silverstone (National)",
            "Silverstone Circuit (National)",
            "Silverstone, Northamptonshire",
            (2026, 9, 19),
            (2026, 9, 20),
        ),
        (
            "Brands Hatch (GP)",
            "Brands Hatch (Grand Prix Circuit)",
            "West Kingsdown, Kent",
            (2026, 10, 3),
            (2026, 10, 4),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, _start, end))| {
            let race_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();
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
                    start_time: None,
                    end_time: None,
                },
                Session {
                    name: "Races 1, 2 & 3".to_string(),
                    session_type: SessionType::Race,
                    start_time: None,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("BTCC at {}", name),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: "UK".to_string(),
                start_date: race_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: btcc_stream_links(),
                status,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_btcc_schedule() {
        let events = get_official_2026_btcc_schedule("btcc");
        assert_eq!(events.len(), 10);
        assert_eq!(events[0].event_name, "BTCC at Donington Park (National)");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[9].event_name, "BTCC at Brands Hatch (GP)");
        assert_eq!(events[9].round, Some(10));
    }
}
