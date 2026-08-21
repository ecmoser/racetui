use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct IndyNxtScraper;

impl StandingsFetcher for IndyNxtScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "indy_nxt".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for IndyNxtScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "INDY NXT race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for IndyNxtScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/indynxt",
            &series.id,
            &indy_nxt_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_indy_nxt_schedule(&series.id, 2026);
            events.extend(get_official_indy_nxt_schedule(&series.id, 2027));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_indy_nxt_schedule(&series.id, 2027));
        }

        Ok(events)
    }
}

fn indy_nxt_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "Peacock / INDYCAR LIVE".to_string(),
            url: "https://www.peacocktv.com/".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "INDY NXT".to_string(),
            url: "https://www.indynxt.com/".to_string(),
            access: StreamAccess::Free,
        },
    ]
}

pub fn get_official_indy_nxt_schedule(series_id: &str, year: i32) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        ("Streets of St. Petersburg", "St. Petersburg", "USA", (3, 1)),
        ("Barber Motorsports Park", "Birmingham", "USA", (5, 3)),
        (
            "Indianapolis Motor Speedway (Road Course)",
            "Indianapolis",
            "USA",
            (5, 15),
        ),
        ("Streets of Detroit", "Detroit", "USA", (6, 7)),
        ("Road America", "Elkhart Lake", "USA", (6, 28)),
        ("Mid-Ohio Sports Car Course", "Lexington", "USA", (7, 12)),
        ("Iowa Speedway", "Newton", "USA", (7, 18)),
        ("Exhibition Place", "Toronto", "Canada", (7, 26)),
        ("World Wide Technology Raceway", "Madison", "USA", (8, 9)),
        ("Portland International Raceway", "Portland", "USA", (8, 16)),
        ("Milwaukee Mile", "West Allis", "USA", (8, 30)),
        ("Nashville Superspeedway", "Lebanon", "USA", (9, 13)),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (circuit, loc, country, (m, d)))| {
            let race_date = NaiveDate::from_ymd_opt(year, m, d).unwrap();
            let sat_date = race_date.pred_opt().unwrap_or(race_date);

            let status = if race_date < today {
                EventStatus::Completed
            } else if sat_date <= today && today <= race_date {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let fri_date = sat_date.pred_opt().unwrap_or(sat_date);
            let sessions = super::json_ld::build_series_sessions(
                series_id,
                &format!("INDY NXT at {}", loc),
                fri_date,
                race_date,
                None,
                None,
            );

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: format!("INDY NXT at {}", loc),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: fri_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: indy_nxt_stream_links(),
                status,
            }
        })
        .collect()
}
