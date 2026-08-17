use std::collections::{HashMap, HashSet};

use chrono::Datelike;
use ratatui::widgets::{ListState, TableState};

use crate::config::UserConfig;
use crate::data::models::{EventStatus, FetchStatus, RaceEvent, Series};

/// Which view the user is currently looking at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Calendar,
}

/// Active multi-select filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFilters {
    pub favorites_only: bool,
    pub statuses: HashSet<EventStatus>,
    pub car_styles: HashSet<String>,
    pub regions: HashSet<String>,
    pub series: HashSet<String>,
}

impl Default for ActiveFilters {
    fn default() -> Self {
        let mut statuses = HashSet::new();
        statuses.insert(EventStatus::Upcoming);
        statuses.insert(EventStatus::Live);
        Self {
            favorites_only: false,
            statuses,
            car_styles: HashSet::new(),
            regions: HashSet::new(),
            series: HashSet::new(),
        }
    }
}

impl ActiveFilters {
    /// Returns true if active filters match the default state (Upcoming + Live, no category filters).
    pub fn is_default(&self) -> bool {
        !self.favorites_only
            && self.car_styles.is_empty()
            && self.regions.is_empty()
            && self.series.is_empty()
            && self.statuses.len() == 2
            && self.statuses.contains(&EventStatus::Upcoming)
            && self.statuses.contains(&EventStatus::Live)
    }

    /// Reset filters back to default state (Upcoming + Live, no category filters).
    pub fn reset_to_default(&mut self) {
        *self = Self::default();
    }
}

/// Filter option in the filter panel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOption {
    All,
    Favorites,
    Status(EventStatus),
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

/// Item in the main list table (day section header or race event entry)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListTableItem<'a> {
    Header(chrono::NaiveDate),
    Event(&'a RaceEvent),
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
    /// Currently selected day in calendar view (1..=31)
    pub calendar_selected_day: u32,

    /// Whether the day events selection popup is visible (for days with multiple races)
    pub show_day_events: bool,
    /// Ratatui list state for day events selection popup
    pub day_events_state: ListState,

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

        let mut day_events_state = ListState::default();
        day_events_state.select(Some(0));

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
            show_day_events: false,
            table_state: TableState::default(),
            filter_list_state,
            calendar_year: now.year(),
            calendar_month: now.month(),
            calendar_selected_day: now.day(),
            day_events_state,
            status_message: None,
            refresh_requested: false,
            pending_favorite_toggle: None,
        };

        // Select the first event by default
        app.select_first_event();
        app
    }

    /// Build the list of all items for the filter panel
    pub fn filter_items(&self) -> Vec<FilterItem> {
        let mut items = Vec::new();

        // ── Filter By ──
        items.push(FilterItem::Header("── Filter By ──"));
        items.push(FilterItem::Entry {
            label: "All Events (Default)".to_string(),
            option: FilterOption::All,
        });
        items.push(FilterItem::Entry {
            label: "★ Favorites Only".to_string(),
            option: FilterOption::Favorites,
        });

        // ── Status ──
        items.push(FilterItem::Header("── Status ──"));
        items.push(FilterItem::Entry {
            label: "Upcoming".to_string(),
            option: FilterOption::Status(EventStatus::Upcoming),
        });
        items.push(FilterItem::Entry {
            label: "In Progress (Live)".to_string(),
            option: FilterOption::Status(EventStatus::Live),
        });
        items.push(FilterItem::Entry {
            label: "Completed".to_string(),
            option: FilterOption::Status(EventStatus::Completed),
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
            FilterOption::All => self.active_filters.is_default(),
            FilterOption::Favorites => self.active_filters.favorites_only,
            FilterOption::Status(status) => self.active_filters.statuses.contains(status),
            FilterOption::CarStyle(style) => self.active_filters.car_styles.contains(style),
            FilterOption::Region(region) => self.active_filters.regions.contains(region),
            FilterOption::Series(series_id) => self.active_filters.series.contains(series_id),
        }
    }

    /// Toggle a specific filter option
    pub fn toggle_filter_option(&mut self, option: &FilterOption) {
        match option {
            FilterOption::All => {
                self.active_filters.reset_to_default();
            }
            FilterOption::Favorites => {
                self.active_filters.favorites_only = !self.active_filters.favorites_only;
            }
            FilterOption::Status(status) => {
                if !self.active_filters.statuses.remove(status) {
                    self.active_filters.statuses.insert(status.clone());
                }
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
        self.select_first_event();
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
        self.active_filters.reset_to_default();
        self.select_first_event();
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

                // Check status filter (only applies in List view)
                if self.view_mode == ViewMode::List && !self.active_filters.statuses.contains(&event.status) {
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

        // Sort chronologically by date, then known times before TBD, then actual UTC start time, then series_id & event_name
        events.sort_by_key(|e| {
            let local_date = e.local_start_date();
            let is_undetermined = if e.race_start_time().is_some() { 0 } else { 1 };
            let start_utc = e.race_start_time();
            (local_date, is_undetermined, start_utc, e.series_id.clone(), e.event_name.clone())
        });

        events
    }

    /// Build the list of table items (blank day separators and race event entries)
    pub fn list_table_items(&self) -> Vec<ListTableItem<'_>> {
        let events = self.filtered_events();
        let mut items = Vec::new();
        let mut current_date: Option<chrono::NaiveDate> = None;

        for event in events {
            let event_date = event.local_start_date();
            if let Some(prev_date) = current_date {
                if prev_date != event_date {
                    current_date = Some(event_date);
                    items.push(ListTableItem::Header(event_date));
                }
            } else {
                current_date = Some(event_date);
            }
            items.push(ListTableItem::Event(event));
        }

        items
    }

    /// Select the next event in the table (skips headers, wraps around).
    pub fn select_next(&mut self) {
        let items = self.list_table_items();
        if items.is_empty() {
            self.table_state.select(None);
            return;
        }

        let current = self.table_state.selected().unwrap_or(0);
        let mut next = (current + 1) % items.len();
        while matches!(items.get(next), Some(ListTableItem::Header(_))) {
            next = (next + 1) % items.len();
            if next == current {
                break;
            }
        }
        if matches!(items.get(next), Some(ListTableItem::Event(_))) {
            self.table_state.select(Some(next));
        }
    }

    /// Select the previous event in the table (skips headers, wraps around).
    pub fn select_previous(&mut self) {
        let items = self.list_table_items();
        if items.is_empty() {
            self.table_state.select(None);
            return;
        }

        let current = self.table_state.selected().unwrap_or(0);
        let mut prev = if current == 0 { items.len() - 1 } else { current - 1 };
        while matches!(items.get(prev), Some(ListTableItem::Header(_))) {
            prev = if prev == 0 { items.len() - 1 } else { prev - 1 };
            if prev == current {
                break;
            }
        }
        if matches!(items.get(prev), Some(ListTableItem::Event(_))) {
            self.table_state.select(Some(prev));
        }
    }

    /// Get the currently selected race event, if any.
    pub fn selected_event(&self) -> Option<&RaceEvent> {
        let items = self.list_table_items();
        if items.is_empty() {
            return None;
        }

        if let Some(index) = self.table_state.selected() {
            if let Some(ListTableItem::Event(event)) = items.get(index) {
                return Some(*event);
            }
        }

        // If selection is None or pointing to a header, fallback to first event
        for (_idx, item) in items.iter().enumerate() {
            if let ListTableItem::Event(event) = item {
                return Some(*event);
            }
        }

        None
    }

    /// Select the first event row in the table (skipping any initial header).
    pub fn select_first_event(&mut self) {
        let items = self.list_table_items();
        for (idx, item) in items.iter().enumerate() {
            if let ListTableItem::Event(_) = item {
                self.table_state.select(Some(idx));
                return;
            }
        }
        self.table_state.select(None);
    }

    /// Update events for a series and mark it as loaded.
    pub fn update_series_data(&mut self, series_id: String, events: Vec<RaceEvent>) {
        let count = events.len();
        self.events.insert(series_id.clone(), events);
        self.fetch_status.insert(series_id, FetchStatus::Loaded(count));

        // Adjust selection if it's out of bounds or on a header
        let total = self.list_table_items().len();
        if total > 0 {
            if self.table_state.selected().map_or(true, |i| i >= total) {
                self.select_first_event();
            } else if let Some(idx) = self.table_state.selected() {
                if matches!(self.list_table_items().get(idx), Some(ListTableItem::Header(_))) {
                    self.select_first_event();
                }
            }
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

    /// Days in the currently displayed calendar month
    pub fn calendar_days_in_current_month(&self) -> u32 {
        crate::ui::calendar_view::days_in_month(self.calendar_year, self.calendar_month)
    }

    /// Move calendar day selection to the next day (advances month if past end of month).
    pub fn calendar_select_next_day(&mut self) {
        let max_days = self.calendar_days_in_current_month();
        if self.calendar_selected_day < max_days {
            self.calendar_selected_day += 1;
        } else {
            self.calendar_select_next_month();
            self.calendar_selected_day = 1;
        }
    }

    /// Move calendar day selection to the previous day (decrements month if before day 1).
    pub fn calendar_select_prev_day(&mut self) {
        if self.calendar_selected_day > 1 {
            self.calendar_selected_day -= 1;
        } else {
            self.calendar_select_prev_month();
            self.calendar_selected_day = self.calendar_days_in_current_month();
        }
    }

    /// Move calendar day selection down by 1 week (+7 days).
    pub fn calendar_select_next_week(&mut self) {
        let max_days = self.calendar_days_in_current_month();
        if self.calendar_selected_day + 7 <= max_days {
            self.calendar_selected_day += 7;
        } else {
            let overflow = (self.calendar_selected_day + 7) - max_days;
            self.calendar_select_next_month();
            let new_max = self.calendar_days_in_current_month();
            self.calendar_selected_day = overflow.min(new_max);
        }
    }

    /// Move calendar day selection up by 1 week (-7 days).
    pub fn calendar_select_prev_week(&mut self) {
        if self.calendar_selected_day > 7 {
            self.calendar_selected_day -= 7;
        } else {
            let underflow = 7 - self.calendar_selected_day;
            self.calendar_select_prev_month();
            let prev_max = self.calendar_days_in_current_month();
            self.calendar_selected_day = prev_max.saturating_sub(underflow).max(1);
        }
    }

    /// Advance calendar to next month, keeping day bounded.
    pub fn calendar_select_next_month(&mut self) {
        if self.calendar_month == 12 {
            self.calendar_month = 1;
            self.calendar_year += 1;
        } else {
            self.calendar_month += 1;
        }
        let max_days = self.calendar_days_in_current_month();
        self.calendar_selected_day = self.calendar_selected_day.min(max_days);
    }

    /// Decrement calendar to previous month, keeping day bounded.
    pub fn calendar_select_prev_month(&mut self) {
        if self.calendar_month == 1 {
            self.calendar_month = 12;
            self.calendar_year -= 1;
        } else {
            self.calendar_month -= 1;
        }
        let max_days = self.calendar_days_in_current_month();
        self.calendar_selected_day = self.calendar_selected_day.min(max_days);
    }

    /// Advance calendar to next year.
    pub fn calendar_select_next_year(&mut self) {
        self.calendar_year += 1;
        let max_days = self.calendar_days_in_current_month();
        self.calendar_selected_day = self.calendar_selected_day.min(max_days);
    }

    /// Decrement calendar to previous year.
    pub fn calendar_select_prev_year(&mut self) {
        self.calendar_year -= 1;
        let max_days = self.calendar_days_in_current_month();
        self.calendar_selected_day = self.calendar_selected_day.min(max_days);
    }

    /// Jump calendar selection and month/year to today.
    pub fn calendar_jump_to_today(&mut self) {
        let now = chrono::Local::now();
        self.calendar_year = now.year();
        self.calendar_month = now.month();
        self.calendar_selected_day = now.day();
    }

    /// Get all events on a specific date, sorted chronologically with TBD at the bottom.
    pub fn events_on_date(&self, date: chrono::NaiveDate) -> Vec<&RaceEvent> {
        let events = self.filtered_events();
        let mut day_events: Vec<_> = events
            .into_iter()
            .filter(|e| e.local_start_date() == date || (e.start_date <= date && date <= e.end_date))
            .collect();
        day_events.sort_by_key(|e| {
            let is_undetermined = if e.race_start_time().is_some() { 0 } else { 1 };
            let start_utc = e.race_start_time();
            (is_undetermined, start_utc, e.series_id.clone(), e.event_name.clone())
        });
        day_events
    }

    /// Get all events on the currently selected calendar day.
    pub fn events_on_selected_calendar_day(&self) -> Vec<&RaceEvent> {
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(
            self.calendar_year,
            self.calendar_month,
            self.calendar_selected_day,
        ) {
            self.events_on_date(date)
        } else {
            Vec::new()
        }
    }

    /// Select next event in day events popup
    pub fn day_events_select_next(&mut self) {
        let count = self.events_on_selected_calendar_day().len();
        if count == 0 {
            return;
        }
        let current = self.day_events_state.selected().unwrap_or(0);
        let next = (current + 1) % count;
        self.day_events_state.select(Some(next));
    }

    /// Select previous event in day events popup
    pub fn day_events_select_prev(&mut self) {
        let count = self.events_on_selected_calendar_day().len();
        if count == 0 {
            return;
        }
        let current = self.day_events_state.selected().unwrap_or(0);
        let prev = if current == 0 { count - 1 } else { current - 1 };
        self.day_events_state.select(Some(prev));
    }

    /// Get currently selected event in day events popup
    pub fn selected_day_event(&self) -> Option<&RaceEvent> {
        let events = self.events_on_selected_calendar_day();
        let idx = self.day_events_state.selected()?;
        events.get(idx).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::models::{CarStyle, EventStatus};
    use chrono::{NaiveDate, TimeZone};

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
        assert!(app.active_filters.is_default());
        assert_eq!(app.table_state.selected(), None);
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

        // Add 3 events on different dates
        let events = vec![
            mock_event("f1", "Race 1", (2026, 3, 1)),
            mock_event("f1", "Race 2", (2026, 3, 15)),
            mock_event("f1", "Race 3", (2026, 3, 29)),
        ];
        app.update_series_data("f1".to_string(), events);

        // Initial selection should be on first event (index 0)
        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(app.selected_event().map(|e| e.event_name.as_str()), Some("Race 1"));

        // Select next: skips separator at index 1, moves to index 2 (Race 2)
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(2));
        assert_eq!(app.selected_event().map(|e| e.event_name.as_str()), Some("Race 2"));

        // Select next: skips separator at index 3, moves to index 4 (Race 3)
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(4));
        assert_eq!(app.selected_event().map(|e| e.event_name.as_str()), Some("Race 3"));

        // Wrap around: wraps to index 0 (Race 1)
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(app.selected_event().map(|e| e.event_name.as_str()), Some("Race 1"));

        // Prev navigation: wraps to index 4 (Race 3)
        app.select_previous();
        assert_eq!(app.table_state.selected(), Some(4));
        assert_eq!(app.selected_event().map(|e| e.event_name.as_str()), Some("Race 3"));
    }

    #[test]
    fn test_chronological_time_sorting_and_tbd_at_bottom() {
        let mut registry = HashMap::new();
        registry.insert("f1".to_string(), mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"));
        registry.insert("nascar".to_string(), mock_series("nascar", "NASCAR", CarStyle::StockCar, "USA"));
        registry.insert("indycar".to_string(), mock_series("indycar", "IndyCar", CarStyle::OpenWheel, "USA"));

        let mut app = App::new(registry, UserConfig::default());

        let mut ev_7pm = mock_event("nascar", "NASCAR 7PM", (2026, 5, 24));
        ev_7pm.sessions = vec![crate::data::models::Session {
            name: "Race".to_string(),
            session_type: crate::data::models::SessionType::Race,
            start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 5, 24, 23, 0, 0).unwrap()), // 7 PM EDT
            end_time: None,
        }];

        let mut ev_12pm = mock_event("indycar", "IndyCar 12PM", (2026, 5, 24));
        ev_12pm.sessions = vec![crate::data::models::Session {
            name: "Race".to_string(),
            session_type: crate::data::models::SessionType::Race,
            start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 5, 24, 16, 0, 0).unwrap()), // 12 PM EDT
            end_time: None,
        }];

        let mut ev_230pm = mock_event("f1", "F1 2:30PM", (2026, 5, 24));
        ev_230pm.sessions = vec![crate::data::models::Session {
            name: "Race".to_string(),
            session_type: crate::data::models::SessionType::Race,
            start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 5, 24, 18, 30, 0).unwrap()), // 2:30 PM EDT
            end_time: None,
        }];

        let ev_tbd = mock_event("nascar", "NASCAR TBD", (2026, 5, 24)); // No sessions (TBD)

        app.update_series_data("nascar".to_string(), vec![ev_7pm, ev_tbd]);
        app.update_series_data("indycar".to_string(), vec![ev_12pm]);
        app.update_series_data("f1".to_string(), vec![ev_230pm]);

        let filtered = app.filtered_events();
        assert_eq!(filtered.len(), 4);
        assert_eq!(filtered[0].event_name, "IndyCar 12PM");
        assert_eq!(filtered[1].event_name, "F1 2:30PM");
        assert_eq!(filtered[2].event_name, "NASCAR 7PM");
        assert_eq!(filtered[3].event_name, "NASCAR TBD"); // TBD at bottom of the day
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

    #[test]
    fn test_status_filtering() {
        let mut registry = HashMap::new();
        registry.insert("f1".to_string(), mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"));

        let mut app = App::new(registry, UserConfig::default());
        let mut upcoming = mock_event("f1", "Upcoming Race", (2026, 6, 1));
        upcoming.status = EventStatus::Upcoming;
        let mut live = mock_event("f1", "Live Race", (2026, 5, 1));
        live.status = EventStatus::Live;
        let mut completed = mock_event("f1", "Past Race", (2026, 3, 1));
        completed.status = EventStatus::Completed;

        app.update_series_data("f1".to_string(), vec![upcoming, live, completed]);

        // By default in List view: completed is hidden (only Upcoming + Live -> 2 events)
        assert_eq!(app.filtered_events().len(), 2);

        // In Calendar view: all 3 events are returned
        app.view_mode = ViewMode::Calendar;
        assert_eq!(app.filtered_events().len(), 3);
        app.view_mode = ViewMode::List;

        // Toggle Completed: now 3 events in list view
        app.toggle_filter_option(&FilterOption::Status(EventStatus::Completed));
        assert_eq!(app.filtered_events().len(), 3);

        // Toggle Upcoming off: now Live + Completed -> 2 events
        app.toggle_filter_option(&FilterOption::Status(EventStatus::Upcoming));
        assert_eq!(app.filtered_events().len(), 2);
        assert_eq!(filtered_name(&app, 0), "Past Race");
        assert_eq!(filtered_name(&app, 1), "Live Race");

        // Reset filters: restores Upcoming + Live default
        app.reset_filters();
        assert_eq!(app.filtered_events().len(), 2);
        assert!(app.active_filters.is_default());
    }

    fn filtered_name(app: &App, idx: usize) -> String {
        app.filtered_events()[idx].event_name.clone()
    }
}
