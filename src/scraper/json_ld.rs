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
        } else {
            EventStatus::Upcoming
        };

        // Create rich sessions with exact start times per series
        let sessions = build_series_sessions(
            series_id,
            &event_name,
            start_date,
            end_date,
            start_time,
            end_time,
        );

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

pub fn build_series_sessions(
    series_id: &str,
    event_name: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
) -> Vec<Session> {
    let mut sessions = Vec::new();
    let name_lower = event_name.to_lowercase();

    match series_id {
        "elms" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            // Free Practice 1 on Friday (uses scraped start_time if on Friday morning, else 09:00 UTC)
            let fp1_time = if let Some(st) = start_time {
                if st.date_naive() == fri_date {
                    Some(st)
                } else {
                    fri_date
                        .and_hms_opt(9, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }
            } else {
                fri_date
                    .and_hms_opt(9, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            };

            sessions.push(Session {
                name: "Free Practice 1".to_string(),
                session_type: SessionType::Practice,
                start_time: fp1_time,
                end_time: fp1_time.map(|t| t + chrono::Duration::minutes(90)),
            });
            sessions.push(Session {
                name: "Free Practice 2".to_string(),
                session_type: SessionType::Practice,
                start_time: sat_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(9, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(14, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });

            // 4 Hours Race on Sunday (typically 11:00 UTC to 15:00 UTC)
            let race_start = sun_date
                .and_hms_opt(11, 0, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_end = end_time.or_else(|| race_start.map(|t| t + chrono::Duration::hours(4)));
            sessions.push(Session {
                name: "Race (4 Hours)".to_string(),
                session_type: SessionType::Race,
                start_time: race_start,
                end_time: race_end,
            });
        }
        "aslms" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            let fp_time = if let Some(st) = start_time {
                if st.date_naive() == fri_date {
                    Some(st)
                } else {
                    fri_date
                        .and_hms_opt(6, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }
            } else {
                fri_date
                    .and_hms_opt(6, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            };

            sessions.push(Session {
                name: "Free Practice".to_string(),
                session_type: SessionType::Practice,
                start_time: fp_time,
                end_time: fp_time.map(|t| t + chrono::Duration::minutes(90)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(2, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(3, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1 (4 Hours)".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(6, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(10, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2 (4 Hours)".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(6, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(10, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "gtwc_eu" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            if name_lower.contains("24") || name_lower.contains("spa") {
                let thu_date = fri_date.pred_opt().unwrap_or(fri_date);
                sessions.push(Session {
                    name: "Free Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: thu_date
                        .and_hms_opt(9, 20, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: thu_date
                        .and_hms_opt(10, 50, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Pre-Qualifying".to_string(),
                    session_type: SessionType::Practice,
                    start_time: thu_date
                        .and_hms_opt(14, 10, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: thu_date
                        .and_hms_opt(15, 10, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: thu_date
                        .and_hms_opt(18, 40, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: thu_date
                        .and_hms_opt(20, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Superpole".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: fri_date
                        .and_hms_opt(13, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sat_date
                    .and_hms_opt(14, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "24 Hours of Spa".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(24)),
                });
            } else if name_lower.contains("endurance")
                || name_lower.contains("1000")
                || name_lower.contains("6h")
                || name_lower.contains("3h")
            {
                sessions.push(Session {
                    name: "Free Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fri_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(14, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Pre-Qualifying".to_string(),
                    session_type: SessionType::Practice,
                    start_time: sat_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(14, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sun_date
                        .and_hms_opt(7, 45, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sun_date
                        .and_hms_opt(8, 45, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sun_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Endurance Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(3)),
                });
            } else {
                // Sprint Cup
                sessions.push(Session {
                    name: "Free Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fri_date
                        .and_hms_opt(7, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(8, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying 1".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(7, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(8, 20, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Race 1".to_string(),
                    session_type: SessionType::Race,
                    start_time: sat_date
                        .and_hms_opt(12, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying 2".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sun_date
                        .and_hms_opt(7, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sun_date
                        .and_hms_opt(8, 20, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Race 2".to_string(),
                    session_type: SessionType::Race,
                    start_time: sun_date
                        .and_hms_opt(12, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sun_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
            }
        }
        "gtwc_am" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Practice 1".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(15, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(16, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Practice 2".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(20, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(21, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(14, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(19, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(20, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(18, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(19, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "igtc" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            if name_lower.contains("bathurst") || name_lower.contains("12") {
                sessions.push(Session {
                    name: "Free Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fri_date
                        .and_hms_opt(3, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(4, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying & Shootout".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(3, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(6, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sat_date
                    .and_hms_opt(18, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Bathurst 12 Hour".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(12)),
                });
            } else if name_lower.contains("nürburgring")
                || name_lower.contains("nurburgring")
                || name_lower.contains("24h")
            {
                sessions.push(Session {
                    name: "Top Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: fri_date
                        .and_hms_opt(15, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(17, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sat_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "24 Hours of Nürburgring".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(24)),
                });
            } else {
                sessions.push(Session {
                    name: "Free Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fri_date
                        .and_hms_opt(10, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(11, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(10, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(11, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sun_date
                    .and_hms_opt(10, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Race (Endurance)".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(8)),
                });
            }
        }
        "worldsbk" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Free Practice 1".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(8, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(9, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Free Practice 2".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(13, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Superpole".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(9, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(9, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(12, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Superpole Race".to_string(),
                session_type: SessionType::Sprint,
                start_time: sun_date
                    .and_hms_opt(9, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(9, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(12, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "bsb" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Free Practice 1".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(10, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(10, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Free Practice 2".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(14, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(12, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(12, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Sprint Race".to_string(),
                session_type: SessionType::Sprint,
                start_time: sat_date
                    .and_hms_opt(15, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(15, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(12, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(13, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 3".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(15, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(16, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "supercars" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Practice 1".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(1, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(2, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Practice 2".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(5, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(6, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying 1".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(2, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(3, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(6, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying 2".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sun_date
                    .and_hms_opt(2, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(3, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(5, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(7, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "tcr" | "tcr_world" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Practice 1".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(8, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(9, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(10, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(15, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "f1_academy" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Practice".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(8, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(9, 10, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: fri_date
                    .and_hms_opt(16, 10, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(16, 40, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(11, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(11, 50, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(9, 5, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(9, 40, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "indy_nxt" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Practice".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(17, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(18, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(17, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(17, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(15, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(16, 35, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "porsche_supercup" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            sessions.push(Session {
                name: "Practice".to_string(),
                session_type: SessionType::Practice,
                start_time: fri_date
                    .and_hms_opt(16, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(16, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(10, 20, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(10, 50, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(10, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(11, 20, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "nls" => {
            let sat_date = end_date;
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(6, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race (4h / 6h)".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(10, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "extreme_e" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Qualifying 1 & 2".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(9, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(11, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Round 1 Grand Final".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(15, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Round 2 Grand Final".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(15, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "dakar" => {
            sessions.push(Session {
                name: "Prologue / Stage 1".to_string(),
                session_type: SessionType::Practice,
                start_time: start_date
                    .and_hms_opt(5, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Marathon Stages".to_string(),
                session_type: SessionType::Other("Rally Raid".to_string()),
                start_time: start_date
                    .succ_opt()
                    .unwrap_or(start_date)
                    .and_hms_opt(4, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Final Stage".to_string(),
                session_type: SessionType::Race,
                start_time: end_date
                    .and_hms_opt(6, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: None,
            });
        }
        "btcc" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Free Practice 1".to_string(),
                session_type: SessionType::Practice,
                start_time: sat_date
                    .and_hms_opt(8, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(9, 10, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(14, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(15, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(10, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(11, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(13, 25, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(14, 10, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 3".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(16, 15, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(17, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
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
                end_time: sat_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 1".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(11, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(12, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying 2".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sun_date
                    .and_hms_opt(7, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(8, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Race 2".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(11, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sun_date
                    .and_hms_opt(12, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
        }
        "super_gt" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Official Practice".to_string(),
                session_type: SessionType::Practice,
                start_time: sat_date
                    .and_hms_opt(0, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(1, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(5, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(6, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            let race_start = sun_date
                .and_hms_opt(5, 30, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: race_start,
                end_time: end_time.or_else(|| race_start.map(|t| t + chrono::Duration::hours(3))),
            });
        }
        "super_formula" => {
            let sat_date = start_date;
            let sun_date = end_date;
            sessions.push(Session {
                name: "Free Practice".to_string(),
                session_type: SessionType::Practice,
                start_time: sat_date
                    .and_hms_opt(0, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(1, 30, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: sat_date
                    .and_hms_opt(5, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(5, 45, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            let race_start = sun_date
                .and_hms_opt(5, 30, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: race_start,
                end_time: end_time.or_else(|| race_start.map(|t| t + chrono::Duration::hours(2))),
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
                end_time: thu_date
                    .and_hms_opt(11, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Leg 1".to_string(),
                session_type: SessionType::Race,
                start_time: fri_date
                    .and_hms_opt(7, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: fri_date
                    .and_hms_opt(17, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Leg 2".to_string(),
                session_type: SessionType::Race,
                start_time: sat_date
                    .and_hms_opt(7, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: sat_date
                    .and_hms_opt(17, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
            });
            sessions.push(Session {
                name: "Power Stage (Final Leg)".to_string(),
                session_type: SessionType::Race,
                start_time: sun_date
                    .and_hms_opt(11, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                end_time: end_time.or_else(|| {
                    sun_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                }),
            });
        }
        "wec" => {
            let fri_date = start_date;
            let sat_date = fri_date.succ_opt().unwrap_or(fri_date);
            let sun_date = end_date;

            if name_lower.contains("le mans") || name_lower.contains("24 hours") {
                let thu_date = fri_date.pred_opt().unwrap_or(fri_date);
                sessions.push(Session {
                    name: "Free Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: thu_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: thu_date
                        .and_hms_opt(16, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Hyperpole".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: thu_date
                        .and_hms_opt(18, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: thu_date
                        .and_hms_opt(19, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sat_date
                    .and_hms_opt(14, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "24 Hours of Le Mans".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(24)),
                });
            } else {
                sessions.push(Session {
                    name: "Free Practice 1".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fri_date
                        .and_hms_opt(8, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(9, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });

                let race_day = if name_lower.contains("qatar")
                    || name_lower.contains("bahrain")
                    || name_lower.contains("spa")
                {
                    sat_date
                } else {
                    sun_date
                };
                let race_start = race_day
                    .and_hms_opt(11, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Race (6 Hours)".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: end_time
                        .or_else(|| race_start.map(|t| t + chrono::Duration::hours(6))),
                });
            }
        }
        "imsa" => {
            let sat_date = if start_date == end_date {
                start_date
            } else {
                end_date.pred_opt().unwrap_or(start_date)
            };
            let sun_date = end_date;

            if name_lower.contains("daytona")
                || name_lower.contains("24 hours")
                || name_lower.contains("rolex 24")
            {
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .pred_opt()
                        .unwrap_or(sat_date)
                        .and_hms_opt(18, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .pred_opt()
                        .unwrap_or(sat_date)
                        .and_hms_opt(19, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sat_date
                    .and_hms_opt(18, 40, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Rolex 24 at Daytona".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(24)),
                });
            } else if name_lower.contains("sebring")
                || name_lower.contains("twelve hours")
                || name_lower.contains("12 hours")
            {
                let fri_date = sat_date.pred_opt().unwrap_or(sat_date);
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: fri_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(14, 15, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sat_date
                    .and_hms_opt(13, 40, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Mobil 1 Twelve Hours of Sebring".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: race_start.map(|t| t + chrono::Duration::hours(12)),
                });
            } else {
                let fri_date = start_date;
                sessions.push(Session {
                    name: "Practice 1".to_string(),
                    session_type: SessionType::Practice,
                    start_time: fri_date
                        .and_hms_opt(15, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: fri_date
                        .and_hms_opt(16, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = sun_date
                    .and_hms_opt(17, 40, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: end_time
                        .or_else(|| race_start.map(|t| t + chrono::Duration::minutes(160))),
                });
            }
        }
        _ => {
            if start_date < end_date {
                let sat_date = end_date.pred_opt().unwrap_or(start_date);
                sessions.push(Session {
                    name: "Practice".to_string(),
                    session_type: SessionType::Practice,
                    start_time: start_date
                        .and_hms_opt(9, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: start_date
                        .and_hms_opt(10, 30, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: sat_date
                        .and_hms_opt(13, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: sat_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                });
                let race_start = end_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: end_time
                        .or_else(|| race_start.map(|t| t + chrono::Duration::hours(2))),
                });
            } else if let Some(st) = start_time {
                sessions.push(Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: Some(st),
                    end_time: end_time.or_else(|| Some(st + chrono::Duration::hours(2))),
                });
            } else {
                let race_start = end_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                sessions.push(Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_start,
                    end_time: end_time
                        .or_else(|| race_start.map(|t| t + chrono::Duration::hours(2))),
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
        assert_eq!(ev.sessions[0].session_type, SessionType::Qualifying);
        assert_eq!(ev.sessions[1].session_type, SessionType::Race);
        assert!(ev.sessions[0].start_time.is_some());
        assert!(ev.sessions[1].start_time.is_some());
    }

    #[test]
    fn test_parse_elms_sessions() {
        let sample = r#"{
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "SportsEvent",
                    "name": "European Le Mans Series — 4 Hours of Barcelona",
                    "startDate": "2026-04-10T09:00:00Z",
                    "endDate": "2026-04-12T15:00:00Z",
                    "location": {
                        "@type": "Place",
                        "name": "Circuit de Barcelona-Catalunya",
                        "address": {
                            "@type": "PostalAddress",
                            "addressCountry": "ES"
                        }
                    }
                }
            ]
        }"#;

        let events = parse_json_ld_str(sample, "elms", &[]).unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_name, "4 Hours of Barcelona");
        assert_eq!(ev.sessions.len(), 4);
        assert_eq!(ev.sessions[0].name, "Free Practice 1");
        assert_eq!(ev.sessions[0].session_type, SessionType::Practice);
        assert_eq!(ev.sessions[1].name, "Free Practice 2");
        assert_eq!(ev.sessions[1].session_type, SessionType::Practice);
        assert_eq!(ev.sessions[2].name, "Qualifying");
        assert_eq!(ev.sessions[2].session_type, SessionType::Qualifying);
        assert_eq!(ev.sessions[3].name, "Race (4 Hours)");
        assert_eq!(ev.sessions[3].session_type, SessionType::Race);
    }

    #[test]
    fn test_parse_worldsbk_sessions() {
        let sample = r#"{
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "SportsEvent",
                    "name": "Superbike World Championship — Australian Round",
                    "startDate": "2026-02-20T08:30:00Z",
                    "endDate": "2026-02-22T13:00:00Z",
                    "location": {
                        "@type": "Place",
                        "name": "Phillip Island Grand Prix Circuit",
                        "address": {
                            "@type": "PostalAddress",
                            "addressCountry": "AU"
                        }
                    }
                }
            ]
        }"#;

        let events = parse_json_ld_str(sample, "worldsbk", &[]).unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.sessions.len(), 6);
        assert_eq!(ev.sessions[0].session_type, SessionType::Practice);
        assert_eq!(ev.sessions[1].session_type, SessionType::Practice);
        assert_eq!(ev.sessions[2].session_type, SessionType::Qualifying);
        assert_eq!(ev.sessions[3].session_type, SessionType::Race);
        assert_eq!(ev.sessions[4].session_type, SessionType::Sprint);
        assert_eq!(ev.sessions[5].session_type, SessionType::Race);
    }

    #[test]
    fn test_parse_f1_academy_sessions() {
        let sample = r#"{
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "SportsEvent",
                    "name": "F1 Academy — Jeddah Round",
                    "startDate": "2026-04-17T08:30:00Z",
                    "endDate": "2026-04-19T10:00:00Z",
                    "location": {
                        "@type": "Place",
                        "name": "Jeddah Corniche Circuit",
                        "address": {
                            "@type": "PostalAddress",
                            "addressCountry": "SA"
                        }
                    }
                }
            ]
        }"#;

        let events = parse_json_ld_str(sample, "f1_academy", &[]).unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.sessions.len(), 4);
        assert_eq!(ev.sessions[0].name, "Practice");
        assert_eq!(ev.sessions[0].session_type, SessionType::Practice);
        assert_eq!(ev.sessions[1].name, "Qualifying");
        assert_eq!(ev.sessions[1].session_type, SessionType::Qualifying);
        assert_eq!(ev.sessions[2].name, "Race 1");
        assert_eq!(ev.sessions[2].session_type, SessionType::Race);
        assert_eq!(ev.sessions[3].name, "Race 2");
        assert_eq!(ev.sessions[3].session_type, SessionType::Race);
    }

    #[test]
    fn test_parse_indy_nxt_sessions() {
        let sample = r#"{
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "SportsEvent",
                    "name": "INDY NXT — Grand Prix of St. Petersburg",
                    "startDate": "2026-02-27T17:30:00Z",
                    "endDate": "2026-03-01T16:35:00Z",
                    "location": {
                        "@type": "Place",
                        "name": "Streets of St. Petersburg",
                        "address": {
                            "@type": "PostalAddress",
                            "addressCountry": "US"
                        }
                    }
                }
            ]
        }"#;

        let events = parse_json_ld_str(sample, "indy_nxt", &[]).unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.sessions.len(), 3);
        assert_eq!(ev.sessions[0].name, "Practice");
        assert_eq!(ev.sessions[0].session_type, SessionType::Practice);
        assert_eq!(ev.sessions[1].name, "Qualifying");
        assert_eq!(ev.sessions[1].session_type, SessionType::Qualifying);
        assert_eq!(ev.sessions[2].name, "Race");
        assert_eq!(ev.sessions[2].session_type, SessionType::Race);
    }
}
