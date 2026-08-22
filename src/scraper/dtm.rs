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

pub struct DtmScraper;

impl StandingsFetcher for DtmScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = create_http_client()?;
        let url = "https://www.dtm.com/en/standings";
        let mut drivers = Vec::new();

        if let Ok(resp) = client.get(url).send().await {
            if let Ok(html) = resp.text().await {
                drivers = parse_dtm_standings_html(&html).unwrap_or_default();
            }
        }

        Ok(SeasonStandings {
            series_id: "dtm".to_string(),
            season,
            drivers,
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

/// Parse DTM standings HTML table.
pub fn parse_dtm_standings_html(html: &str) -> Result<Vec<crate::data::standings::DriverStanding>> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse(".table tbody tr, table tbody tr")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let td_sel = scraper::Selector::parse("td, th").map_err(|e| anyhow::anyhow!("{:?}", e))?;

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

impl ResultsFetcher for DtmScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        let client = create_http_client()?;
        let url = format!("https://www.dtm.com/en/results/{}/{}", season, round);
        let mut results = Vec::new();

        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(html) = resp.text().await {
                if let Ok(parsed) = parse_dtm_results_html(&html, round) {
                    results = parsed.results;
                }
            }
        }

        Ok(RaceResults {
            series_id: "dtm".to_string(),
            round,
            event_name: format!("DTM Round {}", round),
            circuit_name: "".to_string(),
            race_date: Utc::now().date_naive(),
            results,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse DTM race results table into RaceResults.
pub fn parse_dtm_results_html(html: &str, round: u32) -> Result<RaceResults> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse(".table-results tbody tr, table tbody tr")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let td_sel = scraper::Selector::parse("td, th").map_err(|e| anyhow::anyhow!("{:?}", e))?;

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
        series_id: "dtm".to_string(),
        round,
        event_name: format!("DTM Round {}", round),
        circuit_name: "".to_string(),
        race_date: Utc::now().date_naive(),
        results,
        fetched_at: Utc::now(),
    })
}

impl SeriesScraper for DtmScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/dtm",
            &series.id,
            &dtm_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_dtm_schedule(&series.id);
            events.extend(get_official_2027_dtm_schedule(&series.id));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_dtm_schedule(&series.id));
        }

        Ok(events)
    }
}

fn dtm_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "YouTube (DTM Official)".to_string(),
            url: "https://www.youtube.com/@DTM".to_string(),
            access: StreamAccess::Free,
        },
        StreamLink {
            platform: "ProSieben / Joyn".to_string(),
            url: "https://www.joyn.de/".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_2026_dtm_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Motorsport Arena Oschersleben",
            "Oschersleben",
            "Germany",
            (2026, 4, 26),
        ),
        ("DEKRA Lausitzring", "Klettwitz", "Germany", (2026, 5, 24)),
        (
            "Circuit Zandvoort",
            "Zandvoort",
            "Netherlands",
            (2026, 6, 7),
        ),
        ("Norisring", "Nuremberg", "Germany", (2026, 7, 5)),
        ("Nürburgring", "Nürburg", "Germany", (2026, 8, 9)),
        (
            "Sachsenring",
            "Hohenstein-Ernstthal",
            "Germany",
            (2026, 9, 6),
        ),
        ("Red Bull Ring", "Spielberg", "Austria", (2026, 9, 27)),
        (
            "Hockenheimring Baden-Württemberg",
            "Hockenheim",
            "Germany",
            (2026, 10, 18),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, date))| {
            let sun_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let sat_date = sun_date.pred_opt().unwrap_or(sun_date);

            let status = if sun_date < today {
                EventStatus::Completed
            } else if sat_date <= today && today <= sun_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let race1_time = sat_date
                .and_hms_opt(11, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race2_time = sun_date
                .and_hms_opt(11, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Race 1 (Sat)".to_string(),
                    session_type: SessionType::Race,
                    start_time: race1_time,
                    end_time: None,
                },
                Session {
                    name: "Race 2 (Sun)".to_string(),
                    session_type: SessionType::Race,
                    start_time: race2_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("DTM at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sun_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: dtm_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_dtm_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Motorsport Arena Oschersleben",
            "Oschersleben",
            "Germany",
            (2027, 4, 24),
            (2027, 4, 25),
        ),
        (
            "DEKRA Lausitzring",
            "Klettwitz",
            "Germany",
            (2027, 5, 22),
            (2027, 5, 23),
        ),
        (
            "Circuit Zandvoort",
            "Zandvoort",
            "Netherlands",
            (2027, 6, 12),
            (2027, 6, 13),
        ),
        (
            "Norisring",
            "Nuremberg",
            "Germany",
            (2027, 7, 3),
            (2027, 7, 4),
        ),
        (
            "Nürburgring (Sprint)",
            "Nürburg",
            "Germany",
            (2027, 8, 7),
            (2027, 8, 8),
        ),
        (
            "Sachsenring",
            "Hohenstein-Ernstthal",
            "Germany",
            (2027, 9, 4),
            (2027, 9, 5),
        ),
        (
            "Red Bull Ring",
            "Spielberg",
            "Austria",
            (2027, 9, 25),
            (2027, 9, 26),
        ),
        (
            "Hockenheimring Baden-Württemberg",
            "Hockenheim",
            "Germany",
            (2027, 10, 16),
            (2027, 10, 17),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, start, end))| {
            let sat_date = NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap();
            let sun_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();

            let race1_time = sat_date
                .and_hms_opt(11, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race2_time = sun_date
                .and_hms_opt(11, 30, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Qualifying 1".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(7, 30, 0)
                        .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 1 (Sat)".to_string(),
                    session_type: SessionType::Race,
                    start_time: race1_time,
                    end_time: None,
                },
                Session {
                    name: "Qualifying 2".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sun_date
                        .and_hms_opt(7, 30, 0)
                        .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Race 2 (Sun)".to_string(),
                    session_type: SessionType::Race,
                    start_time: race2_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("DTM at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: sun_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: dtm_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_dtm_schedule() {
        let events = get_official_2026_dtm_schedule("dtm");
        assert_eq!(events.len(), 8);
        assert_eq!(events[0].event_name, "DTM at Oschersleben");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[7].event_name, "DTM at Hockenheim");
        assert_eq!(events[7].round, Some(8));
    }

    #[test]
    fn test_parse_dtm_standings_html() {
        let sample = r#"
            <table class="table">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Mirko Bortolotti</td>
                        <td>SSR Performance</td>
                        <td>238</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Kelvin van der Linde</td>
                        <td>Abt Sportsline</td>
                        <td>221</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let drivers = parse_dtm_standings_html(sample).unwrap();
        assert_eq!(drivers.len(), 2);
        assert_eq!(drivers[0].position, 1);
        assert_eq!(drivers[0].driver_name, "Mirko Bortolotti");
        assert_eq!(drivers[0].team, "SSR Performance");
        assert_eq!(drivers[0].points, 238.0);
    }

    #[test]
    fn test_parse_dtm_results_html() {
        let sample = r#"
            <table class="table-results">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Mirko Bortolotti</td>
                        <td>SSR Performance</td>
                        <td>55:12.345</td>
                        <td>25</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Kelvin van der Linde</td>
                        <td>Abt Sportsline</td>
                        <td>+1.234</td>
                        <td>18</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let results = parse_dtm_results_html(sample, 1).unwrap();
        assert_eq!(results.series_id, "dtm");
        assert_eq!(results.round, 1);
        assert_eq!(results.results.len(), 2);
        assert_eq!(results.results[0].position, Some(1));
        assert_eq!(results.results[0].driver_name, "Mirko Bortolotti");
        assert_eq!(results.results[0].team, "SSR Performance");
        assert_eq!(results.results[0].points, 25.0);
    }
}
