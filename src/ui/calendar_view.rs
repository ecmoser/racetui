use chrono::{Datelike, Local, NaiveDate};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

/// Days in a given year and month
pub fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month_start = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    if let Some(next) = next_month_start {
        next.pred_opt().map(|d| d.day()).unwrap_or(30)
    } else {
        30
    }
}

/// Month name string
pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

/// Draw the calendar grid view.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("◀ {} {} ▶", month_name(app.calendar_month), app.calendar_year);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if inner_area.height < 8 || inner_area.width < 28 {
        // Area too small to render calendar grid
        let fallback = Paragraph::new("Terminal too small for calendar view")
            .alignment(Alignment::Center);
        frame.render_widget(fallback, inner_area);
        return;
    }

    let first_day_opt = NaiveDate::from_ymd_opt(app.calendar_year, app.calendar_month, 1);
    let first_day = match first_day_opt {
        Some(d) => d,
        None => return,
    };

    let start_col = first_day.weekday().num_days_from_monday() as usize; // Mon=0 .. Sun=6
    let total_days = days_in_month(app.calendar_year, app.calendar_month);
    let num_weeks = ((start_col as u32 + total_days + 6) / 7) as usize;

    // Vertical layout: 1 header row + week rows
    let mut row_constraints = vec![Constraint::Length(1)]; // Weekday headers
    for _ in 0..num_weeks {
        row_constraints.push(Constraint::Ratio(1, num_weeks as u32));
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner_area);

    // Render weekday headers
    let col_constraints = vec![Constraint::Ratio(1, 7); 7];
    let header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_constraints.clone())
        .split(rows[0]);

    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    for (i, &name) in day_names.iter().enumerate() {
        let p = Paragraph::new(name)
            .alignment(Alignment::Center)
            .style(Style::default().bold().fg(Color::Yellow));
        frame.render_widget(p, header_cols[i]);
    }

    let today = Local::now().date_naive();
    let events = app.filtered_events();

    // Render each week
    let mut day_counter: u32 = 1;
    for week_idx in 0..num_weeks {
        let week_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints.clone())
            .split(rows[week_idx + 1]);

        for col_idx in 0..7 {
            let cell_rect = week_cols[col_idx];
            if (week_idx == 0 && col_idx < start_col) || day_counter > total_days {
                // Empty cell outside current month
                let empty_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(30, 30, 40)));
                frame.render_widget(empty_block, cell_rect);
                continue;
            }

            let day = day_counter;
            day_counter += 1;

            let cell_date = match NaiveDate::from_ymd_opt(app.calendar_year, app.calendar_month, day) {
                Some(d) => d,
                None => continue,
            };

            let is_today = cell_date == today;

            // Collect events for this day
            let day_events: Vec<_> = events
                .iter()
                .filter(|e| e.start_date <= cell_date && cell_date <= e.end_date)
                .collect();

            // Build day cell lines
            let mut lines: Vec<Line> = Vec::new();

            // Day number header
            let day_style = if is_today {
                Style::default().bold().fg(Color::Black).bg(Color::Green)
            } else if !day_events.is_empty() {
                Style::default().bold().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {:2} ", day), day_style),
            ]));

            // Add up to 3 event labels
            let max_display_events = if cell_rect.height > 4 { 3 } else { 1 };
            for event in day_events.iter().take(max_display_events) {
                let series_color = app
                    .series_registry
                    .get(&event.series_id)
                    .map(|s| Color::Rgb(s.color.0, s.color.1, s.color.2))
                    .unwrap_or(Color::White);

                let short_name = app
                    .series_registry
                    .get(&event.series_id)
                    .map(|s| s.short_name.as_str())
                    .unwrap_or(&event.series_id);

                let is_fav = app.config.favorites.contains(&event.series_id);
                let star = if is_fav { "★" } else { " " };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}{}", star, short_name),
                        Style::default().fg(series_color).bold(),
                    ),
                ]));
            }

            if day_events.len() > max_display_events {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("+{} more", day_events.len() - max_display_events),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }

            let cell_block = Block::default()
                .borders(Borders::ALL)
                .border_style(if is_today {
                    Style::default().fg(Color::Green)
                } else if !day_events.is_empty() {
                    Style::default().fg(Color::Rgb(80, 80, 100))
                } else {
                    Style::default().fg(Color::Rgb(40, 40, 50))
                });

            let day_paragraph = Paragraph::new(lines).block(cell_block);
            frame.render_widget(day_paragraph, cell_rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29); // leap year
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn test_month_names() {
        assert_eq!(month_name(1), "January");
        assert_eq!(month_name(8), "August");
        assert_eq!(month_name(12), "December");
    }
}
