pub mod calendar_view;
pub mod confirm_dialog;
pub mod day_events;
pub mod detail_view;
pub mod filter_panel;
pub mod help;
pub mod list_view;
pub mod live_driver_detail;
pub mod live_session_picker;
pub mod live_timing_table;
pub mod live_view;
pub mod standings_view;
pub mod status_bar;

use crate::app::{App, ViewMode};
use ratatui::prelude::*;

/// Main draw function — dispatches to the appropriate view.
pub fn draw(frame: &mut Frame, app: &mut App) {
    // Layout: main content area + status bar at the bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    // Draw the main content based on view mode
    match app.view_mode {
        ViewMode::List => list_view::draw(frame, app, chunks[0]),
        ViewMode::Calendar => calendar_view::draw(frame, app, chunks[0]),
        ViewMode::Live => live_view::draw(frame, app, chunks[0]),
        ViewMode::Standings => standings_view::draw(frame, app, chunks[0]),
    }

    // Draw the status bar
    status_bar::draw(frame, app, chunks[1]);

    // Draw filter panel overlay if visible
    if app.show_filter_panel {
        filter_panel::draw(frame, app);
    }

    // Draw day events selection popup if visible
    if app.show_day_events {
        day_events::draw(frame, app);
    }

    // Draw live session picker popup if visible
    if app.show_live_session_picker {
        live_session_picker::draw(frame, app);
    }

    // Draw detail popup overlay if visible
    if app.show_detail {
        detail_view::draw(frame, app);
    }

    // Draw confirmation dialog on top of everything if active
    if app.pending_favorite_toggle.is_some() {
        confirm_dialog::draw(frame, app);
    }

    // Draw help popup on top of all other overlays if visible
    if app.show_help {
        help::draw(frame, app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use crate::data::models::{CarStyle, EventStatus, RaceEvent, Series};
    use chrono::NaiveDate;
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_list_view() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());

        // 1. Empty state
        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Race Calendar"));
        assert!(content.contains("No events found"));
        assert!(content.contains("Try adjusting your filters"));
        assert!(content.contains("Quit"));

        // 2. Populated state
        let event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };
        app.update_series_data("f1".to_string(), vec![event]);

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Date") && content.contains("Time") && content.contains("Series"));
    }

    #[test]
    fn test_draw_list_view_with_day_headers_and_live_indicator() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());

        let event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Live,
        };
        app.update_series_data("f1".to_string(), vec![event]);

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Bahrain Grand Prix"));
        assert!(content.contains("LIVE"));
    }

    #[test]
    fn test_draw_calendar_view() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.view_mode = ViewMode::Calendar;

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Mon") && content.contains("Sun"));
    }

    #[test]
    fn test_draw_detail_view() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            Series {
                id: "f1".to_string(),
                name: "Formula 1".to_string(),
                short_name: "F1".to_string(),
                car_style: CarStyle::OpenWheel,
                color: (255, 0, 0),
                region: "International".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, UserConfig::default());
        let event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Monaco Grand Prix".to_string(),
            circuit_name: "Circuit de Monaco".to_string(),
            location: "Monte Carlo".to_string(),
            country: "Monaco".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            round: Some(8),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };

        app.update_series_data("f1".to_string(), vec![event]);
        app.show_detail = true;

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Monaco Grand Prix"));
        assert!(content.contains("Circuit de Monaco"));
    }

    #[test]
    fn test_draw_filter_panel() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.show_filter_panel = true;

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Filters"));
        assert!(content.contains("All Events"));
        assert!(content.contains("Favorites Only"));
    }

    #[test]
    fn test_draw_confirm_dialog() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut registry = HashMap::new();
        registry.insert(
            "f1".to_string(),
            Series {
                id: "f1".to_string(),
                name: "Formula 1".to_string(),
                short_name: "F1".to_string(),
                car_style: CarStyle::OpenWheel,
                color: (255, 0, 0),
                region: "International".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, UserConfig::default());
        app.pending_favorite_toggle = Some("f1".to_string());

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Add Favorite"));
        assert!(content.contains("Formula 1"));
        assert!(content.contains("Yes"));
        assert!(content.contains("No"));
    }

    #[test]
    fn test_draw_help_popup() {
        let backend = TestBackend::new(100, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.show_help = true;

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Keybindings"));
        assert!(content.contains("Views"));
        assert!(content.contains("Navigation"));
        assert!(content.contains("Actions"));
        assert!(content.contains("General"));
    }

    #[test]
    fn test_draw_standings_view() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.view_mode = ViewMode::Standings;

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Championship Standings"));
    }
}
