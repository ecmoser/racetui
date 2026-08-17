pub mod calendar_view;
pub mod detail_view;
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
        ViewMode::Calendar => calendar_view::draw(frame, app, chunks[0]),
    }

    // Draw the status bar
    status_bar::draw(frame, app, chunks[1]);

    // Draw detail popup overlay if visible
    if app.show_detail {
        detail_view::draw(frame, app);
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
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(HashMap::new(), UserConfig::default());

        terminal.draw(|f| {
            draw(f, &app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        // Check that Race Calendar and status hints are rendered
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Race Calendar"));
        assert!(content.contains("Quit"));
    }

    #[test]
    fn test_draw_calendar_view() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.view_mode = ViewMode::Calendar;

        terminal.draw(|f| {
            draw(f, &app);
        }).unwrap();

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

        terminal.draw(|f| {
            draw(f, &app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Monaco Grand Prix"));
        assert!(content.contains("Circuit de Monaco"));
    }
}


