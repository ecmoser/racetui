use crossterm::event::KeyEvent;

use crate::data::models::RaceEvent;
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

/// All events that flow through the application's event channel.
/// Both user input events and background task results are unified here.
pub enum AppEvent {
    /// A key was pressed
    Key(KeyEvent),
    /// Terminal was resized
    Resize(u16, u16),
    /// A series finished fetching its data
    SeriesDataFetched {
        series_id: String,
        events: Vec<RaceEvent>,
    },
    /// A series fetch failed
    FetchError { series_id: String, error: String },
    /// A series started fetching (update status indicator)
    FetchStarted { series_id: String },
    /// Periodic tick for updating time-sensitive displays (e.g., countdowns)
    Tick,
    /// User requested a data refresh
    RefreshRequested,
    /// Championship standings data was fetched for a series
    StandingsFetched {
        series_id: String,
        standings: SeasonStandings,
    },
    /// Championship standings fetch failed
    StandingsFetchError { series_id: String, error: String },
    /// Race results were fetched for a specific event
    ResultsFetched {
        series_id: String,
        round: u32,
        results: RaceResults,
    },
    /// Race results fetch failed
    ResultsFetchError {
        series_id: String,
        round: u32,
        error: String,
    },
    /// Qualifying results were fetched for a specific event
    QualifyingFetched {
        series_id: String,
        round: u32,
        results: crate::data::results::QualifyingResults,
    },
    /// Qualifying results fetch failed
    QualifyingFetchError {
        series_id: String,
        round: u32,
        error: String,
    },
    /// Sprint results were fetched for a specific event
    SprintFetched {
        series_id: String,
        round: u32,
        results: crate::data::results::RaceResults,
    },
    /// Sprint results fetch failed
    SprintFetchError {
        series_id: String,
        round: u32,
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_app_event_variants() {
        let standings = SeasonStandings {
            series_id: "f1".to_string(),
            season: 2026,
            drivers: vec![],
            constructors: vec![],
            fetched_at: Utc::now(),
        };
        let event = AppEvent::StandingsFetched {
            series_id: "f1".to_string(),
            standings,
        };
        match event {
            AppEvent::StandingsFetched { series_id, .. } => assert_eq!(series_id, "f1"),
            _ => panic!("Expected StandingsFetched variant"),
        }

        let err_event = AppEvent::StandingsFetchError {
            series_id: "f1".to_string(),
            error: "network error".to_string(),
        };
        match err_event {
            AppEvent::StandingsFetchError { series_id, error } => {
                assert_eq!(series_id, "f1");
                assert_eq!(error, "network error");
            }
            _ => panic!("Expected StandingsFetchError variant"),
        }

        let results = RaceResults {
            series_id: "f1".to_string(),
            round: 1,
            event_name: "Bahrain GP".to_string(),
            circuit_name: "Bahrain".to_string(),
            race_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            results: vec![],
            fetched_at: Utc::now(),
        };
        let results_event = AppEvent::ResultsFetched {
            series_id: "f1".to_string(),
            round: 1,
            results,
        };
        match results_event {
            AppEvent::ResultsFetched {
                series_id, round, ..
            } => {
                assert_eq!(series_id, "f1");
                assert_eq!(round, 1);
            }
            _ => panic!("Expected ResultsFetched variant"),
        }

        let res_err = AppEvent::ResultsFetchError {
            series_id: "f1".to_string(),
            round: 1,
            error: "parse error".to_string(),
        };
        match res_err {
            AppEvent::ResultsFetchError {
                series_id,
                round,
                error,
            } => {
                assert_eq!(series_id, "f1");
                assert_eq!(round, 1);
                assert_eq!(error, "parse error");
            }
            _ => panic!("Expected ResultsFetchError variant"),
        }
    }
}
