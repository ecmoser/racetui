use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Utc};
use scraper::{Html, Selector};
use std::collections::HashSet;

use super::sportstimes::fetch_sportstimes_calendar;
use super::{fetcher, ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct IndyCarScraper;

impl StandingsFetcher for IndyCarScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = fetcher::create_http_client()?;
        let drivers_url = "https://www.indycar.com/Drivers";
        let mut drivers = Vec::new();

        if let Ok(resp) = client.get(drivers_url).send().await {
            if let Ok(html) = resp.text().await {
                drivers = parse_indycar_standings_html(&html).unwrap_or_default();
            }
        }

        let constructors = generate_indycar_engine_standings(&drivers);

        Ok(SeasonStandings {
            series_id: "indycar".to_string(),
            season,
            drivers,
            constructors,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse IndyCar drivers HTML into DriverStandings.
pub fn parse_indycar_standings_html(
    html: &str,
) -> Result<Vec<crate::data::standings::DriverStanding>> {
    let document = Html::parse_document(html);
    let card_selector = Selector::parse(".driver-card, .driver-listing-item")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let first_name_sel = Selector::parse(".driver-card-identity-first-name, [class*='first-name']")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let last_name_sel = Selector::parse(".driver-card-identity-last-name, [class*='last-name']")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let stats_item_sel = Selector::parse(".driver-card-stats-item, [class*='stats-item']")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let val_sel = Selector::parse(".driver-card-stats-item-value, [class*='value'], p:first-child")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let label_sel =
        Selector::parse(".driver-card-stats-item-label, [class*='label'], p:last-child")
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let mut drivers = Vec::new();

    for card in document.select(&card_selector) {
        let first = card
            .select(&first_name_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let last = card
            .select(&last_name_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let full_name = format!("{} {}", first, last).trim().to_string();
        if full_name.is_empty()
            || drivers
                .iter()
                .any(|d: &crate::data::standings::DriverStanding| d.driver_name == full_name)
        {
            continue;
        }

        let mut points = 0.0;
        let mut wins = 0;

        for stat in card.select(&stats_item_sel) {
            let label = stat
                .select(&label_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_lowercase())
                .unwrap_or_default();
            let val = stat
                .select(&val_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if label.contains("point") {
                points = val.parse::<f64>().unwrap_or(0.0);
            } else if label.contains("win") {
                wins = val.parse::<u32>().unwrap_or(0);
            }
        }

        drivers.push(crate::data::standings::DriverStanding {
            position: 0,
            driver_name: full_name,
            driver_code: None,
            driver_number: None,
            team: "".to_string(),
            points,
            wins,
        });
    }

    // Sort by points descending
    drivers.sort_by(|a, b| {
        b.points
            .partial_cmp(&a.points)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (idx, driver) in drivers.iter_mut().enumerate() {
        driver.position = (idx + 1) as u32;
    }

    Ok(drivers)
}

fn generate_indycar_engine_standings(
    drivers: &[crate::data::standings::DriverStanding],
) -> Vec<crate::data::standings::ConstructorStanding> {
    if drivers.is_empty() {
        return Vec::new();
    }
    vec![
        crate::data::standings::ConstructorStanding {
            position: 1,
            name: "Chevrolet".to_string(),
            points: 0.0,
            wins: 0,
        },
        crate::data::standings::ConstructorStanding {
            position: 2,
            name: "Honda".to_string(),
            points: 0.0,
            wins: 0,
        },
    ]
}

impl ResultsFetcher for IndyCarScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        let client = fetcher::create_http_client()?;
        let url = format!("https://www.indycar.com/Results/{}/{}", season, round);
        let mut results = Vec::new();

        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(html) = resp.text().await {
                if let Ok(parsed) = parse_indycar_results_html(&html, round) {
                    results = parsed.results;
                }
            }
        }

        Ok(RaceResults {
            series_id: "indycar".to_string(),
            round,
            event_name: format!("IndyCar Round {}", round),
            circuit_name: "".to_string(),
            race_date: Utc::now().date_naive(),
            results,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse IndyCar race results table into RaceResults.
pub fn parse_indycar_results_html(html: &str, round: u32) -> Result<RaceResults> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse(".results-table tbody tr, table tbody tr")
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
        series_id: "indycar".to_string(),
        round,
        event_name: format!("IndyCar Round {}", round),
        circuit_name: "".to_string(),
        race_date: Utc::now().date_naive(),
        results,
        fetched_at: Utc::now(),
    })
}

impl SeriesScraper for IndyCarScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = fetcher::create_http_client()?;
        // Try live IndyCar calendar API first for full sessions (practice, qual, race)
        let mut events = if let Ok(fetched) = fetch_sportstimes_calendar(
            &client,
            "https://indycarcalendar.com",
            &series.id,
            &indycar_stream_links(),
        )
        .await
        {
            fetched
        } else {
            Vec::new()
        };

        if events.is_empty() {
            // Fallback to scraping official IndyCar website HTML
            if let Ok(resp) = client.get(&series.calendar_url).send().await {
                if let Ok(html_text) = resp.text().await {
                    if let Ok(parsed) = parse_indycar_html(&html_text, &series.id) {
                        events = parsed;
                    }
                }
            }
        }

        if !events.iter().any(|e| e.start_date.year() == 2027) {
            events.extend(get_official_2027_indycar_schedule(&series.id));
        }

        Ok(events)
    }
}

pub fn indycar_stream_links() -> Vec<StreamLink> {
    vec![
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
    ]
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
    let clean = time_str
        .trim()
        .replace("ET", "")
        .replace("EDT", "")
        .replace("EST", "");
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
    let offset_hours = if (3..=11).contains(&date.month()) {
        4
    } else {
        5
    };
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
            let country =
                if state_or_country.contains("Ontario") || state_or_country.contains("Canada") {
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

pub fn get_official_2027_indycar_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Firestone Grand Prix of St. Petersburg",
            "Streets of St. Petersburg",
            "St. Petersburg",
            "USA",
            (2027, 2, 28),
        ),
        (
            "Java House Grand Prix of Arlington",
            "Streets of Arlington",
            "Arlington",
            "USA",
            (2027, 3, 14),
        ),
        (
            "The Thermal Club $1 Million Challenge",
            "The Thermal Club",
            "Thermal",
            "USA",
            (2027, 3, 21),
        ),
        (
            "Acura Grand Prix of Long Beach",
            "Streets of Long Beach",
            "Long Beach",
            "USA",
            (2027, 4, 18),
        ),
        (
            "Children's of Alabama Indy Grand Prix",
            "Barber Motorsports Park",
            "Birmingham",
            "USA",
            (2027, 5, 2),
        ),
        (
            "Sonsio Grand Prix",
            "Indianapolis Motor Speedway (Road Course)",
            "Indianapolis",
            "USA",
            (2027, 5, 15),
        ),
        (
            "111th Running of the Indianapolis 500",
            "Indianapolis Motor Speedway",
            "Indianapolis",
            "USA",
            (2027, 5, 30),
        ),
        (
            "Chevrolet Detroit Grand Prix",
            "Streets of Detroit",
            "Detroit",
            "USA",
            (2027, 6, 6),
        ),
        (
            "Bommarito Automotive Group 500",
            "World Wide Technology Raceway",
            "Madison",
            "USA",
            (2027, 6, 13),
        ),
        (
            "XPEL Grand Prix at Road America",
            "Road America",
            "Elkhart Lake",
            "USA",
            (2027, 6, 27),
        ),
        (
            "Honda Indy 200 at Mid-Ohio",
            "Mid-Ohio Sports Car Course",
            "Lexington",
            "USA",
            (2027, 7, 11),
        ),
        (
            "Hy-Vee IndyCar Race Weekend (Race 1)",
            "Iowa Speedway",
            "Newton",
            "USA",
            (2027, 7, 17),
        ),
        (
            "Hy-Vee IndyCar Race Weekend (Race 2)",
            "Iowa Speedway",
            "Newton",
            "USA",
            (2027, 7, 18),
        ),
        (
            "Ontario Honda Dealers Indy Toronto",
            "Exhibition Place",
            "Toronto",
            "Canada",
            (2027, 7, 25),
        ),
        (
            "BitNile.com Grand Prix of Portland",
            "Portland International Raceway",
            "Portland",
            "USA",
            (2027, 8, 15),
        ),
        (
            "Milwaukee Mile 250 (Race 1)",
            "Milwaukee Mile",
            "West Allis",
            "USA",
            (2027, 8, 28),
        ),
        (
            "Milwaukee Mile 250 (Race 2)",
            "Milwaukee Mile",
            "West Allis",
            "USA",
            (2027, 8, 29),
        ),
        (
            "Big Machine Music City Grand Prix",
            "Nashville Superspeedway",
            "Lebanon",
            "USA",
            (2027, 9, 12),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, country, date))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let sat_date = race_date.pred_opt().unwrap_or(race_date);
            let quali_time = sat_date
                .and_hms_opt(18, 30, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let race_time = race_date
                .and_hms_opt(19, 0, 0)
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = vec![
                Session {
                    name: "Practice 1".to_string(),
                    session_type: SessionType::Practice,
                    start_time: sat_date
                        .and_hms_opt(14, 0, 0)
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
                    end_time: None,
                },
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: quali_time,
                    end_time: None,
                },
                Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: race_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sat_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: indycar_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
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
        assert_eq!(
            events[0].event_name,
            "110th Running of the Indianapolis 500"
        );
        assert_eq!(events[0].circuit_name, "Indianapolis Motor Speedway");
        assert_eq!(events[0].location, "Indianapolis");
        assert_eq!(events[0].country, "USA");
        assert_eq!(events[0].round, Some(1));
    }

    #[test]
    fn test_parse_indycar_standings_html() {
        let sample = r#"
            <div class="driver-card">
                <div class="driver-card-identity-name-container">
                    <p class="driver-card-identity-first-name">Alex</p>
                    <p class="driver-card-identity-last-name">Palou</p>
                </div>
                <div class="driver-card-stats-container">
                    <div class="driver-card-stats-item">
                        <p class="driver-card-stats-item-value">542</p>
                        <p class="driver-card-stats-item-label">Points</p>
                    </div>
                    <div class="driver-card-stats-item">
                        <p class="driver-card-stats-item-value">6</p>
                        <p class="driver-card-stats-item-label">Wins</p>
                    </div>
                </div>
            </div>
            <div class="driver-card">
                <div class="driver-card-identity-name-container">
                    <p class="driver-card-identity-first-name">Will</p>
                    <p class="driver-card-identity-last-name">Power</p>
                </div>
                <div class="driver-card-stats-container">
                    <div class="driver-card-stats-item">
                        <p class="driver-card-stats-item-value">498</p>
                        <p class="driver-card-stats-item-label">Points</p>
                    </div>
                    <div class="driver-card-stats-item">
                        <p class="driver-card-stats-item-value">3</p>
                        <p class="driver-card-stats-item-label">Wins</p>
                    </div>
                </div>
            </div>
        "#;

        let drivers = parse_indycar_standings_html(sample).unwrap();
        assert_eq!(drivers.len(), 2);
        assert_eq!(drivers[0].driver_name, "Alex Palou");
        assert_eq!(drivers[0].position, 1);
        assert_eq!(drivers[0].points, 542.0);
        assert_eq!(drivers[0].wins, 6);

        assert_eq!(drivers[1].driver_name, "Will Power");
        assert_eq!(drivers[1].position, 2);
        assert_eq!(drivers[1].points, 498.0);
        assert_eq!(drivers[1].wins, 3);
    }

    #[test]
    fn test_parse_indycar_results_html() {
        let sample = r#"
            <table class="results-table">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Alex Palou</td>
                        <td>Chip Ganassi Racing</td>
                        <td>1:45:12.345</td>
                        <td>50</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Will Power</td>
                        <td>Team Penske</td>
                        <td>+3.210</td>
                        <td>40</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let results = parse_indycar_results_html(sample, 1).unwrap();
        assert_eq!(results.series_id, "indycar");
        assert_eq!(results.round, 1);
        assert_eq!(results.results.len(), 2);
        assert_eq!(results.results[0].position, Some(1));
        assert_eq!(results.results[0].driver_name, "Alex Palou");
        assert_eq!(results.results[0].team, "Chip Ganassi Racing");
        assert_eq!(results.results[0].points, 50.0);
        assert_eq!(results.results[1].position, Some(2));
        assert_eq!(results.results[1].driver_name, "Will Power");
        assert_eq!(results.results[1].gap_to_leader, "+3.210");
    }
}
