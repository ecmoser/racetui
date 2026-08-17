use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Represents a type/category of racing (used for filtering).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CarStyle {
    OpenWheel,
    SportsCar,
    StockCar,
    Touring,
    Rally,
    Motorcycle,
}

impl CarStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            CarStyle::OpenWheel => "Open Wheel",
            CarStyle::SportsCar => "Sports Car",
            CarStyle::StockCar => "Stock Car",
            CarStyle::Touring => "Touring",
            CarStyle::Rally => "Rally",
            CarStyle::Motorcycle => "Motorcycle",
        }
    }
}

impl std::fmt::Display for CarStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a racing series (e.g., "Formula 1", "NASCAR Cup").
/// This is metadata about the series itself, not individual events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    /// Unique identifier, e.g. "f1", "nascar_cup"
    pub id: String,
    /// Display name, e.g. "Formula 1"
    pub name: String,
    /// Short display name for compact views, e.g. "F1"
    pub short_name: String,
    /// Category of racing
    pub car_style: CarStyle,
    /// RGB color for this series (r, g, b) — values 0-255
    pub color: (u8, u8, u8),
    /// Country/region of origin (for filtering), e.g. "International", "USA", "Japan"
    pub region: String,
    /// URL to the official calendar page (used by scrapers)
    pub calendar_url: String,
    /// Whether this series requires a headless browser to scrape
    pub requires_js: bool,
}

/// The type of a session within a race weekend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    Practice,
    Qualifying,
    SprintQualifying,
    Sprint,
    Race,
    Warmup,
    /// For WRC stages, or any other session type
    Other(String),
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::Practice => write!(f, "Practice"),
            SessionType::Qualifying => write!(f, "Qualifying"),
            SessionType::SprintQualifying => write!(f, "Sprint Qualifying"),
            SessionType::Sprint => write!(f, "Sprint"),
            SessionType::Race => write!(f, "Race"),
            SessionType::Warmup => write!(f, "Warmup"),
            SessionType::Other(name) => write!(f, "{}", name),
        }
    }
}

/// A single session (e.g., "FP1", "Qualifying", "Race") within a race event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Name of the session, e.g. "Free Practice 1", "Race"
    pub name: String,
    /// Type of session
    pub session_type: SessionType,
    /// Start time in UTC (will be converted to local time for display)
    pub start_time: Option<DateTime<Utc>>,
    /// End time in UTC (if known)
    pub end_time: Option<DateTime<Utc>>,
}

/// Whether a livestream is free or paid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamAccess {
    Free,
    Paid,
    /// Some content may be geo-restricted or conditionally free
    Mixed,
    Unknown,
}

impl std::fmt::Display for StreamAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamAccess::Free => write!(f, "Free"),
            StreamAccess::Paid => write!(f, "Paid"),
            StreamAccess::Mixed => write!(f, "Mixed"),
            StreamAccess::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A link to watch a session live or on replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamLink {
    /// Name of the platform, e.g. "F1TV", "YouTube", "Peacock"
    pub platform: String,
    /// URL to open
    pub url: String,
    /// Whether it's free or paid
    pub access: StreamAccess,
}

/// The current status of a race event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventStatus {
    /// Event is in the future
    Upcoming,
    /// Event is happening right now (at least one session is live)
    Live,
    /// Event has finished
    Completed,
    /// Event was cancelled
    Cancelled,
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStatus::Upcoming => write!(f, "Upcoming"),
            EventStatus::Live => write!(f, "LIVE"),
            EventStatus::Completed => write!(f, "Completed"),
            EventStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// A single race event/weekend (e.g., "2026 Monaco Grand Prix").
/// This is the primary unit of data displayed in the calendar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceEvent {
    /// The series this event belongs to (series ID, e.g. "f1")
    pub series_id: String,
    /// Official event name, e.g. "Monaco Grand Prix"
    pub event_name: String,
    /// Circuit/track name, e.g. "Circuit de Monaco"
    pub circuit_name: String,
    /// City or location, e.g. "Monte Carlo"
    pub location: String,
    /// Country, e.g. "Monaco"
    pub country: String,
    /// The first day of the event (date only, for sorting/grouping)
    pub start_date: chrono::NaiveDate,
    /// The last day of the event
    pub end_date: chrono::NaiveDate,
    /// Round number within the series season (if applicable)
    pub round: Option<u32>,
    /// Individual sessions within this event
    pub sessions: Vec<Session>,
    /// Links to watch this event
    pub stream_links: Vec<StreamLink>,
    /// Current status
    pub status: EventStatus,
}

impl RaceEvent {
    /// Get the next upcoming session start time.
    /// Returns None if there are no sessions with known start times,
    /// or all sessions are in the past.
    pub fn next_session_time(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        self.sessions
            .iter()
            .filter_map(|s| s.start_time)
            .filter(|t| *t > now)
            .min()
    }

    /// Get the next upcoming session.
    pub fn next_session(&self) -> Option<&Session> {
        let now = Utc::now();
        self.sessions
            .iter()
            .filter(|s| s.start_time.map_or(false, |t| t > now))
            .min_by_key(|s| s.start_time)
    }

    /// Get the primary race start time (or next session start time if race time not known).
    pub fn race_start_time(&self) -> Option<DateTime<Utc>> {
        self.sessions
            .iter()
            .find(|s| s.session_type == SessionType::Race)
            .and_then(|s| s.start_time)
            .or_else(|| self.next_session_time())
            .or_else(|| self.sessions.iter().filter_map(|s| s.start_time).next())
    }

    /// Format race start time in local time (e.g. " 3:00 PM" or "TBD").
    pub fn format_local_time(&self) -> String {
        match self.race_start_time() {
            Some(t) => t.with_timezone(&chrono::Local).format("%l:%M %p").to_string(),
            None => "TBD".to_string(),
        }
    }

    /// Format race start time compact in local time (e.g. "3:00P" or "").
    pub fn format_local_time_compact(&self) -> String {
        match self.race_start_time() {
            Some(t) => {
                let local = t.with_timezone(&chrono::Local);
                let hour_min = local.format("%l:%M").to_string().trim().to_string();
                let am_pm = if local.format("%p").to_string().to_uppercase() == "AM" {
                    "A"
                } else {
                    "P"
                };
                format!("{}{}", hour_min, am_pm)
            }
            None => String::new(),
        }
    }

    /// Get the event's primary date in local timezone.
    pub fn local_start_date(&self) -> NaiveDate {
        if let Some(dt) = self.race_start_time() {
            dt.with_timezone(&chrono::Local).date_naive()
        } else {
            self.start_date
        }
    }
}

/// The status of a data fetch for a series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchStatus {
    /// Not yet started
    Pending,
    /// Currently fetching
    Fetching,
    /// Successfully loaded (number of events)
    Loaded(usize),
    /// Loaded from cache (number of events, cache age in hours)
    CachedLoad(usize, u64),
    /// Failed with an error message
    Error(String),
}
