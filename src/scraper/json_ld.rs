use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::data::models::{EventStatus, RaceEvent, Session, SessionType, StreamLink};

#[derive(Debug, Deserialize)]
pub struct JsonLdRoot {
    #[serde(rename = "@graph", default)]
    pub graph: Vec<JsonLdSportsEvent>,
}

#[derive(Debug, Deserialize)]
pub struct JsonLdSportsEvent {
    #[serde(rename = "@type")]
    pub event_type: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub location: Option<JsonLdLocation>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JsonLdLocation {
    pub name: Option<String>,
    pub address: Option<JsonLdAddress>,
}

#[derive(Debug, Deserialize)]
pub struct JsonLdAddress {
    #[serde(rename = "addressCountry")]
    pub address_country: Option<String>,
}

/// Fetch and parse dynamic race calendar from HTML containing schema.org JSON-LD (e.g. raceweek.io).
pub async fn fetch_json_ld_calendar(
    client: &Client,
    page_url: &str,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Result<Vec<RaceEvent>> {
    let resp = client
        .get(page_url)
        .header("User-Agent", "Mozilla/5.0 (compatible; racetui/0.1.0)")
        .send()
        .await
        .with_context(|| format!("Failed to fetch HTML from {}", page_url))?;

    let html = resp
        .text()
        .await
        .with_context(|| format!("Failed to read HTML response from {}", page_url))?;

    parse_json_ld_html(&html, series_id, default_stream_links)
}

/// Parse HTML page containing schema.org JSON-LD into RaceEvent models.
pub fn parse_json_ld_html(
    html: &str,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Result<Vec<RaceEvent>> {
    // Find <script type="application/ld+json">...</script>
    let script_start = html
        .find("application/ld+json")
        .context("No JSON-LD script found in HTML")?;
    let tag_close = html[script_start..]
        .find('>')
        .context("Malformed JSON-LD tag")?;
    let content_start = script_start + tag_close + 1;
    let content_end = html[content_start..]
        .find("</script>")
        .context("Unclosed JSON-LD script tag")?;
    let json_text = &html[content_start..content_start + content_end];

    parse_json_ld_str(json_text, series_id, default_stream_links)
}

/// Parse raw JSON-LD string into RaceEvent models.
pub fn parse_json_ld_str(
    json_str: &str,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Result<Vec<RaceEvent>> {
    let root: JsonLdRoot =
        serde_json::from_str(json_str).context("Failed to deserialize JSON-LD graph")?;

    let now = Utc::now();
    let today = now.date_naive();
    let mut events = Vec::new();

    for (idx, item) in root.graph.into_iter().enumerate() {
        if item.event_type.as_deref() != Some("SportsEvent") {
            continue;
        }

        let full_name = item.name.unwrap_or_else(|| format!("Round {}", idx + 1));
        let event_name = clean_event_name(&full_name);

        let (circuit_name, country) = if let Some(loc) = item.location {
            let c_name = loc.name.unwrap_or_default();
            let c_country = loc
                .address
                .and_then(|a| a.address_country)
                .unwrap_or_default();
            (c_name, c_country)
        } else {
            (String::new(), String::new())
        };

        let start_time = item
            .start_date
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let end_time = item
            .end_date
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let start_date = start_time
            .map(|st| st.date_naive())
            .or_else(|| {
                item.start_date
                    .as_deref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            })
            .unwrap_or(today);

        let end_date = end_time
            .map(|et| et.date_naive())
            .or_else(|| {
                item.end_date
                    .as_deref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            })
            .unwrap_or(start_date);

        let status = if end_date < today {
            EventStatus::Completed
        } else if start_date <= today && today <= end_date {
            EventStatus::Live
        } else {
            EventStatus::Upcoming
        };

        // Create rich sessions with exact start times per series
        let sessions = build_series_sessions(series_id, start_date, end_date, start_time, end_time);

        events.push(RaceEvent {
            series_id: series_id.to_string(),
            event_name,
            circuit_name: if circuit_name.is_empty() {
                "Circuit".to_string()
            } else {
                circuit_name
            },
            location: country.clone(),
            country,
            start_date,
            end_date,
            round: Some((events.len() + 1) as u32),
            sessions,
            stream_links: default_stream_links.to_vec(),
            status,
        });
    }

    Ok(events)
}

fn build_series_sessions(
    series_id: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
) -> Vec<Session> {
    let mut sessions = Vec::new();

    match series_id {
        "btcc" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(14, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(10, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(13, 25, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race 3".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(16, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
        }
        "dtm" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Qualifying 1".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(7, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(11, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Qualifying 2".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sun_date
                    .and_hms_opt(7, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(11, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
        }
        "super_gt" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(5, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: start_time.or_else(|| {
                    sun_date
                        .and_hms_opt(4, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }),
                end_time,
            });
        }
        "super_formula" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(0, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: start_time.or_else(|| {
                    sun_date
                        .and_hms_opt(5, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }),
                end_time,
            });
        }
        "wrc" | "wrc2" | "erc" => {
            let thu_date = start_date;
            let fri_date = thu_date.succ_opt().unwrap_or(thu_date);
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;
            sessions.push(Session {
                name: "Shakedown".to_string(),
                session_type: SessionType::Practice,
                start_time: thu_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Leg 1".to_string(),
                session_type: SessionType::Race,
                start_time: fri_date
                    .and_hms_opt(7, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Leg 2".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(7, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Power Stage".to_string(),
                session_type: SessionType::Race,
                start_time: start_time.or_else(|| {
                    sun_date
                        .and_hms_opt(12, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }),
                end_time,
            });
        }
        "wec" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;
            sessions.push(Session {
                name: "Free Practice".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: start_time.or_else(|| {
                    sun_date
                        .and_hms_opt(11, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }),
                end_time,
            });
        }
        "imsa" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: start_time.or_else(|| {
                    sun_date
                        .and_hms_opt(17, 40, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }),
                end_time,
            });
        }
        _ => {
            if let Some(st) = start_time {
                sessions.push(Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: Some(st),
                    end_time,
                });
            } else {
                sessions.push(Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: end_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time,
                });
            }
        }
    }

    sessions
}

fn clean_event_name(full: &str) -> String {
    if let Some((_prefix, suffix)) = full.split_once(" — ") {
        suffix.trim().to_string()
    } else {
        full.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_ld_str() {
        let sample = r#"{
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "SportsEvent",
                    "name": "IMSA WeatherTech SportsCar Championship — 24 Hours of Daytona",
                    "startDate": "2026-01-24T18:40:00Z",
                    "endDate": "2026-01-25T18:40:00Z",
                    "location": {
                        "@type": "Place",
                        "name": "Daytona International Speedway",
                        "address": {
                            "@type": "PostalAddress",
                            "addressCountry": "US"
                        }
                    }
                }
            ]
        }"#;

        let events = parse_json_ld_str(sample, "imsa", &[]).unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_name, "24 Hours of Daytona");
        assert_eq!(ev.circuit_name, "Daytona International Speedway");
        assert_eq!(ev.sessions.len(), 2);
        assert_eq!(ev.sessions[0].name, "Qualifying");
        assert_eq!(ev.sessions[1].name, "Race");
        assert!(ev.sessions[0].start_time.is_some());
        assert!(ev.sessions[1].start_time.is_some());
    }
}
