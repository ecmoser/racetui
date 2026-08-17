mod app;
mod config;
mod data;
mod event;
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
            ui::draw(frame, &app);
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
}
