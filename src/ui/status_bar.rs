use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::data::models::FetchStatus;

/// Draw the bottom status bar with keybind hints and fetch status.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Keybind hints
            Constraint::Percentage(40), // Fetch status / notifications
        ])
        .split(area);

    // Left side: keybind hints

    let hints = if app.search_active {
        " Type to search | Enter: confirm | Esc: cancel".to_string()
    } else {
        " 1-4:Views  q:Quit  j/k/↑/↓:Navigate  /:Search  f:Favorite  ?:Help  Enter:Detail"
            .to_string()
    };

    let hints_widget = Paragraph::new(hints).style(
        Style::default()
            .fg(Color::DarkGray)
            .bg(Color::Rgb(20, 20, 30)),
    );

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

    let live_sessions = app.get_live_sessions();
    let notifications = app.get_notifications();
    let (status_text, status_color) = if let Some(ref msg) = app.status_message {
        (msg.clone(), Color::White)
    } else if !live_sessions.is_empty() {
        let live_desc: Vec<String> = live_sessions
            .iter()
            .map(|(_, short_name, session_name)| format!("{}: {}", short_name, session_name))
            .collect();
        let desc = live_desc.join(", ");
        if app.live_blink_on {
            (format!("● LIVE: {} ", desc), Color::Red)
        } else {
            (format!("  LIVE: {} ", desc), Color::Yellow)
        }
    } else if !notifications.is_empty() {
        let idx = app.notification_cycle_index % notifications.len();
        (format!("{} ", notifications[idx]), Color::Yellow)
    } else if fetching > 0 {
        (
            format!("Fetching... {}/{} loaded ", loaded, total),
            Color::Yellow,
        )
    } else if errors > 0 {
        let mut failed_names: Vec<&str> = app
            .fetch_status
            .iter()
            .filter(|(_, s)| matches!(s, FetchStatus::Error(_)))
            .map(|(id, _)| {
                app.series_registry
                    .get(id)
                    .map(|s| s.short_name.as_str())
                    .unwrap_or(id.as_str())
            })
            .collect();
        failed_names.sort();
        let failed_str = failed_names.join(", ");
        (
            format!(
                "{}/{} loaded, {} failed ({}) ",
                loaded, total, errors, failed_str
            ),
            Color::Red,
        )
    } else {
        (format!("{}/{} series loaded ", loaded, total), Color::Green)
    };

    let status_widget = Paragraph::new(status_text)
        .style(Style::default().fg(status_color).bg(Color::Rgb(20, 20, 30)))
        .alignment(Alignment::Right);

    frame.render_widget(status_widget, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use crate::data::models::{CarStyle, EventStatus, RaceEvent, Series, Session, SessionType};
    use chrono::{NaiveDate, Utc};
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_status_bar_live_indicator() {
        let backend = TestBackend::new(120, 2);
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
                region: "Global".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, UserConfig::default());
        let now = Utc::now();
        let live_event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain GP".to_string(),
            circuit_name: "Bahrain".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            round: Some(1),
            sessions: vec![Session {
                name: "Grand Prix".to_string(),
                session_type: SessionType::Race,
                start_time: Some(now - chrono::Duration::minutes(30)),
                end_time: Some(now + chrono::Duration::minutes(90)),
            }],
            stream_links: vec![],
            status: EventStatus::Live,
        };

        app.update_series_data("f1".to_string(), vec![live_event]);
        app.live_blink_on = true;

        terminal
            .draw(|f| {
                let area = f.area();
                draw(f, &app, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("LIVE: F1: Grand Prix"));
    }
}
