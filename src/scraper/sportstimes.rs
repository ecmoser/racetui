use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::data::models::{EventStatus, RaceEvent, Session, SessionType, StreamLink};

#[derive(Debug, Deserialize)]
pub struct CalendarApiResponse {
    pub races: Vec<CalendarApiRace>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarApiRace {
    pub name: Option<String>,
    pub location: Option<String>,
    pub round: Option<u32>,
    pub slug: Option<String>,
    pub sessions: Option<HashMap<String, String>>,
}

/// Fetch and parse dynamic race calendar from sportstimes / f1calendar API across current and future years.
pub async fn fetch_sportstimes_calendar(
    client: &Client,
    api_url: &str,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Result<Vec<RaceEvent>> {
    let current_year = Utc::now().year();
    let mut all_events = Vec::new();

    // Normalize base URL template (e.g. "https://f1calendar.com/api/year" or "https://f1calendar.com")
    let base_template = if let Some(stripped) = api_url.strip_suffix("/2026") {
        stripped
    } else if let Some(stripped) = api_url.strip_suffix("/2025") {
        stripped
    } else {
        api_url.trim_end_matches('/')
    };

    let base_api_url = if base_template.ends_with("/api/year") {
        base_template.to_string()
    } else {
        format!("{}/api/year", base_template)
    };

    // Query current year and upcoming years (e.g., 2026, 2027, 2028)
    for year in current_year..=current_year + 2 {
        let year_url = format!("{}/{}", base_api_url, year);
        if let Ok(resp) = client
            .get(&year_url)
            .header("User-Agent", "racetui/0.1.0")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(mut events) =
                        parse_sportstimes_json(&text, series_id, default_stream_links)
                    {
                        all_events.append(&mut events);
                    }
                }
            }
        }
    }

    // If multi-year scan did not find anything, fall back to exact input URL
    if all_events.is_empty() {
        let resp = client
            .get(api_url)
            .header("User-Agent", "racetui/0.1.0")
            .send()
            .await
            .with_context(|| format!("Failed to fetch calendar from {}", api_url))?;

        let text = resp
            .text()
            .await
            .with_context(|| format!("Failed to read response from {}", api_url))?;

        all_events = parse_sportstimes_json(&text, series_id, default_stream_links)?;
    }

    Ok(all_events)
}

/// Parse raw sportstimes / f1calendar JSON string into RaceEvent models.
pub fn parse_sportstimes_json(
    json_str: &str,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Result<Vec<RaceEvent>> {
    let api_resp: CalendarApiResponse =
        serde_json::from_str(json_str).context("Failed to deserialize calendar JSON")?;

    let now = Utc::now();
    let today = now.date_naive();
    let mut events = Vec::new();

    for (idx, r) in api_resp.races.into_iter().enumerate() {
        let round_num = r.round.unwrap_or((idx + 1) as u32);
        let raw_name = r.name.unwrap_or_else(|| format!("Round {}", round_num));
        let location = r.location.clone().unwrap_or_else(|| raw_name.clone());

        // Parse individual sessions
        let mut sessions = Vec::new();
        let mut min_date: Option<NaiveDate> = None;
        let mut max_date: Option<NaiveDate> = None;

        if let Some(sessions_map) = r.sessions {
            // Sort sessions chronologically by start time, then order key
            let mut session_entries: Vec<(String, String)> = sessions_map.into_iter().collect();
            session_entries.sort_by_key(|(k, time_str)| {
                let parsed = DateTime::parse_from_rfc3339(time_str).ok();
                (parsed, session_sort_order(k))
            });

            for (session_key, time_str) in session_entries {
                let parsed_time = DateTime::parse_from_rfc3339(&time_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok();

                if let Some(st) = parsed_time {
                    let d = st.date_naive();
                    min_date = Some(min_date.map_or(d, |curr| curr.min(d)));
                    max_date = Some(max_date.map_or(d, |curr| curr.max(d)));
                }

                let session_name = format_session_name(&session_key);
                let session_type = map_session_type(&session_key);

                sessions.push(Session {
                    name: session_name,
                    session_type,
                    start_time: parsed_time,
                    end_time: parsed_time.map(|st| st + chrono::Duration::hours(2)),
                });
            }
        }

        let start_date = min_date.unwrap_or(today);
        let end_date = max_date.unwrap_or(start_date);

        let status = if end_date < today {
            EventStatus::Completed
        } else if start_date <= today && today <= end_date {
            EventStatus::Live
        } else {
            EventStatus::Upcoming
        };

        let event_name = format_event_name(series_id, &raw_name);

        events.push(RaceEvent {
            series_id: series_id.to_string(),
            event_name,
            circuit_name: location.clone(),
            location,
            country: String::new(),
            start_date,
            end_date,
            round: Some(round_num),
            sessions,
            stream_links: default_stream_links.to_vec(),
            status,
        });
    }

    Ok(events)
}

fn session_sort_order(key: &str) -> u32 {
    let k = key.to_lowercase();
    if k.contains("fp1") || k.contains("practice1") || k == "practice" || k.contains("shakedown") {
        10
    } else if k.contains("fp2") || k.contains("practice2") {
        20
    } else if k.contains("fp3") || k.contains("practice3") {
        30
    } else if k.contains("sprintqualifying") || k.contains("sprint_qualifying") {
        40
    } else if k.contains("sprint") {
        50
    } else if k.contains("qualifying1") || k == "qualifying" {
        60
    } else if k.contains("qualifying2") {
        70
    } else if k.contains("warmup") {
        80
    } else if k.contains("feature") || k == "gp" || k == "race" {
        90
    } else {
        100
    }
}

fn map_session_type(key: &str) -> SessionType {
    let k = key.to_lowercase();
    if k.contains("sprintqualifying") || k.contains("sprint_qualifying") || k.contains("shootout") {
        SessionType::SprintQualifying
    } else if k.contains("sprint") {
        SessionType::Sprint
    } else if k.contains("fp")
        || k.contains("practice")
        || k.contains("shakedown")
        || k.contains("warmup")
    {
        SessionType::Practice
    } else if k.contains("qualifying") || k.contains("superpole") {
        SessionType::Qualifying
    } else if k.contains("race") || k.contains("feature") || k == "gp" {
        SessionType::Race
    } else {
        SessionType::Other(key.to_string())
    }
}

fn format_session_name(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "fp1" => "Free Practice 1".to_string(),
        "fp2" => "Free Practice 2".to_string(),
        "fp3" => "Free Practice 3".to_string(),
        "practice" => "Practice".to_string(),
        "practice1" => "Practice 1".to_string(),
        "practice2" => "Practice 2".to_string(),
        "practice3" => "Practice 3".to_string(),
        "finalpractice" => "Final Practice".to_string(),
        "sprintqualifying" => "Sprint Qualifying".to_string(),
        "sprint" => "Sprint Race".to_string(),
        "qualifying" => "Qualifying".to_string(),
        "qualifying1" => "Qualifying 1".to_string(),
        "qualifying2" => "Qualifying 2".to_string(),
        "warmup" => "Warmup".to_string(),
        "gp" => "Grand Prix".to_string(),
        "race" => "Race".to_string(),
        "feature" => "Feature Race".to_string(),
        _ => key.to_string(),
    }
}

fn format_event_name(series_id: &str, raw_name: &str) -> String {
    if raw_name.to_lowercase().contains("grand prix")
        || raw_name.to_lowercase().contains("eprix")
        || raw_name.to_lowercase().contains("500")
        || raw_name.to_lowercase().contains("250")
        || raw_name.to_lowercase().contains("race")
    {
        raw_name.to_string()
    } else {
        match series_id {
            "f1" | "f2" | "f3" => format!("{} Grand Prix", raw_name),
            "formula_e" => format!("{} ePrix", raw_name),
            "motogp" | "moto2" | "moto3" => format!("Grand Prix of {}", raw_name),
            _ => raw_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sportstimes_json() {
        let sample = r#"{
            "races": [
                {
                    "name": "Australian",
                    "location": "Melbourne",
                    "round": 1,
                    "slug": "melbourne",
                    "sessions": {
                        "practice": "2026-03-05T23:00:00Z",
                        "qualifying": "2026-03-06T03:55:00Z",
                        "sprint": "2026-03-07T03:30:00Z",
                        "feature": "2026-03-08T00:25:00Z"
                    }
                }
            ]
        }"#;

        let events = parse_sportstimes_json(sample, "f2", &[]).unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_name, "Australian Grand Prix");
        assert_eq!(ev.round, Some(1));
        assert_eq!(ev.sessions.len(), 4);
        assert_eq!(ev.sessions[0].name, "Practice");
        assert_eq!(ev.sessions[1].name, "Qualifying");
        assert_eq!(ev.sessions[2].name, "Sprint Race");
        assert_eq!(ev.sessions[3].name, "Feature Race");
    }
}
