# FEATURES_PLAN.md — RaceTUI Live Features Implementation Plan

> **Purpose**: This document is a step-by-step implementation plan for the live data features in RaceTUI.
> Each step is designed to be a single git commit. Steps must be completed **in order**.
> Every design decision, file path, data structure, and implementation detail is specified so that
> an AI coding agent can implement each step without needing to make any design decisions.
>
> **Phases**: The plan is organized into three sequential phases:
> - **Phase 1** (Steps 1–18): Core infrastructure, championship standings, and race results
> - **Phase 2** (Steps 19–31): Live timing, weather, and tire data
> - **Phase 3** (Steps 32–44): Track maps, notifications, and daemon mode

---

## Technology Stack

### Existing Dependencies (Do Not Change)

| Component | Choice | Version |
|---|---|---|
| Language | Rust (2021 edition) | stable toolchain |
| TUI framework | `ratatui` | latest (≥0.30) |
| Terminal backend | `crossterm` | latest (≥0.29), features: `["event-stream"]` |
| Async runtime | `tokio` | 1.x, features: `["rt-multi-thread", "macros", "time", "sync"]` |
| HTTP client | `reqwest` | latest (≥0.12), features: `["json", "cookies"]` |
| HTML scraping | `scraper` | latest (≥0.27) |
| Headless browser | `chromiumoxide` | latest, features: `["tokio-runtime"]`, optional |
| Serialization | `serde` + `serde_json` | 1.x, serde features: `["derive"]` |
| Config format | `toml` | latest (≥0.8) |
| Date/time | `chrono` | 0.4, features: `["serde"]` |
| Timezone | `chrono-tz` | latest (≥0.10), features: `["serde"]` |
| XDG paths | `dirs` | latest (≥6.x) |
| CLI args | `clap` | 4.x, features: `["derive"]` |
| Error handling | `anyhow` + `color-eyre` | 1.x / 0.6 |
| Logging | `tracing` + `tracing-subscriber` | latest |
| Futures | `futures` | latest (≥0.3) |

### New Dependencies to Add

| Component | Choice | Version | Purpose |
|---|---|---|---|
| WebSocket client | `tokio-tungstenite` | latest (≥0.26) | WebSocket connections for live timing feeds (F1 SignalR, Al Kamel, etc.) |
| Zlib decompression | `miniz_oxide` | latest (≥0.8) | Pure Rust zlib decompression for F1 SignalR compressed streams (Phase 2 SignalR enhancement) |
| Desktop notifications | `notify-rust` | latest (≥4) | Linux D-Bus desktop notifications (Phase 3) |

---

## Directory Structure (Final Target)

```
racetui/
├── Cargo.toml                         # Updated with new dependencies
├── VISION.md
├── FEATURES_PLAN.md
├── data/
│   └── series.toml                    # Existing series metadata
├── src/
│   ├── main.rs                        # Updated: new view modes, live event handling, daemon mode
│   ├── app.rs                         # Updated: new view modes, live state, standings/results state
│   ├── event.rs                       # Updated: new AppEvent variants for results/standings
│   ├── config/
│   │   ├── mod.rs                     # Existing
│   │   └── settings.rs               # Updated: [daemon] config section, notification prefs, live polling interval
│   ├── data/
│   │   ├── mod.rs                     # Updated: export new modules
│   │   ├── models.rs                  # Existing (unchanged)
│   │   ├── cache.rs                   # Existing (unchanged)
│   │   ├── series_registry.rs         # Existing (unchanged)
│   │   ├── standings.rs              # NEW: DriverStanding, ConstructorStanding, SeasonStandings structs
│   │   └── results.rs                # NEW: RaceResult, DriverResult, ResultsCache read/write
│   ├── scraper/
│   │   ├── mod.rs                     # Updated: export standings/results fetcher traits
│   │   ├── f1.rs                      # Updated: add standings + results fetcher methods
│   │   ├── nascar.rs                  # Updated: add standings + results fetcher methods
│   │   ├── indycar.rs                 # Updated: add standings + results fetcher methods
│   │   ├── motogp.rs                  # Updated: add standings + results fetcher methods
│   │   ├── wrc.rs                     # Updated: add standings + results fetcher methods
│   │   ├── formula_e.rs              # Updated: add standings + results fetcher methods
│   │   ├── imsa.rs                    # Updated: add standings + results fetcher methods
│   │   ├── wec.rs                     # Updated: add standings + results fetcher methods
│   │   ├── f1_live.rs                # NEW: F1 live timing provider (OpenF1 REST polling)
│   │   ├── nascar_live.rs            # NEW: NASCAR live timing provider (cf.nascar.com feeds)
│   │   ├── indycar_live.rs           # NEW: IndyCar live timing provider (racecontrol JSON)
│   │   ├── alkamelsystems_live.rs    # NEW: Al Kamel live timing (IMSA, WEC, Formula E)
│   │   ├── motogp_live.rs           # NEW: MotoGP live timing provider
│   │   ├── wrc_live.rs              # NEW: WRC live timing provider
│   │   └── (all other existing scraper files unchanged)
│   ├── live/
│   │   ├── mod.rs                     # NEW: Live timing infrastructure, LiveProvider trait, manager
│   │   ├── event.rs                   # NEW: LiveEvent enum
│   │   ├── manager.rs                # NEW: LiveSessionManager — orchestrates polling, dispatches LiveEvents
│   │   ├── track_map.rs              # NEW: TrackMap widget — braille rendering engine
│   │   └── weather.rs                # NEW: WeatherData struct and display widget
│   ├── ui/
│   │   ├── mod.rs                     # Updated: dispatch to new view modes
│   │   ├── list_view.rs              # Updated: 🏁 indicator for completed events with results
│   │   ├── calendar_view.rs          # Updated: 🏁 indicator
│   │   ├── detail_view.rs            # Updated: show race results for completed events
│   │   ├── status_bar.rs             # Updated: blinking LIVE indicator
│   │   ├── filter_panel.rs           # Existing (unchanged)
│   │   ├── help.rs                    # Updated: new keybindings documentation
│   │   ├── confirm_dialog.rs         # Existing (unchanged)
│   │   ├── day_events.rs             # Existing (unchanged)
│   │   ├── standings_view.rs         # NEW: Championship standings view with series selector
│   │   ├── live_view.rs              # NEW: Live timing view (timing table + track map sub-tabs)
│   │   ├── live_timing_table.rs      # NEW: Compact leaderboard widget
│   │   ├── live_driver_detail.rs     # NEW: Expanded driver detail panel (tire, pit, sector times)
│   │   ├── live_session_picker.rs    # NEW: Popup to select which live session to watch
│   │   ├── weather_box.rs            # NEW: Small weather info box widget
│   │   └── track_map_view.rs         # NEW: Track map rendering view with driver legend
│   └── notify/
│       ├── mod.rs                     # NEW: Notification infrastructure
│       ├── desktop.rs                # NEW: notify-rust D-Bus notifications
│       ├── terminal.rs               # NEW: OSC 777/99 terminal escape sequence notifications
│       └── scheduler.rs             # NEW: Notification scheduler (checks upcoming sessions)
├── tests/
│   ├── fixtures/                      # NEW: Saved API response JSON fixtures for testing
│   │   ├── openf1_sessions.json
│   │   ├── openf1_drivers.json
│   │   ├── openf1_laps.json
│   │   ├── openf1_weather.json
│   │   ├── openf1_location.json
│   │   ├── nascar_live_feed.json
│   │   ├── nascar_standings.json
│   │   ├── jolpica_standings.json
│   │   └── jolpica_results.json
│   ├── standings_tests.rs            # NEW: Mock-based standings parsing tests
│   ├── results_tests.rs             # NEW: Mock-based results parsing tests
│   └── live_timing_tests.rs         # NEW: Mock-based live timing parsing tests
```

---

## Design Decisions Reference

These decisions have been made. Do not deviate from them.

### Architecture
- **Four view modes**: List (key `1`), Calendar (key `2`), Live (key `3`), Standings (key `4`). Number keys `1`–`4` switch views globally. The `Tab` key no longer toggles views — it is freed for future use.
- **`1`–`9` keys for stream links** continue to work **only** inside the Event Details popup. There is no conflict with global `1`–`4` view switching because the popup captures all input.
- **Live view has two sub-tabs**: "Timing" and "Track Map", toggled with **Left/Right arrow keys**.
- **Weather** is a persistent info box always visible in the Live view (not a sub-tab).
- **Standings** is a full-screen view with a **dropdown selector** at the top to pick which series to display.
- **Results** appear inside the **enhanced detail popup** when viewing a completed event. No separate results view.
- **Completed events with results** are marked with a `🏁` icon in list/calendar views.
- **Live indicator**: An animated/blinking `LIVE` badge with series abbreviation appears in the status bar across ALL views when any session is live.

### Live Timing
- **Polling model**: HTTP REST polling via `reqwest`, with configurable interval (default: 2 seconds). Stored in `config.toml` as `live_poll_interval_secs`.
- **F1 data source**: OpenF1 REST API (`api.openf1.org/v1/`) for everything — drivers, laps, intervals, car data, location, weather, stints, pit stops.
- **NASCAR data source**: `cf.nascar.com/live/feeds/live-feed.json` and related endpoints.
- **Series rollout order**: F1 + NASCAR first, then IndyCar, then Al Kamel (IMSA, WEC, Formula E), then MotoGP + WRC.
- **Auto-detect**: When the app detects a session is live, show a status bar prompt offering to switch to Live view. Do NOT auto-switch.
- **Multiple concurrent sessions**: Show a dropdown/list popup when pressing `3` if multiple sessions are live, letting the user pick one.
- **Live data is ephemeral**: Keep live timing data in memory only. Do NOT cache live timing snapshots to disk.
- **Race results ARE cached**: Final race classifications are stored as separate JSON files in `~/.local/share/racetui/results/{series_id}_round_{N}.json`.
- **Tire data**: Shown in the live timing driver detail expansion for any series whose feed reports tire compounds. Display format: compound name + lap count (e.g., "Soft (12 laps)").
- **WebSocket support**: Add `tokio-tungstenite` dependency. Used for F1 SignalR in a future enhancement and for Al Kamel/other WebSocket feeds.

### Track Maps
- **Rendering**: Braille characters (`⠀⠁⠂...`) using ratatui's `Canvas` widget for highest resolution (2×4 dots per terminal cell).
- **Circuit geometry**: Fetched from MultiViewer API (`api.multiviewer.app/api/v1/circuits/{circuit_key}/{year}`).
- **Driver representation**: Colored dots on the track with a separate legend panel listing driver abbreviations mapped to their colors/numbers.
- **Scope**: Generic `TrackMap` widget that works with any x/y coordinate data, but only populated for F1 initially (via OpenF1 `/location` endpoint).

### Standings
- **Data source**: New dedicated standings fetchers for each series (not reusing calendar scrapers).
- **Scope**: All supported series that have accessible standings APIs.
- **Granularity**: Full-season cumulative standings only (no per-round points breakdown).
- **Standings and results are separate features** — the standings view shows only the points table, not individual race results.

### Results
- **Fetching**: Background fetch at startup for the **most recent completed event per series**. Results for older events are NOT pre-fetched.
- **Storage**: Separate cached JSON files per event at `~/.local/share/racetui/results/{series_id}_round_{N}.json`.
- **Data fields**: Final positions, gaps to leader/interval, points earned, fastest lap indicator, penalty info, starting grid position (positions gained/lost).
- **F1 source**: OpenF1 `/results` + Jolpica API.
- **NASCAR source**: `cf.nascar.com` race results endpoints.
- **Other series**: Scrape from official results pages.

### Notifications
- **Mechanism**: Try `notify-rust` (D-Bus) first, fall back to terminal OSC 777 escape sequences.
- **Daemon mode**: `racetui --daemon` CLI flag starts a headless loop that only checks for upcoming sessions and sends notifications. No TUI is rendered.
- **Config**: Same `config.toml` file with a `[daemon]` TOML section for daemon-specific settings.
- **Configurable options**: Minutes before session start (default: 30), session type filters (default: all), series filters (default: favorites only), notification sound on/off (default: on).

### Events & State
- **Separate `LiveEvent` enum**: Live timing events use their own enum (`LiveEvent`) and a separate mpsc channel, keeping `AppEvent` for calendar/UI events.
- **Module organization**: `src/live/` for generic live timing infrastructure. Per-series live providers live alongside their scrapers in `src/scraper/` (e.g., `f1_live.rs`).
- **Data models**: `src/data/standings.rs` for standings types, `src/data/results.rs` for results types and results cache logic.

---

## Phase 1: Core Infrastructure, Championship Standings, and Race Results

---

### [x] Step 1: Add new dependencies to Cargo.toml

**What to do**: Add the three new crate dependencies needed across all phases. Adding them all now avoids incremental `Cargo.toml` edits later.

**Update `Cargo.toml`**:

Add these lines to the `[dependencies]` section:

```toml
tokio-tungstenite = { version = "0.26", features = ["native-tls"] }
miniz_oxide = "0.8"
notify-rust = "4"
```

**Verify**: Run `cargo check` to ensure all new dependencies resolve and compile.

**Commit message**: `chore: add tokio-tungstenite, miniz_oxide, and notify-rust dependencies`

---

### [x] Step 2: Define championship standings data models

**What to do**: Create `src/data/standings.rs` with the data structures needed to represent championship standings for any series. These models must support both driver-only standings (e.g., IndyCar) and driver + constructor/manufacturer standings (e.g., F1, NASCAR).

**Create `src/data/standings.rs`** with these exact contents:

```rust
use serde::{Deserialize, Serialize};

/// A single entry in a driver championship standings table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverStanding {
    /// Current championship position (1-based)
    pub position: u32,
    /// Driver's full name (e.g., "Max Verstappen")
    pub driver_name: String,
    /// Driver's short abbreviation (e.g., "VER"), if available
    pub driver_code: Option<String>,
    /// Driver's permanent number (e.g., 1), if applicable
    pub driver_number: Option<u32>,
    /// Team/constructor name (e.g., "Red Bull Racing")
    pub team: String,
    /// Total championship points
    pub points: f64,
    /// Number of wins this season
    pub wins: u32,
}

/// A single entry in a constructor/manufacturer/team championship standings table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorStanding {
    /// Current championship position (1-based)
    pub position: u32,
    /// Constructor/team/manufacturer name (e.g., "Red Bull Racing")
    pub name: String,
    /// Total championship points
    pub points: f64,
    /// Number of wins this season
    pub wins: u32,
}

/// Complete championship standings for a single series in a single season.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonStandings {
    /// Series ID (e.g., "f1", "nascar_cup")
    pub series_id: String,
    /// Season year (e.g., 2026)
    pub season: u32,
    /// Driver championship standings (always present)
    pub drivers: Vec<DriverStanding>,
    /// Constructor/manufacturer standings (empty if the series doesn't have them)
    pub constructors: Vec<ConstructorStanding>,
    /// When this data was last fetched (UTC timestamp)
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

impl SeasonStandings {
    /// Returns true if this series has constructor/manufacturer standings.
    pub fn has_constructor_standings(&self) -> bool {
        !self.constructors.is_empty()
    }
}
```

**Update `src/data/mod.rs`**:

Change from:
```rust
pub mod cache;
pub mod models;
```
to:
```rust
pub mod cache;
pub mod models;
pub mod standings;
```

> **Note**: The `results` module will be added in Step 3.

**Verify**: `cargo check`

**Commit message**: `feat: define championship standings data models`

---

### [x] Step 3: Define race results data models and results cache

**What to do**: Create `src/data/results.rs` with data structures for race results AND the cache read/write logic for storing results as separate JSON files per event in `~/.local/share/racetui/results/`.

**Create `src/data/results.rs`** with these exact contents:

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single driver's result in a completed race.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverResult {
    /// Finishing position (1-based). None if DNF/DNS/DSQ.
    pub position: Option<u32>,
    /// Driver's full name
    pub driver_name: String,
    /// Driver's short abbreviation (e.g., "VER"), if available
    pub driver_code: Option<String>,
    /// Driver's permanent number, if applicable
    pub driver_number: Option<u32>,
    /// Team/constructor name
    pub team: String,
    /// Gap to race leader as a display string (e.g., "+5.123s", "+1 Lap", "DNF")
    pub gap_to_leader: String,
    /// Gap to car ahead as a display string (e.g., "+1.456s")
    pub gap_to_ahead: String,
    /// Starting grid position (for calculating positions gained/lost)
    pub grid_position: Option<u32>,
    /// Points earned in this race
    pub points: f64,
    /// Whether this driver set the fastest lap
    pub fastest_lap: bool,
    /// Penalty information, if any (e.g., "+5s Time Penalty")
    pub penalty: Option<String>,
    /// Classification status (e.g., "Finished", "DNF", "DNS", "DSQ")
    pub status: String,
}

impl DriverResult {
    /// Calculate positions gained or lost from grid to finish.
    /// Positive = gained, negative = lost. None if grid or finish position unknown.
    pub fn positions_gained(&self) -> Option<i32> {
        match (self.grid_position, self.position) {
            (Some(grid), Some(finish)) => Some(grid as i32 - finish as i32),
            _ => None,
        }
    }
}

/// Complete race results for a single event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResults {
    /// Series ID (e.g., "f1")
    pub series_id: String,
    /// Round number within the season
    pub round: u32,
    /// Event name (e.g., "Monaco Grand Prix")
    pub event_name: String,
    /// Circuit name
    pub circuit_name: String,
    /// Race date
    pub race_date: chrono::NaiveDate,
    /// Individual driver results, ordered by finishing position
    pub results: Vec<DriverResult>,
    /// When this data was last fetched (UTC timestamp)
    pub fetched_at: DateTime<Utc>,
}

/// Get the results cache directory: ~/.local/share/racetui/results/
fn results_cache_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("racetui")
        .join("results");
    Ok(data_dir)
}

/// Build the cache file path for a specific event's results.
/// Format: ~/.local/share/racetui/results/{series_id}_round_{round}.json
fn results_cache_path(series_id: &str, round: u32) -> Result<PathBuf> {
    let dir = results_cache_dir()?;
    Ok(dir.join(format!("{}_round_{}.json", series_id, round)))
}

/// Write race results to the cache file.
pub fn write_results_cache(results: &RaceResults) -> Result<()> {
    let path = results_cache_path(&results.series_id, results.round)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create results cache dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(results)
        .context("Failed to serialize race results")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write results cache to {}", path.display()))?;
    tracing::debug!("Cached results for {} round {} at {}", results.series_id, results.round, path.display());
    Ok(())
}

/// Read race results from the cache file. Returns None if no cache exists.
pub fn read_results_cache(series_id: &str, round: u32) -> Result<Option<RaceResults>> {
    let path = results_cache_path(series_id, round)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read results cache at {}", path.display()))?;
    let results: RaceResults = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse results cache at {}", path.display()))?;
    Ok(Some(results))
}

/// List all cached result files for a series. Returns (round, path) pairs.
pub fn list_cached_results(series_id: &str) -> Result<Vec<(u32, PathBuf)>> {
    let dir = results_cache_dir()?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let prefix = format!("{}_round_", series_id);
    let mut results = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.starts_with(&prefix) && filename.ends_with(".json") {
            let round_str = filename
                .strip_prefix(&prefix)
                .and_then(|s| s.strip_suffix(".json"));
            if let Some(round_str) = round_str {
                if let Ok(round) = round_str.parse::<u32>() {
                    results.push((round, entry.path()));
                }
            }
        }
    }
    Ok(results)
}
```

**Update `src/data/mod.rs`** to add `pub mod results;`:

```rust
pub mod cache;
pub mod models;
pub mod results;
pub mod standings;
```

**Verify**: `cargo check`

**Commit message**: `feat: define race results data models and cache infrastructure`

---

### [x] Step 4: Update ViewMode enum and App state for new views

**What to do**: Extend the `ViewMode` enum to include `Live` and `Standings` modes. Add new fields to the `App` struct for standings data, results data, live session state, and the new view-related UI state.

**Update `src/app.rs`**:

1. Change the `ViewMode` enum from:
```rust
pub enum ViewMode {
    List,
    Calendar,
}
```
to:
```rust
pub enum ViewMode {
    List,
    Calendar,
    Live,
    Standings,
}
```

2. Add a new enum for Live view sub-tabs, right after `ViewMode`:
```rust
/// Sub-tabs within the Live view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveSubTab {
    /// Timing leaderboard
    Timing,
    /// Track map with driver positions
    TrackMap,
}
```

3. Add these new fields to the `App` struct (after the existing `tick_count` field):

```rust
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
    pub live_timing_data: Option<crate::live::event::LiveTimingData>,
    /// Cached circuit geometry for the current live session
    pub track_map_geometry: Option<Vec<crate::live::track_map::TrackPoint>>,
```

4. In `App::new()`, initialize all new fields with defaults. Add after the existing field initializations:

```rust
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
```

5. Add a helper method to `App` for detecting live sessions:

```rust
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
        } else {
            if current_idx == 0 {
                series_ids.len() - 1
            } else {
                current_idx - 1
            }
        };
        self.standings_selected_series = Some(series_ids[new_idx].clone());
        self.standings_series_index = new_idx;
        self.standings_table_state = TableState::default();
    }
```

> **Note**: The `crate::live::event::LiveTimingData` and `crate::live::track_map::TrackPoint` types referenced here don't exist yet — they will be created in Phase 2 (Step 19). For now, use `Option<()>` as a placeholder for `live_timing_data` and `track_map_geometry`, and replace them in Step 19 when the types are defined. Alternatively, create stub types in `src/live/event.rs` and `src/live/track_map.rs` with just enough to compile.

**Verify**: `cargo check`

**Commit message**: `feat: extend ViewMode with Live and Standings, add new App state fields`

---

### [x] Step 5: Update AppEvent enum for standings and results

**What to do**: Add new variants to the `AppEvent` enum for receiving standings data and race results from background fetch tasks.

**Update `src/event.rs`**:

Add these imports at the top:
```rust
use crate::data::standings::SeasonStandings;
use crate::data::results::RaceResults;
```

Add these new variants to the `AppEvent` enum:

```rust
    /// Championship standings data was fetched for a series
    StandingsFetched {
        series_id: String,
        standings: SeasonStandings,
    },
    /// Championship standings fetch failed
    StandingsFetchError {
        series_id: String,
        error: String,
    },
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
```

**Verify**: `cargo check`

**Commit message**: `feat: add standings and results variants to AppEvent`

---

### [x] Step 6: Update keybindings — number keys for view switching

**What to do**: Change the global keybinding scheme so keys `1`–`4` switch between view modes. Remove the `Tab` key view toggle.

**Update the `handle_key_event` function in `src/main.rs`**:

1. **Remove** the existing `Tab` handler that toggles between List and Calendar. Find the match arm for `KeyCode::Tab` that switches `app.view_mode` and remove it.

2. **Add** new match arms for `KeyCode::Char('1')` through `KeyCode::Char('4')` in the **global** key handler section (when no popup/overlay is active):

```rust
KeyCode::Char('1') => {
    app.view_mode = ViewMode::List;
}
KeyCode::Char('2') => {
    app.view_mode = ViewMode::Calendar;
}
KeyCode::Char('3') => {
    app.view_mode = ViewMode::Live;
}
KeyCode::Char('4') => {
    app.view_mode = ViewMode::Standings;
}
```

3. Ensure the `1`–`9` match arms inside the detail popup handler remain unchanged.

**Verify**: `cargo run`, press `1`–`4` to switch views. `Tab` should do nothing.

**Commit message**: `feat: switch to number-key view navigation (1=List, 2=Calendar, 3=Live, 4=Standings)`

---

### [x] Step 7: Create Standings view UI scaffold

**What to do**: Create the standings view UI component with a series dropdown selector at the top and a standings table below.

**Create `src/ui/standings_view.rs`**:

The view should have this layout:
```
┌─────────────────────────────────────┐
│  Series: ◀ Formula 1 ▶  (←/→)     │  <- Series selector (1 line)
├─────────────────────────────────────┤
│  Pos  #   Driver          Team  Pts│  <- Driver standings table
│  1    1   Max Verstappen  RBR   285│
├─────────────────────────────────────┤
│  Pos  Constructor          Pts     │  <- Constructor standings (if applicable)
│  1    Red Bull Racing      520     │
└─────────────────────────────────────┘
```

Implement:
1. Series selector bar at the top showing current series name with `◀ ▶` arrows.
2. Driver standings table with columns: Pos, #, Driver, Team, Wins, Points.
3. Constructor standings table below (only if `standings.has_constructor_standings()`).
4. Split area 60% driver / 40% constructor when both exist.
5. "No standings data loaded yet" placeholder when standings map is empty.
6. Auto-select first favorited series (or first alphabetically) when none selected.

**Update `src/ui/mod.rs`**: Add `pub mod standings_view;`, add `ViewMode::Standings` dispatch arm.

**Add Standings view keybindings** to `handle_key_event` in `src/main.rs`:
- `Left`/`h` → `app.standings_cycle_series(-1)`
- `Right`/`l` → `app.standings_cycle_series(1)`
- `Down`/`j` → `app.standings_table_state.select_next()`
- `Up`/`k` → `app.standings_table_state.select_previous()`

**Verify**: `cargo run`, press `4`. Should show placeholder message.

**Commit message**: `feat: implement standings view UI scaffold with series selector`

---

### [x] Step 8: Define StandingsFetcher and ResultsFetcher traits, implement F1 standings

**What to do**: Define the `StandingsFetcher` and `ResultsFetcher` traits in `src/scraper/mod.rs`. Implement F1 standings fetching via Jolpica API.

**Update `src/scraper/mod.rs`**:

Add imports and the two new traits (`StandingsFetcher`, `ResultsFetcher`) with their object-safe boxed variants, plus `get_standings_fetcher()` and `get_results_fetcher()` dispatch functions. See the Design Decisions Reference for the trait signatures. Follow the same pattern as the existing `SeriesScraper` / `SeriesScraperBoxed` traits.

**Update `src/scraper/f1.rs`**:

1. Add Jolpica standings API response structs (`JolpicaStandingsResponse`, `StandingsMRData`, `StandingsTable`, `StandingsList`, `JolpicaDriverStanding`, `JolpicaDriver`, `JolpicaConstructor`, `JolpicaConstructorStanding`).
2. Implement `StandingsFetcher` for `F1Scraper`:
   - Fetch `https://api.jolpica.com/ergast/f1/{season}/driverStandings.json`
   - Fetch `https://api.jolpica.com/ergast/f1/{season}/constructorStandings.json`
   - Map to `SeasonStandings` with both `drivers` and `constructors` vectors.

**Verify**: `cargo check`

**Commit message**: `feat: define StandingsFetcher trait, implement F1 standings via Jolpica API`

---

### [x] Step 9: Implement F1 results fetcher

**What to do**: Implement `ResultsFetcher` for F1 using the Jolpica API.

**Update `src/scraper/f1.rs`**:

1. Add Jolpica results API response structs (`JolpicaResultsResponse`, `ResultsMRData`, `ResultsRaceTable`, `JolpicaRaceResult`, `JolpicaDriverResult`, `JolpicaTime`, `JolpicaFastestLap`).
2. Implement `ResultsFetcher` for `F1Scraper`:
   - Fetch `https://api.jolpica.com/ergast/f1/{season}/{round}/results.json`
   - Map to `RaceResults` with full `DriverResult` entries.

**Verify**: `cargo check`

**Commit message**: `feat: implement F1 race results fetcher via Jolpica API`

---

### [x] Step 10: Implement NASCAR standings and results fetchers

**What to do**: Add `StandingsFetcher` and `ResultsFetcher` implementations for NASCAR using `cf.nascar.com` CDN API.

**Update `src/scraper/nascar.rs`**:
1. Add NASCAR standings response structs.
2. Implement `StandingsFetcher` — fetch `https://cf.nascar.com/cws/v1/series/{id}/standings/{year}.json`.
3. Add NASCAR results response structs.
4. Implement `ResultsFetcher` — fetch `https://cf.nascar.com/cws/v1/series/{id}/{year}/results.json`.

**Update `get_standings_fetcher`/`get_results_fetcher` in `src/scraper/mod.rs`** with NASCAR match arms.

**Verify**: `cargo check`

**Commit message**: `feat: implement NASCAR standings and results fetchers`

---

### [x] Step 11: Implement standings and results fetchers for remaining series

**What to do**: Add `StandingsFetcher` and `ResultsFetcher` implementations for all remaining series with accessible APIs.

**Series and data sources**:

| Series | Standings Source | Results Source |
|---|---|---|
| IndyCar / INDY NXT | Scrape `indycar.com/standings` | Scrape `indycar.com/results` |
| Formula E | `results.fiaformulae.com` | `results.fiaformulae.com` |
| IMSA | `results.imsa.com` | `results.imsa.com` |
| WEC | `fiawec.alkamelsystems.com` | `fiawec.alkamelsystems.com` |
| MotoGP / Moto2 / Moto3 | `resources.motogp.com/files/results/` | `resources.motogp.com/files/results/` |
| WRC / WRC2 | `api.wrc.com/results-api/` | `api.wrc.com/results-api/` |
| F2 / F3 / F1 Academy | FIA results pages | FIA results pages |
| DTM | Official DTM website | DTM results |
| BTCC / BSB | TSL Timing results | TSL results |
| WorldSBK | `worldsbk.com` results | WorldSBK results |
| Super GT / Super Formula | Official schedule sites | Scrape results |
| Supercars | `supercars.com` | Official results |
| Others | Scrape official websites | Scrape results |

For each series: implement the traits following F1/NASCAR patterns. For series with no API, return `Err(anyhow!("...not yet available"))` for graceful degradation.

**Update `get_standings_fetcher()`/`get_results_fetcher()` with all new match arms.**

**Verify**: `cargo check`, `cargo run` → press `4` → cycle through series.

**Commit message**: `feat: implement standings and results fetchers for all supported series`

---

### [x] Step 12: Wire up standings and results background fetching

**What to do**: Spawn background tasks at startup to fetch standings for all series and results for the most recent completed event per series.

**Update `src/main.rs`**:

1. Add `spawn_standings_loaders()` function — iterates all series, calls `get_standings_fetcher()`, spawns tokio tasks, sends `AppEvent::StandingsFetched`.
2. Add `spawn_results_loaders()` function — for each series with events, find the most recent completed event, check cache, fetch if needed, send `AppEvent::ResultsFetched`.
3. Call `spawn_standings_loaders()` after `spawn_data_loaders()`.
4. Call `spawn_results_loaders()` after first batch of `SeriesDataFetched` events arrive (use the `results_loading_started` flag).
5. Handle all new `AppEvent` variants in the main event loop.

**Verify**: `cargo run` — standings should populate in the background. Press `4` to see standings data.

**Commit message**: `feat: wire up background standings and results fetching at startup`

---

### [x] Step 13: Enhance detail popup to show race results for completed events

**What to do**: Update `src/ui/detail_view.rs` so that when a completed event is selected and results are available, the popup shows the race results table.

**Update `src/ui/detail_view.rs`**:

1. Add a `draw_results()` function rendering a table with columns: P, #, Driver, Team, Gap, Grid, +/- (positions gained/lost), Pts, Status.
2. In the main `draw()`, check if event is Completed and has results in `app.results`. If so, split the popup: 30% event info, 70% results table.
3. Style: positions gained in green (▲), lost in red (▼). Fastest lap marked with 🟣. Penalties shown in status.

**Verify**: `cargo run`, filter for Completed events, select one with results → popup shows results.

**Commit message**: `feat: show race results in detail popup for completed events`

---

### [x] Step 14: Add 🏁 results indicator in list and calendar views

**What to do**: When a completed event has results available, show `🏁` next to the event name in both views.

**Update `src/ui/list_view.rs`** and **`src/ui/calendar_view.rs`**:

Check `event.status == Completed && event.round.map_or(false, |r| app.results.contains_key(&(event.series_id.clone(), r)))`. If true, prepend `🏁` to the event name display.

**Verify**: `cargo run`, look for `🏁` on completed events.

**Commit message**: `feat: add 🏁 results indicator for completed events in list and calendar views`

---

### [x] Step 15: Add global LIVE session indicator in status bar

**What to do**: Add an animated/blinking "🔴 LIVE" indicator in the status bar across all views when any session is live.

**Update `src/ui/status_bar.rs`**: Use `app.get_live_sessions()` and `app.live_blink_on` to render the indicator.

**Update Tick handler in `src/main.rs`**: Toggle `app.live_blink_on` every 2 ticks.

**Verify**: `cargo run` during a live session — blinking LIVE indicator in status bar.

**Commit message**: `feat: add blinking LIVE session indicator in status bar`

---

### [x] Step 16: Update help popup with new keybindings

**What to do**: Update `src/ui/help.rs` with View Switching and Standings View keybinding sections. Remove old `Tab` entry.

**Commit message**: `feat: update help popup with Phase 1 keybindings`

---

### [x] Step 17: Update README.md for Phase 1

**What to do**: Add documentation for standings, results, new keybindings, and configuration.

**Commit message**: `docs: update README with Phase 1 features`

---

### [x] Step 18: Add mock-based tests for Phase 1

**What to do**: Create `tests/fixtures/` with saved API responses and write tests for standings/results parsing, serialization roundtrips, and results cache operations.

**Commit message**: `test: add mock-based tests for standings and results parsing`

---

## Phase 2: Live Timing, Weather, and Tire Data

---

### [x] Step 19: Create live timing infrastructure — LiveEvent, LiveProvider trait, LiveSessionManager

**What to do**: Create the `src/live/` module with core infrastructure.

**Create `src/live/mod.rs`**: `LiveProvider` trait, `LiveProviderBoxed` boxed version, `get_live_provider()` dispatch.

**Create `src/live/event.rs`**: `LiveDriverEntry`, `WeatherInfo`, `LiveTimingData`, and `LiveEvent` enum. See Design Decisions for exact field lists.

**Create `src/live/weather.rs`**: `WeatherInfo::display_string()` method.

**Create `src/live/manager.rs`**: `LiveSessionManager` that spawns polling tasks and dispatches `LiveEvent`s.

**Create `src/live/track_map.rs`**: Stub with `TrackPoint` struct (full implementation in Phase 3).

**Update `src/main.rs`**: Add `mod live;`.

Replace placeholder types in `App` struct from Step 4 with real types.

**Commit message**: `feat: create live timing infrastructure — LiveEvent, LiveProvider, LiveSessionManager`

---

### [x] Step 20: Implement F1 live timing provider (OpenF1 REST)

**What to do**: Create `src/scraper/f1_live.rs` with F1LiveProvider using OpenF1 API.

Implement `LiveProvider`:
- `is_session_live()`: Check `/v1/sessions?session_key=latest`
- `fetch_timing()`: Fetch drivers, positions, intervals, laps, stints, pits, weather, locations. Build `LiveTimingData`.

**Commit message**: `feat: implement F1 live timing provider via OpenF1 REST API`

---

### [x] Step 21: Implement NASCAR live timing provider

**What to do**: Create `src/scraper/nascar_live.rs` using `cf.nascar.com/live/feeds/live-feed.json`.

**Commit message**: `feat: implement NASCAR live timing provider`

---

### [x] Step 22: Create Live view UI — timing table + weather bar

**What to do**: Create `src/ui/live_view.rs` and `src/ui/live_timing_table.rs`.

Layout: session header → weather bar → sub-tab indicator → timing table.
Timing table columns: P, #, Driver, Gap, Int, Last Lap, S1, S2, S3, Tire, Pits.
Tire compounds color-coded (Soft=red, Medium=yellow, Hard=white, Inter=green, Wet=blue).

Wire up to `src/ui/mod.rs` dispatch.

**Commit message**: `feat: implement Live view UI with timing table and weather bar`

---

### [x] Step 23: Implement live session picker popup

**What to do**: Create `src/ui/live_session_picker.rs` — popup listing concurrent live sessions when multiple are active.

**Commit message**: `feat: implement live session picker popup`

---

### [x] Step 24: Implement driver detail expansion in Live view

**What to do**: Create `src/ui/live_driver_detail.rs` — expanded panel showing tire history, sectors, pit info when a driver is selected.

**Commit message**: `feat: implement driver detail expansion panel in Live view`

---

### [x] Step 25: Wire up live timing event loop and auto-detection

**What to do**: Integrate `LiveSessionManager` into the main event loop with separate `LiveEvent` channel. Add periodic live session detection (every 60s). Show status bar prompt when live session detected.

**Add `live_poll_interval_secs` to `UserConfig`.**

**Commit message**: `feat: wire up live timing event loop with auto-detection`

---

### [x] Step 26: Implement IndyCar live timing provider

**What to do**: Create `src/scraper/indycar_live.rs` using `racecontrol.indycar.com` JSON feeds.

**Commit message**: `feat: implement IndyCar live timing provider`

---

### [x] Step 27: Implement Al Kamel live timing provider (IMSA, WEC, Formula E)

**What to do**: Create `src/scraper/alkamelsystems_live.rs` — shared provider parameterized by series slug.

**Commit message**: `feat: implement Al Kamel Systems live timing provider (IMSA, WEC, Formula E)`

---

### [x] Step 28: Implement MotoGP and WRC live timing providers

**What to do**: Create `src/scraper/motogp_live.rs` and `src/scraper/wrc_live.rs`.

**Commit message**: `feat: implement MotoGP and WRC live timing providers`

---

### [x] Step 29: Update help popup for Phase 2

**What to do**: Add Live view keybindings to help popup.

**Commit message**: `feat: update help popup with Live view keybindings`

---

### [x] Step 30: Update README.md for Phase 2

**What to do**: Document live timing features, supported data sources, new config options.

**Commit message**: `docs: update README with Phase 2 live timing features`

---

### [x] Step 31: Add mock-based tests for Phase 2

**What to do**: Create fixtures and tests for live timing data parsing.

**Commit message**: `test: add mock-based tests for live timing data parsing`

---

## Phase 3: Track Maps, Notifications, and Daemon Mode

---

### [x] Step 32: Implement track map braille rendering engine

**What to do**: Flesh out `src/live/track_map.rs` with the full braille rendering engine using ratatui's `Canvas` widget. Include `TrackPoint`, `DriverPosition`, `TrackMapData`, `draw_track_map()`, and `draw_driver_legend()`.

**Commit message**: `feat: implement track map braille rendering engine with driver legend`

---

### [x] Step 33: Implement circuit geometry fetcher (MultiViewer API)

**What to do**: Add function to fetch circuit outline from `api.multiviewer.app/api/v1/circuits/{key}/{year}`.

**Commit message**: `feat: implement circuit geometry fetcher from MultiViewer API`

---

### [x] Step 34: Add F1 GPS position data to live timing

**What to do**: Update F1 live provider to fetch `/location` for all drivers and populate `x`/`y` fields.

**Commit message**: `feat: add GPS position data to F1 live timing for track map`

---

### [x] Step 35: Integrate track map into Live view Track Map sub-tab

**What to do**: Create `src/ui/track_map_view.rs`. Split area: 75% map, 25% legend. Wire into Live view sub-tab.

**Commit message**: `feat: integrate track map into Live view Track Map sub-tab`

---

### [ ] Step 36: Implement notification infrastructure

**What to do**: Create `src/notify/` module with `desktop.rs` (notify-rust), `terminal.rs` (OSC 777/99), and `mod.rs` (fallback logic).

**Commit message**: `feat: implement notification infrastructure (notify-rust + OSC fallback)`

---

### [ ] Step 37: Add notification configuration to config.toml

**What to do**: Add `DaemonConfig` struct with `[daemon]` TOML section. Fields: `notify_minutes_before`, `notify_session_types`, `notify_series_filter`, `notify_sound`, `poll_interval_secs`.

**Commit message**: `feat: add [daemon] notification configuration section to config.toml`

---

### [ ] Step 38: Implement notification scheduler

**What to do**: Create `src/notify/scheduler.rs` with `NotificationTracker` and `check_and_notify()` function.

**Commit message**: `feat: implement notification scheduler with configurable filters`

---

### [ ] Step 39: Implement daemon mode (--daemon CLI flag)

**What to do**: Add `--daemon` flag to CLI. Implement `run_daemon()` — headless loop that loads data and sends notifications.

**Commit message**: `feat: implement --daemon CLI flag for headless notification mode`

---

### [ ] Step 40: Implement remaining standings fetchers

**What to do**: Fill in all remaining series standings fetchers from Step 11 stubs.

**Commit message**: `feat: implement standings fetchers for all priority series`

---

### [ ] Step 41: Implement remaining results fetchers

**What to do**: Fill in all remaining series results fetchers from Step 11 stubs.

**Commit message**: `feat: implement results fetchers for all priority series`

---

### [ ] Step 42: Update help popup for Phase 3

**What to do**: Final help popup with all keybindings and daemon mode note.

**Commit message**: `feat: finalize help popup with all Phase 3 features`

---

### [ ] Step 43: Update README.md for Phase 3 (final)

**What to do**: Final README with track maps, notifications, daemon mode, full config docs, updated series support table.

**Commit message**: `docs: finalize README with track maps, notifications, and daemon mode`

---

### [ ] Step 44: Add tests for Phase 3

**What to do**: Tests for track map, notifications, daemon config, scheduler logic.

**Commit message**: `test: add tests for track maps, notifications, and daemon mode`

---

## Reference Material

### API Endpoints

| Series | Live Timing Endpoint | Standings Endpoint | Results Endpoint |
|---|---|---|---|
| F1 | `api.openf1.org/v1/` | `api.jolpica.com/ergast/f1/{year}/driverStandings.json` | `api.jolpica.com/ergast/f1/{year}/{round}/results.json` |
| NASCAR | `cf.nascar.com/live/feeds/live-feed.json` | `cf.nascar.com/cws/v1/series/{id}/standings/{year}.json` | `cf.nascar.com/cws/v1/series/{id}/{year}/results.json` |
| IndyCar | `racecontrol.indycar.com/xml/timings.json` | Scrape `indycar.com/standings` | Scrape `indycar.com/results` |
| IMSA | `livetiming.alkamelsystems.com/imsa` | `results.imsa.com` | `results.imsa.com` |
| WEC | `livetiming.alkamelsystems.com/fiawec` | `fiawec.alkamelsystems.com` | `fiawec.alkamelsystems.com` |
| Formula E | `livetiming.alkamelsystems.com/fiaformulae` | `results.fiaformulae.com` | `results.fiaformulae.com` |
| MotoGP | `api.motogp.com` / `resources.motogp.com` | `resources.motogp.com/files/results/` | `resources.motogp.com/files/results/` |
| WRC | `api.wrc.com/results-api/` | `api.wrc.com/results-api/` | `api.wrc.com/results-api/` |
| Circuit Geometry | `api.multiviewer.app/api/v1/circuits/{key}/{year}` | — | — |

### F1 OpenF1 API Key Endpoints

| Endpoint | Purpose | Key Fields |
|---|---|---|
| `/v1/sessions` | Session metadata | `session_key`, `session_name`, `session_type`, `date_start`, `date_end` |
| `/v1/drivers` | Driver info | `driver_number`, `broadcast_name`, `name_acronym`, `team_name`, `team_colour` |
| `/v1/position` | Current positions | `driver_number`, `position` |
| `/v1/intervals` | Gaps/intervals | `driver_number`, `gap_to_leader`, `interval` |
| `/v1/laps` | Lap times + sectors | `driver_number`, `lap_number`, `lap_duration`, `sector_*_duration` |
| `/v1/stints` | Tire stints | `driver_number`, `compound`, `lap_start`, `lap_end`, `tyre_age_at_start` |
| `/v1/pit` | Pit stops | `driver_number`, `pit_duration`, `lap_number` |
| `/v1/location` | GPS coordinates | `driver_number`, `x`, `y` |
| `/v1/weather` | Track weather | `air_temperature`, `track_temperature`, `rainfall` |

### Config.toml Final Structure

```toml
# User preferences
favorites = ["f1", "indycar", "wec"]
cache_ttl_hours = 24
notification_threshold_hours = 2
open_command = "xdg-open"
default_view = "list"
hidden_series = []
live_poll_interval_secs = 2

# Daemon and notification settings
[daemon]
notify_minutes_before = 30
notify_session_types = ["all"]
notify_series_filter = "favorites"
notify_sound = true
poll_interval_secs = 300
```

### Keybinding Summary (Final)

| Key | Context | Action |
|---|---|---|
| `1` | Global (no popup) | Switch to List View |
| `2` | Global (no popup) | Switch to Calendar View |
| `3` | Global (no popup) | Switch to Live View (session picker if multiple live) |
| `4` | Global (no popup) | Switch to Standings View |
| `j` / `↓` | All views | Move down / next item |
| `k` / `↑` | All views | Move up / previous item |
| `h` / `←` | Calendar, Standings, Live | Previous day / Previous series / Previous sub-tab |
| `l` / `→` | Calendar, Standings, Live | Next day / Next series / Next sub-tab |
| `Enter` | List, Calendar, Live | Open detail popup / Select day / Toggle driver detail |
| `F` | List, Calendar | Open filter panel |
| `/` | List, Calendar | Start search |
| `f` | List, Calendar | Toggle favorite |
| `o` | Detail popup | Open first stream link |
| `1`–`9` | Detail popup | Open numbered stream link |
| `?` | Global | Open help popup |
| `r` | Global | Refresh data |
| `Esc` | Any popup | Close popup |
| `q` | Global (no popup) | Quit |
