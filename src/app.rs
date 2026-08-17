use std::collections::{HashMap, HashSet};

use chrono::Datelike;
use ratatui::widgets::{ListState, TableState};

use crate::config::UserConfig;
use crate::data::models::{FetchStatus, RaceEvent, Series};

/// Which view the user is currently looking at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Calendar,
}

/// Active multi-select filters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveFilters {
    pub favorites_only: bool,
    pub car_styles: HashSet<String>,
    pub regions: HashSet<String>,
    pub series: HashSet<String>,
}

impl ActiveFilters {
    pub fn is_empty(&self) -> bool {
        !self.favorites_only && self.car_styles.is_empty() && self.regions.is_empty() && self.series.is_empty()
    }

    pub fn clear(&mut self) {
        self.favorites_only = false;
        self.car_styles.clear();
        self.regions.clear();
        self.series.clear();
    }
}

/// Filter option in the filter panel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOption {
    All,
    Favorites,
    CarStyle(String),
    Region(String),
    Series(String),
}

/// Item in the filter panel (header or selectable entry)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterItem {
    Header(&'static str),
    Entry {
        label: String,
        option: FilterOption,
    },
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

    /// Active multi-select filters
    pub active_filters: ActiveFilters,

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

    /// Ratatui list state for filter panel
    pub filter_list_state: ListState,

    // -- Calendar view state --
    /// Currently displayed year/month in calendar view
    pub calendar_year: i32,
    pub calendar_month: u32,

    /// Status bar message (temporary messages like "Refreshing F1...")
    pub status_message: Option<String>,

    /// Whether user requested manual refresh
    pub refresh_requested: bool,

    /// Pending series ID for favorite/un-favorite confirmation dialog
    pub pending_favorite_toggle: Option<String>,
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

        let mut filter_list_state = ListState::default();
        filter_list_state.select(Some(1)); // select "All Events" by default

        let mut app = Self {
            running: true,
            view_mode,
            series_registry,
            events: HashMap::new(),
            fetch_status,
            config,
            active_filters: ActiveFilters::default(),
            search_query: None,
            search_active: false,
            show_help: false,
            show_detail: false,
            show_filter_panel: false,
            table_state: TableState::default(),
            filter_list_state,
            calendar_year: now.year(),
            calendar_month: now.month(),
            status_message: None,
            refresh_requested: false,
            pending_favorite_toggle: None,
        };

        // Select the first row by default
        app.table_state.select(Some(0));
        app
    }

    /// Build the list of all items for the filter panel
    pub fn filter_items(&self) -> Vec<FilterItem> {
        let mut items = Vec::new();

        // ── Filter By ──
        items.push(FilterItem::Header("── Filter By ──"));
        items.push(FilterItem::Entry {
            label: "All Events".to_string(),
            option: FilterOption::All,
        });
        items.push(FilterItem::Entry {
            label: "★ Favorites Only".to_string(),
            option: FilterOption::Favorites,
        });

        // ── Car Style ──
        items.push(FilterItem::Header("── Car Style ──"));
        for style in &[
            "Open Wheel",
            "Sports Car",
            "Stock Car",
            "Touring",
            "Rally",
            "Motorcycle",
        ] {
            items.push(FilterItem::Entry {
                label: style.to_string(),
                option: FilterOption::CarStyle(style.to_string()),
            });
        }

        // ── Region ──
        items.push(FilterItem::Header("── Region ──"));
        for region in &[
            "International",
            "USA",
            "Europe",
            "UK",
            "Japan",
            "Australia",
            "Asia",
        ] {
            items.push(FilterItem::Entry {
                label: region.to_string(),
                option: FilterOption::Region(region.to_string()),
            });
        }

        // ── Series ──
        items.push(FilterItem::Header("── Series ──"));
        let mut series_list: Vec<&Series> = self.series_registry.values().collect();
        series_list.sort_by_key(|s| &s.name);
        for s in series_list {
            items.push(FilterItem::Entry {
                label: s.name.clone(),
                option: FilterOption::Series(s.id.clone()),
            });
        }

        items
    }

    /// Check if a given filter option is currently active
    pub fn is_filter_option_active(&self, option: &FilterOption) -> bool {
        match option {
            FilterOption::All => self.active_filters.is_empty(),
            FilterOption::Favorites => self.active_filters.favorites_only,
            FilterOption::CarStyle(style) => self.active_filters.car_styles.contains(style),
            FilterOption::Region(region) => self.active_filters.regions.contains(region),
            FilterOption::Series(series_id) => self.active_filters.series.contains(series_id),
        }
    }

    /// Toggle a specific filter option
    pub fn toggle_filter_option(&mut self, option: &FilterOption) {
        match option {
            FilterOption::All => {
                self.active_filters.clear();
            }
            FilterOption::Favorites => {
                self.active_filters.favorites_only = !self.active_filters.favorites_only;
            }
            FilterOption::CarStyle(style) => {
                if !self.active_filters.car_styles.remove(style) {
                    self.active_filters.car_styles.insert(style.clone());
                }
            }
            FilterOption::Region(region) => {
                if !self.active_filters.regions.remove(region) {
                    self.active_filters.regions.insert(region.clone());
                }
            }
            FilterOption::Series(series_id) => {
                if !self.active_filters.series.remove(series_id) {
                    self.active_filters.series.insert(series_id.clone());
                }
            }
        }
        self.table_state.select(Some(0));
    }

    /// Toggle the currently highlighted filter item in the panel
    pub fn toggle_selected_filter(&mut self) {
        let items = self.filter_items();
        if let Some(selected) = self.filter_list_state.selected() {
            if let Some(FilterItem::Entry { option, .. }) = items.get(selected) {
                let opt = option.clone();
                self.toggle_filter_option(&opt);
            }
        }
    }

    /// Reset all filters back to All Events
    pub fn reset_filters(&mut self) {
        self.active_filters.clear();
        self.table_state.select(Some(0));
    }

    /// Move selection down in filter panel, skipping headers
    pub fn filter_select_next(&mut self) {
        let items = self.filter_items();
        if items.is_empty() {
            return;
        }
        let current = self.filter_list_state.selected().unwrap_or(0);
        let mut next = (current + 1) % items.len();
        while matches!(items[next], FilterItem::Header(_)) {
            next = (next + 1) % items.len();
            if next == current {
                break;
            }
        }
        self.filter_list_state.select(Some(next));
    }

    /// Move selection up in filter panel, skipping headers
    pub fn filter_select_previous(&mut self) {
        let items = self.filter_items();
        if items.is_empty() {
            return;
        }
        let current = self.filter_list_state.selected().unwrap_or(0);
        let mut prev = if current == 0 { items.len() - 1 } else { current - 1 };
        while matches!(items[prev], FilterItem::Header(_)) {
            prev = if prev == 0 { items.len() - 1 } else { prev - 1 };
            if prev == current {
                break;
            }
        }
        self.filter_list_state.select(Some(prev));
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

                // Check favorites filter
                if self.active_filters.favorites_only && !self.config.favorites.contains(&event.series_id) {
                    return false;
                }

                // Check series filter
                if !self.active_filters.series.is_empty() && !self.active_filters.series.contains(&event.series_id) {
                    return false;
                }

                // Check car style filter
                if !self.active_filters.car_styles.is_empty() {
                    let matches_style = self
                        .series_registry
                        .get(&event.series_id)
                        .map_or(false, |s| self.active_filters.car_styles.contains(s.car_style.as_str()));
                    if !matches_style {
                        return false;
                    }
                }

                // Check region filter
                if !self.active_filters.regions.is_empty() {
                    let matches_region = self
                        .series_registry
                        .get(&event.series_id)
                        .map_or(false, |s| self.active_filters.regions.contains(&s.region));
                    if !matches_region {
                        return false;
                    }
                }

                true
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

    /// Select the next item in the event list (wraps around).
    pub fn select_next(&mut self) {
        let count = self.filtered_events().len();
        if count == 0 {
            self.table_state.select(None);
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let next = if current + 1 >= count { 0 } else { current + 1 };
        self.table_state.select(Some(next));
    }

    /// Select the previous item in the event list (wraps around).
    pub fn select_previous(&mut self) {
        let count = self.filtered_events().len();
        if count == 0 {
            self.table_state.select(None);
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let prev = if current == 0 { count - 1 } else { current - 1 };
        self.table_state.select(Some(prev));
    }

    /// Get the currently selected race event, if any.
    pub fn selected_event(&self) -> Option<&RaceEvent> {
        let events = self.filtered_events();
        let index = self.table_state.selected()?;
        events.get(index).copied()
    }

    /// Update events for a series and mark it as loaded.
    pub fn update_series_data(&mut self, series_id: String, events: Vec<RaceEvent>) {
        let count = events.len();
        self.events.insert(series_id.clone(), events);
        self.fetch_status.insert(series_id, FetchStatus::Loaded(count));

        // Adjust selection if it's out of bounds
        let total = self.filtered_events().len();
        if total > 0 && self.table_state.selected().map_or(true, |i| i >= total) {
            self.table_state.select(Some(0));
        }
    }

    /// Mark a series as currently fetching.
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

    fn mock_series(id: &str, name: &str, style: CarStyle, region: &str) -> Series {
        Series {
            id: id.to_string(),
            name: name.to_string(),
            short_name: id.to_uppercase(),
            car_style: style,
            color: (255, 0, 0),
            region: region.to_string(),
            calendar_url: "https://example.com".to_string(),
            requires_js: false,
        }
    }

    fn mock_event(series_id: &str, name: &str, date: (i32, u32, u32)) -> RaceEvent {
        let (y, m, d) = date;
        let start = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        RaceEvent {
            series_id: series_id.to_string(),
            event_name: name.to_string(),
            circuit_name: "Mock Circuit".to_string(),
            location: "Mock City".to_string(),
            country: "Mock Country".to_string(),
            start_date: start,
            end_date: start,
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        }
    }

    #[test]
    fn test_app_initial_state() {
        let mut registry = HashMap::new();
        registry.insert("f1".to_string(), mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"));
        let config = UserConfig::default();
        let app = App::new(registry, config);

        assert!(app.running);
        assert_eq!(app.view_mode, ViewMode::List);
        assert!(app.active_filters.is_empty());
        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(app.fetch_status.get("f1"), Some(&FetchStatus::Pending));
    }

    #[test]
    fn test_navigation() {
        let mut registry = HashMap::new();
        registry.insert("f1".to_string(), mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"));
        let mut app = App::new(registry, UserConfig::default());

        // Empty event list navigation
        app.select_next();
        assert_eq!(app.table_state.selected(), None);

        // Add 3 events
        let events = vec![
            mock_event("f1", "Race 1", (2026, 3, 1)),
            mock_event("f1", "Race 2", (2026, 3, 15)),
            mock_event("f1", "Race 3", (2026, 3, 29)),
        ];
        app.update_series_data("f1".to_string(), events);

        assert_eq!(app.table_state.selected(), Some(0));
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(1));
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(2));
        // Wrap around
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(0));

        // Prev navigation
        app.select_previous();
        assert_eq!(app.table_state.selected(), Some(2));
    }

    #[test]
    fn test_multi_select_filtering() {
        let mut registry = HashMap::new();
        registry.insert("f1".to_string(), mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"));
        registry.insert("nascar".to_string(), mock_series("nascar", "NASCAR", CarStyle::StockCar, "USA"));
        registry.insert("wec".to_string(), mock_series("wec", "WEC", CarStyle::SportsCar, "International"));

        let mut config = UserConfig::default();
        config.favorites = ["f1".to_string(), "nascar".to_string()].into_iter().collect();

        let mut app = App::new(registry, config);
        app.update_series_data("f1".to_string(), vec![mock_event("f1", "Bahrain GP", (2026, 3, 1))]);
        app.update_series_data("nascar".to_string(), vec![mock_event("nascar", "Daytona 500", (2026, 2, 15))]);
        app.update_series_data("wec".to_string(), vec![mock_event("wec", "Qatar 1812km", (2026, 2, 28))]);

        // Default: all 3 events
        assert_eq!(app.filtered_events().len(), 3);

        // Toggle Favorites: only F1 and NASCAR
        app.toggle_filter_option(&FilterOption::Favorites);
        assert_eq!(app.filtered_events().len(), 2);

        // Combined with CarStyle StockCar: only NASCAR
        app.toggle_filter_option(&FilterOption::CarStyle("Stock Car".to_string()));
        assert_eq!(app.filtered_events().len(), 1);
        assert_eq!(app.filtered_events()[0].event_name, "Daytona 500");

        // Reset filters
        app.reset_filters();
        assert_eq!(app.filtered_events().len(), 3);
    }
}
