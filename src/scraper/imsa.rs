use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use scraper::{Html, Selector};

use super::fetcher;
use super::json_ld::fetch_json_ld_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct ImsaScraper;

impl StandingsFetcher for ImsaScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "imsa".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for ImsaScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "IMSA race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for ImsaScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let mut events = Vec::new();

        if let Ok(client) = fetcher::create_http_client() {
            // Try live structured web schedule first
            if let Ok(fetched) = fetch_json_ld_calendar(
                &client,
                "https://raceweek.io/imsa",
                &series.id,
                &imsa_stream_links(),
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
                        if !html_text.contains("Just a moment...")
                            && !html_text.contains("challenges.cloudflare.com")
                        {
                            if let Ok(parsed) = parse_imsa_html(&html_text, &series.id) {
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
            events = get_official_2026_imsa_schedule(&series.id);
        }

        if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_imsa_schedule(&series.id));
        }

        Ok(events)
    }
}

pub fn parse_imsa_html(html_text: &str, series_id: &str) -> Result<Vec<RaceEvent>> {
    let document = Html::parse_document(html_text);
    let row_selector = Selector::parse(".event-item, .schedule-row, .event-card")
        .map_err(|e| anyhow::anyhow!("Invalid row selector: {:?}", e))?;
    let title_selector = Selector::parse(".event-title, .event-name, h3")
        .map_err(|e| anyhow::anyhow!("Invalid title selector: {:?}", e))?;

    let mut events = Vec::new();
    let today = Utc::now().date_naive();

    for row in document.select(&row_selector) {
        if let Some(title_elem) = row.select(&title_selector).next() {
            let title = title_elem.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                events.push(RaceEvent {
                    series_id: series_id.to_string(),
                    event_name: title,
                    circuit_name: "IMSA Circuit".to_string(),
                    location: "USA".to_string(),
                    country: "USA".to_string(),
                    start_date: today,
                    end_date: today,
                    round: None,
                    sessions: vec![],
                    stream_links: imsa_stream_links(),
                    status: EventStatus::Upcoming,
                });
            }
        }
    }

    Ok(events)
}

fn imsa_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Peacock".to_string(),
            url: "https://www.peacocktv.com".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "IMSA.tv".to_string(),
            url: "https://www.imsa.com/tv/".to_string(),
            access: StreamAccess::Mixed,
        },
    ]
}

pub fn get_official_2026_imsa_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Rolex 24 at Daytona",
            "Daytona International Speedway",
            "Daytona Beach, Florida",
            "USA",
            (2026, 1, 22),
            (2026, 1, 25),
            (18, 40),
        ),
        (
            "Mobil 1 Twelve Hours of Sebring",
            "Sebring International Raceway",
            "Sebring, Florida",
            "USA",
            (2026, 3, 18),
            (2026, 3, 21),
            (14, 0),
        ),
        (
            "Acura Grand Prix of Long Beach",
            "Streets of Long Beach",
            "Long Beach, California",
            "USA",
            (2026, 4, 17),
            (2026, 4, 18),
            (20, 35),
        ),
        (
            "Motul Course de Monterey",
            "WeatherTech Raceway Laguna Seca",
            "Monterey, California",
            "USA",
            (2026, 5, 8),
            (2026, 5, 10),
            (19, 10),
        ),
        (
            "Chevrolet Detroit Grand Prix",
            "Streets of Detroit",
            "Detroit, Michigan",
            "USA",
            (2026, 5, 29),
            (2026, 5, 30),
            (19, 10),
        ),
        (
            "Sahlen's Six Hours of The Glen",
            "Watkins Glen International",
            "Watkins Glen, New York",
            "USA",
            (2026, 6, 25),
            (2026, 6, 28),
            (15, 10),
        ),
        (
            "Chevrolet Grand Prix",
            "Canadian Tire Motorsport Park",
            "Bowmanville, Ontario",
            "Canada",
            (2026, 7, 10),
            (2026, 7, 12),
            (15, 5),
        ),
        (
            "IMSA SportsCar Weekend",
            "Road America",
            "Elkhart Lake, Wisconsin",
            "USA",
            (2026, 7, 31),
            (2026, 8, 2),
            (19, 10),
        ),
        (
            "Michelin GT Challenge at VIR",
            "Virginia International Raceway",
            "Alton, Virginia",
            "USA",
            (2026, 8, 21),
            (2026, 8, 23),
            (16, 10),
        ),
        (
            "TireRack.com Battle on the Bricks",
            "Indianapolis Motor Speedway",
            "Indianapolis, Indiana",
            "USA",
            (2026, 9, 18),
            (2026, 9, 20),
            (15, 40),
        ),
        (
            "Motul Petit Le Mans",
            "Michelin Raceway Road Atlanta",
            "Braselton, Georgia",
            "USA",
            (2026, 9, 30),
            (2026, 10, 3),
            (16, 10),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(
            |(i, (name, circuit, loc, country, _start, end, (hour, min)))| {
                let race_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();
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

                let sessions = vec![Session {
                    name: "Race".to_string(),
                    session_type: SessionType::Race,
                    start_time,
                    end_time: None,
                }];

                RaceEvent {
                    series_id: series_id.to_string(),
                    event_name: name.to_string(),
                    circuit_name: circuit.to_string(),
                    location: loc.to_string(),
                    country: country.to_string(),
                    start_date: race_date,
                    end_date: race_date,
                    round: Some((i + 1) as u32),
                    sessions,
                    stream_links: imsa_stream_links(),
                    status,
                }
            },
        )
        .collect()
}

pub fn get_official_2027_imsa_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Rolex 24 at Daytona",
            "Daytona International Speedway",
            "Daytona Beach, FL",
            "USA",
            (2027, 1, 30),
            (2027, 1, 31),
            (18, 40),
        ),
        (
            "Mobil 1 Twelve Hours of Sebring",
            "Sebring International Raceway",
            "Sebring, FL",
            "USA",
            (2027, 3, 20),
            (2027, 3, 20),
            (13, 40),
        ),
        (
            "Acura Grand Prix of Long Beach",
            "Long Beach Street Circuit",
            "Long Beach, CA",
            "USA",
            (2027, 4, 17),
            (2027, 4, 17),
            (20, 35),
        ),
        (
            "Motul Course de Monterey",
            "WeatherTech Raceway Laguna Seca",
            "Monterey, CA",
            "USA",
            (2027, 5, 9),
            (2027, 5, 9),
            (19, 10),
        ),
        (
            "Detroit Grand Prix",
            "Detroit Street Circuit",
            "Detroit, MI",
            "USA",
            (2027, 6, 5),
            (2027, 6, 5),
            (19, 10),
        ),
        (
            "Sahlen's Six Hours of The Glen",
            "Watkins Glen International",
            "Watkins Glen, NY",
            "USA",
            (2027, 6, 27),
            (2027, 6, 27),
            (15, 10),
        ),
        (
            "Chevrolet Grand Prix",
            "Canadian Tire Motorsport Park",
            "Bowmanville, ON",
            "Canada",
            (2027, 7, 11),
            (2027, 7, 11),
            (15, 5),
        ),
        (
            "IMSA SportsCar Weekend",
            "Road America",
            "Elkhart Lake, WI",
            "USA",
            (2027, 8, 1),
            (2027, 8, 1),
            (18, 10),
        ),
        (
            "Michelin GT Challenge at VIR",
            "VIRginia International Raceway",
            "Alton, VA",
            "USA",
            (2027, 8, 22),
            (2027, 8, 22),
            (18, 10),
        ),
        (
            "TireRack.com Battle on the Bricks",
            "Indianapolis Motor Speedway",
            "Indianapolis, IN",
            "USA",
            (2027, 9, 19),
            (2027, 9, 19),
            (15, 40),
        ),
        (
            "Motul Petit Le Mans",
            "Michelin Raceway Road Atlanta",
            "Braselton, GA",
            "USA",
            (2027, 10, 9),
            (2027, 10, 9),
            (16, 10),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(
            |(i, (name, circuit, loc, country, _start, end, (hour, min)))| {
                let race_date = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();
                let sat_date = race_date.pred_opt().unwrap_or(race_date);
                let quali_time = sat_date
                    .and_hms_opt(13, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
                let race_time = race_date
                    .and_hms_opt(hour, min, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

                let sessions = vec![
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
                    stream_links: imsa_stream_links(),
                    status: EventStatus::Upcoming,
                }
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_imsa_schedule() {
        let events = get_official_2026_imsa_schedule("imsa");
        assert_eq!(events.len(), 11);
        assert_eq!(events[0].event_name, "Rolex 24 at Daytona");
        assert_eq!(events[0].circuit_name, "Daytona International Speedway");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[10].event_name, "Motul Petit Le Mans");
        assert_eq!(events[10].round, Some(11));
    }
}
