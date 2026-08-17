use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};

use crate::app::App;
use crate::data::models::EventStatus;

/// Draw the list view — a table of race events.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let events = app.filtered_events();

    // Build header row
    let header = Row::new(vec![
        Cell::from("Date").style(Style::default().bold().fg(Color::White)),
        Cell::from("Time").style(Style::default().bold().fg(Color::White)),
        Cell::from("Series").style(Style::default().bold().fg(Color::White)),
        Cell::from("Event").style(Style::default().bold().fg(Color::White)),
        Cell::from("Circuit").style(Style::default().bold().fg(Color::White)),
        Cell::from("Country").style(Style::default().bold().fg(Color::White)),
        Cell::from("Status").style(Style::default().bold().fg(Color::White)),
    ])
    .height(1)
    .bottom_margin(1);

    // Build data rows
    let rows: Vec<Row> = events
        .iter()
        .map(|event| {
            // Get series color
            let series_color = app
                .series_registry
                .get(&event.series_id)
                .map(|s| Color::Rgb(s.color.0, s.color.1, s.color.2))
                .unwrap_or(Color::White);

            // Get series short name
            let series_name = app
                .series_registry
                .get(&event.series_id)
                .map(|s| s.short_name.as_str())
                .unwrap_or(&event.series_id);

            // Format date in local timezone
            let date_str = event.start_date.format("%b %d").to_string();

            // Format time in local timezone
            let time_str = event.format_local_time();

            // Determine if this is a favorited series
            let is_favorite = app.config.favorites.contains(&event.series_id);
            let favorite_marker = if is_favorite { "★ " } else { "  " };

            // Status styling
            let (status_text, status_color) = match &event.status {
                EventStatus::Live => ("● LIVE", Color::Red),
                EventStatus::Upcoming => {
                    // Check if event is within notification threshold
                    if let Some(next_time) = event.next_session_time() {
                        let hours_until = next_time
                            .signed_duration_since(chrono::Utc::now())
                            .num_hours();
                        if hours_until <= app.config.notification_threshold_hours as i64
                            && hours_until >= 0
                        {
                            ("⚡ SOON", Color::Yellow)
                        } else {
                            ("Upcoming", Color::DarkGray)
                        }
                    } else {
                        ("Upcoming", Color::DarkGray)
                    }
                }
                EventStatus::Completed => ("Done", Color::DarkGray),
                EventStatus::Cancelled => ("Cancelled", Color::DarkGray),
            };

            Row::new(vec![
                Cell::from(date_str),
                Cell::from(time_str).style(Style::default().fg(Color::Gray)),
                Cell::from(format!("{}{}", favorite_marker, series_name))
                    .style(Style::default().fg(series_color)),
                Cell::from(event.event_name.as_str()),
                Cell::from(event.circuit_name.as_str()),
                Cell::from(event.country.as_str()),
                Cell::from(status_text).style(Style::default().fg(status_color)),
            ])
        })
        .collect();

    // Column widths
    let widths = [
        Constraint::Length(7),   // Date (e.g. "Aug 16")
        Constraint::Length(9),   // Time (e.g. " 3:00 PM")
        Constraint::Length(12),  // Series (e.g. "★ IndyCar")
        Constraint::Min(20),     // Event name (flexible)
        Constraint::Length(25),  // Circuit
        Constraint::Length(15),  // Country
        Constraint::Length(10),  // Status
    ];

    // Build the title with filter/search info
    let mut title = String::from(" Race Calendar ");
    if let Some(ref query) = app.search_query {
        title = format!(" Search: {} ", query);
    }

    // Build table
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always);

    // Render table with state (for selection tracking)
    let mut table_state = app.table_state.clone();
    frame.render_stateful_widget(table, area, &mut table_state);

    // Draw scrollbar if there are enough items
    if events.len() > area.height as usize {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(events.len())
            .position(app.table_state.selected().unwrap_or(0));
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
