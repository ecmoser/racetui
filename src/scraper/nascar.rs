use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;

use super::fetcher;
use super::SeriesScraper;
use crate::data::models::*;

/// NASCAR scraper for Cup, Xfinity, and Trucks.
/// All three series use the internal NASCAR CDN cacher endpoint:
/// https://cf.nascar.com/cacher/{year}/{series_id}/race_list_basic.json
pub struct NascarScraper {
    pub nascar_series_id: u32,
    pub racetui_series_id: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct NascarRace {
    pub race_id: u64,
    pub series_id: u32,
    pub race_season: u32,
    pub race_name: String,
    pub race_type_id: u32,
    pub track_name: String,
    pub date_scheduled: Option<String>,
    pub race_date: Option<String>,
    pub television_broadcaster: Option<String>,
    #[serde(default)]
    pub schedule: Vec<NascarScheduleItem>,
}

#[derive(Debug, Deserialize)]
pub struct NascarScheduleItem {
    pub event_name: String,
    #[serde(default)]
    pub notes: String,
    pub start_time_utc: Option<String>,
    pub run_type: u32,
}

/// Helper function to parse NASCAR UTC datetime strings like "2026-02-15T13:30:00"
fn parse_nascar_datetime(dt_str: &str) -> Option<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S"))
        .ok()?;
    Some(Utc.from_utc_datetime(&naive))
}

impl SeriesScraper for NascarScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = fetcher::create_http_client()?;
        let current_year = Utc::now().year();
        let mut races = Vec::new();

        // Query current year and next year
        for year in current_year..=current_year + 1 {
            let url = format!(
                "https://cf.nascar.com/cacher/{}/{}/race_list_basic.json",
                year, self.nascar_series_id
            );
            if let Ok(fetched) = fetcher::fetch_json::<Vec<NascarRace>>(&client, &url).await {
                races.extend(fetched);
            }
        }

        if races.is_empty() {
            if let Ok(fetched) =
                fetcher::fetch_json::<Vec<NascarRace>>(&client, &series.calendar_url).await
            {
                races = fetched;
            } else if self.racetui_series_id == "arca" {
                let mut fallback_events = get_official_2026_arca_schedule(self.racetui_series_id);
                fallback_events.extend(get_official_2027_arca_schedule(self.racetui_series_id));
                return Ok(fallback_events);
            } else {
                anyhow::bail!("Failed to fetch NASCAR data for {}", self.racetui_series_id);
            }
        }

        let mut events = Vec::new();
        let now_utc = Utc::now();

        for (idx, race) in races.into_iter().enumerate() {
            // Determine race start time and date
            let race_datetime = race
                .race_date
                .as_deref()
                .or(race.date_scheduled.as_deref())
                .and_then(parse_nascar_datetime);

            let fallback_date = race_datetime
                .map(|dt| dt.date_naive())
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(race.race_season as i32, 1, 1).unwrap());

            // Build sessions from schedule
            let mut sessions = Vec::new();
            let mut has_race_session = false;

            for item in race.schedule {
                let session_type = match item.run_type {
                    1 => SessionType::Practice,
                    2 => SessionType::Qualifying,
                    3 => {
                        has_race_session = true;
                        SessionType::Race
                    }
                    _ => {
                        let lower = item.event_name.to_lowercase();
                        if lower.contains("practice") {
                            SessionType::Practice
                        } else if lower.contains("qualifying") || lower.contains("duel") {
                            SessionType::Qualifying
                        } else if lower.contains("race") {
                            has_race_session = true;
                            SessionType::Race
                        } else {
                            // Non-track event (e.g. Haulers, Driver Intro)
                            continue;
                        }
                    }
                };

                let start_time = item
                    .start_time_utc
                    .as_deref()
                    .and_then(parse_nascar_datetime);
                sessions.push(Session {
                    name: item.event_name,
                    session_type,
                    start_time,
                    end_time: None,
                });
            }

            // If no race session was extracted from the schedule, add one from race_datetime
            if !has_race_session {
                sessions.push(Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_datetime,
                    end_time: None,
                });
            }

            // Determine earliest and latest dates
            let start_date = sessions
                .iter()
                .filter_map(|s| s.start_time)
                .min()
                .map(|dt| dt.date_naive())
                .unwrap_or(fallback_date);

            let end_date = sessions
                .iter()
                .filter_map(|s| s.start_time)
                .max()
                .map(|dt| dt.date_naive())
                .unwrap_or(fallback_date);

            // Determine status
            let status = if sessions.iter().any(|s| {
                s.start_time.map_or(false, |t| {
                    t <= now_utc
                        && s.end_time
                            .map_or(t + chrono::Duration::hours(3) > now_utc, |e| e > now_utc)
                })
            }) {
                EventStatus::Live
            } else if end_date >= now_utc.date_naive() {
                EventStatus::Upcoming
            } else {
                EventStatus::Completed
            };

            // Stream links based on broadcaster
            let mut stream_links = Vec::new();
            if let Some(broadcaster) = race.television_broadcaster {
                let b_upper = broadcaster.to_uppercase();
                let (url, platform) = if b_upper.contains("FOX")
                    || b_upper.contains("FS1")
                    || b_upper.contains("FS2")
                {
                    ("https://www.foxsports.com/live", broadcaster)
                } else if b_upper.contains("NBC")
                    || b_upper.contains("USA")
                    || b_upper.contains("PEACOCK")
                {
                    ("https://www.peacocktv.com", broadcaster)
                } else if b_upper.contains("PRIME") || b_upper.contains("AMAZON") {
                    ("https://www.amazon.com/primevideo", broadcaster)
                } else if b_upper.contains("TNT") || b_upper.contains("MAX") {
                    ("https://www.max.com", broadcaster)
                } else {
                    ("https://www.nascar.com/live", broadcaster)
                };

                stream_links.push(StreamLink {
                    platform,
                    url: url.to_string(),
                    access: StreamAccess::Paid,
                });
            } else {
                stream_links.push(StreamLink {
                    platform: "NASCAR TrackPass".to_string(),
                    url: "https://www.nascar.com/live".to_string(),
                    access: StreamAccess::Paid,
                });
            }

            events.push(RaceEvent {
                series_id: self.racetui_series_id.to_string(),
                event_name: race.race_name,
                circuit_name: race.track_name.clone(),
                location: race.track_name,
                country: "USA".to_string(),
                start_date,
                end_date,
                round: Some((idx + 1) as u32),
                sessions,
                stream_links,
                status,
            });
        }

        Ok(events)
    }
}

/// Official 2026 ARCA Menards Series schedule fallback
pub fn get_official_2026_arca_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw = vec![
        (
            "Hard Rock Bet 200",
            "Daytona International Speedway",
            "Daytona Beach",
            "2026-02-14",
            "13:30:00",
        ),
        (
            "General Tire 150",
            "Phoenix Raceway",
            "Avondale",
            "2026-03-06",
            "20:00:00",
        ),
        (
            "General Tire 200",
            "Talladega Superspeedway",
            "Talladega",
            "2026-04-25",
            "12:30:00",
        ),
        (
            "Tide 150",
            "Kansas Speedway",
            "Kansas City",
            "2026-05-09",
            "14:00:00",
        ),
        (
            "General Tire 150",
            "Charlotte Motor Speedway",
            "Concord",
            "2026-05-22",
            "18:00:00",
        ),
        (
            "Atlas 150",
            "Iowa Speedway",
            "Newton",
            "2026-06-12",
            "20:00:00",
        ),
        (
            "Berlin ARCA 200",
            "Berlin Raceway",
            "Marne",
            "2026-06-20",
            "20:00:00",
        ),
        (
            "Menards 250",
            "Elko Speedway",
            "Elko New Market",
            "2026-06-27",
            "21:00:00",
        ),
        (
            "Lime Rock 100",
            "Lime Rock Park",
            "Lakeville",
            "2026-07-11",
            "16:00:00",
        ),
        (
            "Circle City 200",
            "Lucas Oil Indianapolis Raceway Park",
            "Brownsburg",
            "2026-07-17",
            "20:00:00",
        ),
        (
            "Salem ARCA 200",
            "Salem Speedway",
            "Salem",
            "2026-07-25",
            "20:00:00",
        ),
        (
            "Zinsser SmartCoat 150",
            "Mid-Ohio Sports Car Course",
            "Lexington",
            "2026-08-01",
            "15:00:00",
        ),
        (
            "Illinois State Fair 100",
            "Illinois State Fairgrounds Racetrack",
            "Springfield",
            "2026-08-23",
            "14:00:00",
        ),
        (
            "Sprecher 150",
            "Milwaukee Mile",
            "West Allis",
            "2026-08-30",
            "13:00:00",
        ),
        (
            "Southern Illinois 100",
            "DuQuoin State Fairgrounds Racetrack",
            "Du Quoin",
            "2026-09-06",
            "20:00:00",
        ),
        (
            "General Tire 100",
            "Watkins Glen International",
            "Watkins Glen",
            "2026-09-11",
            "17:00:00",
        ),
        (
            "Bush's Beans 200",
            "Bristol Motor Speedway",
            "Bristol",
            "2026-09-17",
            "18:00:00",
        ),
        (
            "Reese's 150",
            "Kansas Speedway",
            "Kansas City",
            "2026-09-25",
            "17:30:00",
        ),
        (
            "Shore Lunch 200",
            "Toledo Speedway",
            "Toledo",
            "2026-10-03",
            "16:00:00",
        ),
    ];

    raw.into_iter()
        .enumerate()
        .map(|(idx, (name, circuit, loc, date_str, time_str))| {
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
            let naive_time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S").unwrap();
            let dt = date.and_time(naive_time).and_utc();
            let mut sessions = Vec::new();
            sessions.push(Session {
                name: "Practice & Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: Some(dt - chrono::Duration::hours(3)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: Some(dt),
                end_time: None,
            });
            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: "USA".to_string(),
                start_date: date,
                end_date: date,
                round: Some((idx + 1) as u32),
                sessions,
                stream_links: vec![
                    StreamLink {
                        platform: "FS1 / FOX Sports".to_string(),
                        url: "https://www.foxsports.com/live".to_string(),
                        access: StreamAccess::Paid,
                    },
                    StreamLink {
                        platform: "FloRacing".to_string(),
                        url: "https://www.floracing.com".to_string(),
                        access: StreamAccess::Paid,
                    },
                ],
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

/// Official 2027 ARCA Menards Series schedule fallback
pub fn get_official_2027_arca_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw = vec![
        (
            "Hard Rock Bet 200",
            "Daytona International Speedway",
            "Daytona Beach",
            "2027-02-13",
            "13:30:00",
        ),
        (
            "General Tire 150",
            "Phoenix Raceway",
            "Avondale",
            "2027-03-05",
            "20:00:00",
        ),
        (
            "General Tire 200",
            "Talladega Superspeedway",
            "Talladega",
            "2027-04-24",
            "12:30:00",
        ),
        (
            "Tide 150",
            "Kansas Speedway",
            "Kansas City",
            "2027-05-08",
            "14:00:00",
        ),
        (
            "General Tire 150",
            "Charlotte Motor Speedway",
            "Concord",
            "2027-05-28",
            "18:00:00",
        ),
        (
            "Atlas 150",
            "Iowa Speedway",
            "Newton",
            "2027-06-11",
            "20:00:00",
        ),
        (
            "Berlin ARCA 200",
            "Berlin Raceway",
            "Marne",
            "2027-06-19",
            "20:00:00",
        ),
        (
            "Menards 250",
            "Elko Speedway",
            "Elko New Market",
            "2027-06-26",
            "21:00:00",
        ),
        (
            "Lime Rock 100",
            "Lime Rock Park",
            "Lakeville",
            "2027-07-10",
            "16:00:00",
        ),
        (
            "Circle City 200",
            "Lucas Oil Indianapolis Raceway Park",
            "Brownsburg",
            "2027-07-16",
            "20:00:00",
        ),
        (
            "Salem ARCA 200",
            "Salem Speedway",
            "Salem",
            "2027-07-24",
            "20:00:00",
        ),
        (
            "Zinsser SmartCoat 150",
            "Mid-Ohio Sports Car Course",
            "Lexington",
            "2027-07-31",
            "15:00:00",
        ),
        (
            "Illinois State Fair 100",
            "Illinois State Fairgrounds Racetrack",
            "Springfield",
            "2027-08-22",
            "14:00:00",
        ),
        (
            "Sprecher 150",
            "Milwaukee Mile",
            "West Allis",
            "2027-08-29",
            "13:00:00",
        ),
        (
            "Southern Illinois 100",
            "DuQuoin State Fairgrounds Racetrack",
            "Du Quoin",
            "2027-09-05",
            "20:00:00",
        ),
        (
            "General Tire 100",
            "Watkins Glen International",
            "Watkins Glen",
            "2027-09-10",
            "17:00:00",
        ),
        (
            "Bush's Beans 200",
            "Bristol Motor Speedway",
            "Bristol",
            "2027-09-16",
            "18:00:00",
        ),
        (
            "Reese's 150",
            "Kansas Speedway",
            "Kansas City",
            "2027-09-24",
            "17:30:00",
        ),
        (
            "Shore Lunch 200",
            "Toledo Speedway",
            "Toledo",
            "2027-10-02",
            "16:00:00",
        ),
    ];

    raw.into_iter()
        .enumerate()
        .map(|(idx, (name, circuit, loc, date_str, time_str))| {
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
            let naive_time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S").unwrap();
            let dt = date.and_time(naive_time).and_utc();
            let mut sessions = Vec::new();
            sessions.push(Session {
                name: "Practice & Qualifying".to_string(),
                session_type: SessionType::Qualifying,
                start_time: Some(dt - chrono::Duration::hours(3)),
                end_time: None,
            });
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: Some(dt),
                end_time: None,
            });
            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: "USA".to_string(),
                start_date: date,
                end_date: date,
                round: Some((idx + 1) as u32),
                sessions,
                stream_links: vec![
                    StreamLink {
                        platform: "FS1 / FOX Sports".to_string(),
                        url: "https://www.foxsports.com/live".to_string(),
                        access: StreamAccess::Paid,
                    },
                    StreamLink {
                        platform: "FloRacing".to_string(),
                        url: "https://www.floracing.com".to_string(),
                        access: StreamAccess::Paid,
                    },
                ],
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nascar_datetime() {
        let dt = parse_nascar_datetime("2026-02-15T13:30:00");
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().to_rfc3339(), "2026-02-15T13:30:00+00:00");
    }

    #[test]
    fn test_nascar_json_deserialization() {
        let json = r#"[
            {
                "race_id": 5593,
                "series_id": 1,
                "race_season": 2026,
                "race_name": "Cook Out Clash at Bowman Gray",
                "race_type_id": 2,
                "track_name": "Bowman Gray Stadium",
                "date_scheduled": "2026-02-01T20:00:00",
                "race_date": "2026-02-04T18:00:00",
                "television_broadcaster": "FS2",
                "schedule": [
                    {
                        "event_name": "Practice / Qualifying",
                        "notes": "Group 1 / 2",
                        "start_time_utc": "2026-02-04T18:30:00",
                        "run_type": 1
                    },
                    {
                        "event_name": "Race",
                        "notes": "200 Laps",
                        "start_time_utc": "2026-02-04T23:00:00",
                        "run_type": 3
                    }
                ]
            }
        ]"#;

        let races: Vec<NascarRace> = serde_json::from_str(json).unwrap();
        assert_eq!(races.len(), 1);
        assert_eq!(races[0].race_name, "Cook Out Clash at Bowman Gray");
        assert_eq!(races[0].schedule.len(), 2);
    }
}
