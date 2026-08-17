use std::collections::HashSet;
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Utc};
use scraper::{Html, Selector};

use super::{fetcher, SeriesScraper};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};

pub struct IndyCarScraper;

impl SeriesScraper for IndyCarScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = fetcher::create_http_client()?;
        let html_text = client
            .get(&series.calendar_url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch IndyCar schedule from {}", series.calendar_url))?
            .text()
            .await
            .with_context(|| "Failed to read IndyCar schedule response body")?;

        parse_indycar_html(&html_text, &series.id)
    }
}

/// Parse date string in format "Aug 16" or "Mar 1" into a NaiveDate with the given year.
pub fn parse_month_day(date_str: &str, year: i32) -> Option<NaiveDate> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let month_str = parts[0];
    let day_str = parts[1].trim_matches(|c: char| !c.is_ascii_digit());
    let day: u32 = day_str.parse().ok()?;

    let month = match month_str.to_lowercase().as_str() {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    };

    NaiveDate::from_ymd_opt(year, month, day)
}

/// Parse time string like "12:00 PM ET" or "11:30 AM ET" into a DateTime<Utc>.
/// Assumes ET (EDT UTC-4 during daylight saving months Mar-Nov, EST UTC-5 in winter).
pub fn parse_indycar_time(date: NaiveDate, time_str: &str) -> Option<DateTime<Utc>> {
    let clean = time_str.trim().replace("ET", "").replace("EDT", "").replace("EST", "");
    let clean = clean.trim();
    let parts: Vec<&str> = clean.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let time_parts: Vec<&str> = parts[0].split(':').collect();
    let mut hour: u32 = time_parts[0].parse().ok()?;
    let min: u32 = if time_parts.len() > 1 {
        time_parts[1].parse().ok()?
    } else {
        0
    };

    let is_pm = parts[1].eq_ignore_ascii_case("pm");
    if is_pm && hour < 12 {
        hour += 12;
    } else if !is_pm && hour == 12 {
        hour = 0;
    }

    let naive_time = chrono::NaiveTime::from_hms_opt(hour, min, 0)?;
    let naive_dt = NaiveDateTime::new(date, naive_time);

    // US Eastern offset (EDT is UTC-4 from 2nd Sunday in March to 1st Sunday in November)
    let offset_hours = if (3..=11).contains(&date.month()) { 4 } else { 5 };
    let offset = chrono::FixedOffset::west_opt(offset_hours * 3600)?;
    let local_dt = offset.from_local_datetime(&naive_dt).single()?;
    Some(local_dt.with_timezone(&Utc))
}

pub fn parse_indycar_html(html_text: &str, series_id: &str) -> Result<Vec<RaceEvent>> {
    let document = Html::parse_document(html_text);
    let card_selector = Selector::parse(".event-card")
        .map_err(|e| anyhow::anyhow!("Invalid card selector: {:?}", e))?;
    let date_selector = Selector::parse(".event-card-header-date")
        .map_err(|e| anyhow::anyhow!("Invalid date selector: {:?}", e))?;
    let time_selector = Selector::parse(".event-card-header-time")
        .map_err(|e| anyhow::anyhow!("Invalid time selector: {:?}", e))?;
    let title_selector = Selector::parse(".event-card-title")
        .map_err(|e| anyhow::anyhow!("Invalid title selector: {:?}", e))?;
    let track_selector = Selector::parse(".event-card-track-name")
        .map_err(|e| anyhow::anyhow!("Invalid track selector: {:?}", e))?;
    let loc_selector = Selector::parse(".event-card-track-location")
        .map_err(|e| anyhow::anyhow!("Invalid loc selector: {:?}", e))?;

    let year = chrono::Utc::now().year();
    let today = chrono::Utc::now().date_naive();
    let mut events = Vec::new();
    let mut seen_keys = HashSet::new();

    for card in document.select(&card_selector) {
        let title_elem = match card.select(&title_selector).next() {
            Some(e) => e,
            None => continue,
        };
        let date_elem = match card.select(&date_selector).next() {
            Some(e) => e,
            None => continue,
        };

        let title = title_elem
            .text()
            .collect::<String>()
            .trim()
            .replace("&#39;", "'")
            .to_string();
        let date_raw = date_elem.text().collect::<String>().trim().to_string();

        let date = match parse_month_day(&date_raw, year) {
            Some(d) => d,
            None => continue,
        };

        let key = (title.clone(), date);
        if seen_keys.contains(&key) {
            continue;
        }
        seen_keys.insert(key);

        let time_raw = card
            .select(&time_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let track = card
            .select(&track_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "TBD".to_string());

        let location = card
            .select(&loc_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "USA".to_string());

        let (country, city) = if location.contains(',') {
            let parts: Vec<&str> = location.split(',').collect();
            let city = parts[0].trim().to_string();
            let state_or_country = parts[1].trim();
            let country = if state_or_country.contains("Ontario") || state_or_country.contains("Canada") {
                "Canada".to_string()
            } else {
                "USA".to_string()
            };
            (country, city)
        } else {
            ("USA".to_string(), location.clone())
        };

        let start_time_utc = parse_indycar_time(date, &time_raw);

        let status = if date < today {
            EventStatus::Completed
        } else if date == today {
            EventStatus::Live
        } else {
            EventStatus::Upcoming
        };

        let sessions = if let Some(start_dt) = start_time_utc {
            vec![Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: Some(start_dt),
                end_time: Some(start_dt + chrono::Duration::hours(3)),
            }]
        } else {
            vec![]
        };

        let stream_links = vec![
            StreamLink {
                platform: "FOX Sports".to_string(),
                url: "https://www.foxsports.com/live".to_string(),
                access: StreamAccess::Paid,
            },
            StreamLink {
                platform: "Peacock".to_string(),
                url: "https://www.peacocktv.com".to_string(),
                access: StreamAccess::Paid,
            },
            StreamLink {
                platform: "IndyCar Live".to_string(),
                url: "https://www.indycar.com/INDYCAR-Live".to_string(),
                access: StreamAccess::Paid,
            },
        ];

        events.push(RaceEvent {
            series_id: series_id.to_string(),
            event_name: title,
            circuit_name: track,
            location: city,
            country,
            start_date: date,
            end_date: date,
            round: None,
            sessions,
            stream_links,
            status,
        });
    }

    events.sort_by_key(|e| (e.start_date, e.event_name.clone()));
    for (i, event) in events.iter_mut().enumerate() {
        event.round = Some((i + 1) as u32);
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_month_day() {
        assert_eq!(
            parse_month_day("Aug 16", 2026),
            NaiveDate::from_ymd_opt(2026, 8, 16)
        );
        assert_eq!(
            parse_month_day("May 24", 2026),
            NaiveDate::from_ymd_opt(2026, 5, 24)
        );
        assert_eq!(
            parse_month_day("Mar 1", 2026),
            NaiveDate::from_ymd_opt(2026, 3, 1)
        );
    }

    #[test]
    fn test_parse_indycar_time() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        let dt = parse_indycar_time(date, "11:30 AM ET").unwrap();
        // EDT is UTC-4 in May -> 11:30 AM EDT = 15:30 UTC
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-05-24 15:30");
    }

    #[test]
    fn test_parse_indycar_html_sample() {
        let sample_html = r#"
        <div class="schedule-list-container">
            <div class="event-card event-card-completed">
                <button class="event-card-expand-collapse-header-button">
                    <div class="event-card-header-elements">
                        <div class="event-card-header-date">May 24</div>
                        <div class="event-card-header-time">11:00 AM ET</div>
                    </div>
                </button>
                <div class="event-card-container">
                    <div class="event-card-title-container">
                        <h3 class="event-card-title">110th Running of the Indianapolis 500</h3>
                    </div>
                    <div class="event-card-track-details">
                        <div class="event-card-track-name">Indianapolis Motor Speedway</div>
                        <div class="event-card-track-location">Indianapolis, Indiana</div>
                    </div>
                </div>
            </div>
        </div>
        "#;

        let events = parse_indycar_html(sample_html, "indycar").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "110th Running of the Indianapolis 500");
        assert_eq!(events[0].circuit_name, "Indianapolis Motor Speedway");
        assert_eq!(events[0].location, "Indianapolis");
        assert_eq!(events[0].country, "USA");
        assert_eq!(events[0].round, Some(1));
    }
}
