mod action;
mod app;
mod config;
mod data;
mod event;
mod scraper;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self as crossterm_event, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use event::AppEvent;
use ratatui::prelude::*;
use std::io::stdout;
use std::path::Path;
use tokio::sync::mpsc;

/// Spawn background tasks to load data for all series.
/// For each series:
/// 1. Try to load from cache first (instant, non-blocking).
/// 2. If cache is fresh (within TTL), use it.
/// 3. If cache is stale or missing, spawn an async task to scrape/fetch.
fn spawn_data_loaders(
    app: &mut App,
    tx: mpsc::UnboundedSender<AppEvent>,
    force_refresh: bool,
) {
    let series_list: Vec<(String, data::models::Series)> = app
        .series_registry
        .iter()
        .map(|(id, s)| (id.clone(), s.clone()))
        .collect();

    for (series_id, series) in series_list {
        // 1. Try loading from cache (unless forced refresh)
        if !force_refresh {
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

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = stdout().execute(crossterm::cursor::Show);
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_panic_hook();

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
            if event_tx.is_closed() {
                break;
            }
            if crossterm_event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
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
            if tick_tx.is_closed() {
                break;
            }
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Start loading data (cached first, then async fetch)
    spawn_data_loaders(&mut app, tx.clone(), false);

    // Main loop
    while app.running {
        // Draw
        terminal.draw(|frame| {
            ui::draw(frame, &app);
        })?;

        // Handle refresh requested from app
        if app.refresh_requested {
            app.refresh_requested = false;
            spawn_data_loaders(&mut app, tx.clone(), true);
        }

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
                AppEvent::RefreshRequested => {
                    spawn_data_loaders(&mut app, tx.clone(), true);
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

    // Drop communication channel
    drop(rx);
    drop(tx);

    // Restore terminal cleanly
    let _ = disable_raw_mode();
    let _ = stdout().execute(LeaveAlternateScreen);
    let _ = stdout().execute(crossterm::cursor::Show);

    // Save config on exit
    let _ = app.config.save();

    std::process::exit(0);
}

/// Handle key events. Will be expanded in later steps.
fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Global Ctrl+C handler to always cleanly exit
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
    {
        app.running = false;
        return;
    }
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

    // If favorite confirmation dialog is active, handle confirm/cancel
    if let Some(series_id) = app.pending_favorite_toggle.clone() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                let was_fav = app.config.favorites.contains(&series_id);
                app.config.toggle_favorite(&series_id);
                let series_name = app
                    .series_registry
                    .get(&series_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or(series_id.as_str());
                let action = if was_fav { "Removed from" } else { "Added to" };
                app.status_message = Some(format!("{} favorites: {}", action, series_name));
                app.pending_favorite_toggle = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.pending_favorite_toggle = None;
            }
            _ => {}
        }
        return;
    }

    // If help popup is active, handle help keys
    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    // If detail view is active, handle detail view keys
    if app.show_detail {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_detail = false;
                return;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Open first stream link
                if let Some(event) = app.selected_event() {
                    if let Some(link) = event.stream_links.first() {
                        if let Err(e) = action::open_url(&app.config.open_command, &link.url) {
                            app.status_message = Some(format!("Error: {}", e));
                        } else {
                            app.status_message = Some(format!(
                                "Opened {} in {}",
                                link.platform, app.config.open_command
                            ));
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
                            app.status_message = Some(format!(
                                "Opened {} in {}",
                                link.platform, app.config.open_command
                            ));
                        }
                    }
                }
                return;
            }
            _ => {
                return;
            }
        }
    }

    // If filter panel is active, handle filter panel keys
    if app.show_filter_panel {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('F') => {
                app.show_filter_panel = false;
            }
            KeyCode::Char(' ') => {
                app.toggle_selected_filter();
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.reset_filters();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.filter_select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.filter_select_previous();
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
        // Calendar navigation (only active in Calendar view)
        KeyCode::Char('h') | KeyCode::Left => {
            if app.view_mode == app::ViewMode::Calendar {
                if app.calendar_month == 1 {
                    app.calendar_month = 12;
                    app.calendar_year -= 1;
                } else {
                    app.calendar_month -= 1;
                }
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if app.view_mode == app::ViewMode::Calendar {
                if app.calendar_month == 12 {
                    app.calendar_month = 1;
                    app.calendar_year += 1;
                } else {
                    app.calendar_month += 1;
                }
            }
        }
        KeyCode::Char('H') => {
            if app.view_mode == app::ViewMode::Calendar {
                app.calendar_year -= 1;
            }
        }
        KeyCode::Char('L') => {
            if app.view_mode == app::ViewMode::Calendar {
                app.calendar_year += 1;
            }
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
        // Refresh
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.status_message = Some("Refreshing...".to_string());
            app.refresh_requested = true;
        }
        // Help
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
        }
        // Toggle detail view
        KeyCode::Enter => {
            app.show_detail = !app.show_detail;
        }
        // Prompt confirmation to toggle favorite for selected event's series
        KeyCode::Char('f') => {
            if let Some(event) = app.selected_event() {
                app.pending_favorite_toggle = Some(event.series_id.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_handle_quit() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        assert!(app.running);
        handle_key_event(&mut app, key(KeyCode::Char('q')));
        assert!(!app.running);
    }

    #[test]
    fn test_handle_search_flow() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        // Activate search
        handle_key_event(&mut app, key(KeyCode::Char('/')));
        assert!(app.search_active);
        assert_eq!(app.search_query.as_deref(), Some(""));

        // Type characters
        handle_key_event(&mut app, key(KeyCode::Char('m')));
        handle_key_event(&mut app, key(KeyCode::Char('o')));
        handle_key_event(&mut app, key(KeyCode::Char('n')));
        assert_eq!(app.search_query.as_deref(), Some("mon"));

        // Backspace
        handle_key_event(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.search_query.as_deref(), Some("mo"));

        // Enter finishes typing
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(!app.search_active);
        assert_eq!(app.search_query.as_deref(), Some("mo"));

        // Esc clears search query
        handle_key_event(&mut app, key(KeyCode::Esc));
        assert_eq!(app.search_query, None);
    }

    #[test]
    fn test_toggle_view_and_popups() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        assert_eq!(app.view_mode, app::ViewMode::List);

        handle_key_event(&mut app, key(KeyCode::Tab));
        assert_eq!(app.view_mode, app::ViewMode::Calendar);

        handle_key_event(&mut app, key(KeyCode::Tab));
        assert_eq!(app.view_mode, app::ViewMode::List);

        handle_key_event(&mut app, key(KeyCode::Char('?')));
        assert!(app.show_help);
        handle_key_event(&mut app, key(KeyCode::Esc));
        assert!(!app.show_help);

        handle_key_event(&mut app, key(KeyCode::Char('F')));
        assert!(app.show_filter_panel);
        handle_key_event(&mut app, key(KeyCode::Esc));
        assert!(!app.show_filter_panel);
    }

    #[test]
    fn test_handle_refresh_key() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        assert!(!app.refresh_requested);
        handle_key_event(&mut app, key(KeyCode::Char('r')));
        assert!(app.refresh_requested);
        assert_eq!(app.status_message.as_deref(), Some("Refreshing..."));
    }

    #[tokio::test]
    async fn test_spawn_data_loaders_with_unimplemented_scraper() {
        let mut registry = HashMap::new();
        registry.insert(
            "unimplemented_series".to_string(),
            data::models::Series {
                id: "unimplemented_series".to_string(),
                name: "Unimplemented".to_string(),
                short_name: "Unimp".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (0, 0, 0),
                region: "Global".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );
        let mut app = App::new(registry, config::UserConfig::default());
        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();

        spawn_data_loaders(&mut app, tx, false);

        assert_eq!(
            app.fetch_status.get("unimplemented_series"),
            Some(&data::models::FetchStatus::Error("No scraper implemented yet".to_string()))
        );
    }

    #[test]
    fn test_calendar_navigation() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        app.view_mode = app::ViewMode::Calendar;
        app.calendar_year = 2026;
        app.calendar_month = 5;

        // Next month
        handle_key_event(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.calendar_month, 6);
        assert_eq!(app.calendar_year, 2026);

        // Prev month
        handle_key_event(&mut app, key(KeyCode::Char('h')));
        assert_eq!(app.calendar_month, 5);

        // Prev month wrap at Jan
        app.calendar_month = 1;
        handle_key_event(&mut app, key(KeyCode::Char('h')));
        assert_eq!(app.calendar_month, 12);
        assert_eq!(app.calendar_year, 2025);

        // Next month wrap at Dec
        handle_key_event(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.calendar_month, 1);
        assert_eq!(app.calendar_year, 2026);

        // Year navigation
        handle_key_event(&mut app, key(KeyCode::Char('H')));
        assert_eq!(app.calendar_year, 2025);
        handle_key_event(&mut app, key(KeyCode::Char('L')));
        assert_eq!(app.calendar_year, 2026);
    }

    #[test]
    fn test_detail_view_keybindings() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            data::models::Series {
                id: "f1".to_string(),
                name: "Formula 1".to_string(),
                short_name: "F1".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (255, 0, 0),
                region: "International".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );
        let mut config = config::UserConfig::default();
        config.open_command = "true".to_string(); // Use "true" so open_url succeeds in test
        let mut app = App::new(registry, config);

        let event = data::models::RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Monaco Grand Prix".to_string(),
            circuit_name: "Circuit de Monaco".to_string(),
            location: "Monte Carlo".to_string(),
            country: "Monaco".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            round: Some(8),
            sessions: vec![],
            stream_links: vec![
                data::models::StreamLink {
                    platform: "F1TV".to_string(),
                    url: "https://f1tv.formula1.com".to_string(),
                    access: data::models::StreamAccess::Paid,
                },
                data::models::StreamLink {
                    platform: "YouTube".to_string(),
                    url: "https://youtube.com".to_string(),
                    access: data::models::StreamAccess::Free,
                },
            ],
            status: data::models::EventStatus::Upcoming,
        };
        app.update_series_data("f1".to_string(), vec![event]);

        assert!(!app.show_detail);

        // Open with Enter
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.show_detail);

        // Press 'o' to open first link
        handle_key_event(&mut app, key(KeyCode::Char('o')));
        assert_eq!(
            app.status_message.as_deref(),
            Some("Opened F1TV in true")
        );

        // Press '2' to open second link
        handle_key_event(&mut app, key(KeyCode::Char('2')));
        assert_eq!(
            app.status_message.as_deref(),
            Some("Opened YouTube in true")
        );

        // Close with Esc
        handle_key_event(&mut app, key(KeyCode::Esc));
        assert!(!app.show_detail);

        // Open and close with q
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.show_detail);
        handle_key_event(&mut app, key(KeyCode::Char('q')));
        assert!(!app.show_detail);
        assert!(app.running); // Should NOT exit app when closing popup
    }

    #[test]
    fn test_filter_panel_keybindings() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        assert!(!app.show_filter_panel);
        assert!(app.active_filters.is_default());

        // Open with F
        handle_key_event(&mut app, key(KeyCode::Char('F')));
        assert!(app.show_filter_panel);

        // Navigate down to Favorites Only (skipping headers)
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        // Toggle with Space
        handle_key_event(&mut app, key(KeyCode::Char(' ')));
        assert!(app.show_filter_panel); // Panel stays open on toggle
        assert!(app.active_filters.favorites_only);

        // Reset filters with 'a'
        handle_key_event(&mut app, key(KeyCode::Char('a')));
        assert!(app.active_filters.is_default());

        // Toggle Favorites again
        handle_key_event(&mut app, key(KeyCode::Char(' ')));
        assert!(app.active_filters.favorites_only);

        // Close with Enter
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(!app.show_filter_panel);
        assert!(app.active_filters.favorites_only);
    }

    #[test]
    fn test_favorite_confirmation_dialog_flow() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            data::models::Series {
                id: "f1".to_string(),
                name: "Formula 1".to_string(),
                short_name: "F1".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (255, 0, 0),
                region: "International".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, config::UserConfig::default());
        let event = data::models::RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Monaco GP".to_string(),
            circuit_name: "Circuit de Monaco".to_string(),
            location: "Monte Carlo".to_string(),
            country: "Monaco".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: data::models::EventStatus::Upcoming,
        };
        app.update_series_data("f1".to_string(), vec![event]);

        // Press 'f' -> opens confirmation dialog
        handle_key_event(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.pending_favorite_toggle.as_deref(), Some("f1"));
        assert!(!app.config.favorites.contains("f1"));

        // Press 'n' -> cancels without adding
        handle_key_event(&mut app, key(KeyCode::Char('n')));
        assert_eq!(app.pending_favorite_toggle, None);
        assert!(!app.config.favorites.contains("f1"));

        // Press 'f' and confirm with 'y'
        handle_key_event(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.pending_favorite_toggle.as_deref(), Some("f1"));
        handle_key_event(&mut app, key(KeyCode::Char('y')));
        assert_eq!(app.pending_favorite_toggle, None);
        assert!(app.config.favorites.contains("f1"));
        assert_eq!(app.status_message.as_deref(), Some("Added to favorites: Formula 1"));

        // Press 'f' and confirm removal with Enter
        handle_key_event(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.pending_favorite_toggle.as_deref(), Some("f1"));
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert_eq!(app.pending_favorite_toggle, None);
        assert!(!app.config.favorites.contains("f1"));
        assert_eq!(app.status_message.as_deref(), Some("Removed from favorites: Formula 1"));
    }

    #[test]
    fn test_help_popup_keybindings() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        assert!(!app.show_help);

        // Open with ?
        handle_key_event(&mut app, key(KeyCode::Char('?')));
        assert!(app.show_help);

        // Close with Esc
        handle_key_event(&mut app, key(KeyCode::Esc));
        assert!(!app.show_help);

        // Open with ? and close with ?
        handle_key_event(&mut app, key(KeyCode::Char('?')));
        assert!(app.show_help);
        handle_key_event(&mut app, key(KeyCode::Char('?')));
        assert!(!app.show_help);

        // Open with ? and close with q without quitting app
        handle_key_event(&mut app, key(KeyCode::Char('?')));
        assert!(app.show_help);
        handle_key_event(&mut app, key(KeyCode::Char('q')));
        assert!(!app.show_help);
        assert!(app.running);

        // Open with ? and close with Enter
        handle_key_event(&mut app, key(KeyCode::Char('?')));
        assert!(app.show_help);
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(!app.show_help);
    }

    #[test]
    fn test_handle_ctrl_c() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        assert!(app.running);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        handle_key_event(&mut app, ctrl_c);
        assert!(!app.running);
    }
}






