use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};
use scraper::{Html, Selector};

use super::fetcher;
use super::json_ld::fetch_json_ld_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct BtccScraper;

impl StandingsFetcher for BtccScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = fetcher::create_http_client()?;
        let url = "https://www.btcc.net/standings/";
        let mut drivers = Vec::new();

        if let Ok(resp) = client.get(url).send().await {
            if let Ok(html) = resp.text().await {
                drivers = parse_btcc_standings_html(&html).unwrap_or_default();
            }
        }

        Ok(SeasonStandings {
            series_id: "btcc".to_string(),
            season,
            drivers,
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

/// Parse BTCC standings HTML table.
pub fn parse_btcc_standings_html(
    html: &str,
) -> Result<Vec<crate::data::standings::DriverStanding>> {
    let document = Html::parse_document(html);
    let row_sel = Selector::parse(".standings-table tbody tr, table tbody tr")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let td_sel = Selector::parse("td, th").map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let mut drivers = Vec::new();

    for row in document.select(&row_sel) {
        let cols: Vec<String> = row
            .select(&td_sel)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if cols.len() >= 3 {
            let pos = cols[0].parse::<u32>().unwrap_or((drivers.len() + 1) as u32);
            let name = cols[1].clone();
            let team = if cols.len() >= 4 {
                cols[2].clone()
            } else {
                "".to_string()
            };
            let points = cols
                .last()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            drivers.push(crate::data::standings::DriverStanding {
                position: pos,
                driver_name: name,
                driver_code: None,
                driver_number: None,
                team,
                points,
                wins: 0,
            });
        }
    }

    Ok(drivers)
}

impl ResultsFetcher for BtccScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        let client = fetcher::create_http_client()?;
        let url = format!("https://www.btcc.net/results/{}/{}", season, round);
        let mut results = Vec::new();

        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(html) = resp.text().await {
                if let Ok(parsed) = parse_btcc_results_html(&html, round) {
                    results = parsed.results;
                }
            }
        }

        Ok(RaceResults {
            series_id: "btcc".to_string(),
            round,
            event_name: format!("BTCC Round {}", round),
            circuit_name: "".to_string(),
            race_date: Utc::now().date_naive(),
            results,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse BTCC race results table into RaceResults.
pub fn parse_btcc_results_html(html: &str, round: u32) -> Result<RaceResults> {
    let document = Html::parse_document(html);
    let row_sel = Selector::parse(".results-table tbody tr, table tbody tr")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let td_sel = Selector::parse("td, th").map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let mut results = Vec::new();

    for row in document.select(&row_sel) {
        let cols: Vec<String> = row
            .select(&td_sel)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if cols.len() >= 3 {
            let pos = cols[0].parse::<u32>().ok();
            let name = cols[1].clone();
            let team = if cols.len() >= 4 {
                cols[2].clone()
            } else {
                "".to_string()
            };
            let gap = if cols.len() >= 5 {
                cols[3].clone()
            } else {
                "".to_string()
            };
            let points = cols
                .last()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            results.push(crate::data::results::DriverResult {
                position: pos,
                driver_name: name,
                driver_code: None,
                driver_number: None,
                team,
                gap_to_leader: gap,
                gap_to_ahead: "".to_string(),
                grid_position: None,
                points,
                fastest_lap: false,
                penalty: None,
                status: "Finished".to_string(),
            });
        }
    }

    Ok(RaceResults {
        series_id: "btcc".to_string(),
        round,
        event_name: format!("BTCC Round {}", round),
        circuit_name: "".to_string(),
        race_date: Utc::now().date_naive(),
        results,
        fetched_at: Utc::now(),
    })
}

impl SeriesScraper for BtccScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let mut events = Vec::new();

        if let Ok(client) = fetcher::create_http_client() {
            // Try live structured web schedule first
            if let Ok(fetched) = fetch_json_ld_calendar(
                &client,
                "https://raceweek.io/btcc",
                &series.id,
                &btcc_stream_links(),
            )
            .await
            {
                if !fetched.is_empty() {
                    events = fetched;
                }
            }

            if events.is_empty() {
                if let Ok(response) = client.get(&series.calendar_url).send().await {
                    if let Ok(html_text) = response.text().await {
                        if !html_text.contains("Vercel Security Checkpoint")
                            && !html_text.contains("Enable JavaScript to continue")
                        {
                            if let Ok(parsed) = parse_btcc_html(&html_text, &series.id) {
                                if !parsed.is_empty() {
                                    events = parsed;
                                }
                            }
                        }
                    }
                }
            }
        }

        if events.is_empty() {
            events = get_official_2026_btcc_schedule(&series.id);
        }

        if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_btcc_schedule(&series.id));
        }

        Ok(events)
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
        .map(|(i, (name, circuit, loc, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let status = if sun_date < today {
                EventStatus::Completed
            } else if sat_date <= today && today <= sun_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let quali_time = sat_date
                .and_hms_opt(14, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race1_time = sun_date
                .and_hms_opt(10, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race2_time = sun_date
                .and_hms_opt(13, 15, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race3_time = sun_date
                .and_hms_opt(16, 15, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Race 1".to_string(),
                    session_type: SessionType::Race,
                    start_time: race1_time,
                    end_time: None,
                },
                Session {
                    name: "Race 2".to_string(),
                    session_type: SessionType::Race,
                    start_time: race2_time,
                    end_time: None,
                },
                Session {
                    name: "Race 3".to_string(),
                    session_type: SessionType::Race,
                    start_time: race3_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("BTCC at {}", name),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: "UK".to_string(),
                start_date: sun_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: btcc_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_btcc_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Donington Park (National)",
            "Donington Park (National Circuit)",
            "Castle Donington",
            (2027, 4, 24),
            (2027, 4, 25),
        ),
        (
            "Brands Hatch (Indy)",
            "Brands Hatch (Indy Circuit)",
            "West Kingsdown",
            (2027, 5, 8),
            (2027, 5, 9),
        ),
        (
            "Snetterton (300)",
            "Snetterton Motor Racing Circuit (300)",
            "Norwich",
            (2027, 5, 22),
            (2027, 5, 23),
        ),
        (
            "Thruxton",
            "Thruxton Circuit",
            "Andover",
            (2027, 6, 5),
            (2027, 6, 6),
        ),
        (
            "Oulton Park (Island)",
            "Oulton Park (Island Circuit)",
            "Little Budworth",
            (2027, 6, 19),
            (2027, 6, 20),
        ),
        (
            "Croft",
            "Croft Circuit",
            "Dalton-on-Tees",
            (2027, 7, 24),
            (2027, 7, 25),
        ),
        (
            "Knockhill",
            "Knockhill Racing Circuit",
            "Fife",
            (2027, 8, 14),
            (2027, 8, 15),
        ),
        (
            "Donington Park (GP)",
            "Donington Park (Grand Prix Circuit)",
            "Castle Donington",
            (2027, 8, 28),
            (2027, 8, 29),
        ),
        (
            "Silverstone (National)",
            "Silverstone Circuit (National)",
            "Silverstone",
            (2027, 9, 18),
            (2027, 9, 19),
        ),
        (
            "Brands Hatch (GP)",
            "Brands Hatch (Grand Prix Circuit)",
            "West Kingsdown",
            (2027, 10, 2),
            (2027, 10, 3),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let quali_time = sat_date
                .and_hms_opt(14, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race1_time = sun_date
                .and_hms_opt(10, 45, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race2_time = sun_date
                .and_hms_opt(13, 25, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race3_time = sun_date
                .and_hms_opt(16, 15, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Race 1".to_string(),
                    session_type: SessionType::Race,
                    start_time: race1_time,
                    end_time: None,
                },
                Session {
                    name: "Race 2".to_string(),
                    session_type: SessionType::Race,
                    start_time: race2_time,
                    end_time: None,
                },
                Session {
                    name: "Race 3".to_string(),
                    session_type: SessionType::Race,
                    start_time: race3_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("BTCC at {}", name),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: "UK".to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: btcc_stream_links(),
                status: EventStatus::Upcoming,
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

    #[test]
    fn test_parse_btcc_standings_html() {
        let sample = r#"
            <table class="standings-table">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Jake Hill</td>
                        <td>Laser Tools Racing with MB Motorsport</td>
                        <td>421</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Tom Ingram</td>
                        <td>Team BRISTOL STREET MOTORS</td>
                        <td>413</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let drivers = parse_btcc_standings_html(sample).unwrap();
        assert_eq!(drivers.len(), 2);
        assert_eq!(drivers[0].position, 1);
        assert_eq!(drivers[0].driver_name, "Jake Hill");
        assert_eq!(drivers[0].team, "Laser Tools Racing with MB Motorsport");
        assert_eq!(drivers[0].points, 421.0);
    }

    #[test]
    fn test_parse_btcc_results_html() {
        let sample = r#"
            <table class="results-table">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Jake Hill</td>
                        <td>Laser Tools Racing with MB Motorsport</td>
                        <td>21:12.345</td>
                        <td>20</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Tom Ingram</td>
                        <td>Team BRISTOL STREET MOTORS</td>
                        <td>+0.456</td>
                        <td>17</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let results = parse_btcc_results_html(sample, 1).unwrap();
        assert_eq!(results.series_id, "btcc");
        assert_eq!(results.round, 1);
        assert_eq!(results.results.len(), 2);
        assert_eq!(results.results[0].position, Some(1));
        assert_eq!(results.results[0].driver_name, "Jake Hill");
        assert_eq!(
            results.results[0].team,
            "Laser Tools Racing with MB Motorsport"
        );
        assert_eq!(results.results[0].points, 20.0);
    }
}
