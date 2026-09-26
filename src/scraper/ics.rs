use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use std::collections::HashMap;

use crate::data::models::{EventStatus, RaceEvent, Session, SessionType, StreamLink};

/// A parsed VEVENT from an iCalendar (.ics) feed.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IcsVEvent {
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// Parse an iCalendar (.ics) string into a list of `RaceEvent`s.
pub fn parse_ics_str(
    ics_content: &str,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Result<Vec<RaceEvent>> {
    let unfolded = unfold_ics_lines(ics_content);
    let vevents = parse_vevents(&unfolded);

    if vevents.is_empty() {
        return Ok(Vec::new());
    }

    // Group VEvents by event/race weekend (by approximate weekend date and location/name)
    let grouped = group_vevents_into_events(vevents, series_id, default_stream_links);
    Ok(grouped)
}

/// Fetch an iCalendar feed from a URL and parse it into `RaceEvent`s.
#[allow(dead_code)]
pub async fn fetch_ics_calendar(
    client: &reqwest::Client,
    url: &str,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Result<Vec<RaceEvent>> {
    let res = client
        .get(url)
        .header("User-Agent", "racetui/0.1.0 (terminal motorsport calendar)")
        .send()
        .await
        .with_context(|| format!("Failed to fetch ICS calendar from {}", url))?;

    if !res.status().is_success() {
        anyhow::bail!("ICS calendar fetch returned status: {}", res.status());
    }

    let text = res
        .text()
        .await
        .with_context(|| "Failed to read ICS response body")?;

    parse_ics_str(&text, series_id, default_stream_links)
}

/// RFC 5545: Unfold lines where lines starting with a space or tab continue the previous line.
fn unfold_ics_lines(input: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(last) = lines.last_mut() {
                last.push_str(&line[1..]);
            }
        } else if !line.is_empty() {
            lines.push(line.to_string());
        }
    }
    lines
}

/// Parse unfolded lines into raw `IcsVEvent` structs.
fn parse_vevents(lines: &[String]) -> Vec<IcsVEvent> {
    let mut events = Vec::new();
    let mut in_vevent = false;
    let mut props: HashMap<String, String> = HashMap::new();

    for line in lines {
        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            in_vevent = true;
            props.clear();
        } else if line.eq_ignore_ascii_case("END:VEVENT") {
            if in_vevent {
                if let Some(event) = build_vevent_from_props(&props) {
                    events.push(event);
                }
                in_vevent = false;
            }
        } else if in_vevent {
            if let Some((key_part, val_part)) = line.split_once(':') {
                let key_name = key_part
                    .split(';')
                    .next()
                    .unwrap_or(key_part)
                    .to_uppercase();
                // Store raw key with parameters or base key
                props.insert(key_name, val_part.to_string());
                if key_part.contains(';') {
                    // Keep parameter values (notably TZID=Area/City) case-sensitive.
                    props.insert(key_part.to_string(), val_part.to_string());
                }
            }
        }
    }

    events
}

fn build_vevent_from_props(props: &HashMap<String, String>) -> Option<IcsVEvent> {
    let summary = props.get("SUMMARY")?.trim().to_string();
    let description = props
        .get("DESCRIPTION")
        .map(|s| unescape_ics_text(s.trim()));
    let location = props.get("LOCATION").map(|s| unescape_ics_text(s.trim()));
    let url = props.get("URL").map(|s| s.trim().to_string());

    let (start_raw, start_tzid) = ics_datetime_property(props, "DTSTART");
    let (end_raw, end_tzid) = ics_datetime_property(props, "DTEND");
    let (start_time, start_date) = parse_ics_datetime_or_date(start_raw, start_tzid);
    let (end_time, end_date) = parse_ics_datetime_or_date(end_raw, end_tzid);

    Some(IcsVEvent {
        summary,
        description,
        location,
        url,
        start_time,
        end_time,
        start_date,
        end_date,
    })
}

fn ics_datetime_property<'a>(
    props: &'a HashMap<String, String>,
    name: &str,
) -> (Option<&'a String>, Option<&'a str>) {
    // Prefer the parameterized key: parse_vevents also stores a bare DTSTART
    // entry for convenience, but that would discard a TZID.
    if let Some((key, value)) = props
        .iter()
        .find(|(key, _)| key.starts_with(&format!("{};", name)))
    {
        let tzid = key
            .split(';')
            .find_map(|parameter| parameter.strip_prefix("TZID="));
        return (Some(value), tzid);
    }

    (props.get(name), None)
}

fn parse_ics_datetime_or_date(
    val: Option<&String>,
    tzid: Option<&str>,
) -> (Option<DateTime<Utc>>, Option<NaiveDate>) {
    let raw = match val {
        Some(s) => s.trim(),
        None => return (None, None),
    };

    // 1. Format: YYYYMMDDTHHMMSSZ (UTC)
    if let Ok(dt) = DateTime::parse_from_str(raw, "%Y%m%dT%H%M%SZ") {
        let utc = dt.with_timezone(&Utc);
        return (Some(utc), Some(utc.date_naive()));
    }

    // 2. Format: YYYYMMDDTHHMMSS with an IANA TZID parameter.
    if let Ok(ndt) = NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S") {
        if let Some(tzid) = tzid {
            if let Ok(tz) = tzid.parse::<chrono_tz::Tz>() {
                if let Some(local) = tz.from_local_datetime(&ndt).earliest() {
                    let utc = local.with_timezone(&Utc);
                    return (Some(utc), Some(utc.date_naive()));
                }
            }
        }

        // RFC 5545 permits a floating time without TZID. Retain the previous UTC
        // interpretation for that ambiguous format; FIA WEC feeds use TZID/UTC.
        let utc = DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
        return (Some(utc), Some(ndt.date()));
    }

    // 3. Format: YYYYMMDD (Date only)
    if let Ok(d) = NaiveDate::parse_from_str(raw, "%Y%m%d") {
        return (None, Some(d));
    }

    (None, None)
}

fn unescape_ics_text(s: &str) -> String {
    s.replace(r"\,", ",")
        .replace(r"\;", ";")
        .replace(r"\n", "\n")
        .replace(r"\N", "\n")
        .replace(r"\\", r"\")
}

/// Classify session type from session summary text.
fn classify_session_type(name: &str) -> SessionType {
    let lower = name.to_lowercase();
    if lower.contains("qualif") || lower.contains("superpole") || lower.contains("pole") {
        SessionType::Qualifying
    } else if lower.contains("sprint") || lower.contains("superpole race") || lower.contains("heat")
    {
        SessionType::Sprint
    } else if lower.contains("practice")
        || lower.contains("fp1")
        || lower.contains("fp2")
        || lower.contains("fp3")
        || lower.contains("shakedown")
        || lower.contains("warmup")
        || lower.contains("warm-up")
    {
        SessionType::Practice
    } else {
        SessionType::Race
    }
}

/// Group raw VEvents into cohesive `RaceEvent`s.
fn group_vevents_into_events(
    mut vevents: Vec<IcsVEvent>,
    series_id: &str,
    default_stream_links: &[StreamLink],
) -> Vec<RaceEvent> {
    // Sort vevents chronologically by start date/time
    vevents.sort_by_key(|v| {
        (
            v.start_date
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            v.start_time,
        )
    });

    let mut race_events: Vec<RaceEvent> = Vec::new();

    for vevent in vevents {
        let start_date = vevent.start_date.unwrap_or_else(|| Utc::now().date_naive());
        let end_date = vevent.end_date.unwrap_or(start_date);

        let session_type = classify_session_type(&vevent.summary);
        let session = Session {
            name: vevent.summary.clone(),
            session_type: session_type.clone(),
            start_time: vevent.start_time,
            end_time: vevent.end_time,
        };

        // Check if this session belongs to an existing event (within 4 days and similar location/name)
        let mut attached = false;
        for ev in race_events.iter_mut().rev() {
            let days_diff = start_date
                .signed_duration_since(ev.start_date)
                .num_days()
                .abs();
            if days_diff <= 4 {
                // If same weekend, extend end_date and add session
                if end_date > ev.end_date {
                    ev.end_date = end_date;
                }
                ev.sessions.push(session.clone());
                attached = true;
                break;
            }
        }

        if !attached {
            let event_name = clean_ics_event_name(&vevent.summary);
            let loc = vevent.location.clone().unwrap_or_default();
            let circuit = if !loc.is_empty() {
                loc.clone()
            } else {
                event_name.clone()
            };

            let ev = RaceEvent {
                series_id: series_id.to_string(),
                event_name,
                circuit_name: circuit,
                location: loc.clone(),
                country: loc,
                start_date,
                end_date,
                round: Some((race_events.len() + 1) as u32),
                sessions: vec![session],
                stream_links: default_stream_links.to_vec(),
                status: EventStatus::Upcoming,
            };
            race_events.push(ev);
        }
    }

    // Recalculate dynamic status for each event
    for ev in &mut race_events {
        ev.status = ev.current_status();
    }

    race_events
}

fn clean_ics_event_name(summary: &str) -> String {
    // Remove session qualifiers like " - Race", " - Free Practice 1", etc.
    let patterns = [
        " - Free Practice 1",
        " - Free Practice 2",
        " - Free Practice 3",
        " - Free Practice",
        " - FP1",
        " - FP2",
        " - FP3",
        " - Qualifying",
        " - Superpole",
        " - Sprint",
        " - Race 1",
        " - Race 2",
        " - Race",
    ];

    let mut cleaned = summary.to_string();
    for p in patterns {
        if let Some(idx) = cleaned.to_lowercase().find(&p.to_lowercase()) {
            cleaned.truncate(idx);
            break;
        }
    }
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ics_sample() {
        let sample = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//toomuchracing.com//NONSGML v1.0//EN
BEGIN:VEVENT
UID:elms-2026-barcelona-fp1@toomuchracing
DTSTART:20260410T090000Z
DTEND:20260410T103000Z
SUMMARY:ELMS 4 Hours of Barcelona - Free Practice 1
LOCATION:Circuit de Barcelona-Catalunya\, Spain
DESCRIPTION:Free Practice 1 session
END:VEVENT
BEGIN:VEVENT
UID:elms-2026-barcelona-qual@toomuchracing
DTSTART:20260411T130000Z
DTEND:20260411T140000Z
SUMMARY:ELMS 4 Hours of Barcelona - Qualifying
LOCATION:Circuit de Barcelona-Catalunya\, Spain
DESCRIPTION:Qualifying session
END:VEVENT
BEGIN:VEVENT
UID:elms-2026-barcelona-race@toomuchracing
DTSTART:20260412T113000Z
DTEND:20260412T153000Z
SUMMARY:ELMS 4 Hours of Barcelona - Race
LOCATION:Circuit de Barcelona-Catalunya\, Spain
DESCRIPTION:4 Hours Race
END:VEVENT
END:VCALENDAR"#;

        let events = parse_ics_str(sample, "elms", &[]).unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_name, "ELMS 4 Hours of Barcelona");
        assert_eq!(ev.circuit_name, "Circuit de Barcelona-Catalunya, Spain");
        assert_eq!(ev.sessions.len(), 3);
        assert_eq!(ev.sessions[0].session_type, SessionType::Practice);
        assert_eq!(ev.sessions[1].session_type, SessionType::Qualifying);
        assert_eq!(ev.sessions[2].session_type, SessionType::Race);
    }

    #[test]
    fn test_parse_ics_tzid_as_utc() {
        let sample = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
UID:wec-fuji-race
DTSTART;TZID=Asia/Tokyo:20260927T110000
DTEND;TZID=Asia/Tokyo:20260927T170000
SUMMARY:6 Hours of Fuji - Race
END:VEVENT
END:VCALENDAR"#;

        let events = parse_ics_str(sample, "wec", &[]).unwrap();
        let start = events[0].sessions[0].start_time.unwrap();
        assert_eq!(start.to_rfc3339(), "2026-09-27T02:00:00+00:00");
    }
}
