use crossterm::event::KeyEvent;

use crate::data::models::RaceEvent;

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
    FetchError {
        series_id: String,
        error: String,
    },
    /// A series started fetching (update status indicator)
    FetchStarted {
        series_id: String,
    },
    /// Periodic tick for updating time-sensitive displays (e.g., countdowns)
    Tick,
    /// User requested a data refresh
    RefreshRequested,
}

