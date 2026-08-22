use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{EventStatus, RaceEvent, Series, StreamLink};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct WecScraper;

impl StandingsFetcher for WecScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        Ok(SeasonStandings {
            series_id: "wec".to_string(),
            season,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        })
    }
}

impl ResultsFetcher for WecScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "WEC race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for WecScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/wec",
            &series.id,
            &wec_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_wec_schedule(&series.id);
            events.extend(get_official_2027_wec_schedule(&series.id));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_wec_schedule(&series.id));
        }

        Ok(events)
    }
}

fn wec_stream_links() -> Vec<StreamLink> {
    super::raceday_watch::get_raceday_stream_links("wec", "")
}

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
            (11, 0),
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

pub fn get_official_2027_wec_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Qatar 1812 Km",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2027, 2, 27),
            (8, 0),
        ),
        (
            "6 Hours of Imola",
            "Autodromo Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2027, 4, 18),
            (11, 0),
        ),
        (
            "6 Hours of Spa-Francorchamps",
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2027, 5, 8),
            (11, 0),
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
            (2027, 9, 5),
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
}
