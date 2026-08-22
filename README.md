# 🏁 RaceTUI

A fast, fully-featured terminal user interface (TUI) for viewing motorsport and racing series calendars, live timing, telemetry, ASCII track maps, and championship standings from around the world.

![RaceTUI](https://img.shields.io/badge/rust-2021_edition-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)

---

## Features

- 📅 **36 Racing Series Worldwide**: Formula 1, Formula 2, Formula 3, IndyCar, INDY NXT, NASCAR (Cup, Xfinity, Trucks, ARCA), IMSA, WEC, ELMS, AsLMS, Formula E, MotoGP, Moto2, Moto3, WorldSBK, BSB, WRC, Super GT, Super Formula, DTM, BTCC, Supercars, and many more.
- 📋 **Four Interactive Views**:
  - `1`: **List View** — Chronological session schedule with countdowns, live indicators, and favorite badges.
  - `2`: **Calendar Grid View** — Monthly overview grid with Sunday-start weeks, day event navigation, and session counts.
  - `3`: **Live Timing & Track Map** — Real-time leaderboard, telemetry, sector split times, tire compounds, live weather, and animated ASCII circuit maps with driver positions.
  - `4`: **Championship Standings View** — Driver and Constructor championship standings with series search and quick cycling.
- 🗺️ **ASCII Track Maps & Driver Tracking**:
  - Braille/ASCII circuit renderings with numbered driver dots positioned dynamically along the circuit based on live lap progress.
  - Circuit metadata (lap length, number of turns, DRS zones, direction).
  - Side leaderboard panel showing live running order, gaps to leader, intervals, and current tire compound.
  - Switch smoothly between the Timing Leaderboard and Track Map with `h` / `l` (`←` / `→`).
- ⏱️ **Real-Time Live Timing & Telemetry**:
  - **Live Leaderboard Table**: Real-time positions, driver numbers, team names, gaps to leader, intervals, last lap times, sector splits (S1/S2/S3), color-coded tire compounds (Soft, Medium, Hard, Inter, Wet, Alternate, Primary), pit stop counts, and `[FL]` fastest lap highlights.
  - **Expanded Driver Detail Panel**: Press `Enter` on any driver row in Live view to view a 3-column telemetry card featuring Pace & Position, Sectors & Laps with fastest split badges, and Tires & Pit Strategy.
  - **Real-Time Weather Bar**: Ambient air and track temperatures, relative humidity, wind speed & direction, and rainfall detection.
  - **Multi-Session Live Picker**: When multiple sessions are live simultaneously, press `3` to choose which live session to monitor.
  - **Auto-Detection & Status Bar Prompts**: Background watcher alerts you in the status bar whenever a live session goes green across any series.
- 🔔 **Desktop Notifications & Background Daemon Mode**:
  - Background daemon (`racetui --daemon`) monitors active and upcoming sessions without keeping the full TUI open.
  - Native desktop notifications for:
    - Approaching session start reminders (e.g. 15m, 5m before start).
    - Session green flag and checkered flag events.
    - Live race lead changes and overtakes for P1.
    - Safety Car and Yellow Flag deployments.
  - Systemd user service generator (`racetui daemon install-service`) for automated startup.
- 🏁 **Session-Specific Race Results & Tabbed Details**:
  - Interactive tabs for **Race Results**, **Qualifying**, **Sprint**, and **Schedule**.
  - Driver classifications with positions gained/lost (`+/-`), gaps, points, penalties, and `[FL]` fastest lap highlights.
  - Q1 / Q2 / Q3 qualifying split times.
  - Automatic on-demand live fetching and local caching for all priority series.
- 🔍 **Search & Multi-Select Filter Panel**: Filter events by series, car style (Open Wheel, Sports Car, Stock Car, Touring, Rally, Motorcycle), region, or status (`Completed`, `Live`, `Upcoming`).
- ⭐ **Favorites & Notifications**: Star your favorite series to get countdown highlights and status bar notification cycling for upcoming sessions.
- 🔴 **Global LIVE Session Indicator**: Real-time status bar indicator showing active live sessions across any series.
- 📺 **One-Key Livestream Opening**: Jump directly to official broadcasts and streams (F1TV, Peacock, FOX Sports, Max, YouTube, etc.) with customizable browser commands.
- 💾 **Intelligent Caching & Custom Configs**: Fast startup with JSON caching in `~/.local/share/racetui/cache/` for schedules, standings, and race results. Override series settings via `~/.config/racetui/series.toml`.
- ⏰ **Local Timezone Conversion**: Automatically converts all session times to your local timezone.
- ⌨️ **Vim & Arrow Key Navigation**: Smooth, responsive scrolling and navigation with full keyboard support.

---

## Supported Series & Live Timing Providers

| Series | Category | Region | Live Timing Provider | Schedule & Results Source |
|---|---|---|---|---|
| **Formula 1** | Open Wheel | International | OpenF1 Live API | Jolpica API (Ergast) |
| **Formula 2** | Open Wheel | International | OpenF1 / Timing Feed | FOM / Official |
| **Formula 3** | Open Wheel | International | OpenF1 / Timing Feed | FOM / Official |
| **Formula E** | Open Wheel | International | Al Kamel Systems | Pulselive API / Official |
| **IndyCar Series** | Open Wheel | USA | Race Control JSON | Official & SportsTimes |
| **INDY NXT** | Open Wheel | USA | Race Control JSON | Official & SportsTimes |
| **F1 Academy** | Open Wheel | International | OpenF1 / Timing Feed | FOM / Official |
| **Super Formula** | Open Wheel | Japan | Timing Feed | Official Schedule |
| **NASCAR Cup Series** | Stock Car | USA | NASCAR Live Feed | NASCAR CDN API |
| **NASCAR Xfinity Series** | Stock Car | USA | NASCAR Live Feed | NASCAR CDN API |
| **NASCAR Craftsman Truck Series** | Stock Car | USA | NASCAR Live Feed | NASCAR CDN API |
| **ARCA Menards Series** | Stock Car | USA | NASCAR Live Feed | NASCAR CDN API / Official |
| **IMSA WeatherTech SportsCar Championship** | Sports Car | USA | Al Kamel Systems | Official Schedule & Results |
| **FIA World Endurance Championship (WEC)** | Sports Car | International | Al Kamel Systems | Official Schedule & Results |
| **European Le Mans Series (ELMS)** | Sports Car | Europe | Al Kamel Systems | Official Schedule |
| **Asian Le Mans Series (AsLMS)** | Sports Car | Asia | Al Kamel Systems | Official Schedule |
| **24H Series** | Sports Car | International | Al Kamel Systems | Official Schedule |
| **Nürburgring Langstrecken-Serie (NLS)** | Sports Car | Europe | Timing Feed | Official Schedule |
| **GT World Challenge Europe** | Sports Car | Europe | Timing Feed | Official Schedule |
| **GT World Challenge America** | Sports Car | USA | Timing Feed | Official Schedule |
| **Intercontinental GT Challenge (IGTC)** | Sports Car | International | Timing Feed | Official Schedule |
| **Super GT** | Sports Car | Japan | Timing Feed | Official Schedule & Results |
| **Porsche Supercup** | Sports Car | Europe | Timing Feed | Official Schedule |
| **MotoGP** | Motorcycle | International | Pulselive Timing Feed | Pulselive API / Official |
| **Moto2** | Motorcycle | International | Pulselive Timing Feed | Pulselive API / Official |
| **Moto3** | Motorcycle | International | Pulselive Timing Feed | Pulselive API / Official |
| **WorldSBK (Superbike World Championship)** | Motorcycle | International | Pulselive Timing Feed | Official Schedule |
| **British Superbike Championship (BSB)** | Motorcycle | UK | Timing Feed | Official Schedule |
| **World Rally Championship (WRC)** | Rally | International | WRC Timing Feed | WRC API / Official |
| **WRC2** | Rally | International | WRC Timing Feed | WRC API / Official |
| **European Rally Championship (ERC)** | Rally | Europe | WRC Timing Feed | WRC Promoter API / Official |
| **Dakar Rally** | Rally | International | Timing Feed | Official Schedule |
| **Extreme E / Extreme H** | Rally | International | Timing Feed | Official Schedule |
| **British Touring Car Championship (BTCC)** | Touring | UK | Timing Feed | Official Schedule & Results |
| **DTM (Deutsche Tourenwagen Masters)** | Touring | Europe | Timing Feed | Official Schedule & Results |
| **Repco Supercars Championship** | Touring | Australia | Timing Feed | Official Schedule |
| **TCR World Tour** | Touring | International | Timing Feed | Official Schedule |

---

## Installation

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (1.74+)

```bash
# Build and install to ~/.cargo/bin or ~/.local/bin:
cargo install --path .

# Or build release binary:
cargo build --release
cp target/release/racetui ~/.local/bin/
```

### With Headless Browser Support
For scraping dynamic JavaScript-heavy sites with Chromium fallback:

```bash
cargo install --path . --features headless-browser
```
*(Requires Google Chrome or Chromium to be installed on your system).*

---

## CLI Usage

```bash
# Interactive TUI
racetui               # Start RaceTUI in default view
racetui --calendar    # Launch directly into Calendar grid view
racetui --standings   # Launch directly into Championship Standings view
racetui --series f1   # Filter to a specific series on startup
racetui --refresh     # Force fresh network scrape (bypass cache)
racetui --clear-cache # Clear all cached series schedules, results, and standings

# Background Daemon & Desktop Notifications
racetui --daemon               # Run background daemon directly
racetui daemon start           # Start the background daemon
racetui daemon stop            # Stop the running background daemon
racetui daemon status          # Check daemon running status and PID
racetui daemon install-service # Generate and install systemd user service
```

---

## Keybindings

### View Switching
| Key | View |
|---|---|
| `1` | List View (All scheduled sessions) |
| `2` | Calendar View (Monthly race calendar) |
| `3` | Live Timing View (or open Live Session Picker if multiple live) |
| `4` | Championship Standings View |

### Navigation
| Key | Action |
|---|---|
| `j` / `↓` | Move down / next session / scroll standings table / select driver row (Live view) |
| `k` / `↑` | Move up / previous session / scroll standings table / select driver row (Live view) |
| `h` / `←` | Prev day (Calendar) / Prev series (Standings) / Switch sub-tab (Live view) |
| `l` / `→` | Next day (Calendar) / Next series (Standings) / Switch sub-tab (Live view) |
| `H` / `L` | Previous / Next month (Calendar) |
| `t` | Jump to today (Calendar) |
| `Enter` | Open Event Details (List) / Open day sessions (Calendar) / Expand driver telemetry (Live) |

### Live Timing & Track Map View
| Key | Action |
|---|---|
| `3` | Switch to Live view (or open session picker when multiple active) |
| `h` / `l` (`←` / `→`) | Switch sub-tabs (Timing Leaderboard vs. Track Map) |
| `j` / `k` (`↓` / `↑`) | Select driver in timing table or track map leaderboard |
| `Enter` | Expand / collapse 3-column driver telemetry & pit strategy card |
| `Esc` / `q` | Collapse driver telemetry card (before quitting) |

### Event Details Popup
| Key | Action |
|---|---|
| `Tab` / `h` / `l` | Switch detail tabs (Race Results, Qualifying, Sprint, Schedule) |
| `1` - `5` | Select detail tab directly by number |
| `l` | Launch Live Timing session from details view (when session is live) |
| `o` | Open first stream link in configured browser |
| `Esc` / `q` | Close details popup |

### Overlays & Filters
| Key | Action |
|---|---|
| `F` | Open Multi-Select Filter Panel (Status, Session Type, Car Style, Region, Series) |
| `Space` | Toggle selected filter item (in Filter Panel) |
| `a` | Reset all filters to default (in Filter Panel) |
| `/` | Text search events (List view) or search series (Standings view) |
| `f` | Toggle favorite for selected series (with confirmation dialog) |
| `?` | Open Help popup |
| `Esc` | Close popup, cancel search, or dismiss overlay |

### Actions
| Key | Action |
|---|---|
| `r` | Refresh all data and reload schedules & standings |
| `q` | Quit RaceTUI |

---

## Configuration

The configuration file is automatically created at:
- **Linux/macOS**: `~/.config/racetui/config.toml`
- **Windows**: `%APPDATA%\racetui\config.toml`

### Example `config.toml`
```toml
# Favorited series IDs (marked with ★ and highlighted when sessions approach)
favorites = ["f1", "indycar", "wec", "motogp", "imsa"]

# Cache expiration in hours (default: 24)
cache_ttl_hours = 24

# Hours before session start to trigger "⚡ SOON" notification highlight (default: 2)
notification_threshold_hours = 2

# Live timing polling interval in seconds (default: 5)
live_poll_interval_secs = 5

# Command used to open stream links in your browser or application
# Options: "xdg-open", "open", "firefox", "chromium", etc.
open_command = "xdg-open"

# Default startup view ("list", "calendar", "standings")
default_view = "list"

# Hidden series IDs to omit from all views
hidden_series = []

# Background Daemon & Notification settings
[daemon]
enabled = true
poll_interval_secs = 60
notify_favorites_only = true
notify_lead_changes = true
notify_safety_car = true
remind_before_mins = [15, 5, 0]
```

### Custom Series Overrides
You can customize series names, colors, car styles, regions, and calendar endpoints by placing a `series.toml` in:
- `~/.config/racetui/series.toml` (highest priority user configuration)
- `./data/series.toml` (repository override)
- Embedded default configuration (built directly into the binary)

---

## Cache & Storage

RaceTUI caches data locally in `~/.local/share/racetui/`:
- `cache/` — Schedules, standings, and session results (JSON)
- `racetui.log` — Diagnostics and scraper logs
- `daemon.pid` — Process ID when running in background daemon mode

---

## License

This project is licensed under the [MIT License](LICENSE).
