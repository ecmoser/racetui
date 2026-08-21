mod action;
mod app;
mod config;
mod data;
mod event;
mod live;
mod scraper;
mod ui;

use anyhow::Result;
use app::App;
use chrono::Datelike;
use clap::Parser;
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

/// Spawn background tasks to load data for all series.
/// For each series:
/// 1. Try to load from cache first (instant, non-blocking).
/// 2. If cache is fresh (within TTL), use it.
/// 3. If cache is stale or missing, spawn an async task to scrape/fetch.
fn spawn_data_loaders(app: &mut App, tx: mpsc::UnboundedSender<AppEvent>, force_refresh: bool) {
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
                app.mark_fetch_error(&series_id, "No scraper implemented yet".to_string());
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

/// Spawn background tasks to load standings for all series.
fn spawn_standings_loaders(
    app: &mut App,
    tx: mpsc::UnboundedSender<AppEvent>,
    force_refresh: bool,
) {
    let current_year = chrono::Utc::now().year() as u32;
    let series_ids: Vec<String> = app.series_registry.keys().cloned().collect();

    for series_id in series_ids {
        // 1. Try loading from cache (unless forced refresh)
        if !force_refresh {
            if let Ok(Some(cached)) =
                data::standings::read_standings_cache(&series_id, current_year)
            {
                let age_hours = chrono::Utc::now()
                    .signed_duration_since(cached.fetched_at)
                    .num_hours() as u64;
                app.standings.insert(series_id.clone(), cached);

                if age_hours < app.config.cache_ttl_hours {
                    continue;
                }
            }
        }

        // 2. Check if we have a standings fetcher for this series
        let fetcher = match scraper::get_standings_fetcher(&series_id) {
            Some(f) => f,
            None => continue,
        };

        // 3. Spawn async fetch task
        let fetch_tx = tx.clone();
        let sid = series_id.clone();
        tokio::spawn(async move {
            match fetcher.fetch_standings_boxed(current_year).await {
                Ok(standings) => {
                    if let Err(e) = data::standings::write_standings_cache(&standings) {
                        tracing::warn!("Failed to write standings cache for {}: {}", sid, e);
                    }
                    let _ = fetch_tx.send(AppEvent::StandingsFetched {
                        series_id: sid,
                        standings,
                    });
                }
                Err(e) => {
                    let _ = fetch_tx.send(AppEvent::StandingsFetchError {
                        series_id: sid,
                        error: e.to_string(),
                    });
                }
            }
        });
    }
}

/// Spawn background tasks to load race results and qualifying for the most recent completed event per series.
fn spawn_results_loaders(app: &mut App, tx: mpsc::UnboundedSender<AppEvent>) {
    let series_ids: Vec<String> = app.series_registry.keys().cloned().collect();

    for series_id in series_ids {
        // Find most recent completed event with a round number for this series
        let recent_completed = app.events.get(&series_id).and_then(|evs| {
            evs.iter()
                .filter(|e| e.status == data::models::EventStatus::Completed && e.round.is_some())
                .max_by_key(|e| e.end_date)
        });

        if let Some(event) = recent_completed {
            let round = match event.round {
                Some(r) => r,
                None => continue,
            };
            let season = event.end_date.year() as u32;

            // 1. Race results cache / fetch
            if let Ok(Some(cached)) = data::results::read_results_cache(&series_id, round) {
                app.results.insert((series_id.clone(), round), cached);
            } else if let Some(fetcher) = scraper::get_results_fetcher(&series_id) {
                let fetch_tx = tx.clone();
                let sid = series_id.clone();
                tokio::spawn(async move {
                    match fetcher.fetch_results_boxed(season, round).await {
                        Ok(results) => {
                            let _ = data::results::write_results_cache(&results);
                            let _ = fetch_tx.send(AppEvent::ResultsFetched {
                                series_id: sid,
                                round,
                                results,
                            });
                        }
                        Err(e) => {
                            let _ = fetch_tx.send(AppEvent::ResultsFetchError {
                                series_id: sid,
                                round,
                                error: e.to_string(),
                            });
                        }
                    }
                });
            }

            // 2. Qualifying results cache / fetch
            if let Ok(Some(cached)) = data::results::read_qualifying_cache(&series_id, round) {
                app.qualifying_results
                    .insert((series_id.clone(), round), cached);
            } else if let Some(fetcher) = scraper::get_results_fetcher(&series_id) {
                let fetch_tx = tx.clone();
                let sid = series_id.clone();
                tokio::spawn(async move {
                    match fetcher.fetch_qualifying_boxed(season, round).await {
                        Ok(results) => {
                            let _ = data::results::write_qualifying_cache(&results);
                            let _ = fetch_tx.send(AppEvent::QualifyingFetched {
                                series_id: sid,
                                round,
                                results,
                            });
                        }
                        Err(e) => {
                            let _ = fetch_tx.send(AppEvent::QualifyingFetchError {
                                series_id: sid,
                                round,
                                error: e.to_string(),
                            });
                        }
                    }
                });
            }
        }
    }
}

/// Trigger on-demand fetching of results / qualifying / sprint for the currently selected event if opened in detail view.
fn fetch_results_on_demand(app: &mut App, tx: mpsc::UnboundedSender<AppEvent>) {
    if !app.show_detail {
        return;
    }
    let event = match app.selected_event() {
        Some(e) if e.status == data::models::EventStatus::Completed && e.round.is_some() => e,
        _ => return,
    };
    let round = event.round.unwrap();
    let series_id = event.series_id.clone();
    let season = event.end_date.year() as u32;

    match app.detail_tab {
        app::DetailTab::Qualifying => {
            if let Ok(Some(cached)) = data::results::read_qualifying_cache(&series_id, round) {
                app.qualifying_results
                    .insert((series_id.clone(), round), cached);
                return;
            }
            let key = (series_id.clone(), round, "qualifying".to_string());
            if !app
                .qualifying_results
                .contains_key(&(series_id.clone(), round))
                && !app.results_fetching.contains(&key)
            {
                if let Some(fetcher) = scraper::get_results_fetcher(&series_id) {
                    app.results_fetching.insert(key);
                    let sid = series_id.clone();
                    tokio::spawn(async move {
                        match fetcher.fetch_qualifying_boxed(season, round).await {
                            Ok(results) => {
                                let _ = data::results::write_qualifying_cache(&results);
                                let _ = tx.send(AppEvent::QualifyingFetched {
                                    series_id: sid,
                                    round,
                                    results,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::QualifyingFetchError {
                                    series_id: sid,
                                    round,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
            }
        }
        app::DetailTab::Sprint => {
            let key = (series_id.clone(), round, "sprint".to_string());
            if !app.sprint_results.contains_key(&(series_id.clone(), round))
                && !app.results_fetching.contains(&key)
            {
                if let Some(fetcher) = scraper::get_results_fetcher(&series_id) {
                    app.results_fetching.insert(key);
                    let sid = series_id.clone();
                    tokio::spawn(async move {
                        match fetcher.fetch_sprint_boxed(season, round).await {
                            Ok(results) => {
                                let _ = tx.send(AppEvent::SprintFetched {
                                    series_id: sid,
                                    round,
                                    results,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::SprintFetchError {
                                    series_id: sid,
                                    round,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
            }
        }
        app::DetailTab::SprintQualifying => {
            if let Ok(Some(cached)) = data::results::read_qualifying_cache(&series_id, round) {
                app.qualifying_results
                    .insert((series_id.clone(), round), cached);
                return;
            }
            let key = (series_id.clone(), round, "qualifying".to_string());
            if !app
                .qualifying_results
                .contains_key(&(series_id.clone(), round))
                && !app.results_fetching.contains(&key)
            {
                if let Some(fetcher) = scraper::get_results_fetcher(&series_id) {
                    app.results_fetching.insert(key);
                    let sid = series_id.clone();
                    tokio::spawn(async move {
                        match fetcher.fetch_qualifying_boxed(season, round).await {
                            Ok(results) => {
                                let _ = data::results::write_qualifying_cache(&results);
                                let _ = tx.send(AppEvent::QualifyingFetched {
                                    series_id: sid,
                                    round,
                                    results,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::QualifyingFetchError {
                                    series_id: sid,
                                    round,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
            }
        }
        app::DetailTab::Race => {
            if let Ok(Some(cached)) = data::results::read_results_cache(&series_id, round) {
                app.results.insert((series_id.clone(), round), cached);
                return;
            }
            let key = (series_id.clone(), round, "race".to_string());
            if !app.results.contains_key(&(series_id.clone(), round))
                && !app.results_fetching.contains(&key)
            {
                if let Some(fetcher) = scraper::get_results_fetcher(&series_id) {
                    app.results_fetching.insert(key);
                    let sid = series_id.clone();
                    tokio::spawn(async move {
                        match fetcher.fetch_results_boxed(season, round).await {
                            Ok(results) => {
                                let _ = data::results::write_results_cache(&results);
                                let _ = tx.send(AppEvent::ResultsFetched {
                                    series_id: sid,
                                    round,
                                    results,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::ResultsFetchError {
                                    series_id: sid,
                                    round,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
            }
        }
        app::DetailTab::Schedule => {}
    }
}

fn setup_logging() {
    if let Some(data_dir) = dirs::data_dir() {
        let log_dir = data_dir.join("racetui");
        std::fs::create_dir_all(&log_dir).ok();
        if let Ok(log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("racetui.log"))
        {
            let _ = tracing_subscriber::fmt()
                .with_writer(log_file)
                .with_ansi(false)
                .try_init();
        }
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
    let cli = Cli::parse();

    // Handle --clear-cache
    if cli.clear_cache {
        data::cache::clear_all_cache()?;
        println!("Cache cleared.");
        return Ok(());
    }

    setup_panic_hook();
    setup_logging();

    // Load series registry
    let registry = data::series_registry::load_series_registry(Path::new("data/series.toml"))?;

    // Load user config
    let mut config = config::UserConfig::load()?;

    // Handle --refresh: set TTL to 0 so everything is re-fetched
    if cli.refresh {
        config.cache_ttl_hours = 0;
    }

    // Handle --calendar: override default view
    if cli.calendar {
        config.default_view = "calendar".to_string();
    }

    // Create app state
    let mut app = App::new(registry, config);

    // Handle --series: set initial filter
    if let Some(ref series_id) = cli.series {
        app.active_filters.series.insert(series_id.clone());
    }

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
            if crossterm_event::poll(std::time::Duration::from_millis(10)).unwrap_or(false) {
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
    spawn_data_loaders(&mut app, tx.clone(), cli.refresh);
    spawn_standings_loaders(&mut app, tx.clone(), cli.refresh);

    // Main loop
    while app.running {
        // Draw
        terminal.draw(|frame| {
            ui::draw(frame, &mut app);
        })?;

        // Handle refresh requested from app
        if app.refresh_requested {
            app.refresh_requested = false;
            app.results_loading_started = false;
            spawn_data_loaders(&mut app, tx.clone(), true);
            spawn_standings_loaders(&mut app, tx.clone(), true);
        }

        // Wait for next event, then drain any pending events to prevent momentum/lag
        if let Some(mut event) = rx.recv().await {
            loop {
                match event {
                    AppEvent::Key(key) => {
                        handle_key_event(&mut app, key);
                        fetch_results_on_demand(&mut app, tx.clone());
                    }
                    AppEvent::Resize(_, _) => {
                        // Terminal auto-redraws on resize
                    }
                    AppEvent::Tick => {
                        app.tick_count += 1;
                        // Toggle live session blinking every 2 ticks
                        if app.tick_count % 2 == 0 {
                            app.live_blink_on = !app.live_blink_on;
                        }
                        // Cycle notifications every 3 seconds
                        if app.tick_count % 3 == 0 {
                            let notification_count = app.get_notifications().len();
                            if notification_count > 0 {
                                app.notification_cycle_index =
                                    (app.notification_cycle_index + 1) % notification_count;
                            }
                        }
                        // Auto-clear status message after 3 seconds
                        if let Some(set_at) = app.status_message_set_at {
                            if set_at.elapsed() >= std::time::Duration::from_secs(3) {
                                app.status_message = None;
                                app.status_message_set_at = None;
                            }
                        }
                    }
                    AppEvent::RefreshRequested => {
                        app.results_loading_started = false;
                        spawn_data_loaders(&mut app, tx.clone(), true);
                        spawn_standings_loaders(&mut app, tx.clone(), true);
                    }
                    AppEvent::SeriesDataFetched { series_id, events } => {
                        app.update_series_data(series_id, events);
                        if !app.results_loading_started {
                            app.results_loading_started = true;
                            spawn_results_loaders(&mut app, tx.clone());
                        }
                    }
                    AppEvent::FetchError { series_id, error } => {
                        tracing::warn!("Failed to fetch series {}: {}", series_id, error);
                        app.mark_fetch_error(&series_id, error);
                    }
                    AppEvent::FetchStarted { series_id } => {
                        app.mark_fetching(&series_id);
                    }
                    AppEvent::StandingsFetched {
                        series_id,
                        standings,
                    } => {
                        app.standings.insert(series_id, standings);
                    }
                    AppEvent::StandingsFetchError { series_id, error } => {
                        tracing::warn!("Failed to fetch standings for {}: {}", series_id, error);
                    }
                    AppEvent::ResultsFetched {
                        series_id,
                        round,
                        results,
                    } => {
                        app.results_fetching.remove(&(
                            series_id.clone(),
                            round,
                            "race".to_string(),
                        ));
                        app.results.insert((series_id, round), results);
                    }
                    AppEvent::ResultsFetchError {
                        series_id,
                        round,
                        error,
                    } => {
                        app.results_fetching.remove(&(
                            series_id.clone(),
                            round,
                            "race".to_string(),
                        ));
                        tracing::warn!(
                            "Failed to fetch results for {} round {}: {}",
                            series_id,
                            round,
                            error
                        );
                    }
                    AppEvent::QualifyingFetched {
                        series_id,
                        round,
                        results,
                    } => {
                        app.results_fetching.remove(&(
                            series_id.clone(),
                            round,
                            "qualifying".to_string(),
                        ));
                        app.qualifying_results.insert((series_id, round), results);
                    }
                    AppEvent::QualifyingFetchError {
                        series_id,
                        round,
                        error,
                    } => {
                        app.results_fetching.remove(&(
                            series_id.clone(),
                            round,
                            "qualifying".to_string(),
                        ));
                        tracing::warn!(
                            "Failed to fetch qualifying results for {} round {}: {}",
                            series_id,
                            round,
                            error
                        );
                    }
                    AppEvent::SprintFetched {
                        series_id,
                        round,
                        results,
                    } => {
                        app.results_fetching.remove(&(
                            series_id.clone(),
                            round,
                            "sprint".to_string(),
                        ));
                        app.sprint_results.insert((series_id, round), results);
                    }
                    AppEvent::SprintFetchError {
                        series_id,
                        round,
                        error,
                    } => {
                        app.results_fetching.remove(&(
                            series_id.clone(),
                            round,
                            "sprint".to_string(),
                        ));
                        tracing::warn!(
                            "Failed to fetch sprint results for {} round {}: {}",
                            series_id,
                            round,
                            error
                        );
                    }
                }

                if !app.running {
                    break;
                }

                match rx.try_recv() {
                    Ok(next_ev) => event = next_ev,
                    Err(_) => break,
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
                if app.view_mode == app::ViewMode::Standings {
                    let matching = app.standings_series_list();
                    if let Some(first) = matching.first() {
                        app.standings_selected_series = Some(first.clone());
                        app.standings_series_index = 0;
                        app.standings_table_state = ratatui::widgets::TableState::default();
                    }
                }
            }
            KeyCode::Char(c) => {
                app.search_query.get_or_insert_with(String::new).push(c);
                if app.view_mode == app::ViewMode::Standings {
                    let matching = app.standings_series_list();
                    if let Some(first) = matching.first() {
                        app.standings_selected_series = Some(first.clone());
                        app.standings_series_index = 0;
                        app.standings_table_state = ratatui::widgets::TableState::default();
                    }
                }
            }
            KeyCode::Left | KeyCode::Right if app.view_mode == app::ViewMode::Standings => {
                let dir = if key.code == KeyCode::Right { 1 } else { -1 };
                app.standings_cycle_series(dir);
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
                app.set_status_message(format!("{} favorites: {}", action, series_name));
                app.pending_favorite_toggle = None;
            }
            KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('Q') => {
                app.pending_favorite_toggle = None;
            }
            _ => {}
        }
        return;
    }

    // If help popup is active, handle help keys
    if app.show_help {
        match key.code {
            KeyCode::Esc
            | KeyCode::Enter
            | KeyCode::Char('?')
            | KeyCode::Char('q')
            | KeyCode::Char('Q') => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    // If day events popup is active, handle day selection keys
    if app.show_day_events {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_day_events = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.day_events_select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.day_events_select_prev();
            }
            KeyCode::Enter => {
                if let Some(event) = app.selected_day_event().cloned() {
                    app.show_day_events = false;
                    // Find this event in list_table_items to set table_state
                    let items = app.list_table_items();
                    if let Some(idx) = items.iter().position(|item| match item {
                        app::ListTableItem::Session(s) => {
                            s.event.series_id == event.series_id
                                && s.event.event_name == event.event_name
                        }
                        _ => false,
                    }) {
                        app.table_state.select(Some(idx));
                    }
                    app.detail_tab = app::DetailTab::Race;
                    app.show_detail = true;
                }
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
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                app.next_detail_tab();
                return;
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                app.prev_detail_tab();
                return;
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                app.select_detail_tab_by_digit(c);
                return;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Open first stream link
                if let Some(event) = app.selected_event() {
                    if let Some(link) = event.stream_links.first() {
                        if let Err(e) = action::open_url(&app.config.open_command, &link.url) {
                            app.set_status_message(format!("Error: {}", e));
                        } else {
                            app.set_status_message(format!(
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
        // Navigation (List / Calendar / Live / Standings)
        KeyCode::Char('h') | KeyCode::Left => match app.view_mode {
            app::ViewMode::Calendar => app.calendar_select_prev_day(),
            app::ViewMode::Live => app.live_sub_tab = app::LiveSubTab::Timing,
            app::ViewMode::Standings => app.standings_cycle_series(-1),
            _ => {}
        },
        KeyCode::Char('l') | KeyCode::Right => match app.view_mode {
            app::ViewMode::Calendar => app.calendar_select_next_day(),
            app::ViewMode::Live => app.live_sub_tab = app::LiveSubTab::TrackMap,
            app::ViewMode::Standings => app.standings_cycle_series(1),
            _ => {}
        },
        KeyCode::Char('j') | KeyCode::Down => match app.view_mode {
            app::ViewMode::Calendar => app.calendar_select_next_week(),
            app::ViewMode::Live => app.live_select_next(),
            app::ViewMode::Standings => app.standings_select_next(),
            _ => app.select_next(),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.view_mode {
            app::ViewMode::Calendar => app.calendar_select_prev_week(),
            app::ViewMode::Live => app.live_select_previous(),
            app::ViewMode::Standings => app.standings_select_previous(),
            _ => app.select_previous(),
        },
        KeyCode::Char('H') | KeyCode::PageUp => {
            if app.view_mode == app::ViewMode::Calendar {
                app.calendar_select_prev_month();
            }
        }
        KeyCode::Char('L') | KeyCode::PageDown => {
            if app.view_mode == app::ViewMode::Calendar {
                app.calendar_select_next_month();
            }
        }
        // Jump to today in calendar
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if app.view_mode == app::ViewMode::Calendar {
                app.calendar_jump_to_today();
            }
        }
        // View switching
        KeyCode::Char('1') => {
            app.view_mode = app::ViewMode::List;
        }
        KeyCode::Char('2') => {
            app.view_mode = app::ViewMode::Calendar;
        }
        KeyCode::Char('3') => {
            app.view_mode = app::ViewMode::Live;
        }
        KeyCode::Char('4') => {
            app.view_mode = app::ViewMode::Standings;
        }
        // Search
        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_query = Some(String::new());
        }
        // Refresh
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.set_status_message("Refreshing...".to_string());
            app.refresh_requested = true;
        }
        // Help
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
        }
        // Toggle detail view / View day races in calendar / Toggle live driver detail
        KeyCode::Enter => {
            if app.view_mode == app::ViewMode::Live {
                app.live_driver_detail_open = !app.live_driver_detail_open;
            } else if app.view_mode == app::ViewMode::Calendar {
                let day_events = app.events_on_selected_calendar_day();
                if day_events.is_empty() {
                    app.set_status_message(format!(
                        "No races on {} {}, {}",
                        crate::ui::calendar_view::month_name(app.calendar_month),
                        app.calendar_selected_day,
                        app.calendar_year
                    ));
                } else if day_events.len() == 1 {
                    let event = day_events[0].event;
                    let items = app.list_table_items();
                    if let Some(idx) = items.iter().position(|item| match item {
                        app::ListTableItem::Session(s) => {
                            s.event.series_id == event.series_id
                                && s.event.event_name == event.event_name
                        }
                        _ => false,
                    }) {
                        app.table_state.select(Some(idx));
                    }
                    app.detail_tab = app::DetailTab::Race;
                    app.show_detail = true;
                } else {
                    app.show_day_events = true;
                    app.day_events_state.select(Some(0));
                }
            } else if !app.show_detail {
                let event = app.selected_event();
                let has_sprint_q = event.map_or(false, |e| e.has_sprint_qualifying());
                let has_sprint = event.map_or(false, |e| e.has_sprint());
                let (session_type, session_name) = match app.selected_session() {
                    Some(s) => (Some(s.session_type), s.session_name.to_lowercase()),
                    None => (None, String::new()),
                };

                app.detail_tab = if session_name.contains("sprint qualifying")
                    || session_name.contains("sprint shootout")
                    || session_type == Some(data::models::SessionType::SprintQualifying)
                {
                    if has_sprint_q {
                        app::DetailTab::SprintQualifying
                    } else if has_sprint {
                        app::DetailTab::Sprint
                    } else {
                        app::DetailTab::Qualifying
                    }
                } else if session_name.contains("sprint")
                    || session_type == Some(data::models::SessionType::Sprint)
                {
                    if has_sprint {
                        app::DetailTab::Sprint
                    } else {
                        app::DetailTab::Race
                    }
                } else if session_name.contains("qualifying")
                    || session_name.contains("superpole")
                    || session_type == Some(data::models::SessionType::Qualifying)
                {
                    app::DetailTab::Qualifying
                } else if session_name.contains("practice")
                    || session_name.contains("fp")
                    || session_name.contains("shakedown")
                    || session_name.contains("warmup")
                    || session_type == Some(data::models::SessionType::Practice)
                    || session_type == Some(data::models::SessionType::Warmup)
                {
                    app::DetailTab::Schedule
                } else {
                    app::DetailTab::Race
                };
                app.show_detail = true;
            } else {
                app.show_detail = false;
            }
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
            } else if app.show_day_events {
                app.show_day_events = false;
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
    use chrono::Datelike;
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

        // Keys 1-4 switch views
        handle_key_event(&mut app, key(KeyCode::Char('2')));
        assert_eq!(app.view_mode, app::ViewMode::Calendar);

        handle_key_event(&mut app, key(KeyCode::Char('3')));
        assert_eq!(app.view_mode, app::ViewMode::Live);

        handle_key_event(&mut app, key(KeyCode::Char('4')));
        assert_eq!(app.view_mode, app::ViewMode::Standings);

        handle_key_event(&mut app, key(KeyCode::Char('1')));
        assert_eq!(app.view_mode, app::ViewMode::List);

        // Tab should do nothing in normal mode now
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
    fn test_standings_view_keybindings() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            data::models::Series {
                id: "f1".to_string(),
                name: "Formula 1".to_string(),
                short_name: "F1".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (235, 0, 0),
                region: "Global".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );
        registry.insert(
            "indycar".to_string(),
            data::models::Series {
                id: "indycar".to_string(),
                name: "IndyCar".to_string(),
                short_name: "IndyCar".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (0, 100, 255),
                region: "USA".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, config::UserConfig::default());
        app.view_mode = app::ViewMode::Standings;
        app.standings.insert(
            "f1".to_string(),
            crate::data::standings::SeasonStandings {
                series_id: "f1".to_string(),
                season: 2026,
                drivers: vec![
                    crate::data::standings::DriverStanding {
                        position: 1,
                        driver_name: "Max Verstappen".to_string(),
                        driver_code: Some("VER".to_string()),
                        driver_number: Some(1),
                        team: "Red Bull Racing".to_string(),
                        points: 25.0,
                        wins: 1,
                    },
                    crate::data::standings::DriverStanding {
                        position: 2,
                        driver_name: "Lando Norris".to_string(),
                        driver_code: Some("NOR".to_string()),
                        driver_number: Some(4),
                        team: "McLaren".to_string(),
                        points: 18.0,
                        wins: 0,
                    },
                ],
                constructors: vec![],
                fetched_at: chrono::Utc::now(),
            },
        );
        app.standings_selected_series = Some("f1".to_string());

        // Test Left/Right/h/l series cycling
        handle_key_event(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.standings_selected_series.as_deref(), Some("indycar"));

        handle_key_event(&mut app, key(KeyCode::Char('h')));
        assert_eq!(app.standings_selected_series.as_deref(), Some("f1"));

        // Test Up/Down/j/k driver selection
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.standings_table_state.selected(), Some(1));

        handle_key_event(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.standings_table_state.selected(), Some(0));
    }

    #[test]
    fn test_standings_view_search_keybindings() {
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            data::models::Series {
                id: "f1".to_string(),
                name: "Formula 1".to_string(),
                short_name: "F1".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (235, 0, 0),
                region: "Global".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );
        registry.insert(
            "indycar".to_string(),
            data::models::Series {
                id: "indycar".to_string(),
                name: "IndyCar Series".to_string(),
                short_name: "IndyCar".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (0, 100, 255),
                region: "USA".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, config::UserConfig::default());
        app.view_mode = app::ViewMode::Standings;

        // Activate search
        handle_key_event(&mut app, key(KeyCode::Char('/')));
        assert!(app.search_active);

        // Type 'i' 'n' 'd' -> matches indycar
        handle_key_event(&mut app, key(KeyCode::Char('i')));
        handle_key_event(&mut app, key(KeyCode::Char('n')));
        handle_key_event(&mut app, key(KeyCode::Char('d')));
        assert_eq!(app.standings_selected_series.as_deref(), Some("indycar"));

        // Enter finishes search
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(!app.search_active);
        assert_eq!(app.standings_selected_series.as_deref(), Some("indycar"));
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
            Some(&data::models::FetchStatus::Error(
                "No scraper implemented yet".to_string()
            ))
        );
    }

    #[test]
    fn test_calendar_navigation() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        app.view_mode = app::ViewMode::Calendar;
        app.calendar_year = 2026;
        app.calendar_month = 5;
        app.calendar_selected_day = 15;

        // Next day
        handle_key_event(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.calendar_selected_day, 16);

        // Prev day
        handle_key_event(&mut app, key(KeyCode::Char('h')));
        assert_eq!(app.calendar_selected_day, 15);

        // Next week (+7 days)
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.calendar_selected_day, 22);

        // Prev week (-7 days)
        handle_key_event(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.calendar_selected_day, 15);

        // Next month with L
        handle_key_event(&mut app, key(KeyCode::Char('L')));
        assert_eq!(app.calendar_month, 6);

        // Prev month with H
        handle_key_event(&mut app, key(KeyCode::Char('H')));
        assert_eq!(app.calendar_month, 5);

        // Jump to today with 't'
        handle_key_event(&mut app, key(KeyCode::Char('t')));
        let now = chrono::Local::now();
        assert_eq!(app.calendar_year, now.year());
        assert_eq!(app.calendar_month, now.month());
        assert_eq!(app.calendar_selected_day, now.day());
    }

    #[test]
    fn test_calendar_day_selection_and_events_popup() {
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
        registry.insert(
            "indycar".to_string(),
            data::models::Series {
                id: "indycar".to_string(),
                name: "IndyCar".to_string(),
                short_name: "IndyCar".to_string(),
                car_style: data::models::CarStyle::OpenWheel,
                color: (0, 0, 255),
                region: "USA".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, config::UserConfig::default());
        app.view_mode = app::ViewMode::Calendar;
        app.calendar_year = 2026;
        app.calendar_month = 5;
        app.calendar_selected_day = 24;

        let event1 = data::models::RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Monaco Grand Prix".to_string(),
            circuit_name: "Circuit de Monaco".to_string(),
            location: "Monte Carlo".to_string(),
            country: "Monaco".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            round: Some(8),
            sessions: vec![],
            stream_links: vec![],
            status: data::models::EventStatus::Upcoming,
        };
        let event2 = data::models::RaceEvent {
            series_id: "indycar".to_string(),
            event_name: "Indy 500".to_string(),
            circuit_name: "Indianapolis Motor Speedway".to_string(),
            location: "Indianapolis".to_string(),
            country: "USA".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            round: Some(6),
            sessions: vec![],
            stream_links: vec![],
            status: data::models::EventStatus::Upcoming,
        };

        app.update_series_data("f1".to_string(), vec![event1]);
        app.update_series_data("indycar".to_string(), vec![event2]);

        // Press Enter on day 24 (has 2 events) -> opens day events popup
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.show_day_events);
        assert!(!app.show_detail);

        // Navigate in popup
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.day_events_state.selected(), Some(1));

        // Press Enter to select Indy 500 -> opens detail view
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(!app.show_day_events);
        assert!(app.show_detail);
        assert_eq!(
            app.selected_event().map(|e| e.event_name.as_str()),
            Some("Indy 500")
        );
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
        assert_eq!(app.status_message.as_deref(), Some("Opened F1TV in true"));

        // Tab switching in detail view for event without sprint (Race, Qualifying, Schedule)
        assert_eq!(app.detail_tab, app::DetailTab::Race);
        handle_key_event(&mut app, key(KeyCode::Tab));
        assert_eq!(app.detail_tab, app::DetailTab::Qualifying);
        handle_key_event(&mut app, key(KeyCode::Tab));
        assert_eq!(app.detail_tab, app::DetailTab::Schedule);
        handle_key_event(&mut app, key(KeyCode::Tab));
        assert_eq!(app.detail_tab, app::DetailTab::Race);

        // Digit keys for direct tab selection
        handle_key_event(&mut app, key(KeyCode::Char('2')));
        assert_eq!(app.detail_tab, app::DetailTab::Qualifying);
        handle_key_event(&mut app, key(KeyCode::Char('3')));
        assert_eq!(app.detail_tab, app::DetailTab::Schedule);
        handle_key_event(&mut app, key(KeyCode::Char('1')));
        assert_eq!(app.detail_tab, app::DetailTab::Race);

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
        assert_eq!(
            app.status_message.as_deref(),
            Some("Added to favorites: Formula 1")
        );

        // Press 'f' and confirm removal with Enter
        handle_key_event(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.pending_favorite_toggle.as_deref(), Some("f1"));
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert_eq!(app.pending_favorite_toggle, None);
        assert!(!app.config.favorites.contains("f1"));
        assert_eq!(
            app.status_message.as_deref(),
            Some("Removed from favorites: Formula 1")
        );
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

    #[test]
    fn test_2027_schedules_and_session_start_times() {
        use chrono::Datelike;
        // Test F1 2027
        let f1_events = scraper::f1::get_official_2027_f1_schedule("f1");
        assert_eq!(f1_events.len(), 24);
        assert!(f1_events.iter().all(|e| e.start_date.year() == 2027));
        assert!(f1_events
            .iter()
            .all(|e| !e.sessions.is_empty() && e.sessions.iter().all(|s| s.start_time.is_some())));

        // Test F2 2027
        let f2_events = scraper::f2::get_official_2027_f2_schedule("f2");
        assert_eq!(f2_events.len(), 14);
        assert!(f2_events.iter().all(|e| e.start_date.year() == 2027));

        // Test F3 2027
        let f3_events = scraper::f3::get_official_2027_f3_schedule("f3");
        assert_eq!(f3_events.len(), 10);
        assert!(f3_events.iter().all(|e| e.start_date.year() == 2027));

        // Test Formula E 2027
        let fe_events = scraper::formula_e::get_official_2027_formula_e_schedule("formula_e");
        assert_eq!(fe_events.len(), 15);
        assert!(fe_events.iter().all(|e| e.start_date.year() == 2027));

        // Test IndyCar 2027
        let indy_events = scraper::indycar::get_official_2027_indycar_schedule("indycar");
        assert_eq!(indy_events.len(), 18);
        assert!(indy_events.iter().all(|e| e.start_date.year() == 2027));

        // Test MotoGP 2027
        let motogp_events = scraper::motogp::get_official_2027_motogp_schedule("motogp");
        assert_eq!(motogp_events.len(), 21);
        assert!(motogp_events.iter().all(|e| e.start_date.year() == 2027));

        // Test Moto2 & Moto3 2027
        let moto2_events = scraper::motogp::get_official_2027_motogp_schedule("moto2");
        assert_eq!(moto2_events.len(), 21);
        assert_eq!(moto2_events[0].event_name, "Thai Moto2 Grand Prix");
        assert!(moto2_events[0]
            .sessions
            .iter()
            .all(|s| s.start_time.is_some()));

        let moto3_events = scraper::motogp::get_official_2027_motogp_schedule("moto3");
        assert_eq!(moto3_events.len(), 21);
        assert_eq!(moto3_events[0].event_name, "Thai Moto3 Grand Prix");
        assert!(moto3_events[0]
            .sessions
            .iter()
            .all(|s| s.start_time.is_some()));

        // Test IMSA 2027
        let imsa_events = scraper::imsa::get_official_2027_imsa_schedule("imsa");
        assert_eq!(imsa_events.len(), 11);
        assert!(imsa_events.iter().all(|e| e.start_date.year() == 2027));

        // Test WEC 2027
        let wec_events = scraper::wec::get_official_2027_wec_schedule("wec");
        assert_eq!(wec_events.len(), 8);
        assert!(wec_events.iter().all(|e| e.start_date.year() == 2027));

        // Test BTCC 2027
        let btcc_events = scraper::btcc::get_official_2027_btcc_schedule("btcc");
        assert_eq!(btcc_events.len(), 10);
        assert!(btcc_events.iter().all(|e| e.start_date.year() == 2027));

        // Test DTM 2027
        let dtm_events = scraper::dtm::get_official_2027_dtm_schedule("dtm");
        assert_eq!(dtm_events.len(), 8);
        assert!(dtm_events.iter().all(|e| e.start_date.year() == 2027));

        // Test Super Formula 2027
        let sf_events =
            scraper::super_formula::get_official_2027_super_formula_schedule("super_formula");
        assert_eq!(sf_events.len(), 7);
        assert!(sf_events.iter().all(|e| e.start_date.year() == 2027));

        // Test Super GT 2027
        let sgt_events = scraper::super_gt::get_official_2027_super_gt_schedule("super_gt");
        assert_eq!(sgt_events.len(), 8);
        assert!(sgt_events.iter().all(|e| e.start_date.year() == 2027));

        // Test WRC 2027
        let wrc_events = scraper::wrc::get_official_2027_wrc_schedule("wrc");
        assert_eq!(wrc_events.len(), 14);
        assert!(wrc_events.iter().all(|e| e.start_date.year() == 2027));
        assert!(wrc_events
            .iter()
            .all(|e| !e.sessions.is_empty() && e.sessions.iter().all(|s| s.start_time.is_some())));
    }

    #[test]
    fn test_cli_parsing() {
        let cli =
            Cli::try_parse_from(["racetui", "--refresh", "--calendar", "--series", "f1"]).unwrap();
        assert!(cli.refresh);
        assert!(cli.calendar);
        assert_eq!(cli.series.as_deref(), Some("f1"));
        assert!(!cli.clear_cache);

        let clear_cli = Cli::try_parse_from(["racetui", "--clear-cache"]).unwrap();
        assert!(clear_cli.clear_cache);
        assert!(!clear_cli.refresh);
        assert!(!clear_cli.calendar);
        assert!(clear_cli.series.is_none());
    }

    #[test]
    fn test_status_message_timeout() {
        let mut app = App::new(HashMap::new(), config::UserConfig::default());
        app.set_status_message("Test message".to_string());
        assert_eq!(app.status_message.as_deref(), Some("Test message"));
        assert!(app.status_message_set_at.is_some());

        // Simulate set_at 4 seconds in the past
        app.status_message_set_at =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(4));
        if let Some(set_at) = app.status_message_set_at {
            if set_at.elapsed() >= std::time::Duration::from_secs(3) {
                app.status_message = None;
                app.status_message_set_at = None;
            }
        }
        assert!(app.status_message.is_none());
        assert!(app.status_message_set_at.is_none());
    }

    #[tokio::test]
    async fn test_all_scrapers_execution() {
        let registry =
            data::series_registry::load_series_registry(Path::new("data/series.toml")).unwrap();
        assert_eq!(registry.len(), 36);
        for (id, series) in &registry {
            let scraper = scraper::get_scraper(id);
            assert!(scraper.is_some(), "No scraper registered for {}", id);
            let res = scraper.unwrap().scrape_boxed(series).await;
            assert!(res.is_ok(), "Scraper failed for {}: {:?}", id, res.err());
            let events = res.unwrap();
            assert!(!events.is_empty(), "Scraper returned 0 events for {}", id);
        }
    }

    #[tokio::test]
    async fn test_spawn_standings_loaders() {
        let registry =
            data::series_registry::load_series_registry(Path::new("data/series.toml")).unwrap();
        let mut app = App::new(registry, config::UserConfig::default());
        let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

        spawn_standings_loaders(&mut app, tx, false);

        // Receive at least one standings event or timeout gracefully
        let timeout = tokio::time::sleep(std::time::Duration::from_millis(500));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    match event {
                        AppEvent::StandingsFetched { series_id, standings } => {
                            app.standings.insert(series_id, standings);
                            break;
                        }
                        AppEvent::StandingsFetchError { .. } => {}
                        _ => {}
                    }
                }
                _ = &mut timeout => {
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn test_spawn_results_loaders() {
        let registry =
            data::series_registry::load_series_registry(Path::new("data/series.toml")).unwrap();
        let mut app = App::new(registry, config::UserConfig::default());

        let completed_event = data::models::RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain GP".to_string(),
            circuit_name: "Bahrain".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: data::models::EventStatus::Completed,
        };
        app.update_series_data("f1".to_string(), vec![completed_event]);

        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();
        spawn_results_loaders(&mut app, tx);
    }

    #[test]
    fn test_session_specific_detail_tab_selection() {
        use chrono::TimeZone;
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
            event_name: "Chinese Grand Prix".to_string(),
            circuit_name: "Shanghai International Circuit".to_string(),
            location: "Shanghai".to_string(),
            country: "China".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 13).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            round: Some(2),
            sessions: vec![
                data::models::Session {
                    name: "Free Practice 1".to_string(),
                    session_type: data::models::SessionType::Practice,
                    start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 3, 13, 3, 30, 0).unwrap()),
                    end_time: None,
                },
                data::models::Session {
                    name: "Sprint Qualifying".to_string(),
                    session_type: data::models::SessionType::SprintQualifying,
                    start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 3, 13, 7, 30, 0).unwrap()),
                    end_time: None,
                },
                data::models::Session {
                    name: "Sprint Race".to_string(),
                    session_type: data::models::SessionType::Sprint,
                    start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 3, 14, 3, 0, 0).unwrap()),
                    end_time: None,
                },
                data::models::Session {
                    name: "Qualifying".to_string(),
                    session_type: data::models::SessionType::Qualifying,
                    start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 3, 14, 7, 0, 0).unwrap()),
                    end_time: None,
                },
                data::models::Session {
                    name: "Grand Prix".to_string(),
                    session_type: data::models::SessionType::Race,
                    start_time: Some(chrono::Utc.with_ymd_and_hms(2026, 3, 15, 7, 0, 0).unwrap()),
                    end_time: None,
                },
            ],
            stream_links: vec![],
            status: data::models::EventStatus::Completed,
        };

        app.update_series_data("f1".to_string(), vec![event]);
        app.active_filters
            .statuses
            .insert(data::models::EventStatus::Completed);

        // Verify available tabs for this sprint weekend
        let selected_ev = app.selected_event().unwrap();
        assert!(selected_ev.has_sprint());
        assert!(selected_ev.has_sprint_qualifying());
        let available = app.available_detail_tabs(selected_ev);
        let labels: Vec<String> = available.into_iter().map(|(_, l)| l).collect();
        assert_eq!(
            labels,
            vec![
                "1. Race".to_string(),
                "2. Qualifying".to_string(),
                "3. Sprint".to_string(),
                "4. Sprint Qual".to_string(),
                "5. Schedule".to_string(),
            ]
        );

        let (sprint_idx, qual_idx, race_idx, fp1_idx) = {
            let items = app.list_table_items();
            let s_idx = items
                .iter()
                .position(|item| match item {
                    app::ListTableItem::Session(s) => s.session_name == "Sprint Race",
                    _ => false,
                })
                .unwrap();
            let q_idx = items
                .iter()
                .position(|item| match item {
                    app::ListTableItem::Session(s) => s.session_name == "Qualifying",
                    _ => false,
                })
                .unwrap();
            let r_idx = items
                .iter()
                .position(|item| match item {
                    app::ListTableItem::Session(s) => s.session_name == "Grand Prix",
                    _ => false,
                })
                .unwrap();
            let f_idx = items
                .iter()
                .position(|item| match item {
                    app::ListTableItem::Session(s) => s.session_name == "Free Practice 1",
                    _ => false,
                })
                .unwrap();
            (s_idx, q_idx, r_idx, f_idx)
        };

        // Select row corresponding to Sprint Race
        app.table_state.select(Some(sprint_idx));
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.show_detail);
        assert_eq!(app.detail_tab, app::DetailTab::Sprint);
        app.show_detail = false;

        // Select row corresponding to Qualifying
        app.table_state.select(Some(qual_idx));
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.show_detail);
        assert_eq!(app.detail_tab, app::DetailTab::Qualifying);
        app.show_detail = false;

        // Select row corresponding to Grand Prix (Race)
        app.table_state.select(Some(race_idx));
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.show_detail);
        assert_eq!(app.detail_tab, app::DetailTab::Race);
        app.show_detail = false;

        // Select row corresponding to Free Practice 1
        app.table_state.select(Some(fp1_idx));
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.show_detail);
        assert_eq!(app.detail_tab, app::DetailTab::Schedule);
    }

    #[test]
    fn test_live_view_keybindings() {
        let mut app = App::new(HashMap::new(), config::settings::UserConfig::default());
        assert_eq!(app.view_mode, app::ViewMode::List);

        // Switch to Live view
        handle_key_event(&mut app, key(KeyCode::Char('3')));
        assert_eq!(app.view_mode, app::ViewMode::Live);
        assert_eq!(app.live_sub_tab, app::LiveSubTab::Timing);

        // Switch sub-tab to Track Map
        handle_key_event(&mut app, key(KeyCode::Right));
        assert_eq!(app.live_sub_tab, app::LiveSubTab::TrackMap);

        // Switch sub-tab back to Timing
        handle_key_event(&mut app, key(KeyCode::Left));
        assert_eq!(app.live_sub_tab, app::LiveSubTab::Timing);

        // Toggle driver detail with Enter
        assert!(!app.live_driver_detail_open);
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(app.live_driver_detail_open);
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert!(!app.live_driver_detail_open);
    }
}
