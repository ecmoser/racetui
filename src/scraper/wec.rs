use anyhow::Result;
#[cfg(test)]
use chrono::{DateTime, Datelike};
use chrono::{NaiveDate, Utc};
use futures::stream::{self, StreamExt};
use std::collections::BTreeSet;

use super::fetcher::{create_http_client, fetch_url};
use super::ics::fetch_ics_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{EventStatus, RaceEvent, Series, SessionType, StreamLink};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct WecScraper;

/// Avoid serial round-trip delays while remaining gentle to FIA's calendar service.
const WEC_FETCH_CONCURRENCY: usize = 6;

impl StandingsFetcher for WecScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = create_http_client()?;
        let driver_url = "https://www.fiawec.com/en/page/drivers-classification";
        let mut drivers = Vec::new();
        let mut constructors = Vec::new();

        if let Ok(resp) = client.get(driver_url).send().await {
            if let Ok(html) = resp.text().await {
                drivers = parse_wec_standings_html(&html).unwrap_or_default();
            }
        }

        let mfg_url = "https://www.fiawec.com/en/page/manufacturers-classification";
        if let Ok(resp) = client.get(mfg_url).send().await {
            if let Ok(html) = resp.text().await {
                constructors = parse_wec_constructors_html(&html).unwrap_or_default();
            }
        }

        Ok(SeasonStandings {
            series_id: "wec".to_string(),
            season,
            drivers,
            constructors,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse WEC driver standings table HTML.
pub fn parse_wec_standings_html(html: &str) -> Result<Vec<crate::data::standings::DriverStanding>> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse("table.table-standing tbody tr, table tbody tr")
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
            let points = cols
                .last()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            drivers.push(crate::data::standings::DriverStanding {
                position: pos,
                driver_name: name,
                driver_code: None,
                driver_number: None,
                team: "".to_string(),
                points,
                wins: 0,
            });
        }
    }

    Ok(drivers)
}

/// Parse WEC manufacturer standings table HTML.
pub fn parse_wec_constructors_html(
    html: &str,
) -> Result<Vec<crate::data::standings::ConstructorStanding>> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse("table.table-standing tbody tr, table tbody tr")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let td_sel = scraper::Selector::parse("td, th").map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let mut constructors = Vec::new();

    for row in document.select(&row_sel) {
        let cols: Vec<String> = row
            .select(&td_sel)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if cols.len() >= 3 {
            let pos = cols[0]
                .parse::<u32>()
                .unwrap_or((constructors.len() + 1) as u32);
            let name = cols[1].clone();
            let points = cols
                .last()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            constructors.push(crate::data::standings::ConstructorStanding {
                position: pos,
                name,
                points,
                wins: 0,
            });
        }
    }

    Ok(constructors)
}

impl ResultsFetcher for WecScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        let client = create_http_client()?;
        let url = format!("https://www.fiawec.com/en/race/result/{}", round);
        let mut results = Vec::new();

        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(html) = resp.text().await {
                if let Ok(parsed) = parse_wec_results_html(&html, round) {
                    results = parsed.results;
                }
            }
        }

        Ok(RaceResults {
            series_id: "wec".to_string(),
            round,
            event_name: format!("FIA WEC Round {}", round),
            circuit_name: "".to_string(),
            race_date: Utc::now().date_naive(),
            results,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse WEC race results table into RaceResults.
pub fn parse_wec_results_html(html: &str, round: u32) -> Result<RaceResults> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse(
        ".results-table tbody tr, table.table-standing tbody tr, table tbody tr",
    )
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
            let team = cols[1].clone();
            let car_num = cols.get(2).cloned().unwrap_or_default();
            let drivers = cols.get(3).cloned().unwrap_or_default();
            let points = cols
                .last()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            let driver_name = if !drivers.is_empty() {
                drivers
            } else {
                team.clone()
            };

            results.push(crate::data::results::DriverResult {
                position: pos,
                driver_name,
                driver_code: None,
                driver_number: car_num.trim_start_matches('#').parse::<u32>().ok(),
                team,
                gap_to_leader: "".to_string(),
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
        series_id: "wec".to_string(),
        round,
        event_name: format!("FIA WEC Round {}", round),
        circuit_name: "".to_string(),
        race_date: Utc::now().date_naive(),
        results,
        fetched_at: Utc::now(),
    })
}

impl SeriesScraper for WecScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        fetch_official_wec_calendar(&client, &series.calendar_url, &series.id).await
    }
}

/// Fetch WEC session times from FIA WEC's own iCalendar feeds. The feeds contain
/// timezone-aware timestamps, which are stored as UTC and converted only for display.
async fn fetch_official_wec_calendar(
    client: &reqwest::Client,
    calendar_url: &str,
    series_id: &str,
) -> Result<Vec<RaceEvent>> {
    let calendar_html = fetch_url(client, calendar_url).await?;
    let event_pages = parse_official_wec_event_pages(&calendar_html);
    if event_pages.is_empty() {
        anyhow::bail!("FIA WEC calendar did not contain any event pages");
    }

    let mut events: Vec<RaceEvent> = stream::iter(event_pages)
        .map(|event_page| fetch_official_wec_event(client, series_id, event_page))
        .buffer_unordered(WEC_FETCH_CONCURRENCY)
        .flat_map(stream::iter)
        .collect()
        .await;

    if events.is_empty() {
        anyhow::bail!("FIA WEC calendar did not yield any session timestamps");
    }

    events.sort_by_key(|event| (event.start_date, event.end_date));
    Ok(events)
}

async fn fetch_official_wec_event(
    client: &reqwest::Client,
    series_id: &str,
    event_page: String,
) -> Vec<RaceEvent> {
    let event_html = match fetch_url(client, &event_page).await {
        Ok(html) => html,
        Err(_) => return Vec::new(),
    };
    let Some((start_date, end_date)) = parse_official_wec_event_dates(&event_html) else {
        return Vec::new();
    };
    let Some(ics_url) = parse_official_wec_ics_url(&event_html) else {
        return Vec::new();
    };
    let Ok(mut event_sessions) =
        fetch_ics_calendar(client, &ics_url, series_id, &wec_stream_links()).await
    else {
        return Vec::new();
    };

    for event in &mut event_sessions {
        apply_official_wec_event_dates(event, start_date, end_date);
        combine_qualifying_sessions(event);
    }
    event_sessions
}

fn apply_official_wec_event_dates(
    event: &mut RaceEvent,
    start_date: NaiveDate,
    end_date: NaiveDate,
) {
    event.start_date = start_date;
    event.end_date = end_date;

    // A future official feed can contain TBC sessions with no timestamps. In
    // that case, use the published event date to avoid treating a past weekend
    // as Upcoming (which would incorrectly put it in the default list view).
    event.status = if end_date < Utc::now().date_naive() {
        EventStatus::Completed
    } else {
        event.current_status()
    };
}

fn parse_official_wec_event_pages(html: &str) -> Vec<String> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("a[href]").expect("valid anchor selector");
    document
        .select(&selector)
        .filter_map(|anchor| anchor.value().attr("href"))
        .filter(|href| href.contains("/en/race/") && !href.contains("/calendar/"))
        .filter(|href| !href.contains("/result/"))
        .map(absolute_fiawec_url)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn parse_official_wec_ics_url(html: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("a[href]").expect("valid anchor selector");
    document
        .select(&selector)
        .filter_map(|anchor| anchor.value().attr("href"))
        .find(|href| href.contains("/en/race/calendar/"))
        .map(absolute_fiawec_url)
}

fn parse_official_wec_event_dates(html: &str) -> Option<(NaiveDate, NaiveDate)> {
    let document = scraper::Html::parse_document(html);
    let text = document.root_element().text().collect::<Vec<_>>().join(" ");
    let words: Vec<_> = text.split_whitespace().collect();

    for window in words.windows(6) {
        if !window[0].eq_ignore_ascii_case("from") || !window[2].eq_ignore_ascii_case("to") {
            continue;
        }
        let start_day = window[1].trim_end_matches(|c: char| !c.is_ascii_digit());
        let end_day = window[3].trim_end_matches(|c: char| !c.is_ascii_digit());
        let month = window[4].trim_end_matches(|c: char| !c.is_ascii_alphabetic());
        let year = window[5].trim_end_matches(|c: char| !c.is_ascii_digit());
        let start =
            NaiveDate::parse_from_str(&format!("{} {} {}", year, month, start_day), "%Y %B %d")
                .ok()?;
        let end = NaiveDate::parse_from_str(&format!("{} {} {}", year, month, end_day), "%Y %B %d")
            .ok()?;
        return Some((start, end));
    }

    None
}

fn absolute_fiawec_url(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else {
        format!("https://www.fiawec.com{}", href)
    }
}

/// FIA WEC publishes separate qualifying and Hyperpole sessions for each class.
/// Present them as one qualifying block so an event does not occupy four entries
/// in the calendar/list views.
fn combine_qualifying_sessions(event: &mut RaceEvent) {
    let qualifying_indices: Vec<usize> = event
        .sessions
        .iter()
        .enumerate()
        .filter_map(|(index, session)| {
            (session.session_type == SessionType::Qualifying).then_some(index)
        })
        .collect();

    if qualifying_indices.len() <= 1 {
        return;
    }

    let start_time = qualifying_indices
        .iter()
        .filter_map(|&index| event.sessions[index].start_time)
        .min();
    let end_time = qualifying_indices
        .iter()
        .filter_map(|&index| event.sessions[index].end_time)
        .max();
    let first_index = qualifying_indices[0];

    event.sessions = event
        .sessions
        .drain(..)
        .enumerate()
        .filter_map(|(index, mut session)| {
            if !qualifying_indices.contains(&index) {
                return Some(session);
            }
            if index != first_index {
                return None;
            }

            session.name = "Qualifying".to_string();
            session.start_time = start_time;
            session.end_time = end_time;
            Some(session)
        })
        .collect();
}

fn wec_stream_links() -> Vec<StreamLink> {
    super::raceday_watch::get_raceday_stream_links("wec", "")
}

#[cfg(test)]
pub fn get_official_2026_wec_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Qatar 1812 Km",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2026, 2, 28),
            (8, 0),
        ),
        (
            "6 Hours of Imola",
            "Autodromo Internazionale Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2026, 4, 19),
            (11, 0),
        ),
        (
            "TotalEnergies 6 Hours of Spa-Francorchamps",
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2026, 5, 9),
            // Official 14:00 CEST start (12:00 UTC / 8:00 AM in Detroit).
            (12, 0),
        ),
        (
            "24 Hours of Le Mans",
            "Circuit de la Sarthe",
            "Le Mans",
            "France",
            (2026, 6, 13),
            (14, 0),
        ),
        (
            "Rolex 6 Hours of São Paulo",
            "Autódromo José Carlos Pace (Interlagos)",
            "São Paulo",
            "Brazil",
            (2026, 7, 12),
            (14, 30),
        ),
        (
            "Lone Star Le Mans",
            "Circuit of The Americas",
            "Austin, Texas",
            "USA",
            (2026, 9, 6),
            (18, 0),
        ),
        (
            "6 Hours of Fuji",
            "Fuji Speedway",
            "Oyama, Shizuoka",
            "Japan",
            (2026, 9, 27),
            (2, 0),
        ),
        (
            "Bapco Energies 8 Hours of Bahrain",
            "Bahrain International Circuit",
            "Sakhir",
            "Bahrain",
            (2026, 11, 7),
            (11, 0),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, country, date, (hour, min)))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let start_time = race_date
                .and_hms_opt(hour, min, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let status = if race_date < today {
                EventStatus::Completed
            } else if race_date == today {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let fri_date = race_date
                .pred_opt()
                .unwrap_or(race_date)
                .pred_opt()
                .unwrap_or(race_date);
            let sessions = super::json_ld::build_series_sessions(
                series_id, name, fri_date, race_date, start_time, None,
            );

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: fri_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: wec_stream_links(),
                status,
            }
        })
        .collect()
}

#[cfg(test)]
pub fn get_official_2027_wec_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Qatar 1812 Km",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            // The official schedule lists a 13:00 AST start (10:00 UTC).
            (2027, 3, 27),
            (10, 0),
        ),
        (
            "6 Hours of Imola",
            "Autodromo Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2027, 4, 11),
            (11, 0),
        ),
        (
            "6 Hours of Silverstone",
            "Silverstone Circuit",
            "Silverstone",
            "United Kingdom",
            (2027, 4, 25),
            (10, 0),
        ),
        (
            "6 Hours of Spa-Francorchamps",
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2027, 5, 15),
            (12, 0),
        ),
        (
            "24 Hours of Le Mans",
            "Circuit de la Sarthe",
            "Le Mans",
            "France",
            (2027, 6, 12),
            (14, 0),
        ),
        (
            "6 Hours of São Paulo",
            "Autódromo José Carlos Pace",
            "São Paulo",
            "Brazil",
            (2027, 7, 11),
            (14, 30),
        ),
        (
            "Lone Star Le Mans (6 Hours of COTA)",
            "Circuit of the Americas",
            "Austin, TX",
            "USA",
            (2027, 9, 12),
            (18, 0),
        ),
        (
            "6 Hours of Fuji",
            "Fuji International Speedway",
            "Oyama",
            "Japan",
            (2027, 9, 26),
            (2, 0),
        ),
        (
            "Bapco Energies 8 Hours of Bahrain",
            "Bahrain International Circuit",
            "Sakhir",
            "Bahrain",
            (2027, 11, 6),
            (11, 0),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(
            |(i, (name, circuit, loc, country, (y, m, d), (hour, min)))| {
                let race_date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
                let fri_date = race_date
                    .pred_opt()
                    .unwrap_or(race_date)
                    .pred_opt()
                    .unwrap_or(race_date);
                let start_time = race_date
                    .and_hms_opt(hour, min, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

                let sessions = super::json_ld::build_series_sessions(
                    series_id, name, fri_date, race_date, start_time, None,
                );

                RaceEvent {
                    series_id: series_id.to_string(),
                    event_name: name.to_string(),
                    circuit_name: circuit.to_string(),
                    location: loc.to_string(),
                    country: country.to_string(),
                    start_date: fri_date,
                    end_date: race_date,
                    round: Some((i + 1) as u32),
                    sessions,
                    stream_links: wec_stream_links(),
                    status: EventStatus::Upcoming,
                }
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};
    use chrono_tz::America::Detroit;

    #[test]
    fn test_official_wec_schedule() {
        let events = get_official_2026_wec_schedule("wec");
        assert_eq!(events.len(), 8);
        assert_eq!(events[0].event_name, "Qatar 1812 Km");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[3].event_name, "24 Hours of Le Mans");
        assert_eq!(events[7].event_name, "Bapco Energies 8 Hours of Bahrain");
        assert_eq!(events[7].round, Some(8));
    }

    #[test]
    fn test_2026_wec_race_starts_are_correct_in_detroit() {
        let events = get_official_2026_wec_schedule("wec");
        let expected = [
            // Qatar is on EST; the remainder through COTA are on EDT.
            (2026, 2, 28, 3, 0),
            (2026, 4, 19, 7, 0),
            (2026, 5, 9, 8, 0),
            (2026, 6, 13, 10, 0),
            (2026, 7, 12, 10, 30),
            (2026, 9, 6, 14, 0),
            // Fuji begins late Saturday evening in Detroit, a day earlier locally.
            (2026, 9, 26, 22, 0),
            // Bahrain is after Detroit returns to EST.
            (2026, 11, 7, 6, 0),
        ];

        for (event, (year, month, day, hour, minute)) in events.iter().zip(expected) {
            let start = event.race_start_time().expect("WEC race start time");
            let detroit = start.with_timezone(&Detroit);
            assert_eq!(
                (
                    detroit.year(),
                    detroit.month(),
                    detroit.day(),
                    detroit.hour(),
                    detroit.minute(),
                ),
                (year, month, day, hour, minute),
                "{}",
                event.event_name
            );
        }
    }

    #[test]
    fn test_2027_wec_calendar_and_timezone_conversion() {
        let events = get_official_2027_wec_schedule("wec");
        assert_eq!(events.len(), 9);
        assert_eq!(events[0].event_name, "Qatar 1812 Km");
        assert_eq!(
            events[0].end_date,
            NaiveDate::from_ymd_opt(2027, 3, 27).unwrap()
        );
        assert_eq!(events[2].event_name, "6 Hours of Silverstone");
        assert_eq!(
            events[2].end_date,
            NaiveDate::from_ymd_opt(2027, 4, 25).unwrap()
        );

        // Store the race instant in UTC and convert only for display. This lets a
        // user's system timezone change without changing the event itself.
        let fuji_start = events[7].race_start_time().expect("Fuji race start");
        assert_eq!(
            fuji_start
                .with_timezone(&chrono_tz::America::Detroit)
                .to_rfc3339(),
            "2027-09-25T22:00:00-04:00"
        );
        assert_eq!(
            fuji_start
                .with_timezone(&chrono_tz::America::Los_Angeles)
                .to_rfc3339(),
            "2027-09-25T19:00:00-07:00"
        );
        assert_eq!(
            fuji_start
                .with_timezone(&chrono_tz::Europe::London)
                .to_rfc3339(),
            "2027-09-26T03:00:00+01:00"
        );
    }

    #[test]
    fn test_parse_official_calendar_links() {
        let html = r#"
            <a href="/en/race/6-hours-of-fuji-2026">Fuji</a>
            <a href="/en/race/calendar/4954">Calendar</a>
            <a href="/en/race/result/6">Results</a>
        "#;
        assert_eq!(
            parse_official_wec_event_pages(html),
            vec!["https://www.fiawec.com/en/race/6-hours-of-fuji-2026"]
        );
        assert_eq!(
            parse_official_wec_ics_url(html).as_deref(),
            Some("https://www.fiawec.com/en/race/calendar/4954")
        );
    }

    #[test]
    fn test_parse_official_event_dates_without_session_times() {
        let html = "<h1>24 Hours of Le Mans</h1><p>From 6 to 13 June 2027</p>";
        assert_eq!(
            parse_official_wec_event_dates(html),
            Some((
                NaiveDate::from_ymd_opt(2027, 6, 6).unwrap(),
                NaiveDate::from_ymd_opt(2027, 6, 13).unwrap(),
            ))
        );
    }

    #[test]
    fn test_combine_class_qualifying_sessions() {
        let timestamp = |hour, minute| {
            Utc.with_ymd_and_hms(2026, 9, 26, hour, minute, 0)
                .single()
                .unwrap()
        };
        let mut event = RaceEvent {
            series_id: "wec".to_string(),
            event_name: "6 Hours of Fuji".to_string(),
            circuit_name: "Fuji Speedway".to_string(),
            location: "Oyama".to_string(),
            country: "Japan".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 9, 25).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 9, 27).unwrap(),
            round: Some(6),
            sessions: vec![
                crate::data::models::Session {
                    name: "Qualifying - LMGT3".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: Some(timestamp(14, 0)),
                    end_time: Some(timestamp(14, 20)),
                },
                crate::data::models::Session {
                    name: "Hyperpole - LMGT3".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: Some(timestamp(14, 20)),
                    end_time: Some(timestamp(14, 40)),
                },
                crate::data::models::Session {
                    name: "Qualifying - Hypercar".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: Some(timestamp(14, 40)),
                    end_time: Some(timestamp(15, 0)),
                },
                crate::data::models::Session {
                    name: "Hyperpole - Hypercar".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: Some(timestamp(15, 0)),
                    end_time: Some(timestamp(15, 20)),
                },
            ],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };

        combine_qualifying_sessions(&mut event);

        assert_eq!(event.sessions.len(), 1);
        assert_eq!(event.sessions[0].name, "Qualifying");
        assert_eq!(event.sessions[0].start_time, Some(timestamp(14, 0)));
        assert_eq!(event.sessions[0].end_time, Some(timestamp(15, 20)));
    }

    #[test]
    fn test_tbd_sessions_on_a_past_weekend_are_completed() {
        let yesterday = Utc::now()
            .date_naive()
            .pred_opt()
            .expect("a previous calendar day");
        let mut event = RaceEvent {
            series_id: "wec".to_string(),
            event_name: "24 Hours of Le Mans".to_string(),
            circuit_name: "Circuit de la Sarthe".to_string(),
            location: "Le Mans".to_string(),
            country: "France".to_string(),
            start_date: yesterday,
            end_date: yesterday,
            round: Some(3),
            sessions: vec![crate::data::models::Session {
                name: "24 Hours of Le Mans - Race".to_string(),
                session_type: SessionType::Race,
                start_time: None,
                end_time: None,
            }],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };

        apply_official_wec_event_dates(&mut event, yesterday, yesterday);

        assert_eq!(event.status, EventStatus::Completed);
    }

    #[test]
    fn test_parse_wec_standings_html() {
        let html_sample = r#"
            <table class="table-standing">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Porsche Penske Motorsport</td>
                        <td>#6</td>
                        <td>Estre / Lotterer / Vanthoor</td>
                        <td>152</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Toyota Gazoo Racing</td>
                        <td>#7</td>
                        <td>Conway / Kobayashi / de Vries</td>
                        <td>128</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let drivers = parse_wec_standings_html(html_sample).unwrap();
        assert_eq!(drivers.len(), 2);
        assert_eq!(drivers[0].position, 1);
        assert_eq!(drivers[0].driver_name, "Porsche Penske Motorsport");
        assert_eq!(drivers[0].points, 152.0);

        let mfg = parse_wec_constructors_html(html_sample).unwrap();
        assert_eq!(mfg.len(), 2);
        assert_eq!(mfg[0].position, 1);
        assert_eq!(mfg[0].name, "Porsche Penske Motorsport");
        assert_eq!(mfg[0].points, 152.0);
    }

    #[test]
    fn test_parse_wec_results_html() {
        let sample = r#"
            <table class="results-table">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Porsche Penske Motorsport</td>
                        <td>#6</td>
                        <td>Estre / Lotterer / Vanthoor</td>
                        <td>25</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Toyota Gazoo Racing</td>
                        <td>#7</td>
                        <td>Conway / Kobayashi / de Vries</td>
                        <td>18</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let results = parse_wec_results_html(sample, 1).unwrap();
        assert_eq!(results.series_id, "wec");
        assert_eq!(results.round, 1);
        assert_eq!(results.results.len(), 2);
        assert_eq!(results.results[0].position, Some(1));
        assert_eq!(
            results.results[0].driver_name,
            "Estre / Lotterer / Vanthoor"
        );
        assert_eq!(results.results[0].driver_number, Some(6));
        assert_eq!(results.results[0].points, 25.0);
    }
}
