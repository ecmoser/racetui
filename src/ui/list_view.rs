use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table,
};

use crate::app::{App, ListTableItem};
use crate::data::models::EventStatus;

/// Draw the list view — a table of race events grouped by day.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let items = app.list_table_items();

    // Build the title with filter/search info
    let mut title = String::from(" Race Calendar ");
    if let Some(ref query) = app.search_query {
        title = format!(" Search: {} ", query);
    }

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if items.is_empty() {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No events found.",
                Style::default().fg(Color::DarkGray).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Try adjusting your filters or press 'r' to refresh data.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let empty_p = Paragraph::new(empty_text)
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(empty_p, area);
        return;
    }

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
    let rows: Vec<Row> = items
        .iter()
        .map(|item| match item {
            ListTableItem::Header(_) => {
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
            }
            ListTableItem::Session(session) => {
                let event = session.event;
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

                // Format date in local timezone (including day of week)
                let date_str = session.date.format("%a, %b %d").to_string();

                // Format time in local timezone with live indicator if currently live
                let raw_time = session.format_local_time();
                let (time_cell, is_live) = if session.status == EventStatus::Live {
                    (
                        Cell::from(format!("● {:>8}", raw_time))
                            .style(Style::default().bold().fg(Color::Red)),
                        true,
                    )
                } else {
                    (
                        Cell::from(format!("  {:>8}", raw_time))
                            .style(Style::default().fg(Color::Gray)),
                        false,
                    )
                };

                // Determine if this is a favorited series
                let is_favorite = app.config.favorites.contains(&event.series_id);
                let favorite_marker = if is_favorite { "★ " } else { "  " };

                // Status styling
                let (status_text, status_color) = match &session.status {
                    EventStatus::Live => ("● LIVE", Color::Red),
                    EventStatus::Upcoming => {
                        // Check if session is within notification threshold
                        if let Some(start_time) = session.start_time {
                            let hours_until = start_time
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

                let is_soon = is_favorite
                    && session.start_time.map_or(false, |t| {
                        let until = t.signed_duration_since(chrono::Utc::now());
                        until > chrono::Duration::zero()
                            && until
                                <= chrono::Duration::hours(
                                    app.config.notification_threshold_hours as i64,
                                )
                    });

                let row_style = if is_soon {
                    Style::default().bg(Color::Rgb(50, 40, 20))
                } else {
                    Style::default()
                };

                let _ = is_live;
                Row::new(vec![
                    Cell::from(date_str),
                    time_cell,
                    Cell::from(format!("{}{}", favorite_marker, series_name))
                        .style(Style::default().fg(series_color)),
                    Cell::from(session.display_title()),
                    Cell::from(event.circuit_name.as_str()),
                    Cell::from(event.country.as_str()),
                    Cell::from(status_text).style(Style::default().fg(status_color)),
                ])
                .style(row_style)
            }
        })
        .collect();

    // Column widths
    let widths = [
        Constraint::Length(12),  // Date (e.g. "Sat, Aug 16")
        Constraint::Length(11),  // Time (e.g. "● 12:00 PM")
        Constraint::Length(12),  // Series (e.g. "★ IndyCar")
        Constraint::Min(20),     // Event name (flexible)
        Constraint::Max(25),     // Circuit
        Constraint::Max(15),     // Country
        Constraint::Length(10),  // Status
    ];

    // Build table
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
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
    if items.len() > area.height as usize {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(items.len())
            .position(app.table_state.selected().unwrap_or(0));
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
