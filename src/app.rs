use std::collections::HashMap;

use chrono::Datelike;
use ratatui::widgets::TableState;

use crate::config::UserConfig;
use crate::data::models::{FetchStatus, RaceEvent, Series};

/// Which view the user is currently looking at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Calendar,
}

/// Which filter category is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterCategory {
    All,
    Favorites,
    CarStyle(String),
    Region(String),
    Series(String),
}

/// All application state lives here.
pub struct App {
    /// Whether the app should keep running
    pub running: bool,

    /// Current view mode
    pub view_mode: ViewMode,

    /// All series definitions, keyed by ID
    pub series_registry: HashMap<String, Series>,

    /// All fetched race events, keyed by series ID
    pub events: HashMap<String, Vec<RaceEvent>>,

    /// Fetch status per series
    pub fetch_status: HashMap<String, FetchStatus>,

    /// User config (favorites, preferences, etc.)
    pub config: UserConfig,

    /// Current active filter
    pub active_filter: FilterCategory,

    /// Search query (when user presses '/')
    pub search_query: Option<String>,

    /// Whether the search input is currently active
    pub search_active: bool,

    /// Whether the help popup is visible
    pub show_help: bool,

    /// Whether the detail view is visible
    pub show_detail: bool,

    /// Whether the filter panel is visible
    pub show_filter_panel: bool,

    // -- List view state --
    /// Ratatui table state for list view (tracks selected row)
    pub table_state: TableState,

    // -- Calendar view state --
    /// Currently displayed year/month in calendar view
    pub calendar_year: i32,
    pub calendar_month: u32,

    /// Status bar message (temporary messages like "Refreshing F1...")
    pub status_message: Option<String>,

    /// Whether user requested manual refresh
    pub refresh_requested: bool,
}

impl App {
    /// Create a new App with the given series registry and config.
    pub fn new(series_registry: HashMap<String, Series>, config: UserConfig) -> Self {
        let now = chrono::Local::now();
        let view_mode = match config.default_view.as_str() {
            "calendar" => ViewMode::Calendar,
            _ => ViewMode::List,
        };

        // Initialize fetch status for all series as Pending
        let fetch_status: HashMap<String, FetchStatus> = series_registry
            .keys()
            .map(|id| (id.clone(), FetchStatus::Pending))
            .collect();

        let mut app = Self {
            running: true,
            view_mode,
            series_registry,
            events: HashMap::new(),
            fetch_status,
            config,
            active_filter: FilterCategory::All,
            search_query: None,
            search_active: false,
            show_help: false,
            show_detail: false,
            show_filter_panel: false,
            table_state: TableState::default(),
            calendar_year: now.year(),
            calendar_month: now.month(),
            status_message: None,
            refresh_requested: false,
        };

        // Select the first row by default
        app.table_state.select(Some(0));
        app
    }

    /// Get a flat, sorted list of all race events that match the current filters.
    /// Sorted by start_date ascending (soonest first).
    pub fn filtered_events(&self) -> Vec<&RaceEvent> {
        let mut events: Vec<&RaceEvent> = self
            .events
            .values()
            .flatten()
            .filter(|event| {
                // Don't show events from hidden series
                if self.config.hidden_series.contains(&event.series_id) {
                    return false;
                }

                // Apply active filter
                match &self.active_filter {
                    FilterCategory::All => true,
                    FilterCategory::Favorites => {
                        self.config.favorites.contains(&event.series_id)
                    }
                    FilterCategory::CarStyle(style) => {
                        self.series_registry
                            .get(&event.series_id)
                            .map_or(false, |s| s.car_style.to_string() == *style)
                    }
                    FilterCategory::Region(region) => {
                        self.series_registry
                            .get(&event.series_id)
                            .map_or(false, |s| s.region == *region)
                    }
                    FilterCategory::Series(id) => event.series_id == *id,
                }
            })
            .filter(|event| {
                // Apply search filter
                if let Some(query) = &self.search_query {
                    let q = query.to_lowercase();
                    event.event_name.to_lowercase().contains(&q)
                        || event.circuit_name.to_lowercase().contains(&q)
                        || event.country.to_lowercase().contains(&q)
                        || event.location.to_lowercase().contains(&q)
                        || event.series_id.to_lowercase().contains(&q)
                        || self
                            .series_registry
                            .get(&event.series_id)
                            .map_or(false, |s| {
                                s.name.to_lowercase().contains(&q)
                                    || s.short_name.to_lowercase().contains(&q)
                            })
                } else {
                    true
                }
            })
            .collect();

        events.sort_by_key(|e| e.start_date);
        events
    }

    /// Get the currently selected event (in list view).
    pub fn selected_event(&self) -> Option<&RaceEvent> {
        let events = self.filtered_events();
        self.table_state
            .selected()
            .and_then(|i| events.get(i).copied())
    }

    /// Move selection up in the list.
    pub fn select_previous(&mut self) {
        let count = self.filtered_events().len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    /// Move selection down in the list.
    pub fn select_next(&mut self) {
        let count = self.filtered_events().len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    /// Update events for a series after a successful fetch.
    pub fn update_series_data(&mut self, series_id: String, new_events: Vec<RaceEvent>) {
        let count = new_events.len();
        self.events.insert(series_id.clone(), new_events);
        self.fetch_status
            .insert(series_id, FetchStatus::Loaded(count));
    }

    /// Mark a series fetch as started.
    pub fn mark_fetching(&mut self, series_id: &str) {
        self.fetch_status
            .insert(series_id.to_string(), FetchStatus::Fetching);
    }

    /// Mark a series fetch as failed.
    pub fn mark_fetch_error(&mut self, series_id: &str, error: String) {
        self.fetch_status
            .insert(series_id.to_string(), FetchStatus::Error(error));
    }

    /// Mark a series as loaded from cache.
    pub fn mark_cached_load(&mut self, series_id: &str, count: usize, age_hours: u64) {
        self.fetch_status
            .insert(series_id.to_string(), FetchStatus::CachedLoad(count, age_hours));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::models::{CarStyle, EventStatus};
    use chrono::NaiveDate;

    fn make_test_series() -> HashMap<String, Series> {
        let mut map = HashMap::new();
        map.insert(
            "f1".to_string(),
            Series {
                id: "f1".to_string(),
                name: "Formula 1".to_string(),
                short_name: "F1".to_string(),
                car_style: CarStyle::OpenWheel,
                color: (255, 0, 0),
                region: "International".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );
        map.insert(
            "nascar".to_string(),
            Series {
                id: "nascar".to_string(),
                name: "NASCAR Cup".to_string(),
                short_name: "Cup".to_string(),
                car_style: CarStyle::StockCar,
                color: (0, 0, 255),
                region: "USA".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );
        map
    }

    fn make_test_event(series_id: &str, name: &str, date: NaiveDate) -> RaceEvent {
        RaceEvent {
            series_id: series_id.to_string(),
            event_name: name.to_string(),
            circuit_name: "Track".to_string(),
            location: "City".to_string(),
            country: "Country".to_string(),
            start_date: date,
            end_date: date,
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        }
    }

    #[test]
    fn test_app_initial_state() {
        let registry = make_test_series();
        let config = UserConfig::default();
        let app = App::new(registry, config);

        assert!(app.running);
        assert_eq!(app.view_mode, ViewMode::List);
        assert_eq!(app.active_filter, FilterCategory::All);
        assert_eq!(app.fetch_status.get("f1"), Some(&FetchStatus::Pending));
        assert_eq!(app.fetch_status.get("nascar"), Some(&FetchStatus::Pending));
    }

    #[test]
    fn test_filtered_events_sorting_and_filtering() {
        let registry = make_test_series();
        let mut config = UserConfig::default();
        config.favorites.insert("f1".to_string());
        let mut app = App::new(registry, config);

        let e1 = make_test_event("nascar", "Daytona 500", NaiveDate::from_ymd_opt(2026, 2, 15).unwrap());
        let e2 = make_test_event("f1", "Bahrain GP", NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
        let e3 = make_test_event("f1", "Monaco GP", NaiveDate::from_ymd_opt(2026, 5, 24).unwrap());

        app.update_series_data("nascar".to_string(), vec![e1]);
        app.update_series_data("f1".to_string(), vec![e3, e2]);

        // Filter: All -> Sorted by date ascending
        let all = app.filtered_events();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].event_name, "Daytona 500");
        assert_eq!(all[1].event_name, "Bahrain GP");
        assert_eq!(all[2].event_name, "Monaco GP");

        // Filter: Favorites -> only F1
        app.active_filter = FilterCategory::Favorites;
        let favs = app.filtered_events();
        assert_eq!(favs.len(), 2);
        assert_eq!(favs[0].event_name, "Bahrain GP");

        // Search query
        app.active_filter = FilterCategory::All;
        app.search_query = Some("Daytona".to_string());
        let search_res = app.filtered_events();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].event_name, "Daytona 500");
    }

    #[test]
    fn test_navigation() {
        let registry = make_test_series();
        let config = UserConfig::default();
        let mut app = App::new(registry, config);

        let e1 = make_test_event("f1", "Race 1", NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
        let e2 = make_test_event("f1", "Race 2", NaiveDate::from_ymd_opt(2026, 3, 15).unwrap());
        app.update_series_data("f1".to_string(), vec![e1, e2]);

        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(app.selected_event().unwrap().event_name, "Race 1");

        app.select_next();
        assert_eq!(app.table_state.selected(), Some(1));
        assert_eq!(app.selected_event().unwrap().event_name, "Race 2");

        // Wrap around
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(0));

        app.select_previous();
        assert_eq!(app.table_state.selected(), Some(1));
    }
}
