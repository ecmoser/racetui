# 🏁 RaceTUI

A fast, terminal user interface (TUI) for viewing motorsport and racing series calendars from around the world.

![RaceTUI](https://img.shields.io/badge/rust-2021_edition-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)

---

## Features

- 📅 **36 Racing Series Worldwide**: Formula 1, IndyCar, NASCAR, IMSA, WEC, MotoGP, WRC, Super GT, Super Formula, DTM, BTCC, and many more.
- 📋 **Dual Views**: Interactive multi-column **List View** and monthly **Calendar Grid View** (with Sunday-start weeks).
- 🔍 **Search & Multi-Select Filter Panel**: Filter events by series, car style (Open Wheel, Sports Car, Stock Car, Touring, Rally, Motorcycle), region, or status.
- ⭐ **Favorites & Notifications**: Star your favorite series to get countdown highlights and status bar notification cycling for upcoming sessions.
- 📺 **One-Key Livestream Opening**: Jump directly to official broadcasts and streams (F1TV, Peacock, FOX Sports, Max, YouTube, etc.) with customizable browser commands.
- 💾 **Intelligent Caching**: Fast startup with JSON caching in `~/.local/share/racetui/cache/` and configurable TTL.
- ⏰ **Local Timezone Conversion**: Automatically converts all session times to your local timezone.
- ⌨️ **Vim & Arrow Key Navigation**: Smooth, responsive scrolling and navigation with full keyboard support.

---

## Supported Series

| Series | Category | Region | Data Source |
|---|---|---|---|
| **Formula 1** | Open Wheel | International | Jolpica API (Ergast) |
| **Formula 2** | Open Wheel | International | FOM Apigee / Official |
| **Formula 3** | Open Wheel | International | FOM Apigee / Official |
| **Formula E** | Open Wheel | International | Pulselive API / Official |
| **IndyCar Series** | Open Wheel | USA | Official Schedule & SportsTimes |
| **INDY NXT** | Open Wheel | USA | Official Schedule & SportsTimes |
| **F1 Academy** | Open Wheel | International | FOM Apigee / Official |
| **Super Formula** | Open Wheel | Japan | Official Schedule |
| **NASCAR Cup Series** | Stock Car | USA | NASCAR CDN API |
| **NASCAR Xfinity Series** | Stock Car | USA | NASCAR CDN API |
| **NASCAR Craftsman Truck Series** | Stock Car | USA | NASCAR CDN API |
| **ARCA Menards Series** | Stock Car | USA | NASCAR CDN API / Official |
| **IMSA WeatherTech SportsCar Championship** | Sports Car | USA | Official Schedule |
| **FIA World Endurance Championship (WEC)** | Sports Car | International | Official Schedule |
| **European Le Mans Series (ELMS)** | Sports Car | Europe | Official Schedule |
| **Asian Le Mans Series (AsLMS)** | Sports Car | Asia | Official Schedule |
| **Nürburgring Langstrecken-Serie (NLS)** | Sports Car | Europe | Official Schedule |
| **GT World Challenge Europe** | Sports Car | Europe | Official Schedule |
| **GT World Challenge America** | Sports Car | USA | Official Schedule |
| **Intercontinental GT Challenge (IGTC)** | Sports Car | International | Official Schedule |
| **Super GT** | Sports Car | Japan | Official Schedule |
| **Porsche Supercup** | Sports Car | Europe | Official Schedule |
| **MotoGP** | Motorcycle | International | Pulselive API / Official |
| **Moto2** | Motorcycle | International | Pulselive API / Official |
| **Moto3** | Motorcycle | International | Pulselive API / Official |
| **WorldSBK (Superbike World Championship)** | Motorcycle | International | Official Schedule |
| **British Superbike Championship (BSB)** | Motorcycle | UK | Official Schedule |
| **World Rally Championship (WRC)** | Rally | International | WRC API / Official |
| **WRC2** | Rally | International | WRC API / Official |
| **European Rally Championship (ERC)** | Rally | Europe | WRC Promoter API / Official |
| **Dakar Rally** | Rally | International | Official Schedule |
| **Extreme E / Extreme H** | Rally | International | Official Schedule |
| **British Touring Car Championship (BTCC)** | Touring | UK | Official Schedule |
| **DTM (Deutsche Tourenwagen Masters)** | Touring | Europe | Official Schedule |
| **Repco Supercars Championship** | Touring | Australia | Official Schedule |
| **TCR World Tour** | Touring | International | Official Schedule |

---

## Installation

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (1.74+)

```bash
cargo install --path .
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
racetui              # Start RaceTUI in default view
racetui --calendar   # Launch directly into Calendar grid view
racetui --series f1  # Filter to a specific series on startup
racetui --refresh    # Force fresh network scrape (bypass cache)
racetui --clear-cache # Clear all cached series schedules and exit
```

---

## Keybindings

### Navigation
| Key | Action |
|---|---|
| `j` / `↓` | Move down / next session |
| `k` / `↑` | Move up / previous session |
| `h` / `←` | Previous day (Calendar) |
| `l` / `→` | Next day (Calendar) |
| `Tab` | Toggle between List View and Calendar Grid View |
| `Enter` | Open Event Details popup (or day sessions list in Calendar) |

### Overlays & Filters
| Key | Action |
|---|---|
| `F` | Open Multi-Select Filter Panel (Status, Session Type, Car Style, Region, Series) |
| `Space` | Toggle selected filter item (in Filter Panel) |
| `a` | Reset all filters to default (in Filter Panel) |
| `/` | Start text search across events, circuits, and locations |
| `f` | Toggle favorite for selected series (with confirmation dialog) |
| `?` | Open Help popup |
| `Esc` | Close popup, cancel search, or dismiss overlay |

### Actions
| Key | Action |
|---|---|
| `o` | Open first stream link in configured browser/app |
| `1` - `9` | Open specific stream link by number (in Event Details view) |
| `r` | Trigger asynchronous background data refresh |
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

# Command used to open stream links in your browser or application
# Options: "xdg-open", "open", "firefox", "chromium", etc.
open_command = "xdg-open"

# Default startup view ("list" or "calendar")
default_view = "list"

# Hidden series IDs to omit from all views
hidden_series = []
```

---

## Logs

Application logs and scraper diagnostic traces are written to:
```bash
~/.local/share/racetui/racetui.log
```

---

## License

This project is licensed under the [MIT License](LICENSE).
