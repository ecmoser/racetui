# CALENDAR_PLAN.md — RaceTUI Implementation Plan

> **Purpose**: This document is a step-by-step implementation plan for the RaceTUI calendar feature.
> Each step is designed to be a single git commit. Steps must be completed **in order**.
> Every design decision, file path, data structure, and implementation detail is specified so that
> an AI coding agent can implement each step without needing to make any design decisions.

---

## Technology Stack (Do Not Change)

| Component | Choice | Version (verify on crates.io before starting) |
|---|---|---|
| Language | Rust (2021 edition) | stable toolchain |
| TUI framework | `ratatui` | latest (≥0.30) |
| Terminal backend | `crossterm` | latest (≥0.29), features: `["event-stream"]` |
| Async runtime | `tokio` | 1.x, features: `["rt-multi-thread", "macros", "time", "sync"]` |
| HTTP client | `reqwest` | latest (≥0.12), features: `["json", "cookies"]` |
| HTML scraping | `scraper` | latest (≥0.27) |
| Headless browser | `chromiumoxide` | latest, features: `["tokio-runtime"]` |
| Serialization | `serde` + `serde_json` | 1.x, serde features: `["derive"]` |
| Config format | `toml` | latest (≥0.8) |
| Date/time | `chrono` | 0.4, features: `["serde"]` |
| Timezone | `chrono-tz` | latest (≥0.10), features: `["serde"]` |
| XDG paths | `dirs` | latest (≥6.x) |
| CLI args | `clap` | 4.x, features: `["derive"]` |
| Error handling | `anyhow` + `color-eyre` | 1.x / 0.6 |
| Logging | `tracing` + `tracing-subscriber` | latest |
| Futures | `futures` | latest (≥0.3) — needed for `chromiumoxide` handler stream |

---

## Directory Structure (Final Target)

```
racetui/
├── Cargo.toml
├── VISION.md
├── CALENDAR_PLAN.md
├── data/
│   └── series.toml            # Series metadata & scraper config
├── src/
│   ├── main.rs                # Entry point, terminal setup, main event loop
│   ├── app.rs                 # App struct (all application state)
│   ├── event.rs               # AppEvent enum, event channel setup
│   ├── ui/
│   │   ├── mod.rs             # UI module root, main draw dispatcher
│   │   ├── list_view.rs       # List view renderer
│   │   ├── calendar_view.rs   # Calendar grid view renderer
│   │   ├── detail_view.rs     # Event detail popup/panel
│   │   ├── status_bar.rs      # Bottom status bar (keybinds, notifications)
│   │   ├── filter_panel.rs    # Filter/search overlay
│   │   └── help.rs            # Help popup (keybindings reference)
│   ├── config/
│   │   ├── mod.rs             # Config module root
│   │   └── settings.rs        # UserConfig struct, load/save logic
│   ├── data/
│   │   ├── mod.rs             # Data module root
│   │   ├── models.rs          # Core data models (RaceEvent, Series, Session, etc.)
│   │   ├── cache.rs           # Cache read/write logic
│   │   └── series_registry.rs # Load series definitions from series.toml
│   ├── scraper/
│   │   ├── mod.rs             # Scraper module root, SeriesScraper trait
│   │   ├── fetcher.rs         # HTTP fetcher (reqwest) + headless browser fallback
│   │   ├── f1.rs              # F1 scraper (Jolpica API)
│   │   ├── f2.rs              # F2 scraper
│   │   ├── f3.rs              # F3 scraper
│   │   ├── formula_e.rs       # Formula E scraper
│   │   ├── indycar.rs         # IndyCar scraper
│   │   ├── nascar.rs          # NASCAR scraper (all 3 series)
│   │   ├── imsa.rs            # IMSA scraper
│   │   ├── wec.rs             # WEC scraper
│   │   ├── motogp.rs          # MotoGP scraper
│   │   ├── wrc.rs             # WRC scraper
│   │   ├── btcc.rs            # BTCC scraper
│   │   ├── super_formula.rs   # Super Formula scraper
│   │   ├── super_gt.rs        # Super GT scraper
│   │   └── dtm.rs             # DTM scraper
│   └── action.rs              # Open-URL / livestream launch logic
```

---

## Design Decisions Reference

These decisions have been made. Do not deviate from them.

- **Views**: Two views — List View (default) and Calendar Grid View — toggled with a keybind.
- **Navigation**: Both vim-style (`h/j/k/l`, `/` search) AND arrow keys + Enter. Support both simultaneously.
- **Config location**: `~/.config/racetui/config.toml` (XDG). Use the `dirs` crate to resolve.
- **Cache location**: `~/.local/share/racetui/cache/` (XDG data dir). Cache as JSON files, one per series.
- **Cache TTL**: Default 24 hours, configurable in `config.toml`. Manual refresh keybind (`r` or `R`).
- **Timezone**: Auto-detect local timezone using `chrono::Local`. Display all times in user's local time.
- **Colors**: Each racing series gets a unique color. Colors are defined in `data/series.toml`.
- **Livestream links**: Opened via a configurable command. Default: system browser (xdg-open / open). Configurable in `config.toml` to use e.g. `mpv` for direct stream URLs.
- **Loading UX**: Show series-by-series fetch progress, but do NOT block browsing of already-fetched data. User can browse cached/loaded data while other series are still being fetched.
- **Scraper architecture**: Trait-based (`SeriesScraper`) with one Rust module per series. Common metadata (URLs, selectors) is configurable via `data/series.toml` so that minor URL changes don't require recompilation. The trait impls use these config values.
- **Notification system**: Favorited series with upcoming events (within configurable time threshold, default 2 hours) are highlighted visually in the list and shown in the status bar.

---

## Steps

---

### [x] Step 1: Initialize Rust Project with Dependencies

**What to do**: Create a new Rust binary project with all required dependencies.

**Commands to run**:
```bash
cd /home/ecmoser/github/racetui
cargo init --name racetui .
```

**Then replace the generated `Cargo.toml` with this exact content**:

```toml
[package]
name = "racetui"
version = "0.1.0"
edition = "2021"
description = "A TUI for viewing racing series calendars from around the world"
license = "MIT"

[dependencies]
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
reqwest = { version = "0.12", features = ["json", "cookies"] }
scraper = "0.27"
chromiumoxide = { version = "0.7", features = ["tokio-runtime"], optional = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = { version = "0.10", features = ["serde"] }
dirs = "6"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = "0.3"
futures = "0.3"

[features]
default = []
headless-browser = ["chromiumoxide"]
```

> **Note on `chromiumoxide`**: It's behind a feature flag `headless-browser` because it has heavy dependencies (including linking to system libraries). Most series won't need it. This way, users who don't need JS-rendered scraping can build without it.
>
> **Important**: After writing `Cargo.toml`, verify the versions compile by running `cargo check`. If any version doesn't exist, go to `https://crates.io/crates/<name>` and use the latest version listed there. The versions above are approximate — crates.io is the source of truth.

**Replace `src/main.rs` with this minimal skeleton**:

```rust
fn main() {
    println!("RaceTUI - coming soon!");
}
```

**Create empty `data/` directory**:
```bash
mkdir -p data
```

**Verify it compiles**:
```bash
cargo check
```

**Commit message**: `feat: initialize Rust project with all dependencies`

---

### [x] Step 2: Core Data Models

**What to do**: Define the core data structures that represent racing series, events, and sessions. These structs will be used throughout the entire application.

**Create `src/data/mod.rs`**:
```rust
pub mod models;
```

**Create `src/data/models.rs`** with these exact structs:

```rust
use chrono::{DateTime, Utc};
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

impl std::fmt::Display for CarStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CarStyle::OpenWheel => write!(f, "Open Wheel"),
            CarStyle::SportsCar => write!(f, "Sports Car"),
            CarStyle::StockCar => write!(f, "Stock Car"),
            CarStyle::Touring => write!(f, "Touring"),
            CarStyle::Rally => write!(f, "Rally"),
            CarStyle::Motorcycle => write!(f, "Motorcycle"),
        }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamLink {
    /// Name of the platform, e.g. "F1TV", "YouTube", "Peacock"
    pub platform: String,
    /// URL to open
    pub url: String,
    /// Whether it's free or paid
    pub access: StreamAccess,
}

/// The current status of a race event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

**Update `src/main.rs`** to declare the module:
```rust
mod data;

fn main() {
    println!("RaceTUI - coming soon!");
}
```

**Verify**: `cargo check`

**Commit message**: `feat: define core data models (Series, RaceEvent, Session, etc.)`

---

### [x] Step 3: Series Registry and `series.toml` Configuration

**What to do**: Create the `data/series.toml` file containing metadata for all 16 racing series, and the Rust code to load it.

**Create `data/series.toml`** with this exact content:

```toml
# Series metadata and scraper configuration.
# Colors are RGB values (0-255).
# `requires_js` indicates whether the calendar page needs a headless browser.
# Most series have JSON APIs or SSR HTML, so requires_js is false for almost all.

[[series]]
id = "f1"
name = "Formula 1"
short_name = "F1"
car_style = "OpenWheel"
region = "International"
color = [255, 24, 1]
calendar_url = "https://api.jolpi.ca/ergast/f1/current.json"
requires_js = false

[[series]]
id = "f2"
name = "Formula 2"
short_name = "F2"
car_style = "OpenWheel"
region = "International"
color = [0, 129, 210]
calendar_url = "https://api.formula1.com/v1/core-editorial-races/f2/listing"
requires_js = false

[[series]]
id = "f3"
name = "Formula 3"
short_name = "F3"
car_style = "OpenWheel"
region = "International"
color = [228, 0, 43]
calendar_url = "https://api.formula1.com/v1/core-editorial-races/f3/listing"
requires_js = false

[[series]]
id = "formula_e"
name = "Formula E"
short_name = "FE"
car_style = "OpenWheel"
region = "International"
color = [0, 174, 239]
calendar_url = "https://api.formula-e.pulselive.com/formula-e/v1/"
requires_js = false

[[series]]
id = "indycar"
name = "IndyCar Series"
short_name = "IndyCar"
car_style = "OpenWheel"
region = "USA"
color = [0, 0, 128]
calendar_url = "https://www.indycar.com/Schedule"
requires_js = false

[[series]]
id = "nascar_cup"
name = "NASCAR Cup Series"
short_name = "Cup"
car_style = "StockCar"
region = "USA"
color = [255, 200, 0]
calendar_url = "https://cf.nascar.com/cacher/2026/1/race_list_basic.json"
requires_js = false

[[series]]
id = "nascar_xfinity"
name = "NASCAR Xfinity Series"
short_name = "Xfinity"
car_style = "StockCar"
region = "USA"
color = [0, 100, 200]
calendar_url = "https://cf.nascar.com/cacher/2026/2/race_list_basic.json"
requires_js = false

[[series]]
id = "nascar_trucks"
name = "NASCAR Craftsman Truck Series"
short_name = "Trucks"
car_style = "StockCar"
region = "USA"
color = [200, 50, 50]
calendar_url = "https://cf.nascar.com/cacher/2026/3/race_list_basic.json"
requires_js = false

[[series]]
id = "imsa"
name = "IMSA WeatherTech SportsCar Championship"
short_name = "IMSA"
car_style = "SportsCar"
region = "USA"
color = [0, 150, 68]
calendar_url = "https://www.imsa.com/weathertech/schedule/"
requires_js = false

[[series]]
id = "wec"
name = "FIA World Endurance Championship"
short_name = "WEC"
car_style = "SportsCar"
region = "International"
color = [0, 85, 150]
calendar_url = "https://www.fiawec.com/"
requires_js = false

[[series]]
id = "motogp"
name = "MotoGP"
short_name = "MotoGP"
car_style = "Motorcycle"
region = "International"
color = [190, 0, 50]
calendar_url = "https://api.motogp.pulselive.com/motogp/v1/results/seasons"
requires_js = false

[[series]]
id = "wrc"
name = "World Rally Championship"
short_name = "WRC"
car_style = "Rally"
region = "International"
color = [37, 100, 200]
calendar_url = "https://api.wrc.com/contel-page/83388/calendar/active-season/"
requires_js = false

[[series]]
id = "btcc"
name = "British Touring Car Championship"
short_name = "BTCC"
car_style = "Touring"
region = "UK"
color = [0, 60, 120]
calendar_url = "https://www.btcc.net/calendar/"
requires_js = false

[[series]]
id = "super_formula"
name = "Super Formula"
short_name = "SF"
car_style = "OpenWheel"
region = "Japan"
color = [230, 0, 126]
calendar_url = "https://superformula.net/sf3/race_taxonomy/2026/"
requires_js = false

[[series]]
id = "super_gt"
name = "Super GT"
short_name = "SGT"
car_style = "SportsCar"
region = "Japan"
color = [200, 160, 0]
calendar_url = "https://supergt.net/en/"
requires_js = false

[[series]]
id = "dtm"
name = "DTM"
short_name = "DTM"
car_style = "Touring"
region = "Europe"
color = [130, 0, 200]
calendar_url = "https://www.dtm.com/en/calendar"
requires_js = true

# ── Endurance / Sports Car ──

[[series]]
id = "elms"
name = "European Le Mans Series"
short_name = "ELMS"
car_style = "SportsCar"
region = "Europe"
color = [0, 114, 206]
calendar_url = "https://www.europeanlemansseries.com/calendar"
requires_js = false

[[series]]
id = "aslms"
name = "Asian Le Mans Series"
short_name = "AsLMS"
car_style = "SportsCar"
region = "Asia"
color = [230, 80, 20]
calendar_url = "https://www.asianlemansseries.com/calendar"
requires_js = false

[[series]]
id = "nls"
name = "Nürburgring Langstrecken-Serie"
short_name = "NLS"
car_style = "SportsCar"
region = "Europe"
color = [20, 128, 60]
calendar_url = "https://www.nuerburgring-langstrecken-serie.de/en/calendar-2026/"
requires_js = false

[[series]]
id = "gtwc_eu"
name = "GT World Challenge Europe"
short_name = "GTWC Eu"
car_style = "SportsCar"
region = "Europe"
color = [235, 90, 0]
calendar_url = "https://www.gt-world-challenge-europe.com/calendar"
requires_js = false

[[series]]
id = "gtwc_am"
name = "GT World Challenge America"
short_name = "GTWC Am"
car_style = "SportsCar"
region = "USA"
color = [180, 30, 50]
calendar_url = "https://www.gt-world-challenge-america.com/calendar"
requires_js = false

[[series]]
id = "igtc"
name = "Intercontinental GT Challenge"
short_name = "IGTC"
car_style = "SportsCar"
region = "International"
color = [212, 175, 55]
calendar_url = "https://www.intercontinentalgtchallenge.com/calendar"
requires_js = false

# ── Touring Car ──

[[series]]
id = "supercars"
name = "Repco Supercars Championship"
short_name = "Supercars"
car_style = "Touring"
region = "Australia"
color = [230, 50, 20]
calendar_url = "https://www.supercars.com/calendar"
requires_js = false

[[series]]
id = "tcr_world"
name = "Kumho FIA TCR World Tour"
short_name = "TCR World"
car_style = "Touring"
region = "International"
color = [220, 30, 30]
calendar_url = "https://www.fiatcrworldtour.com/calendar/"
requires_js = false

# ── Open Wheel ──

[[series]]
id = "indy_nxt"
name = "INDY NXT by Firestone"
short_name = "Indy NXT"
car_style = "OpenWheel"
region = "USA"
color = [210, 25, 30]
calendar_url = "https://www.indynxt.com/schedule"
requires_js = false

[[series]]
id = "f1_academy"
name = "F1 Academy"
short_name = "F1A"
car_style = "OpenWheel"
region = "International"
color = [140, 40, 200]
calendar_url = "https://api.formula1.com/v1/core-editorial-races/f1academy/listing"
requires_js = false

# ── Motorcycle ──

[[series]]
id = "moto2"
name = "Moto2 World Championship"
short_name = "Moto2"
car_style = "Motorcycle"
region = "International"
color = [70, 130, 200]
calendar_url = "https://api.motogp.pulselive.com/motogp/v1/results/seasons"
requires_js = false

[[series]]
id = "moto3"
name = "Moto3 World Championship"
short_name = "Moto3"
car_style = "Motorcycle"
region = "International"
color = [240, 100, 0]
calendar_url = "https://api.motogp.pulselive.com/motogp/v1/results/seasons"
requires_js = false

[[series]]
id = "worldsbk"
name = "FIM Superbike World Championship"
short_name = "WorldSBK"
car_style = "Motorcycle"
region = "International"
color = [210, 10, 10]
calendar_url = "https://www.worldsbk.com/en/calendar"
requires_js = false

[[series]]
id = "bsb"
name = "British Superbike Championship"
short_name = "BSB"
car_style = "Motorcycle"
region = "UK"
color = [170, 15, 40]
calendar_url = "https://www.britishsuperbike.com/calendar"
requires_js = false

# ── Rally ──

[[series]]
id = "wrc2"
name = "WRC2"
short_name = "WRC2"
car_style = "Rally"
region = "International"
color = [100, 160, 220]
calendar_url = "https://api.wrc.com/contel-page/83388/calendar/active-season/"
requires_js = false

[[series]]
id = "dakar"
name = "Dakar Rally"
short_name = "Dakar"
car_style = "Rally"
region = "International"
color = [215, 115, 30]
calendar_url = "https://www.dakar.com/en/calendar"
requires_js = false

[[series]]
id = "erc"
name = "FIA European Rally Championship"
short_name = "ERC"
car_style = "Rally"
region = "Europe"
color = [255, 185, 0]
calendar_url = "https://www.fiaerc.com/calendar"
requires_js = false

# ── Other ──

[[series]]
id = "porsche_supercup"
name = "Porsche Mobil 1 Supercup"
short_name = "Supercup"
car_style = "SportsCar"
region = "Europe"
color = [180, 0, 30]
calendar_url = "https://motorsports.porsche.com/international/en/category/mobil1supercup"
requires_js = false

[[series]]
id = "arca"
name = "ARCA Menards Series"
short_name = "ARCA"
car_style = "StockCar"
region = "USA"
color = [235, 235, 0]
calendar_url = "https://cf.nascar.com/cacher/2026/4/race_list_basic.json"
requires_js = false

[[series]]
id = "extreme_e"
name = "Extreme E / Extreme H"
short_name = "XE/XH"
car_style = "Rally"
region = "International"
color = [50, 220, 180]
calendar_url = "https://www.extreme-e.com/calendar"
requires_js = false
```

**Create `src/data/series_registry.rs`**:

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use super::models::{CarStyle, Series};

/// Intermediate deserialization struct matching the TOML format.
/// This is needed because TOML arrays of tables deserialize differently
/// than the final `Series` struct.
#[derive(Debug, Deserialize)]
struct SeriesToml {
    series: Vec<SeriesTomlEntry>,
}

#[derive(Debug, Deserialize)]
struct SeriesTomlEntry {
    id: String,
    name: String,
    short_name: String,
    car_style: String,
    region: String,
    color: [u8; 3],
    calendar_url: String,
    requires_js: bool,
}

impl SeriesTomlEntry {
    fn into_series(self) -> Series {
        let car_style = match self.car_style.as_str() {
            "OpenWheel" => CarStyle::OpenWheel,
            "SportsCar" => CarStyle::SportsCar,
            "StockCar" => CarStyle::StockCar,
            "Touring" => CarStyle::Touring,
            "Rally" => CarStyle::Rally,
            "Motorcycle" => CarStyle::Motorcycle,
            other => panic!("Unknown car style in series.toml: '{}'", other),
        };
        Series {
            id: self.id,
            name: self.name,
            short_name: self.short_name,
            car_style,
            color: (self.color[0], self.color[1], self.color[2]),
            region: self.region,
            calendar_url: self.calendar_url,
            requires_js: self.requires_js,
        }
    }
}

/// Load all series definitions from a `series.toml` file.
/// Returns a HashMap keyed by series ID for fast lookup.
pub fn load_series_registry(path: &Path) -> Result<HashMap<String, Series>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read series registry at {}", path.display()))?;
    let parsed: SeriesToml = toml::from_str(&content)
        .with_context(|| "Failed to parse series.toml")?;
    let mut map = HashMap::new();
    for entry in parsed.series {
        let id = entry.id.clone();
        map.insert(id, entry.into_series());
    }
    Ok(map)
}
```

**Update `src/data/mod.rs`**:
```rust
pub mod models;
pub mod series_registry;
```

**Update `src/main.rs`** to smoke-test the registry:
```rust
mod data;

use std::path::Path;

fn main() {
    let registry = data::series_registry::load_series_registry(Path::new("data/series.toml"))
        .expect("Failed to load series registry");
    println!("Loaded {} series:", registry.len());
    for (id, series) in &registry {
        println!(
            "  {} ({}) - {} - color: ({},{},{})",
            series.name, id, series.car_style, series.color.0, series.color.1, series.color.2
        );
    }
}
```

**Verify**: `cargo run` — should print all 16 series.

**Commit message**: `feat: add series registry with 16 racing series definitions`

---

### [x] Step 4: User Configuration System

**What to do**: Create the config module that loads/saves user preferences from `~/.config/racetui/config.toml`.

**Create `src/config/mod.rs`**:
```rust
pub mod settings;

pub use settings::UserConfig;
```

**Create `src/config/settings.rs`**:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// User configuration, persisted to ~/.config/racetui/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Series IDs that the user has favorited (e.g. ["f1", "wec"])
    #[serde(default)]
    pub favorites: HashSet<String>,

    /// Cache time-to-live in hours. Default: 24.
    #[serde(default = "default_cache_ttl_hours")]
    pub cache_ttl_hours: u64,

    /// How many hours before an event to highlight it as "upcoming soon".
    /// Default: 2 hours.
    #[serde(default = "default_notification_threshold_hours")]
    pub notification_threshold_hours: u64,

    /// Command to open livestream URLs.
    /// Default: "xdg-open" on Linux, "open" on macOS, "start" on Windows.
    /// Can be set to e.g. "mpv" for direct streams.
    #[serde(default = "default_open_command")]
    pub open_command: String,

    /// Default view mode: "list" or "calendar".
    #[serde(default = "default_view_mode")]
    pub default_view: String,

    /// Hidden series IDs (series the user has chosen to hide).
    #[serde(default)]
    pub hidden_series: HashSet<String>,
}

fn default_cache_ttl_hours() -> u64 {
    24
}

fn default_notification_threshold_hours() -> u64 {
    2
}

fn default_open_command() -> String {
    if cfg!(target_os = "macos") {
        "open".to_string()
    } else if cfg!(target_os = "windows") {
        "start".to_string()
    } else {
        "xdg-open".to_string()
    }
}

fn default_view_mode() -> String {
    "list".to_string()
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            favorites: HashSet::new(),
            cache_ttl_hours: default_cache_ttl_hours(),
            notification_threshold_hours: default_notification_threshold_hours(),
            open_command: default_open_command(),
            default_view: default_view_mode(),
            hidden_series: HashSet::new(),
        }
    }
}

impl UserConfig {
    /// Get the config file path: ~/.config/racetui/config.toml
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("racetui");
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from disk. If the file doesn't exist, return defaults.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| "Failed to parse config.toml")?;
        Ok(config)
    }

    /// Save config to disk. Creates parent directories if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }

    /// Toggle a series as favorite. Returns true if it's now a favorite.
    pub fn toggle_favorite(&mut self, series_id: &str) -> bool {
        if self.favorites.contains(series_id) {
            self.favorites.remove(series_id);
            false
        } else {
            self.favorites.insert(series_id.to_string());
            true
        }
    }

    /// Toggle a series as hidden. Returns true if it's now hidden.
    pub fn toggle_hidden(&mut self, series_id: &str) -> bool {
        if self.hidden_series.contains(series_id) {
            self.hidden_series.remove(series_id);
            false
        } else {
            self.hidden_series.insert(series_id.to_string());
            true
        }
    }
}
```

**Update `src/main.rs`** to declare the module:
```rust
mod config;
mod data;

use std::path::Path;

fn main() {
    // Test config
    let config = config::UserConfig::load().expect("Failed to load config");
    println!("Config loaded. Favorites: {:?}", config.favorites);

    // Test series registry
    let registry = data::series_registry::load_series_registry(Path::new("data/series.toml"))
        .expect("Failed to load series registry");
    println!("Loaded {} series", registry.len());
}
```

**Verify**: `cargo run`

**Commit message**: `feat: add user configuration system with XDG config support`

---

### [x] Step 5: Cache System

**What to do**: Create the cache module that stores/loads scraped race event data as JSON files in `~/.local/share/racetui/cache/`.

**Create `src/data/cache.rs`**:

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::models::RaceEvent;

/// Metadata stored alongside cached events for a series.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// When this data was fetched
    fetched_at: DateTime<Utc>,
    /// The race events
    events: Vec<RaceEvent>,
}

/// Get the cache directory: ~/.local/share/racetui/cache/
fn cache_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("racetui")
        .join("cache");
    Ok(data_dir)
}

/// Get the cache file path for a specific series.
/// e.g., ~/.local/share/racetui/cache/f1.json
fn cache_file_path(series_id: &str) -> Result<PathBuf> {
    Ok(cache_dir()?.join(format!("{}.json", series_id)))
}

/// Read cached events for a series.
/// Returns None if no cache file exists.
/// Returns Some((events, fetched_at)) if cache exists.
pub fn read_cache(series_id: &str) -> Result<Option<(Vec<RaceEvent>, DateTime<Utc>)>> {
    let path = cache_file_path(series_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read cache file {}", path.display()))?;
    let entry: CacheEntry = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse cache file {}", path.display()))?;
    Ok(Some((entry.events, entry.fetched_at)))
}

/// Write events to the cache for a series.
pub fn write_cache(series_id: &str, events: &[RaceEvent]) -> Result<()> {
    let path = cache_file_path(series_id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory {}", parent.display()))?;
    }
    let entry = CacheEntry {
        fetched_at: Utc::now(),
        events: events.to_vec(),
    };
    let content = serde_json::to_string_pretty(&entry)
        .context("Failed to serialize cache")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write cache to {}", path.display()))?;
    Ok(())
}

/// Check if the cache for a series is still valid (not expired).
/// Returns true if the cache is fresh (within TTL), false otherwise.
pub fn is_cache_fresh(series_id: &str, ttl_hours: u64) -> Result<bool> {
    match read_cache(series_id)? {
        None => Ok(false),
        Some((_, fetched_at)) => {
            let age = Utc::now().signed_duration_since(fetched_at);
            Ok(age.num_hours() < ttl_hours as i64)
        }
    }
}

/// Get the age of the cache in hours. Returns None if no cache exists.
pub fn cache_age_hours(series_id: &str) -> Result<Option<u64>> {
    match read_cache(series_id)? {
        None => Ok(None),
        Some((_, fetched_at)) => {
            let age = Utc::now().signed_duration_since(fetched_at);
            Ok(Some(age.num_hours() as u64))
        }
    }
}

/// Clear the cache for a specific series.
pub fn clear_cache(series_id: &str) -> Result<()> {
    let path = cache_file_path(series_id)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove cache file {}", path.display()))?;
    }
    Ok(())
}

/// Clear all cached data.
pub fn clear_all_cache() -> Result<()> {
    let dir = cache_dir()?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("Failed to remove cache directory {}", dir.display()))?;
    }
    Ok(())
}
```

**Update `src/data/mod.rs`**:
```rust
pub mod cache;
pub mod models;
pub mod series_registry;
```

**Verify**: `cargo check`

**Commit message**: `feat: add JSON-based cache system for scraped data`

---

### [x] Step 6: Event System and App State

**What to do**: Create the `AppEvent` enum for the unified event channel, and the `App` struct that holds all application state.

**Create `src/event.rs`**:

```rust
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
}
```

**Create `src/app.rs`**:

```rust
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
```

**Update `src/main.rs`**:
```rust
mod app;
mod config;
mod data;
mod event;

fn main() {
    println!("RaceTUI - modules defined, ready for TUI setup");
}
```

**Verify**: `cargo check`

**Commit message**: `feat: add AppEvent enum and App state struct`

---

### [x] Step 7: Terminal Setup and Basic Event Loop

**What to do**: Set up the ratatui terminal, crossterm backend, and the main event loop with the async tokio runtime. At this step, the TUI will show a blank screen with "RaceTUI" title and respond to `q` to quit.

**Fully replace `src/main.rs`** with:

```rust
mod app;
mod config;
mod data;
mod event;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self as crossterm_event, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use event::AppEvent;
use ratatui::prelude::*;
use std::io::stdout;
use std::path::Path;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load series registry
    let registry = data::series_registry::load_series_registry(Path::new("data/series.toml"))?;

    // Load user config
    let config = config::UserConfig::load()?;

    // Create app state
    let mut app = App::new(registry, config);

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create event channel
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Spawn crossterm event reader task
    let event_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            if crossterm_event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                match crossterm_event::read() {
                    Ok(Event::Key(key)) => {
                        // Only handle key press events (not release/repeat)
                        if key.kind == KeyEventKind::Press {
                            if event_tx.send(AppEvent::Key(key)).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Event::Resize(w, h)) => {
                        if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    // Spawn tick timer (every 1 second, for countdowns)
    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Main loop
    while app.running {
        // Draw
        terminal.draw(|frame| {
            draw_ui(frame, &app);
        })?;

        // Wait for next event
        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::Key(key) => {
                    handle_key_event(&mut app, key);
                }
                AppEvent::Resize(_, _) => {
                    // Terminal auto-redraws on resize, nothing to do
                }
                AppEvent::Tick => {
                    // Will be used for countdown timers later
                }
                AppEvent::SeriesDataFetched { series_id, events } => {
                    app.update_series_data(series_id, events);
                }
                AppEvent::FetchError { series_id, error } => {
                    app.mark_fetch_error(&series_id, error);
                }
                AppEvent::FetchStarted { series_id } => {
                    app.mark_fetching(&series_id);
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    // Save config on exit
    app.config.save()?;

    Ok(())
}

/// Placeholder draw function. Will be replaced in Step 8.
fn draw_ui(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    let block = ratatui::widgets::Block::default()
        .title(" RaceTUI ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let text = ratatui::widgets::Paragraph::new("Loading... Press 'q' to quit, '?' for help")
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(text, area);
}

/// Handle key events. Will be expanded in later steps.
fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) {
    // If search is active, handle search input
    if app.search_active {
        match key.code {
            KeyCode::Esc => {
                app.search_active = false;
                app.search_query = None;
            }
            KeyCode::Enter => {
                app.search_active = false;
            }
            KeyCode::Backspace => {
                if let Some(ref mut query) = app.search_query {
                    query.pop();
                    if query.is_empty() {
                        app.search_query = None;
                    }
                }
            }
            KeyCode::Char(c) => {
                app.search_query
                    .get_or_insert_with(String::new)
                    .push(c);
            }
            _ => {}
        }
        return;
    }

    // Normal mode keybindings
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.running = false;
        }
        // Navigation (vim + arrows)
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_previous();
        }
        // Toggle view mode
        KeyCode::Tab => {
            app.view_mode = match app.view_mode {
                app::ViewMode::List => app::ViewMode::Calendar,
                app::ViewMode::Calendar => app::ViewMode::List,
            };
        }
        // Search
        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_query = Some(String::new());
        }
        // Help
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
        }
        // Toggle detail view
        KeyCode::Enter => {
            app.show_detail = !app.show_detail;
        }
        // Toggle favorite for selected event's series
        KeyCode::Char('f') => {
            if let Some(event) = app.selected_event() {
                let series_id = event.series_id.clone();
                app.config.toggle_favorite(&series_id);
            }
        }
        // Filter panel
        KeyCode::Char('F') => {
            app.show_filter_panel = !app.show_filter_panel;
        }
        // Escape closes overlays
        KeyCode::Esc => {
            if app.show_help {
                app.show_help = false;
            } else if app.show_detail {
                app.show_detail = false;
            } else if app.show_filter_panel {
                app.show_filter_panel = false;
            } else if app.search_query.is_some() {
                app.search_query = None;
            }
        }
        _ => {}
    }
}
```

**Verify**: `cargo run` — should show a bordered TUI screen. Press `q` to quit cleanly.

**Commit message**: `feat: set up terminal, async event loop, and basic keybindings`

---

### [x] Step 8: List View UI

**What to do**: Implement the list view — a table showing all race events sorted by date with columns for Date, Series, Event, Circuit, Country, and Status. Series names are colored with their unique color.

**Create `src/ui/mod.rs`**:

```rust
pub mod list_view;
pub mod status_bar;

use ratatui::prelude::*;
use crate::app::{App, ViewMode};

/// Main draw function — dispatches to the appropriate view.
pub fn draw(frame: &mut Frame, app: &App) {
    // Layout: main content area + status bar at the bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),     // Main content
            Constraint::Length(1),  // Status bar
        ])
        .split(frame.area());

    // Draw the main content based on view mode
    match app.view_mode {
        ViewMode::List => list_view::draw(frame, app, chunks[0]),
        ViewMode::Calendar => {
            // Placeholder for calendar view (Step 12)
            let placeholder = ratatui::widgets::Paragraph::new("Calendar view coming soon. Press Tab to switch back.")
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, chunks[0]);
        }
    }

    // Draw the status bar
    status_bar::draw(frame, app, chunks[1]);
}
```

**Create `src/ui/list_view.rs`**:

```rust
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};

use crate::app::App;
use crate::data::models::EventStatus;

/// Draw the list view — a table of race events.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let events = app.filtered_events();

    // Build header row
    let header = Row::new(vec![
        Cell::from("Date").style(Style::default().bold().fg(Color::White)),
        Cell::from("Series").style(Style::default().bold().fg(Color::White)),
        Cell::from("Event").style(Style::default().bold().fg(Color::White)),
        Cell::from("Circuit").style(Style::default().bold().fg(Color::White)),
        Cell::from("Country").style(Style::default().bold().fg(Color::White)),
        Cell::from("Status").style(Style::default().bold().fg(Color::White)),
    ])
    .height(1)
    .bottom_margin(1);

    // Build data rows
    let rows: Vec<Row> = events
        .iter()
        .map(|event| {
            // Get series color
            let series_color = app
                .series_registry
                .get(&event.series_id)
                .map(|s| Color::Rgb(s.color.0, s.color.1, s.color.2))
                .unwrap_or(Color::White);

            // Get series short name
            let series_name = app
                .series_registry
                .get(&event.series_id)
                .map(|s| s.short_name.as_str())
                .unwrap_or(&event.series_id);

            // Format date in local timezone
            let date_str = event.start_date.format("%b %d").to_string();

            // Determine if this is a favorited series
            let is_favorite = app.config.favorites.contains(&event.series_id);
            let favorite_marker = if is_favorite { "★ " } else { "  " };

            // Status styling
            let (status_text, status_color) = match &event.status {
                EventStatus::Live => ("● LIVE", Color::Red),
                EventStatus::Upcoming => {
                    // Check if event is within notification threshold
                    if let Some(next_time) = event.next_session_time() {
                        let hours_until = next_time
                            .signed_duration_since(chrono::Utc::now())
                            .num_hours();
                        if hours_until <= app.config.notification_threshold_hours as i64
                            && hours_until >= 0
                        {
                            ("⚡ SOON", Color::Yellow)
                        } else {
                            ("Upcoming", Color::DarkGray)
                        }
                    } else {
                        ("Upcoming", Color::DarkGray)
                    }
                }
                EventStatus::Completed => ("Done", Color::DarkGray),
                EventStatus::Cancelled => ("Cancelled", Color::DarkGray),
            };

            Row::new(vec![
                Cell::from(date_str),
                Cell::from(format!("{}{}", favorite_marker, series_name))
                    .style(Style::default().fg(series_color)),
                Cell::from(event.event_name.as_str()),
                Cell::from(event.circuit_name.as_str()),
                Cell::from(event.country.as_str()),
                Cell::from(status_text).style(Style::default().fg(status_color)),
            ])
        })
        .collect();

    // Column widths
    let widths = [
        Constraint::Length(7),   // Date (e.g. "Aug 16")
        Constraint::Length(12),  // Series (e.g. "★ IndyCar")
        Constraint::Min(20),     // Event name (flexible)
        Constraint::Length(25),  // Circuit
        Constraint::Length(15),  // Country
        Constraint::Length(10),  // Status
    ];

    // Build the title with filter/search info
    let mut title = String::from(" Race Calendar ");
    if let Some(ref query) = app.search_query {
        title = format!(" Search: {} ", query);
    }

    // Build table
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always);

    // Render table with state (for selection tracking)
    // We need a mutable copy of the table state for rendering
    let mut table_state = app.table_state.clone();
    frame.render_stateful_widget(table, area, &mut table_state);

    // Draw scrollbar if there are enough items
    if events.len() > area.height as usize {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(events.len())
            .position(app.table_state.selected().unwrap_or(0));
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
```

**Create `src/ui/status_bar.rs`**:

```rust
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, ViewMode};
use crate::data::models::FetchStatus;

/// Draw the bottom status bar with keybind hints and fetch status.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),  // Keybind hints
            Constraint::Percentage(40),  // Fetch status / notifications
        ])
        .split(area);

    // Left side: keybind hints
    let view_name = match app.view_mode {
        ViewMode::List => "List",
        ViewMode::Calendar => "Calendar",
    };

    let hints = if app.search_active {
        " Type to search | Enter: confirm | Esc: cancel".to_string()
    } else {
        format!(
            " q:Quit  j/k/↑/↓:Navigate  Tab:{}  /:Search  f:Favorite  ?:Help  Enter:Detail",
            if view_name == "List" { "Calendar" } else { "List" }
        )
    };

    let hints_widget = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Rgb(20, 20, 30)));

    frame.render_widget(hints_widget, chunks[0]);

    // Right side: fetch status summary
    let total = app.fetch_status.len();
    let loaded = app
        .fetch_status
        .values()
        .filter(|s| matches!(s, FetchStatus::Loaded(_) | FetchStatus::CachedLoad(_, _)))
        .count();
    let fetching = app
        .fetch_status
        .values()
        .filter(|s| matches!(s, FetchStatus::Fetching))
        .count();
    let errors = app
        .fetch_status
        .values()
        .filter(|s| matches!(s, FetchStatus::Error(_)))
        .count();

    let status_text = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else if fetching > 0 {
        format!("Fetching... {}/{} loaded ", loaded, total)
    } else if errors > 0 {
        format!("{}/{} loaded, {} errors ", loaded, total, errors)
    } else {
        format!("{}/{} series loaded ", loaded, total)
    };

    let status_color = if fetching > 0 {
        Color::Yellow
    } else if errors > 0 {
        Color::Red
    } else {
        Color::Green
    };

    let status_widget = Paragraph::new(status_text)
        .style(Style::default().fg(status_color).bg(Color::Rgb(20, 20, 30)))
        .alignment(Alignment::Right);

    frame.render_widget(status_widget, chunks[1]);
}
```

**Update `src/main.rs`**:
1. Add `mod ui;` to the module declarations at the top.
2. In the main loop, change `draw_ui(frame, &app)` to `ui::draw(frame, &app)`.
3. Remove the old `draw_ui` function entirely.

**Verify**: `cargo run` — should show a styled table (empty, since no data is loaded yet) with a status bar.

**Commit message**: `feat: implement list view UI with table, status bar, and keybinds`

---

### [x] Step 9: Scraper Trait and HTTP Fetcher

**What to do**: Define the `SeriesScraper` trait that all per-series scrapers will implement, and create the shared HTTP fetcher module.

**Create `src/scraper/mod.rs`**:

```rust
pub mod fetcher;
pub mod f1;

use anyhow::Result;

use crate::data::models::{RaceEvent, Series};

/// Trait that all series scrapers must implement.
/// Each series has its own module that implements this trait.
///
/// The `scrape` method is called with the series metadata (which includes
/// the calendar URL and other config from series.toml).
/// It should return a Vec of RaceEvent for the current season.
///
/// NOTE: This trait uses Rust 1.75+ RPITIT (return-position impl Trait in traits).
/// If your Rust toolchain is older than 1.75, use the `async-trait` crate instead.
pub trait SeriesScraper: Send + Sync {
    /// Scrape/fetch the race calendar for this series.
    /// The `series` parameter contains metadata from series.toml,
    /// including the calendar_url.
    fn scrape(
        &self,
        series: &Series,
    ) -> impl std::future::Future<Output = Result<Vec<RaceEvent>>> + Send;
}

/// Get the appropriate scraper for a series ID.
/// Returns None if no scraper is implemented yet for this series.
pub fn get_scraper(series_id: &str) -> Option<Box<dyn SeriesScraperBoxed>> {
    match series_id {
        "f1" => Some(Box::new(f1::F1Scraper)),
        // More scrapers will be added in later steps:
        // "f2" => Some(Box::new(f2::F2Scraper)),
        // etc.
        _ => None,
    }
}

/// Object-safe version of SeriesScraper for dynamic dispatch.
/// This is needed because the base SeriesScraper trait uses
/// `impl Future` which isn't object-safe.
pub trait SeriesScraperBoxed: Send + Sync {
    fn scrape_boxed(
        &self,
        series: &Series,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<RaceEvent>>> + Send + '_>>;
}

/// Blanket implementation: any type that implements SeriesScraper
/// automatically implements SeriesScraperBoxed.
impl<T: SeriesScraper> SeriesScraperBoxed for T {
    fn scrape_boxed(
        &self,
        series: &Series,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<RaceEvent>>> + Send + '_>>
    {
        Box::pin(self.scrape(series))
    }
}
```

**Create `src/scraper/fetcher.rs`**:

```rust
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

/// Create a shared HTTP client with reasonable defaults.
/// This should be created once and reused across all scrapers.
pub fn create_http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("RaceTUI/0.1.0")
        .build()
        .context("Failed to create HTTP client")
}

/// Fetch a URL and return the response body as a string.
pub async fn fetch_url(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {} for URL: {}", status, url);
    }

    response
        .text()
        .await
        .with_context(|| format!("Failed to read response body from: {}", url))
}

/// Fetch a URL and parse the response as JSON.
pub async fn fetch_json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {} for URL: {}", status, url);
    }

    response
        .json::<T>()
        .await
        .with_context(|| format!("Failed to parse JSON from: {}", url))
}
```

**Update `src/main.rs`**: Add `mod scraper;` to the module declarations.

**Verify**: `cargo check`

**Commit message**: `feat: define SeriesScraper trait and shared HTTP fetcher`

---

### [x] Step 10: F1 Scraper (Jolpica API)

**What to do**: Implement the first scraper — Formula 1 using the Jolpica API (JSON, no scraping needed). This serves as the reference implementation for all other scrapers.

**Create `src/scraper/f1.rs`**:

```rust
use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Deserialize;

use super::fetcher;
use super::SeriesScraper;
use crate::data::models::*;

/// F1 scraper using the Jolpica API (successor to Ergast).
/// API endpoint: https://api.jolpi.ca/ergast/f1/current.json
pub struct F1Scraper;

// --- Jolpica API response structs ---
// These match the JSON structure returned by the API.

#[derive(Debug, Deserialize)]
struct JolpicaResponse {
    #[serde(rename = "MRData")]
    mr_data: MRData,
}

#[derive(Debug, Deserialize)]
struct MRData {
    #[serde(rename = "RaceTable")]
    race_table: RaceTable,
}

#[derive(Debug, Deserialize)]
struct RaceTable {
    #[serde(rename = "Races")]
    races: Vec<JolpicaRace>,
}

#[derive(Debug, Deserialize)]
struct JolpicaRace {
    round: String,
    #[serde(rename = "raceName")]
    race_name: String,
    #[serde(rename = "Circuit")]
    circuit: JolpicaCircuit,
    date: String,
    time: Option<String>,
    #[serde(rename = "FirstPractice")]
    first_practice: Option<JolpicaSession>,
    #[serde(rename = "SecondPractice")]
    second_practice: Option<JolpicaSession>,
    #[serde(rename = "ThirdPractice")]
    third_practice: Option<JolpicaSession>,
    #[serde(rename = "Qualifying")]
    qualifying: Option<JolpicaSession>,
    #[serde(rename = "Sprint")]
    sprint: Option<JolpicaSession>,
    #[serde(rename = "SprintQualifying")]
    sprint_qualifying: Option<JolpicaSession>,
    // Newer API versions may use "SprintShootout" instead
    #[serde(rename = "SprintShootout")]
    sprint_shootout: Option<JolpicaSession>,
}

#[derive(Debug, Deserialize)]
struct JolpicaCircuit {
    #[serde(rename = "circuitName")]
    circuit_name: String,
    #[serde(rename = "Location")]
    location: JolpicaLocation,
}

#[derive(Debug, Deserialize)]
struct JolpicaLocation {
    locality: String,
    country: String,
}

#[derive(Debug, Deserialize)]
struct JolpicaSession {
    date: String,
    time: Option<String>,
}

// --- Helper functions ---

/// Parse a date string "YYYY-MM-DD" and optional time "HH:MM:SSZ" into a UTC DateTime.
fn parse_datetime(date: &str, time: Option<&str>) -> Option<chrono::DateTime<Utc>> {
    let naive_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    if let Some(time_str) = time {
        // Time format from Jolpica: "14:00:00Z"
        let cleaned = time_str.trim_end_matches('Z');
        let naive_time = NaiveTime::parse_from_str(cleaned, "%H:%M:%S").ok()?;
        let naive_dt = naive_date.and_time(naive_time);
        Some(Utc.from_utc_datetime(&naive_dt))
    } else {
        None
    }
}

/// Convert a JolpicaSession to a Session.
fn convert_session(
    name: &str,
    session_type: SessionType,
    jolpica: &JolpicaSession,
) -> Session {
    Session {
        name: name.to_string(),
        session_type,
        start_time: parse_datetime(&jolpica.date, jolpica.time.as_deref()),
        end_time: None,
    }
}

// --- Trait implementation ---

impl SeriesScraper for F1Scraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = fetcher::create_http_client()?;
        let response: JolpicaResponse =
            fetcher::fetch_json(&client, &series.calendar_url)
                .await
                .context("Failed to fetch F1 calendar from Jolpica API")?;

        let mut events = Vec::new();

        for race in response.mr_data.race_table.races {
            let round: u32 = race.round.parse().unwrap_or(0);

            // Parse the race date
            let start_date = NaiveDate::parse_from_str(&race.date, "%Y-%m-%d")
                .with_context(|| format!("Failed to parse F1 race date: {}", race.date))?;

            // Build sessions list
            let mut sessions = Vec::new();

            if let Some(ref fp1) = race.first_practice {
                sessions.push(convert_session("Free Practice 1", SessionType::Practice, fp1));
            }
            if let Some(ref fp2) = race.second_practice {
                sessions.push(convert_session("Free Practice 2", SessionType::Practice, fp2));
            }
            if let Some(ref fp3) = race.third_practice {
                sessions.push(convert_session("Free Practice 3", SessionType::Practice, fp3));
            }
            if let Some(ref sq) = race.sprint_qualifying {
                sessions.push(convert_session(
                    "Sprint Qualifying",
                    SessionType::SprintQualifying,
                    sq,
                ));
            }
            if let Some(ref ss) = race.sprint_shootout {
                sessions.push(convert_session(
                    "Sprint Shootout",
                    SessionType::SprintQualifying,
                    ss,
                ));
            }
            if let Some(ref sprint) = race.sprint {
                sessions.push(convert_session("Sprint", SessionType::Sprint, sprint));
            }
            if let Some(ref quali) = race.qualifying {
                sessions.push(convert_session("Qualifying", SessionType::Qualifying, quali));
            }

            // The race itself
            sessions.push(Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: parse_datetime(&race.date, race.time.as_deref()),
                end_time: None,
            });

            // Determine event date range (earliest session to race day)
            let earliest_date = sessions
                .iter()
                .filter_map(|s| s.start_time)
                .min()
                .map(|dt| dt.date_naive())
                .unwrap_or(start_date);

            // Determine status based on dates
            let now = Utc::now();
            let status = if sessions.iter().any(|s| {
                s.start_time
                    .map_or(false, |t| {
                        t <= now && s.end_time.map_or(
                            t + chrono::Duration::hours(2) > now,
                            |e| e > now,
                        )
                    })
            }) {
                EventStatus::Live
            } else if start_date >= now.date_naive() {
                EventStatus::Upcoming
            } else {
                EventStatus::Completed
            };

            // F1TV stream link
            let stream_links = vec![StreamLink {
                platform: "F1TV".to_string(),
                url: "https://f1tv.formula1.com".to_string(),
                access: StreamAccess::Paid,
            }];

            events.push(RaceEvent {
                series_id: "f1".to_string(),
                event_name: race.race_name,
                circuit_name: race.circuit.circuit_name,
                location: race.circuit.location.locality,
                country: race.circuit.location.country,
                start_date: earliest_date,
                end_date: start_date,
                round: Some(round),
                sessions,
                stream_links,
                status,
            });
        }

        Ok(events)
    }
}
```

**Verify**: `cargo check`

**Commit message**: `feat: implement F1 scraper using Jolpica API`

---

### [x] Step 11: Data Fetch Orchestration

**What to do**: Wire up the scraper system to the main event loop. On startup, load cached data first, then spawn async tasks to refresh stale data in the background. The TUI remains interactive while data loads.

**Add this function to `src/main.rs`**:

```rust
/// Spawn background tasks to load data for all series.
/// For each series:
/// 1. Try to load from cache first (instant, non-blocking).
/// 2. If cache is fresh (within TTL), use it.
/// 3. If cache is stale or missing, spawn an async task to scrape/fetch.
fn spawn_data_loaders(
    app: &mut App,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let series_list: Vec<(String, data::models::Series)> = app
        .series_registry
        .iter()
        .map(|(id, s)| (id.clone(), s.clone()))
        .collect();

    for (series_id, series) in series_list {
        // 1. Try loading from cache
        match data::cache::read_cache(&series_id) {
            Ok(Some((events, fetched_at))) => {
                let age_hours = chrono::Utc::now()
                    .signed_duration_since(fetched_at)
                    .num_hours() as u64;
                let count = events.len();

                // Load cached data immediately so user can browse
                app.update_series_data(series_id.clone(), events);
                app.mark_cached_load(&series_id, count, age_hours);

                // If cache is still fresh, skip fetching
                if age_hours < app.config.cache_ttl_hours {
                    continue;
                }
                // Otherwise, fall through to refresh in background
            }
            Ok(None) => {
                // No cache, will fetch
            }
            Err(e) => {
                tracing::warn!("Failed to read cache for {}: {}", series_id, e);
            }
        }

        // 2. Check if we have a scraper for this series
        let scraper_impl = match scraper::get_scraper(&series_id) {
            Some(s) => s,
            None => {
                app.mark_fetch_error(
                    &series_id,
                    "No scraper implemented yet".to_string(),
                );
                continue;
            }
        };

        // 3. Spawn async fetch task
        let fetch_tx = tx.clone();
        let sid = series_id.clone();
        let series_clone = series.clone();

        // Notify that fetching has started
        let _ = tx.send(AppEvent::FetchStarted {
            series_id: sid.clone(),
        });

        tokio::spawn(async move {
            match scraper_impl.scrape_boxed(&series_clone).await {
                Ok(events) => {
                    // Write to cache
                    if let Err(e) = data::cache::write_cache(&sid, &events) {
                        tracing::warn!("Failed to write cache for {}: {}", sid, e);
                    }
                    let _ = fetch_tx.send(AppEvent::SeriesDataFetched {
                        series_id: sid,
                        events,
                    });
                }
                Err(e) => {
                    let _ = fetch_tx.send(AppEvent::FetchError {
                        series_id: sid,
                        error: e.to_string(),
                    });
                }
            }
        });
    }
}
```

**Call this function** in `main()` right after creating the App and before the main loop:
```rust
spawn_data_loaders(&mut app, tx.clone());
```

**Add a `RefreshRequested` variant** to the `AppEvent` enum in `src/event.rs`:
```rust
/// User requested a data refresh
RefreshRequested,
```

**Update keybinding** for `r`/`R` in `handle_key_event`:
```rust
KeyCode::Char('r') | KeyCode::Char('R') => {
    app.status_message = Some("Refreshing...".to_string());
    // The actual refresh is triggered by sending RefreshRequested
    // which is handled in the main loop where `tx` is available.
}
```

**Handle `RefreshRequested`** in the main loop:
- When received, call `spawn_data_loaders(&mut app, tx.clone())`.
- To send this event from the key handler, you'll need to pass `tx` into `handle_key_event` or restructure so the main loop checks a `refresh_requested` bool on `App`.

> **Simplest approach**: Add `pub refresh_requested: bool` to `App`. Set it to `true` in the `r`/`R` keybind handler. In the main loop, after handling events, check `if app.refresh_requested { app.refresh_requested = false; spawn_data_loaders(&mut app, tx.clone()); }`.

**Verify**: `cargo run` — F1 data should load (either from cache or from Jolpica API). You should see F1 events in the list. Other series will show "No scraper implemented yet" in their status.

**Commit message**: `feat: implement data fetch orchestration with caching`

---

### [x] Step 12: Calendar Grid View

**What to do**: Implement the calendar grid view — a traditional month calendar with race events shown on their respective days. Navigate months with `h/l` or left/right arrows.

**Create `src/ui/calendar_view.rs`**:

Build a month-grid widget showing a 7-column (Mon–Sun) grid. Each cell shows the day number and any race events on that day (colored by series). The current day is highlighted. Days with events from favorited series are specially marked.

**Implementation details**:

- Layout: A header row with day names (Mon, Tue, Wed, ..., Sun), then up to 6 rows for the days of the month.
- Each cell shows: day number, and up to 3 event short names (series short_name). If more than 3 events, show "+N more".
- Navigation: `h`/Left moves to the previous month, `l`/Right moves to the next month in Calendar view mode. `H` goes to the previous year, `L` goes to the next year. These keybinds only apply in Calendar view mode — check `app.view_mode` before applying.
- Title bar shows "◀ August 2026 ▶" centered.
- Use `chrono::NaiveDate` to calculate the first day of the month and which weekday it falls on.
- Highlight today's date with a distinct background color (e.g., `Color::Rgb(40, 60, 40)`).
- Show events in each cell with their series color.
- The calendar draws itself inside a `Block` with `Borders::ALL`.
- Each day cell is rendered as a `Paragraph` widget inside a sub-area from a grid layout.
- Use `Layout::default().direction(Direction::Horizontal).constraints(vec![Constraint::Ratio(1, 7); 7])` for the 7 columns.
- Use `Layout::default().direction(Direction::Vertical)` for the rows (1 header + up to 6 week rows).
- To find which day of the week the 1st of the month falls on, use `chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap().weekday()`.
- Use `chrono::Weekday::num_days_from_monday()` to get 0-indexed column position (Mon=0, Sun=6).

**Update `src/ui/mod.rs`**: Add `pub mod calendar_view;` and replace the calendar placeholder in the `draw` function with `calendar_view::draw(frame, app, chunks[0])`.

**Add calendar-specific keybinds** to `handle_key_event` in `src/main.rs`:
When `app.view_mode == ViewMode::Calendar` AND NOT in search mode:
- `h` / `KeyCode::Left`: Previous month. Decrement `app.calendar_month`. If month goes below 1, set to 12 and decrement `app.calendar_year`.
- `l` / `KeyCode::Right`: Next month. Increment `app.calendar_month`. If month goes above 12, set to 1 and increment `app.calendar_year`.

> **Important**: The vim `h`/`l` keys for calendar navigation conflict with the list view's default behavior. Only bind `h`/`l` to month navigation when `app.view_mode == ViewMode::Calendar`. In list view, `h` and `l` are unused (they don't do anything).

**Verify**: `cargo run`, press `Tab` to switch to Calendar view. F1 events should appear on their respective dates. Press `h`/`l` to navigate months.

**Commit message**: `feat: implement calendar grid view with month navigation`

---

### [x] Step 13: Event Detail View

**What to do**: When the user presses `Enter` on a selected event, show a detailed panel/popup with all session times, stream links, circuit info, and a countdown to the next session.

**Create `src/ui/detail_view.rs`**:

**Implementation details**:

- Render as a centered popup overlay (60% width, 70% height of the terminal, centered).
- To center: calculate the popup `Rect` manually:
  ```rust
  let popup_width = area.width * 60 / 100;
  let popup_height = area.height * 70 / 100;
  let popup_x = (area.width - popup_width) / 2;
  let popup_y = (area.height - popup_height) / 2;
  let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);
  ```
- Use `ratatui::widgets::Clear` to clear the area behind the popup before drawing it.
- Content sections (top to bottom):
  1. **Title**: Event name in Bold (e.g., "Monaco Grand Prix")
  2. **Subtitle line**: `"Formula 1 · Round 8"` — series name and round number
  3. **Circuit info**: `"Circuit de Monaco · Monte Carlo, Monaco"` — circuit, location, country
  4. **Blank line separator**
  5. **Session schedule table** with columns: `[Session Name, Date, Time (local), Status]`
     - Convert UTC times to local time using `chrono::Local` (call `.with_timezone(&chrono::Local)`)
     - Format date as "Sat, Jun 14" (using `%a, %b %d`)
     - Format time as "2:00 PM" (using `%l:%M %p`)
     - Past sessions: style with `Color::DarkGray`
     - The next upcoming session: style with `Color::Cyan` and `Modifier::BOLD`
     - Live sessions: show "● LIVE" in `Color::Red`
  6. **Blank line separator**
  7. **Stream links**: One line per link: `"[1] F1TV (Paid $) — https://f1tv.formula1.com"`
     - Number each link 1-9 so user can press that key to open it
     - Free streams: show "✓ Free" in Green
     - Paid streams: show "$ Paid" in Yellow
  8. **Blank line separator**
  9. **Countdown**: If there's a next upcoming session: `"⏱ Next: Qualifying in 2h 34m"`
     - Calculate using `next_session_time()` minus `Utc::now()`
     - Format as days/hours/minutes: "1d 5h 23m", "5h 23m", "23m", "< 1m"
- Draw a `Block` border around the popup with the event name as the title.
- All this content can be rendered as a single `Paragraph` widget with `Line`/`Span` composition for colors.

**Keybinds within detail view** (add to `handle_key_event`, gated on `app.show_detail`):
- `Esc` or `Enter` or `q`: close detail view (set `app.show_detail = false`)
- `o`: open the first stream link using `action::open_url`
- `1-9`: open the Nth stream link

**Update `src/ui/mod.rs`**: Add `pub mod detail_view;`. In the `draw` function, AFTER drawing the main view, check if `app.show_detail` is true and if so, call `detail_view::draw(frame, app)` (renders on top as a popup — pass `frame.area()` as the area to center within).

**Verify**: `cargo run`, select an F1 event, press `Enter`. The detail popup should show session times and stream info.

**Commit message**: `feat: implement event detail view popup with session times`

---

### [x] Step 14: Filter Panel

**What to do**: Implement the filter panel overlay that lets users filter events by category (car style, region, series, favorites only).

**Create `src/ui/filter_panel.rs`**:

**Implementation details**:

- Render as a left-side panel (30% width, full height) that overlays the content.
- Use `Clear` widget to clear the area behind the panel.
- The panel area: `Rect::new(0, 0, area.width * 30 / 100, area.height)`.
- Content is a `List` widget with `ListItem`s organized into sections:
  ```
  ── Filter By ──
    All Events
    ★ Favorites Only
  ── Car Style ──
    Open Wheel
    Sports Car
    Stock Car
    Touring
    Rally
    Motorcycle
  ── Region ──
    International
    USA
    UK
    Japan
    Europe
  ── Series ──
    Formula 1
    Formula 2
    Formula 3
    ...all 16 series...
  ```
- Section headers ("── Filter By ──") are rendered as non-selectable items with `Color::DarkGray` and `Modifier::BOLD`. They should be skipped during navigation.
- Each selectable item corresponds to a `FilterCategory` value.
- The currently active filter has a `●` prefix and is highlighted with a different color (e.g., `Color::Cyan`).
- Non-active items have a `○` prefix.
- Use `ListState` for selection tracking (stored in `App` as `pub filter_list_state: ListState`).

**Navigation logic**:
- Build a flat `Vec` of all items (headers + selectable items).
- Track which indices are selectable vs. header-only.
- When navigating with `j`/`k`, skip header indices.
- When `Enter` is pressed, map the selected index to the corresponding `FilterCategory` and update `app.active_filter`.

**Add state to `App`** in `src/app.rs`:
```rust
pub filter_list_state: ratatui::widgets::ListState,
```
Initialize it in `App::new()`:
```rust
filter_list_state: {
    let mut s = ratatui::widgets::ListState::default();
    s.select(Some(0));
    s
},
```

**Update keybindings** in `handle_key_event`:
When `app.show_filter_panel` is true, intercept `j`/`k`/`↑`/`↓`/`Enter`/`Esc` at the TOP of the function (before the normal keybinds) to navigate the filter list instead of the main event list. `Esc` or `F` closes the panel.

**Update `src/ui/mod.rs`**: Add `pub mod filter_panel;`. In the `draw` function, AFTER drawing the main view but BEFORE drawing help/detail popups, check if `app.show_filter_panel` and call `filter_panel::draw(frame, app)`.

**Verify**: `cargo run`, press `F`. The filter panel should appear. Select "Favorites Only" and press Enter.

**Commit message**: `feat: implement filter panel with category/region/series filtering`

---

### [x] Step 15: Help Popup

**What to do**: Implement the help popup that shows all keybindings.

**Create `src/ui/help.rs`**:

**Implementation details**:

- Render as a centered popup (50% width, 60% height).
- Calculate popup position the same way as the detail view.
- Use `Clear` to clear behind the popup.
- Content: A `Paragraph` with styled `Line`/`Span` elements:
  ```
   ── Navigation ──
   j / ↓              Move down
   k / ↑              Move up
   Tab                Toggle List / Calendar view
   h / ← (calendar)   Previous month
   l / → (calendar)   Next month

   ── Actions ──
   Enter              Show event details
   f                  Toggle favorite
   F                  Open filter panel
   /                  Search events
   o                  Open stream link (in detail view)
   1-9                Open Nth stream link (in detail view)
   r                  Refresh all data

   ── General ──
   ?                  Toggle this help
   Esc                Close popup / cancel
   q                  Quit
  ```
- Section headers ("── Navigation ──") use `Color::Cyan` and `Modifier::BOLD`.
- Keybind names use `Color::Yellow`.
- Descriptions use `Color::White`.
- The block title is `" Keybindings "`.
- Dismiss with `?` or `Esc` (already handled in the main key handler since `?` toggles `show_help`).

**Update `src/ui/mod.rs`**: Add `pub mod help;`. In the `draw` function, if `app.show_help`, call `help::draw(frame, app)`. **Draw it LAST** so it's on top of everything else (including the filter panel and detail view).

**Verify**: `cargo run`, press `?`. Help popup should appear with all keybindings listed.

**Commit message**: `feat: implement help popup with keybinding reference`

---

### [x] Step 16: Livestream Opening

**What to do**: Implement the action to open livestream URLs using the configured command.

**Create `src/action.rs`**:

```rust
use anyhow::{Context, Result};
use std::process::Command;

/// Open a URL using the configured open command.
/// The command is run in the background (detached) so it doesn't block the TUI.
pub fn open_url(open_command: &str, url: &str) -> Result<()> {
    Command::new(open_command)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to open URL '{}' with command '{}'",
                url, open_command
            )
        })?;
    Ok(())
}
```

**Update `src/main.rs`**: Add `mod action;`.

**Update keybindings** in `handle_key_event`:
When `app.show_detail` is true, BEFORE the normal keybinds:
```rust
if app.show_detail {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.show_detail = false;
            return;
        }
        KeyCode::Char('o') => {
            // Open first stream link
            if let Some(event) = app.selected_event() {
                if let Some(link) = event.stream_links.first() {
                    if let Err(e) = action::open_url(&app.config.open_command, &link.url) {
                        app.status_message = Some(format!("Error: {}", e));
                    } else {
                        app.status_message = Some(format!("Opened {} in {}", link.platform, app.config.open_command));
                    }
                }
            }
            return;
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let index = (c as u8 - b'1') as usize;
            if let Some(event) = app.selected_event() {
                if let Some(link) = event.stream_links.get(index) {
                    if let Err(e) = action::open_url(&app.config.open_command, &link.url) {
                        app.status_message = Some(format!("Error: {}", e));
                    } else {
                        app.status_message = Some(format!("Opened {} in {}", link.platform, app.config.open_command));
                    }
                }
            }
            return;
        }
        _ => { return; }
    }
}
```

**Verify**: `cargo run`, open an F1 event detail, press `o`. It should open the F1TV URL in your browser.

**Commit message**: `feat: implement livestream URL opening with configurable command`

---

### [x] Step 17: Implement Remaining Scrapers — NASCAR (All 3 Series)

**What to do**: Implement the NASCAR scraper for Cup, Xfinity, and Trucks. All three use the same NASCAR internal API/CDN (`cf.nascar.com`).

**Create `src/scraper/nascar.rs`**:

**Research steps** (the implementing agent should do this):
1. Open `https://www.nascar.com/nascar-cup-series/2026/schedule/` in a browser.
2. Open browser DevTools → Network tab.
3. Look for XHR/Fetch requests that return JSON data.
4. The endpoint pattern is typically: `https://cf.nascar.com/cacher/<year>/race_list_basic.json` or similar. Look for requests to `cf.nascar.com`.
5. NASCAR uses internal series IDs: 1 = Cup, 2 = Xfinity, 3 = Trucks.

**Implementation**:
- Create a `NascarScraper` struct that takes the NASCAR internal series ID as a field.
- Implement `SeriesScraper` for it.
- In `get_scraper()`, return:
  - `"nascar_cup"` → `NascarScraper { nascar_series_id: 1, racetui_series_id: "nascar_cup" }`
  - `"nascar_xfinity"` → `NascarScraper { nascar_series_id: 2, racetui_series_id: "nascar_xfinity" }`
  - `"nascar_trucks"` → `NascarScraper { nascar_series_id: 3, racetui_series_id: "nascar_trucks" }`
- Define deserialization structs for the NASCAR JSON response.
- Map each event to a `RaceEvent` with appropriate stream links (Fox Sports, NBC Sports — both Paid).

**Fallback**: If the NASCAR CDN API endpoints have changed or are not accessible:
1. Create `data/nascar_cup_2026.json` (and similar for Xfinity/Trucks) with manually curated schedule data.
2. Load from the static file instead.
3. Leave a `TODO` comment noting that the API endpoint needs to be updated.

**Update `src/scraper/mod.rs`**: Add `pub mod nascar;` and register all 3 NASCAR series in `get_scraper()`.

**Verify**: `cargo run` — NASCAR events should appear in the list.

**Commit message**: `feat: implement NASCAR scraper for Cup, Xfinity, and Trucks`

---

### [x] Step 18: Implement Remaining Scrapers — IndyCar, IMSA, BTCC

**What to do**: Implement scrapers for series whose websites are likely server-rendered HTML (can use `reqwest + scraper` without a headless browser).

For each series, create a file in `src/scraper/`:

**General approach for HTML scraping**:
1. Fetch the HTML with `fetcher::fetch_url()`.
2. Parse with `scraper::Html::parse_document()`.
3. Use CSS selectors to find the calendar elements.
4. To discover the right CSS selectors:
   - Fetch the HTML and print/log it to see the structure.
   - Look for patterns like `<div class="schedule-item">`, `<table class="calendar">`, etc.
   - Use `scraper::Selector::parse("css-selector").unwrap()` to create selectors.
5. Extract text content with `.text().collect::<String>()` or `.value().attr("href")` for links.

**`src/scraper/indycar.rs`** — IndyCar:
- URL: `https://www.indycar.com/Schedule`
- Fetch HTML, find schedule elements.
- Extract: event name, track name, date, TV network info.
- Stream links: `StreamLink { platform: "Peacock", url: "https://www.peacocktv.com", access: StreamAccess::Paid }`.
- If HTML doesn't contain schedule data (JS-rendered), set `requires_js = true` in series.toml and return an error suggesting headless browser.

**`src/scraper/imsa.rs`** — IMSA:
- URL: `https://www.imsa.com/weathertech/schedule/`
- HTML scraping approach.
- Stream links: IMSA.tv (Mixed), Peacock (Paid).

**`src/scraper/btcc.rs`** — BTCC:
- URL: `https://www.btcc.net/calendar/`
- HTML scraping approach.
- Stream links: ITV (Free in UK — use `StreamAccess::Mixed`).

**Update `src/scraper/mod.rs`**: Add the modules and register them in `get_scraper()`.

**Verify**: `cargo run` — events from these series should appear (or show appropriate errors if scraping fails).

**Commit message**: `feat: implement IndyCar, IMSA, and BTCC scrapers`

---

### [x] Step 19: Implement Remaining Scrapers — F2, F3, Formula E, WEC, MotoGP, WRC, DTM

**What to do**: Implement scrapers for the remaining series. Research found that most of these have **JSON APIs**, so this step is mostly API consumption, not web scraping.

**Create these files**:

**`src/scraper/f2.rs`** and **`src/scraper/f3.rs`** — FOM Apigee API:
- F2 and F3 both use the FOM (Formula One Management) Apigee API backend, shared with the F1 website infrastructure.
- API endpoints:
  - F2: `https://api.formula1.com/v1/core-editorial-races/f2/listing`
  - F3: `https://api.formula1.com/v1/core-editorial-races/f3/listing`
- These endpoints may require a public API key embedded in the website's `cwpConfig`. Inspect `https://www.fiaformula2.com` page source to find the `apikey` header value.
- If the API requires an API key header, add it to the request: `.header("apikey", "<key_from_page_source>")`.
- Parse the JSON response to extract: round number, event name, circuit, country, dates, session timetable (Practice, Qualifying, Sprint Race, Feature Race).
- Stream links: F1TV (Paid).
- **Fallback**: If the FOM API is inaccessible or requires authentication that can't be easily obtained, fall back to the official eCal calendar widget or a static curated JSON file.

**`src/scraper/formula_e.rs`** — Pulselive API:
- Formula E uses the Pulselive backend API.
- API endpoint: `https://api.formula-e.pulselive.com/formula-e/v1/races` (or similar — inspect the `fiaformulae.com` calendar page network requests to find the exact endpoint).
- Parse JSON to extract: season, round number, E-Prix title, city/circuit, country, session timetable (FP1, FP2, Qualifying, Race).
- Stream links: YouTube (Free for practice), regional broadcasters (Mixed).
- **Note**: Free Practice sessions are streamed 100% free on YouTube; tag these with `StreamAccess::Free`.

**`src/scraper/wec.rs`** — HTML scraping or static JSON:
- WEC only has ~8 rounds per year. The website at `https://www.fiawec.com/` is server-rendered HTML.
- **Option A**: Scrape the SSR HTML from the homepage/calendar page with `reqwest + scraper`.
- **Option B**: Maintain a static `data/wec_2026.json` file with the 8 rounds curated manually. This is very low maintenance given the small number of events.
- Stream links: FIAWEC+ (Paid), YouTube for free practice/highlights (Mixed).

**`src/scraper/motogp.rs`** — Pulselive API:
- MotoGP uses the Pulselive backend API (community-documented).
- Step 1: Fetch seasons list from `https://api.motogp.pulselive.com/motogp/v1/results/seasons` to get the current season UUID.
- Step 2: Fetch events with `https://api.motogp.pulselive.com/motogp/v1/results/events?seasonUuid={uuid}`.
- Parse JSON to extract: Grand Prix name, circuit, country, dates, session types for all classes (MotoGP, Moto2, Moto3).
- For the calendar, focus on MotoGP class sessions (FP1, Practice, FP2, Q1, Q2, Sprint, Race).
- Stream links: MotoGP VideoPass (Paid).

**`src/scraper/wrc.rs`** — WRC JSON API:
- WRC has an internal JSON API.
- API endpoint: `https://api.wrc.com/contel-page/83388/calendar/active-season/` (or inspect `wrc.com/calendar` network requests for the current endpoint).
- Parse JSON to extract: rally title, surface type, country, dates, itinerary/stage schedule.
- Stream links: Rally.TV (Paid), Red Bull TV highlights (Free/Mixed).

**`src/scraper/dtm.rs`** — Static curated JSON:
- DTM only has ~8 rounds per year. The website uses a JavaScript SPA that's hard to scrape.
- **Best approach**: Maintain a static `data/dtm_2026.json` file with the 8 rounds manually curated.
- Each round has two races (Saturday + Sunday), each with Qualifying and Race.
- Stream links: DTM YouTube (Free internationally), ProSieben/Joyn (Free in Germany).
- **Note**: DTM streams **100% free** on YouTube with English commentary for international viewers — mark these as `StreamAccess::Free`!

**For IMSA and WEC static data files** (if using Option B): Create `data/imsa_2026.json`, `data/wec_2026.json`, `data/dtm_2026.json` with manually curated event data in the `RaceEvent` JSON format. The scraper loads and parses these files instead of making network requests.

**Update `src/scraper/mod.rs`**: Add all modules and register them in `get_scraper()`.

**Verify**: `cargo run` — all series with working APIs should show data. Static-data series should load from their JSON files.

**Commit message**: `feat: implement scrapers for F2, F3, FE, WEC, MotoGP, WRC, DTM`

---

### [x] Step 20: Implement Remaining Scrapers — Super Formula, Super GT

**What to do**: Implement scrapers for the Japanese series. These sites are likely server-rendered and simpler to scrape.

**Create `src/scraper/super_formula.rs`**:
- URL: `https://superformula.net/sf2/en/race/`
- Fetch HTML with reqwest.
- Parse with scraper crate.
- Find schedule elements with CSS selectors.
- May need to handle mixed Japanese/English text.
- Stream links: YouTube/Motorsport.tv (Mixed).

**Create `src/scraper/super_gt.rs`**:
- URL: `https://supergt.net/en/schedules`
- HTML scraping approach.
- Stream links: YouTube/Motorsport.tv (Mixed).

**Update `src/scraper/mod.rs`**: Add modules and register in `get_scraper()`.

**Verify**: `cargo run`

**Commit message**: `feat: implement Super Formula and Super GT scrapers`

---

### [x] Step 21: Implement Additional Scrapers (Expansion Pack)

**What to do**: Implement scrapers for the 19 newly added series. Most of these use identical platforms to series we already implemented, making them trivial to add!

**Create these files**:

**`src/scraper/sro.rs`** — Fanatec GTWC Europe, GTWC America, IGTC
- These share the SRO server-rendered platform. Build one generic `SroScraper` and use it for `gtwc_eu`, `gtwc_am`, and `igtc`.
- Stream links: YouTube (Free).

**`src/scraper/lmem.rs`** — ELMS, AsLMS
- Share the LMEM / ACO server-rendered platform. Build one generic `LmemScraper`.
- Stream links: YouTube (Free).

**`src/scraper/motogp.rs` (Update)** — Moto2, Moto3
- These run on the exact same weekends as MotoGP and use the identical Pulselive API. Update the existing `motogp` scraper to also handle `moto2` and `moto3` series IDs.

**`src/scraper/wrc.rs` (Update)** — WRC2, ERC
- WRC2 uses the identical API as WRC.
- ERC uses the identical Contel JSON API backend (`api.wrc.com`). Update the existing `wrc` scraper to handle these.

**`src/scraper/nascar.rs` (Update)** — ARCA Menards
- Uses the identical NASCAR CDN JSON endpoint. Update `nascar.rs` to handle `arca` (series ID 4).

**`src/scraper/f1_academy.rs`** — F1 Academy
- Uses the identical FOM Apigee API as F2/F3 (`f1academy` series ID).

**`src/scraper/indy_nxt.rs`** — INDY NXT
- Uses the identical platform as IndyCar. Update or create scraper for `indy_nxt`.

**Create individual scrapers for the rest**:
- **`src/scraper/nls.rs`**: HTML scraping. Streams: YouTube (Free).
- **`src/scraper/supercars.rs`**: HTML scraping. Streams: SuperView (Paid).
- **`src/scraper/tcr.rs`**: HTML scraping. Streams: YouTube (Mixed).
- **`src/scraper/worldsbk.rs`**: HTML scraping. Streams: VideoPass (Paid).
- **`src/scraper/bsb.rs`**: HTML scraping. Streams: Discovery+ (Paid).
- **`src/scraper/dakar.rs`**: Static JSON or HTML. 1 event/year.
- **`src/scraper/porsche.rs`**: HTML scraping. Streams: F1TV (Paid).
- **`src/scraper/extreme_e.rs`**: HTML scraping. Streams: YouTube (Free).

**Update `src/scraper/mod.rs`**: Add and register all new modules in `get_scraper()`.

**Verify**: `cargo run` — all 19 new series should fetch and display data.

**Commit message**: `feat: implement scrapers for 19 additional racing series`

---

### [x] Step 22: Headless Browser Fallback

**What to do**: Implement the `chromiumoxide`-based headless browser fallback for sites that require JavaScript rendering. This is behind the `headless-browser` feature flag.

**Update `src/scraper/fetcher.rs`** — add these functions:

```rust
/// Fetch a URL using a headless browser (for JavaScript-rendered pages).
/// Only available when compiled with the `headless-browser` feature.
#[cfg(feature = "headless-browser")]
pub async fn fetch_with_browser(url: &str) -> Result<String> {
    use chromiumoxide::Browser;
    use chromiumoxide::BrowserConfig;
    use futures::StreamExt;

    let (browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .build()
            .map_err(|e| anyhow::anyhow!("Browser config error: {}", e))?,
    )
    .await
    .context("Failed to launch headless browser")?;

    // The handler must be polled in the background
    tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    let page = browser.new_page(url).await.context("Failed to open page")?;

    // Wait for page to fully load
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let html = page.content().await.context("Failed to get page content")?;

    Ok(html)
}

#[cfg(not(feature = "headless-browser"))]
pub async fn fetch_with_browser(_url: &str) -> Result<String> {
    anyhow::bail!(
        "Headless browser support is not compiled in. \
         Build with: cargo build --features headless-browser"
    )
}
```

**Update scrapers that need JS rendering**: For each scraper that previously returned an error for JS-requiring sites (F2, F3, Formula E, WEC, WRC, DTM — any that couldn't find an internal API), update the `scrape` method to:
1. Call `fetcher::fetch_with_browser(&series.calendar_url).await?` to get the fully-rendered HTML.
2. Parse the HTML with `scraper::Html::parse_document()` as usual.
3. Use CSS selectors to extract calendar data from the rendered HTML.
4. If the headless browser feature is not compiled in, the function will return an appropriate error message.

**Verify**:
- Without feature: `cargo run` — JS-requiring scrapers show "not compiled in" error.
- With feature: `cargo run --features headless-browser` — requires Chrome/Chromium installed.

**Commit message**: `feat: implement headless browser fallback for JS-rendered sites`

---

### [x] Step 23: CLI Arguments

**What to do**: Add command-line arguments using `clap` for common operations.

**Update `src/main.rs`** — add at the top, before `main()`:

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "racetui", about = "TUI for racing series calendars")]
struct Cli {
    /// Force refresh all data (ignore cache)
    #[arg(long)]
    refresh: bool,

    /// Clear all cached data and exit
    #[arg(long)]
    clear_cache: bool,

    /// Only show events from this series (e.g., "f1", "nascar_cup")
    #[arg(long)]
    series: Option<String>,

    /// Start in calendar view instead of list view
    #[arg(long)]
    calendar: bool,
}
```

**Handle these arguments at the start of `main()`**:

```rust
let cli = Cli::parse();

// Handle --clear-cache
if cli.clear_cache {
    data::cache::clear_all_cache()?;
    println!("Cache cleared.");
    return Ok(());
}
```

After creating `config` but before creating `app`:
```rust
// Handle --refresh: set TTL to 0 so everything is re-fetched
let mut config = config;
if cli.refresh {
    config.cache_ttl_hours = 0;
}

// Handle --calendar: override default view
if cli.calendar {
    config.default_view = "calendar".to_string();
}
```

After creating `app`:
```rust
// Handle --series: set initial filter
if let Some(ref series_id) = cli.series {
    app.active_filter = app::FilterCategory::Series(series_id.clone());
}
```

**Verify**: `cargo run -- --help` should show the CLI usage. `cargo run -- --clear-cache` should clear cache and exit. `cargo run -- --refresh` should force-refresh all data.

**Commit message**: `feat: add CLI arguments for refresh, cache, and filtering`

---

### [x] Step 24: Notification Highlights for Favorites

**What to do**: Implement visual highlights for favorited series with events happening soon.

**Add to `App` struct** in `src/app.rs`:
```rust
/// Index for cycling through notification messages
pub notification_cycle_index: usize,
/// Tick counter for timing notification cycles
pub tick_count: u64,
```
Initialize both to `0` in `App::new()`.

**Add a method to `App`** in `src/app.rs`:
```rust
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
        let series_name = self.series_registry
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
```

**Update the `Tick` handler** in `main.rs`:
```rust
AppEvent::Tick => {
    app.tick_count += 1;
    // Cycle notifications every 3 seconds
    if app.tick_count % 3 == 0 {
        let notification_count = app.get_notifications().len();
        if notification_count > 0 {
            app.notification_cycle_index = (app.notification_cycle_index + 1) % notification_count;
        }
    }
}
```

**Update `status_bar.rs`**: On the right side of the status bar, if there are notifications, show the current one (indexed by `notification_cycle_index`) instead of just the fetch status:
```rust
let notifications = app.get_notifications();
if !notifications.is_empty() {
    let idx = app.notification_cycle_index % notifications.len();
    // Show the notification in Yellow
    let notif_text = &notifications[idx];
    // Render it
}
```

**Update `list_view.rs`**: Events from favorited series within the notification threshold get a subtle warm background highlight:
```rust
// In the row-building loop, after creating the Row:
// If this is a favorited series and event is within notification threshold
let row_style = if app.config.favorites.contains(&event.series_id)
    && event.next_session_time().map_or(false, |t| {
        let until = t.signed_duration_since(chrono::Utc::now());
        until > chrono::Duration::zero()
            && until <= chrono::Duration::hours(app.config.notification_threshold_hours as i64)
    })
{
    Style::default().bg(Color::Rgb(50, 40, 20))
} else {
    Style::default()
};
// Apply: Row::new(cells).style(row_style)
```

**Verify**: Favorite a series (press `f`) that has an upcoming event. The status bar should show a countdown notification.

**Commit message**: `feat: add notification highlights for favorited series`

---

### [x] Step 25: Polish and Error Handling

**What to do**: Final polish pass — improve error handling, add graceful degradation, and clean up the UI.

**Task list (implement all of these)**:

1. **Panic handler** — Add this at the very start of `main()`, before any terminal setup:
   ```rust
   let original_hook = std::panic::take_hook();
   std::panic::set_hook(Box::new(move |panic_info| {
       // Restore terminal before printing panic
       let _ = disable_raw_mode();
       let _ = stdout().execute(LeaveAlternateScreen);
       original_hook(panic_info);
   }));
   ```

2. **Logging to file** — Set up `tracing-subscriber` to log to `~/.local/share/racetui/racetui.log`:
   ```rust
   use tracing_subscriber::fmt;
   use std::fs::OpenOptions;

   let log_dir = dirs::data_dir().unwrap().join("racetui");
   std::fs::create_dir_all(&log_dir).ok();
   let log_file = OpenOptions::new()
       .create(true)
       .append(true)
       .open(log_dir.join("racetui.log"))
       .unwrap();
   tracing_subscriber::fmt()
       .with_writer(log_file)
       .with_ansi(false)
       .init();
   ```

3. **Empty state** — In `list_view.rs`, when `events.is_empty()`, render a centered message instead of an empty table:
   ```
   No events found.

   Try adjusting your filters or press 'r' to refresh data.
   ```
   Style: `Color::DarkGray`, centered.

4. **Graceful scraper failures** — Already handled (errors show in status bar). Additionally, in the filter panel, show a warning icon next to series that failed to load:
   - Check `app.fetch_status.get(series_id)` for each series in the filter panel.
   - If it's `FetchStatus::Error(_)`, show `⚠` next to the name.

5. **Terminal resize** — Ratatui handles resize automatically (the `draw` function is called every loop iteration, which re-layouts everything). Just make sure the `Resize` event is consumed (it already is).

6. **Narrow terminal handling** — Test with 80-column terminal. If the table is too wide, adjust constraints in `list_view.rs`:
   - Make "Circuit" column `Constraint::Max(25)` instead of `Length(25)`.
   - Make "Country" column `Constraint::Max(15)`.
   - This allows columns to shrink on narrow terminals.

7. **Config save verification** — The config is already saved on exit. Verify by:
   - Start app, press `f` on an event to favorite it, press `q`.
   - Restart app — the favorite should persist.

8. **Status message timeout** — Status messages (like "Refreshing..." or "Opened F1TV") should auto-clear after 3 seconds. In the `Tick` handler:
   ```rust
   // Auto-clear status message after 3 seconds
   // Track when the status message was set with a timestamp on App
   ```
   Add `pub status_message_set_at: Option<std::time::Instant>` to `App`. When setting `status_message`, also set `status_message_set_at = Some(Instant::now())`. In the Tick handler, if the message has been showing for > 3 seconds, clear it.

**Verify**: Test all edge cases: resize terminal, search for non-existent text, filter to empty results, open detail on events with no sessions, etc.

**Commit message**: `fix: polish UI, improve error handling, add logging`

---

### [x] Step 26: README

**What to do**: Create a comprehensive `README.md` in the project root.

**Content structure**:

```markdown
# 🏁 RaceTUI

A terminal user interface for viewing racing series calendars from around the world.

## Features

- 📅 Calendar data from 16+ racing series worldwide
- 📋 List view and calendar grid view
- 🔍 Search and filter by series, car type, or region
- ⭐ Favorite series with upcoming event notifications
- 📺 One-key livestream opening
- 💾 Intelligent caching (configurable TTL)
- ⏰ Auto-detected timezone conversion
- ⌨️ Vim-style and arrow key navigation

## Supported Series

| Series | Category | Data Source |
|---|---|---|
| Formula 1 | Open Wheel | Jolpica API |
| Formula 2 | Open Wheel | Web Scraping |
| Formula 3 | Open Wheel | Web Scraping |
| Formula E | Open Wheel | Web Scraping |
| IndyCar | Open Wheel | Web Scraping |
| NASCAR Cup | Stock Car | NASCAR API |
| NASCAR Xfinity | Stock Car | NASCAR API |
| NASCAR Trucks | Stock Car | NASCAR API |
| IMSA | Sports Car | Web Scraping |
| WEC | Sports Car | Web Scraping |
| MotoGP | Motorcycle | MotoGP API |
| WRC | Rally | Web Scraping |
| BTCC | Touring | Web Scraping |
| Super Formula | Open Wheel | Web Scraping |
| Super GT | Sports Car | Web Scraping |
| DTM | Touring | Web Scraping |

## Installation

```bash
cargo install --path .
```

### With headless browser support

For series that require JavaScript rendering:

```bash
cargo install --path . --features headless-browser
```

Requires Chrome or Chromium to be installed.

## Usage

```bash
racetui              # Start the TUI
racetui --refresh    # Force refresh all data
racetui --clear-cache # Clear cached data
racetui --series f1  # Show only F1 events
racetui --calendar   # Start in calendar view
```

## Keybindings

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Tab` | Toggle list/calendar view |
| `h` / `←` | Previous month (calendar) |
| `l` / `→` | Next month (calendar) |
| `Enter` | Show event details |
| `f` | Toggle favorite |
| `F` | Open filter panel |
| `/` | Search |
| `o` | Open stream link |
| `r` | Refresh data |
| `?` | Show help |
| `Esc` | Close popup/cancel |
| `q` | Quit |

## Configuration

Config file location: `~/.config/racetui/config.toml`

```toml
# Favorited series (shown with ★)
favorites = ["f1", "wec", "indycar"]

# Cache TTL in hours (default: 24)
cache_ttl_hours = 24

# Hours before event to show notification (default: 2)
notification_threshold_hours = 2

# Command to open URLs (default: xdg-open)
open_command = "xdg-open"

# Default view: "list" or "calendar"
default_view = "list"

# Hidden series
hidden_series = []
```

## License

MIT
```

**Verify**: Read through the README. Ensure all instructions are accurate and match the actual implementation.

**Commit message**: `docs: add comprehensive README`

---

## Series Data Source Reference

This table summarizes the data source strategy for each series. Refer to it when implementing scrapers.

| Series ID | Series Name | Data Source Strategy | URL/Endpoint | Needs JS? |
|---|---|---|---|---|
| `f1` | Formula 1 | **JSON API (Jolpica)** | `https://api.jolpi.ca/ergast/f1/current.json` | No |
| `f2` | Formula 2 | **JSON API (FOM Apigee)** | `https://api.formula1.com/v1/core-editorial-races/f2/listing` | No |
| `f3` | Formula 3 | **JSON API (FOM Apigee)** | `https://api.formula1.com/v1/core-editorial-races/f3/listing` | No |
| `formula_e` | Formula E | **JSON API (Pulselive)** | `https://api.formula-e.pulselive.com/formula-e/v1/` | No |
| `indycar` | IndyCar | HTML scraping (SSR) | `https://www.indycar.com/Schedule` | No |
| `nascar_cup` | NASCAR Cup | **Internal CDN API** | `https://cf.nascar.com/cacher/{year}/1/race_list_basic.json` | No |
| `nascar_xfinity` | NASCAR Xfinity | **Internal CDN API** | `https://cf.nascar.com/cacher/{year}/2/race_list_basic.json` | No |
| `nascar_trucks` | NASCAR Trucks | **Internal CDN API** | `https://cf.nascar.com/cacher/{year}/3/race_list_basic.json` | No |
| `imsa` | IMSA | Static curated JSON (11 rounds/yr) + Al Kamel timing | `https://www.imsa.com/weathertech/schedule/` | No |
| `wec` | WEC | HTML scraping (SSR) or static JSON (8 rounds/yr) | `https://www.fiawec.com/` | No |
| `motogp` | MotoGP | **JSON API (Pulselive)** | `https://api.motogp.pulselive.com/motogp/v1/results/events?seasonUuid={uuid}` | No |
| `wrc` | WRC | **JSON API (WRC internal)** | `https://api.wrc.com/contel-page/83388/calendar/active-season/` | No |
| `btcc` | BTCC | HTML scraping or **WordPress REST API** | `https://www.btcc.net/wp-json/wp/v2/` or `https://www.btcc.net/calendar/` | No |
| `super_formula` | Super Formula | HTML scraping (SSR) | `https://superformula.net/sf3/race_taxonomy/2026/` | No |
| `super_gt` | Super GT | HTML scraping or **WordPress REST API** | `https://supergt.net/wp-json/` or `https://supergt.net/en/` | No |
| `dtm` | DTM | Static curated JSON (8 rounds/yr) | Manual curation + `https://www.dtm.com/en/calendar` | No |
| `elms` | European Le Mans Series | HTML scraping (SSR, ACO/LMEM platform) | `https://www.europeanlemansseries.com/calendar` | No |
| `aslms` | Asian Le Mans Series | HTML scraping (SSR, ACO/LMEM platform) | `https://www.asianlemansseries.com/calendar` | No |
| `nls` | Nürburgring Langstrecken-Serie | HTML scraping (SSR, WordPress) | `https://www.nuerburgring-langstrecken-serie.de/en/calendar-2026/` | No |
| `gtwc_eu` | GT World Challenge Europe | HTML scraping (SSR, SRO platform) | `https://www.gt-world-challenge-europe.com/calendar` | No |
| `gtwc_am` | GT World Challenge America | HTML scraping (SSR, SRO platform) | `https://www.gt-world-challenge-america.com/calendar` | No |
| `igtc` | Intercontinental GT Challenge | HTML scraping (SSR, SRO platform) | `https://www.intercontinentalgtchallenge.com/calendar` | No |
| `supercars` | Repco Supercars Championship | HTML scraping (SSR) | `https://www.supercars.com/calendar` | No |
| `tcr_world` | TCR World Tour | HTML scraping (SSR) | `https://www.fiatcrworldtour.com/calendar/` | No |
| `indy_nxt` | INDY NXT by Firestone | HTML scraping (SSR, same as IndyCar) | `https://www.indynxt.com/schedule` | No |
| `f1_academy` | F1 Academy | **JSON API (FOM Apigee)** | `https://api.formula1.com/v1/core-editorial-races/f1academy/listing` | No |
| `moto2` | Moto2 | **JSON API (Pulselive)** — same as MotoGP | `https://api.motogp.pulselive.com/motogp/v1/results/seasons` | No |
| `moto3` | Moto3 | **JSON API (Pulselive)** — same as MotoGP | `https://api.motogp.pulselive.com/motogp/v1/results/seasons` | No |
| `worldsbk` | WorldSBK | HTML scraping (Dorna platform) | `https://www.worldsbk.com/en/calendar` | No |
| `bsb` | British Superbike Championship | HTML scraping (SSR) | `https://www.britishsuperbike.com/calendar` | No |
| `wrc2` | WRC2 | **JSON API (WRC)** — same as WRC | `https://api.wrc.com/contel-page/83388/calendar/active-season/` | No |
| `dakar` | Dakar Rally | HTML scraping or static JSON (1 event/yr) | `https://www.dakar.com/en/calendar` | No |
| `erc` | European Rally Championship | **JSON API (WRC Promoter)** — same backend as WRC | `https://www.fiaerc.com/calendar` | No |
| `porsche_supercup` | Porsche Supercup | HTML scraping (SSR) | `https://motorsports.porsche.com/` | No |
| `arca` | ARCA Menards Series | **Internal CDN API** — same as NASCAR | `https://cf.nascar.com/cacher/2026/4/race_list_basic.json` | No |
| `extreme_e` | Extreme E / Extreme H | HTML scraping (SSR) | `https://www.extreme-e.com/calendar` | No |

---

## Color Assignments

Each series has a unique RGB color defined in `data/series.toml`. These were chosen to:
1. Be visually distinct from each other.
2. Roughly match the series' brand colors where possible.
3. Be readable on a dark terminal background.

| Series | Color (RGB) | Hex | Visual |
|---|---|---|---|
| F1 | (255, 24, 1) | #FF1801 | Red |
| F2 | (0, 129, 210) | #0081D2 | Blue |
| F3 | (228, 0, 43) | #E4002B | Crimson |
| Formula E | (0, 174, 239) | #00AEEF | Cyan/Teal |
| IndyCar | (0, 0, 128) | #000080 | Navy |
| NASCAR Cup | (255, 200, 0) | #FFC800 | Yellow |
| NASCAR Xfinity | (0, 100, 200) | #0064C8 | Medium Blue |
| NASCAR Trucks | (200, 50, 50) | #C83232 | Dark Red |
| IMSA | (0, 150, 68) | #009644 | Green |
| WEC | (0, 85, 150) | #005596 | Dark Blue |
| MotoGP | (190, 0, 50) | #BE0032 | Deep Red |
| WRC | (37, 100, 200) | #2564C8 | Royal Blue |
| BTCC | (0, 60, 120) | #003C78 | Dark Navy |
| Super Formula | (230, 0, 126) | #E6007E | Pink/Magenta |
| Super GT | (200, 160, 0) | #C8A000 | Gold |
| DTM | (130, 0, 200) | #8200C8 | Purple |
| ELMS | (0, 114, 206) | #0072CE | Blue |
| AsLMS | (230, 80, 20) | #E65014 | Orange-Red |
| NLS | (20, 128, 60) | #14803C | Green |
| GTWC Europe | (235, 90, 0) | #EB5A00 | Orange |
| GTWC America | (180, 30, 50) | #B41E32 | Crimson |
| IGTC | (212, 175, 55) | #D4AF37 | Gold |
| Supercars | (230, 50, 20) | #E63214 | Red-Orange |
| TCR World | (220, 30, 30) | #DC1E1E | Red |
| INDY NXT | (210, 25, 30) | #D2191E | Racing Red |
| F1 Academy | (140, 40, 200) | #8C28C8 | Violet |
| Moto2 | (70, 130, 200) | #4682C8 | Metallic Blue |
| Moto3 | (240, 100, 0) | #F06400 | Bright Orange |
| WorldSBK | (210, 10, 10) | #D20A0A | Flame Red |
| BSB | (170, 15, 40) | #AA0F28 | Crimson |
| WRC2 | (100, 160, 220) | #64A0DC | Light Blue |
| Dakar | (215, 115, 30) | #D7731E | Desert Ochre |
| ERC | (255, 185, 0) | #FFB900 | Gold-Yellow |
| Porsche Supercup | (180, 0, 30) | #B4001E | Porsche Red |
| ARCA | (235, 235, 0) | #EBEB00 | Neon Yellow |
| Extreme E/H | (50, 220, 180) | #32DCB4 | Eco Cyan |
