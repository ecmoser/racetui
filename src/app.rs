use std::collections::{HashMap, HashSet};

use chrono::Datelike;
use ratatui::widgets::{ListState, TableState};

use crate::config::UserConfig;
use crate::data::models::{
    EventStatus, FetchStatus, RaceEvent, ScheduledSession, Series, SessionCategory, SessionType,
};

/// Which view the user is currently looking at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Calendar,
    Live,
    Standings,
}

/// Sub-tabs within the Live view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveSubTab {
    /// Timing leaderboard
    Timing,
    /// Track map with driver positions
    TrackMap,
}

/// Active multi-select filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFilters {
    pub favorites_only: bool,
    pub statuses: HashSet<EventStatus>,
    pub session_categories: HashSet<SessionCategory>,
    pub car_styles: HashSet<String>,
    pub regions: HashSet<String>,
    pub series: HashSet<String>,
}

impl Default for ActiveFilters {
    fn default() -> Self {
        let mut statuses = HashSet::new();
        statuses.insert(EventStatus::Upcoming);
        statuses.insert(EventStatus::Live);
        let mut session_categories = HashSet::new();
        session_categories.insert(SessionCategory::Race);
        session_categories.insert(SessionCategory::Qualifying);
        session_categories.insert(SessionCategory::Practice);
        session_categories.insert(SessionCategory::Other);
        Self {
            favorites_only: false,
            statuses,
            session_categories,
            car_styles: HashSet::new(),
            regions: HashSet::new(),
            series: HashSet::new(),
        }
    }
}

impl ActiveFilters {
    /// Returns true if active filters match the default state (Upcoming + Live, all session types, no category filters).
    pub fn is_default(&self) -> bool {
        !self.favorites_only
            && self.car_styles.is_empty()
            && self.regions.is_empty()
            && self.series.is_empty()
            && self.session_categories.len() == 4
            && self.statuses.len() == 2
            && self.statuses.contains(&EventStatus::Upcoming)
            && self.statuses.contains(&EventStatus::Live)
    }

    /// Reset filters back to default state (Upcoming + Live, all session types, no category filters).
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
    SessionCategory(SessionCategory),
    CarStyle(String),
    Region(String),
    Series(String),
}

/// Item in the filter panel (header or selectable entry)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterItem {
    Header(&'static str),
    Entry { label: String, option: FilterOption },
}

/// Item in the main list table (day section header or scheduled session entry)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListTableItem<'a> {
    Header(chrono::NaiveDate),
    Session(ScheduledSession<'a>),
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
    /// Timestamp when the status bar message was set (for auto-clearing after 3s)
    pub status_message_set_at: Option<std::time::Instant>,

    /// Whether user requested manual refresh
    pub refresh_requested: bool,

    /// Pending series ID for favorite/un-favorite confirmation dialog
    pub pending_favorite_toggle: Option<String>,

    /// Index for cycling through notification messages
    pub notification_cycle_index: usize,

    /// Tick counter for timing notification cycles
    pub tick_count: u64,

    // -- Standings view state --
    /// Cached championship standings per series
    pub standings: HashMap<String, crate::data::standings::SeasonStandings>,
    /// Which series is currently selected in the Standings view dropdown
    pub standings_selected_series: Option<String>,
    /// Index of the selected series in the standings series list
    pub standings_series_index: usize,
    /// Whether the standings series dropdown is open
    pub standings_dropdown_open: bool,
    /// Scroll offset for the standings table
    pub standings_table_state: TableState,

    // -- Results state --
    /// Cached race results, keyed by (series_id, round)
    pub results: HashMap<(String, u32), crate::data::results::RaceResults>,
    /// Whether results background loading has been triggered
    pub results_loading_started: bool,

    // -- Live view state --
    /// Currently active Live sub-tab
    pub live_sub_tab: LiveSubTab,
    /// Which series/session the Live view is currently showing (series_id)
    pub live_active_series: Option<String>,
    /// Whether the live session picker popup is visible
    pub show_live_session_picker: bool,
    /// List state for the live session picker popup
    pub live_session_picker_state: ListState,
    /// Scroll offset in the live timing leaderboard
    pub live_timing_table_state: TableState,
    /// Currently selected driver index in the live timing table (for detail expansion)
    pub live_selected_driver: Option<usize>,
    /// Whether the driver detail expansion is visible
    pub live_driver_detail_open: bool,
    /// Tick counter for blinking the LIVE indicator (toggles every N ticks)
    pub live_blink_on: bool,
    /// Current live timing data snapshot (updated by LiveEvent::TimingUpdate)
    pub live_timing_data: Option<()>,
    /// Cached circuit geometry for the current live session
    pub track_map_geometry: Option<()>,
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
            status_message_set_at: None,
            refresh_requested: false,
            pending_favorite_toggle: None,
            notification_cycle_index: 0,
            tick_count: 0,
            standings: HashMap::new(),
            standings_selected_series: None,
            standings_series_index: 0,
            standings_dropdown_open: false,
            standings_table_state: TableState::default(),
            results: HashMap::new(),
            results_loading_started: false,
            live_sub_tab: LiveSubTab::Timing,
            live_active_series: None,
            show_live_session_picker: false,
            live_session_picker_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            live_timing_table_state: TableState::default(),
            live_selected_driver: None,
            live_driver_detail_open: false,
            live_blink_on: true,
            live_timing_data: None,
            track_map_geometry: None,
        };

        // Select the first event by default
        app.select_first_event();
        app
    }

    /// Set a status bar message and record its timestamp for auto-clearing.
    pub fn set_status_message(&mut self, msg: String) {
        self.status_message = Some(msg);
        self.status_message_set_at = Some(std::time::Instant::now());
    }

    /// Get a list of currently live sessions across all series.
    /// Returns (series_id, series_short_name, session_name) tuples.
    pub fn get_live_sessions(&self) -> Vec<(String, String, String)> {
        let now = chrono::Utc::now();
        let mut live = Vec::new();
        for (series_id, events) in &self.events {
            for event in events {
                if event.status == crate::data::models::EventStatus::Live {
                    let active_session = event.sessions.iter().find(|s| {
                        let started = s.start_time.map_or(false, |t| t <= now);
                        let ended = s.end_time.map_or(false, |t| t <= now);
                        started && !ended
                    });
                    let session_name = active_session
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| "Session".to_string());
                    let short_name = self
                        .series_registry
                        .get(series_id)
                        .map(|s| s.short_name.clone())
                        .unwrap_or_else(|| series_id.clone());
                    live.push((series_id.clone(), short_name, session_name));
                }
            }
        }
        live
    }

    /// Cycle through series in the Standings view.
    /// `direction`: -1 for previous, +1 for next.
    pub fn standings_cycle_series(&mut self, direction: i32) {
        let mut series_ids: Vec<String> = self.standings.keys().cloned().collect();
        series_ids.sort();
        if series_ids.is_empty() {
            return;
        }
        let current_idx = self
            .standings_selected_series
            .as_ref()
            .and_then(|id| series_ids.iter().position(|s| s == id))
            .unwrap_or(0);
        let new_idx = if direction > 0 {
            (current_idx + 1) % series_ids.len()
        } else if current_idx == 0 {
            series_ids.len() - 1
        } else {
            current_idx - 1
        };
        self.standings_selected_series = Some(series_ids[new_idx].clone());
        self.standings_series_index = new_idx;
        self.standings_table_state = TableState::default();
    }

    /// Get notification messages for upcoming favorited events.
    /// Returns a Vec of (series_short_name, session_name, time_until_string).
    pub fn get_notifications(&self) -> Vec<String> {
        let now = chrono::Utc::now();
        let threshold = chrono::Duration::hours(self.config.notification_threshold_hours as i64);
        let mut notifications = Vec::new();

        for (series_id, events) in &self.events {
            if !self.config.favorites.contains(series_id) {
                continue;
            }
            let series_name = self
                .series_registry
                .get(series_id)
                .map(|s| s.short_name.as_str())
                .unwrap_or(series_id);

            for event in events {
                if let Some(next_session) = event.next_session() {
                    if let Some(start_time) = next_session.start_time {
                        let time_until = start_time.signed_duration_since(now);
                        if time_until > chrono::Duration::zero() && time_until <= threshold {
                            let hours = time_until.num_hours();
                            let minutes = time_until.num_minutes() % 60;
                            let time_str = if hours > 0 {
                                format!("{}h {}m", hours, minutes)
                            } else {
                                format!("{}m", minutes)
                            };
                            notifications.push(format!(
                                "★ {} {} in {}",
                                series_name, next_session.name, time_str
                            ));
                        }
                    }
                }
            }
        }
        notifications
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

        // ── Session Types ──
        items.push(FilterItem::Header("── Session Types ──"));
        for cat in [
            SessionCategory::Race,
            SessionCategory::Qualifying,
            SessionCategory::Practice,
            SessionCategory::Other,
        ] {
            items.push(FilterItem::Entry {
                label: cat.to_string(),
                option: FilterOption::SessionCategory(cat),
            });
        }

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
            FilterOption::SessionCategory(cat) => {
                self.active_filters.session_categories.contains(cat)
            }
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
            FilterOption::SessionCategory(cat) => {
                if !self.active_filters.session_categories.remove(cat) {
                    self.active_filters.session_categories.insert(*cat);
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
        let mut prev = if current == 0 {
            items.len() - 1
        } else {
            current - 1
        };
        while matches!(items[prev], FilterItem::Header(_)) {
            prev = if prev == 0 { items.len() - 1 } else { prev - 1 };
            if prev == current {
                break;
            }
        }
        self.filter_list_state.select(Some(prev));
    }

    /// Get all scheduled session occurrences across all series.
    pub fn all_scheduled_sessions(&self) -> Vec<ScheduledSession<'_>> {
        let mut all = Vec::new();
        let now = chrono::Utc::now();
        for events in self.events.values() {
            for event in events {
                if event.sessions.is_empty() {
                    let date = event.local_start_date();
                    all.push(ScheduledSession {
                        event,
                        session_name: "Race".to_string(),
                        session_type: SessionType::Race,
                        start_time: event.race_start_time(),
                        date,
                        status: event.status.clone(),
                    });
                } else {
                    for session in &event.sessions {
                        let date = if let Some(st) = session.start_time {
                            st.with_timezone(&chrono::Local).date_naive()
                        } else {
                            event.local_start_date()
                        };
                        let status = if let Some(st) = session.start_time {
                            let end = session.end_time.unwrap_or(st + chrono::Duration::hours(2));
                            if event.status == EventStatus::Completed {
                                EventStatus::Completed
                            } else if event.status == EventStatus::Live || (st <= now && now <= end)
                            {
                                EventStatus::Live
                            } else if now > end {
                                EventStatus::Completed
                            } else {
                                EventStatus::Upcoming
                            }
                        } else {
                            event.status.clone()
                        };
                        all.push(ScheduledSession {
                            event,
                            session_name: session.name.clone(),
                            session_type: session.session_type.clone(),
                            start_time: session.start_time,
                            date,
                            status,
                        });
                    }
                }
            }
        }
        all
    }

    /// Get a flat, sorted list of all scheduled sessions matching current filters.
    pub fn filtered_sessions(&self) -> Vec<ScheduledSession<'_>> {
        let mut sessions = self.all_scheduled_sessions();

        // 1. Hidden series check
        sessions.retain(|s| !self.config.hidden_series.contains(&s.event.series_id));

        // 2. Status filter (only applies in List view)
        if self.view_mode == ViewMode::List && !self.active_filters.statuses.is_empty() {
            sessions.retain(|s| self.active_filters.statuses.contains(&s.status));
        }

        // 3. Session category filter
        if !self.active_filters.session_categories.is_empty() {
            sessions.retain(|s| {
                self.active_filters
                    .session_categories
                    .contains(&s.session_type.category())
            });
        }

        // 4. Favorites filter
        if self.active_filters.favorites_only {
            sessions.retain(|s| self.config.favorites.contains(&s.event.series_id));
        }

        // 5. Series multi-select filter
        if !self.active_filters.series.is_empty() {
            sessions.retain(|s| self.active_filters.series.contains(&s.event.series_id));
        }

        // 6. Car style multi-select filter
        if !self.active_filters.car_styles.is_empty() {
            sessions.retain(|s| {
                self.series_registry
                    .get(&s.event.series_id)
                    .map_or(false, |ser| {
                        self.active_filters
                            .car_styles
                            .contains(ser.car_style.as_str())
                    })
            });
        }

        // 7. Region multi-select filter
        if !self.active_filters.regions.is_empty() {
            sessions.retain(|s| {
                self.series_registry
                    .get(&s.event.series_id)
                    .map_or(false, |ser| {
                        self.active_filters.regions.contains(&ser.region)
                    })
            });
        }

        // 8. Text search filter
        if let Some(ref query) = self.search_query {
            let q = query.to_lowercase();
            sessions.retain(|s| {
                s.event.event_name.to_lowercase().contains(&q)
                    || s.session_name.to_lowercase().contains(&q)
                    || s.event.circuit_name.to_lowercase().contains(&q)
                    || s.event.location.to_lowercase().contains(&q)
                    || s.event.country.to_lowercase().contains(&q)
                    || s.event.series_id.to_lowercase().contains(&q)
                    || self
                        .series_registry
                        .get(&s.event.series_id)
                        .map_or(false, |ser| {
                            ser.name.to_lowercase().contains(&q)
                                || ser.short_name.to_lowercase().contains(&q)
                        })
            });
        }

        // 9. Chronological sort by date, then known times before TBD, then start_time, then series & round
        sessions.sort_by_key(|s| {
            let is_undetermined = if s.start_time.is_some() { 0 } else { 1 };
            (
                s.date,
                is_undetermined,
                s.start_time,
                s.event.series_id.clone(),
                s.event.round,
                s.session_name.clone(),
            )
        });

        sessions
    }

    /// Get a flat, sorted list of all race events that match the current filters.
    pub fn filtered_events(&self) -> Vec<&RaceEvent> {
        let sessions = self.filtered_sessions();
        let mut seen = HashSet::new();
        let mut events = Vec::new();
        for s in sessions {
            let key = (&s.event.series_id, &s.event.event_name, s.event.round);
            if seen.insert(key) {
                events.push(s.event);
            }
        }
        events
    }

    /// Build the list of table items (blank day separators and scheduled session entries)
    pub fn list_table_items(&self) -> Vec<ListTableItem<'_>> {
        let sessions = self.filtered_sessions();
        let mut items = Vec::new();
        let mut current_date: Option<chrono::NaiveDate> = None;

        for session in sessions {
            let session_date = session.date;
            if let Some(prev_date) = current_date {
                if prev_date != session_date {
                    current_date = Some(session_date);
                    items.push(ListTableItem::Header(session_date));
                }
            } else {
                current_date = Some(session_date);
            }
            items.push(ListTableItem::Session(session));
        }

        items
    }

    /// Select the next session entry in the table (skips headers, wraps around).
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
        if matches!(items.get(next), Some(ListTableItem::Session(_))) {
            if next < current {
                *self.table_state.offset_mut() = 0;
            }
            self.table_state.select(Some(next));
        }
    }

    /// Select the previous session entry in the table (skips headers, wraps around).
    pub fn select_previous(&mut self) {
        let items = self.list_table_items();
        if items.is_empty() {
            self.table_state.select(None);
            return;
        }

        let current = self.table_state.selected().unwrap_or(0);
        let mut prev = if current == 0 {
            items.len() - 1
        } else {
            current - 1
        };
        while matches!(items.get(prev), Some(ListTableItem::Header(_))) {
            prev = if prev == 0 { items.len() - 1 } else { prev - 1 };
            if prev == current {
                break;
            }
        }
        if matches!(items.get(prev), Some(ListTableItem::Session(_))) {
            self.table_state.select(Some(prev));
        }
    }

    /// Get the currently selected scheduled session, if any.
    pub fn selected_session(&self) -> Option<ScheduledSession<'_>> {
        let items = self.list_table_items();
        if items.is_empty() {
            return None;
        }

        if let Some(index) = self.table_state.selected() {
            if let Some(ListTableItem::Session(session)) = items.get(index) {
                return Some(session.clone());
            }
        }

        // If selection is None or pointing to a header, fallback to first session
        for (_idx, item) in items.iter().enumerate() {
            if let ListTableItem::Session(session) = item {
                return Some(session.clone());
            }
        }

        None
    }

    /// Get the currently selected race event, if any.
    pub fn selected_event(&self) -> Option<&RaceEvent> {
        self.selected_session().map(|s| s.event)
    }

    /// Select the first session row in the table (skipping any initial header).
    pub fn select_first_event(&mut self) {
        let items = self.list_table_items();
        for (idx, item) in items.iter().enumerate() {
            if let ListTableItem::Session(_) = item {
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
        self.fetch_status
            .insert(series_id, FetchStatus::Loaded(count));

        // Adjust selection if it's out of bounds or on a header
        let total = self.list_table_items().len();
        if total > 0 {
            if self.table_state.selected().map_or(true, |i| i >= total) {
                self.select_first_event();
            } else if let Some(idx) = self.table_state.selected() {
                if matches!(
                    self.list_table_items().get(idx),
                    Some(ListTableItem::Header(_))
                ) {
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
        self.fetch_status.insert(
            series_id.to_string(),
            FetchStatus::CachedLoad(count, age_hours),
        );
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

    /// Get all scheduled sessions on a specific date, sorted chronologically with TBD at the bottom.
    pub fn events_on_date(&self, date: chrono::NaiveDate) -> Vec<ScheduledSession<'_>> {
        let sessions = self.filtered_sessions();
        sessions.into_iter().filter(|s| s.date == date).collect()
    }

    /// Get all scheduled sessions on the currently selected calendar day.
    pub fn events_on_selected_calendar_day(&self) -> Vec<ScheduledSession<'_>> {
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

    /// Select next session in day events popup
    pub fn day_events_select_next(&mut self) {
        let count = self.events_on_selected_calendar_day().len();
        if count == 0 {
            return;
        }
        let current = self.day_events_state.selected().unwrap_or(0);
        let next = (current + 1) % count;
        self.day_events_state.select(Some(next));
    }

    /// Select previous session in day events popup
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
        let sessions = self.events_on_selected_calendar_day();
        let idx = self.day_events_state.selected()?;
        sessions.get(idx).map(|s| s.event)
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
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );
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
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );
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
        assert_eq!(
            app.selected_event().map(|e| e.event_name.as_str()),
            Some("Race 1")
        );

        // Select next: skips separator at index 1, moves to index 2 (Race 2)
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(2));
        assert_eq!(
            app.selected_event().map(|e| e.event_name.as_str()),
            Some("Race 2")
        );

        // Select next: skips separator at index 3, moves to index 4 (Race 3)
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(4));
        assert_eq!(
            app.selected_event().map(|e| e.event_name.as_str()),
            Some("Race 3")
        );

        // Wrap around: wraps to index 0 (Race 1)
        app.select_next();
        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(
            app.selected_event().map(|e| e.event_name.as_str()),
            Some("Race 1")
        );

        // Prev navigation: wraps to index 4 (Race 3)
        app.select_previous();
        assert_eq!(app.table_state.selected(), Some(4));
        assert_eq!(
            app.selected_event().map(|e| e.event_name.as_str()),
            Some("Race 3")
        );
    }

    #[test]
    fn test_chronological_time_sorting_and_tbd_at_bottom() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );
        registry.insert(
            "nascar".to_string(),
            mock_series("nascar", "NASCAR", CarStyle::StockCar, "USA"),
        );
        registry.insert(
            "indycar".to_string(),
            mock_series("indycar", "IndyCar", CarStyle::OpenWheel, "USA"),
        );

        let mut app = App::new(registry, UserConfig::default());

        let mut ev_7pm = mock_event("nascar", "NASCAR 7PM", (2027, 5, 24));
        ev_7pm.sessions = vec![crate::data::models::Session {
            name: "Race".to_string(),
            session_type: crate::data::models::SessionType::Race,
            start_time: Some(chrono::Utc.with_ymd_and_hms(2027, 5, 24, 23, 0, 0).unwrap()), // 7 PM EDT
            end_time: None,
        }];

        let mut ev_12pm = mock_event("indycar", "IndyCar 12PM", (2027, 5, 24));
        ev_12pm.sessions = vec![crate::data::models::Session {
            name: "Race".to_string(),
            session_type: crate::data::models::SessionType::Race,
            start_time: Some(chrono::Utc.with_ymd_and_hms(2027, 5, 24, 16, 0, 0).unwrap()), // 12 PM EDT
            end_time: None,
        }];

        let mut ev_230pm = mock_event("f1", "F1 2:30PM", (2027, 5, 24));
        ev_230pm.sessions = vec![crate::data::models::Session {
            name: "Race".to_string(),
            session_type: crate::data::models::SessionType::Race,
            start_time: Some(
                chrono::Utc
                    .with_ymd_and_hms(2027, 5, 24, 18, 30, 0)
                    .unwrap(),
            ), // 2:30 PM EDT
            end_time: None,
        }];

        let ev_tbd = mock_event("nascar", "NASCAR TBD", (2027, 5, 24)); // No sessions (TBD)

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
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );
        registry.insert(
            "nascar".to_string(),
            mock_series("nascar", "NASCAR", CarStyle::StockCar, "USA"),
        );
        registry.insert(
            "wec".to_string(),
            mock_series("wec", "WEC", CarStyle::SportsCar, "International"),
        );

        let mut config = UserConfig::default();
        config.favorites = ["f1".to_string(), "nascar".to_string()]
            .into_iter()
            .collect();

        let mut app = App::new(registry, config);
        app.update_series_data(
            "f1".to_string(),
            vec![mock_event("f1", "Bahrain GP", (2026, 3, 1))],
        );
        app.update_series_data(
            "nascar".to_string(),
            vec![mock_event("nascar", "Daytona 500", (2026, 2, 15))],
        );
        app.update_series_data(
            "wec".to_string(),
            vec![mock_event("wec", "Qatar 1812km", (2026, 2, 28))],
        );

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
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );

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

    #[test]
    fn test_session_type_filtering() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );

        let mut app = App::new(registry, UserConfig::default());
        let mut event = mock_event("f1", "Bahrain GP", (2027, 3, 1));
        event.sessions = vec![
            crate::data::models::Session {
                name: "Practice 1".to_string(),
                session_type: crate::data::models::SessionType::Practice,
                start_time: Some(
                    chrono::Utc
                        .with_ymd_and_hms(2027, 2, 27, 11, 30, 0)
                        .unwrap(),
                ),
                end_time: None,
            },
            crate::data::models::Session {
                name: "Qualifying".to_string(),
                session_type: crate::data::models::SessionType::Qualifying,
                start_time: Some(chrono::Utc.with_ymd_and_hms(2027, 2, 28, 15, 0, 0).unwrap()),
                end_time: None,
            },
            crate::data::models::Session {
                name: "Race".to_string(),
                session_type: crate::data::models::SessionType::Race,
                start_time: Some(chrono::Utc.with_ymd_and_hms(2027, 3, 1, 15, 0, 0).unwrap()),
                end_time: None,
            },
        ];
        app.update_series_data("f1".to_string(), vec![event]);

        // Default: all 3 sessions are shown
        assert_eq!(app.filtered_sessions().len(), 3);

        // Toggle Practice off: only Qualifying and Race remain (2 sessions)
        app.toggle_filter_option(&FilterOption::SessionCategory(SessionCategory::Practice));
        assert_eq!(app.filtered_sessions().len(), 2);
        assert_eq!(app.filtered_sessions()[0].session_name, "Qualifying");
        assert_eq!(app.filtered_sessions()[1].session_name, "Race");

        // Toggle Qualifying off: only Race remains (1 session)
        app.toggle_filter_option(&FilterOption::SessionCategory(SessionCategory::Qualifying));
        assert_eq!(app.filtered_sessions().len(), 1);
        assert_eq!(app.filtered_sessions()[0].session_name, "Race");

        // Reset filters: all 3 sessions return
        app.reset_filters();
        assert_eq!(app.filtered_sessions().len(), 3);
    }

    #[test]
    fn test_notifications_for_favorites() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );

        let mut config = UserConfig::default();
        config.favorites.insert("f1".to_string());
        config.notification_threshold_hours = 2;

        let mut app = App::new(registry, config);

        // Session 1 hour in the future (within 2h threshold)
        let soon_time = chrono::Utc::now() + chrono::Duration::minutes(75);
        let mut event = mock_event("f1", "Monaco GP", (2026, 5, 24));
        event.sessions = vec![crate::data::models::Session {
            name: "Race".to_string(),
            session_type: crate::data::models::SessionType::Race,
            start_time: Some(soon_time),
            end_time: None,
        }];
        app.update_series_data("f1".to_string(), vec![event]);

        let notifs = app.get_notifications();
        assert_eq!(notifs.len(), 1);
        assert!(notifs[0].starts_with("★ F1 Race in 1h "));

        // If not a favorite, no notification
        app.config.favorites.clear();
        assert!(app.get_notifications().is_empty());
    }

    #[test]
    fn test_standings_cycle_series() {
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.standings.insert(
            "f1".to_string(),
            crate::data::standings::SeasonStandings {
                series_id: "f1".to_string(),
                season: 2026,
                drivers: vec![],
                constructors: vec![],
                fetched_at: chrono::Utc::now(),
            },
        );
        app.standings.insert(
            "indycar".to_string(),
            crate::data::standings::SeasonStandings {
                series_id: "indycar".to_string(),
                season: 2026,
                drivers: vec![],
                constructors: vec![],
                fetched_at: chrono::Utc::now(),
            },
        );
        app.standings.insert(
            "nascar_cup".to_string(),
            crate::data::standings::SeasonStandings {
                series_id: "nascar_cup".to_string(),
                season: 2026,
                drivers: vec![],
                constructors: vec![],
                fetched_at: chrono::Utc::now(),
            },
        );

        // Initially None, cycle +1 -> first series alphabetically ("f1") -> index 0 + 1 = "indycar" (index 1)
        app.standings_cycle_series(1);
        assert_eq!(app.standings_selected_series.as_deref(), Some("indycar"));

        app.standings_cycle_series(1);
        assert_eq!(app.standings_selected_series.as_deref(), Some("nascar_cup"));

        app.standings_cycle_series(1);
        assert_eq!(app.standings_selected_series.as_deref(), Some("f1"));

        app.standings_cycle_series(-1);
        assert_eq!(app.standings_selected_series.as_deref(), Some("nascar_cup"));
    }

    #[test]
    fn test_get_live_sessions() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            mock_series("f1", "Formula 1", CarStyle::OpenWheel, "International"),
        );

        let mut app = App::new(registry, UserConfig::default());
        let now = chrono::Utc::now();
        let mut event = mock_event("f1", "Monaco GP", (2026, 5, 24));
        event.status = EventStatus::Live;
        event.sessions = vec![crate::data::models::Session {
            name: "Grand Prix".to_string(),
            session_type: SessionType::Race,
            start_time: Some(now - chrono::Duration::minutes(30)),
            end_time: Some(now + chrono::Duration::minutes(90)),
        }];
        app.update_series_data("f1".to_string(), vec![event]);

        let live = app.get_live_sessions();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].0, "f1");
        assert_eq!(live[0].1, "F1");
        assert_eq!(live[0].2, "Grand Prix");
    }

    fn filtered_name(app: &App, idx: usize) -> String {
        app.filtered_events()[idx].event_name.clone()
    }
}
