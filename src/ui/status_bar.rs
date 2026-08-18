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

    let notifications = app.get_notifications();
    let (status_text, status_color) = if let Some(ref msg) = app.status_message {
        (msg.clone(), Color::White)
    } else if !notifications.is_empty() {
        let idx = app.notification_cycle_index % notifications.len();
        (format!("{} ", notifications[idx]), Color::Yellow)
    } else if fetching > 0 {
        (format!("Fetching... {}/{} loaded ", loaded, total), Color::Yellow)
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
        (format!("{}/{} loaded, {} failed ({}) ", loaded, total, errors, failed_str), Color::Red)
    } else {
        (format!("{}/{} series loaded ", loaded, total), Color::Green)
    };

    let status_widget = Paragraph::new(status_text)
        .style(Style::default().fg(status_color).bg(Color::Rgb(20, 20, 30)))
        .alignment(Alignment::Right);

    frame.render_widget(status_widget, chunks[1]);
}
