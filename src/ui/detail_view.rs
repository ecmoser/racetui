use chrono::{Local, Utc};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;
use crate::data::models::StreamAccess;

/// Format a duration into a human-readable string like "1d 5h 23m", "5h 23m", "23m", "< 1m".
pub fn format_duration(dur: chrono::Duration) -> String {
    if dur.num_seconds() <= 0 {
        return "now".to_string();
    }
    let total_mins = dur.num_minutes();
    let days = total_mins / (24 * 60);
    let hours = (total_mins % (24 * 60)) / 60;
    let mins = total_mins % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m", mins)
    } else {
        "< 1m".to_string()
    }
}

/// Draw the event detail view popup.
pub fn draw(frame: &mut Frame, app: &App) {
    let event = match app.selected_event() {
        Some(e) => e,
        None => return,
    };

    let area = frame.area();
    let popup_width = (area.width * 60 / 100).max(50).min(area.width);
    let popup_height = (area.height * 70 / 100).max(18).min(area.height);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let series_name = app
        .series_registry
        .get(&event.series_id)
        .map(|s| s.name.as_str())
        .unwrap_or(&event.series_id);

    let mut lines: Vec<Line> = Vec::new();

    // 1. Title
    lines.push(Line::from(vec![
        Span::styled(
            &event.event_name,
            Style::default().bold().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
        ),
    ]));

    // 2. Subtitle
    let subtitle = match event.round {
        Some(round) => format!("{} · Round {}", series_name, round),
        None => series_name.to_string(),
    };
    lines.push(Line::from(vec![
        Span::styled(subtitle, Style::default().fg(Color::Yellow)),
    ]));

    // 3. Circuit Info
    let circuit_info = format!(
        "{} · {}, {}",
        event.circuit_name, event.location, event.country
    );
    lines.push(Line::from(vec![
        Span::styled(circuit_info, Style::default().fg(Color::White)),
    ]));

    // Separator
    lines.push(Line::from(""));

    // 4. Session schedule
    lines.push(Line::from(vec![
        Span::styled("── Sessions ──", Style::default().bold().fg(Color::Yellow)),
    ]));

    let now_utc = Utc::now();
    let next_session_time = event.next_session_time();

    if event.sessions.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  No session times available", Style::default().fg(Color::DarkGray)),
        ]));
    } else {
        for session in &event.sessions {
            let (date_str, time_str, is_past, is_live, is_next) = match session.start_time {
                Some(start) => {
                    let local_dt = start.with_timezone(&Local);
                    let d = local_dt.format("%a, %b %d").to_string();
                    let t = local_dt.format("%l:%M %p").to_string();

                    let is_past = session.end_time.map_or(
                        start + chrono::Duration::hours(2) < now_utc,
                        |e| e < now_utc,
                    );
                    let is_live = start <= now_utc && !is_past;
                    let is_next = Some(start) == next_session_time;
                    (d, t, is_past, is_live, is_next)
                }
                None => (
                    event.start_date.format("%a, %b %d").to_string(),
                    "TBD".to_string(),
                    false,
                    false,
                    false,
                ),
            };

            let status_badge = if is_live {
                Span::styled(" ● LIVE ", Style::default().bold().fg(Color::Red))
            } else if is_next {
                Span::styled(" ▶ NEXT ", Style::default().bold().fg(Color::Cyan))
            } else if is_past {
                Span::styled("   Done ", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled("        ", Style::default())
            };

            let row_style = if is_live {
                Style::default().bold().fg(Color::Red)
            } else if is_next {
                Style::default().bold().fg(Color::Cyan)
            } else if is_past {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(vec![
                status_badge,
                Span::styled(format!("{:<20}", session.name), row_style),
                Span::styled(format!(" {:<14}", date_str), row_style),
                Span::styled(format!(" {:<10}", time_str), row_style),
            ]));
        }
    }

    // Separator
    lines.push(Line::from(""));

    // 5. Stream Links
    lines.push(Line::from(vec![
        Span::styled("── Watch Links ──", Style::default().bold().fg(Color::Yellow)),
    ]));

    if event.stream_links.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  No streaming links available", Style::default().fg(Color::DarkGray)),
        ]));
    } else {
        for (i, link) in event.stream_links.iter().enumerate() {
            let access_badge = match link.access {
                StreamAccess::Free => Span::styled(" [Free] ", Style::default().bold().fg(Color::Green)),
                StreamAccess::Paid => Span::styled(" [Paid $] ", Style::default().bold().fg(Color::Yellow)),
                StreamAccess::Mixed => Span::styled(" [Mixed] ", Style::default().fg(Color::Cyan)),
                StreamAccess::Unknown => Span::styled(" [Link] ", Style::default().fg(Color::DarkGray)),
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  [{}] ", i + 1), Style::default().bold().fg(Color::Cyan)),
                Span::styled(format!("{:<15}", link.platform), Style::default().bold().fg(Color::White)),
                access_badge,
                Span::styled(&link.url, Style::default().fg(Color::Blue)),
            ]));
        }
    }

    // Separator
    lines.push(Line::from(""));

    // 6. Countdown
    if let Some(next_session) = event.next_session() {
        if let Some(start_time) = next_session.start_time {
            let diff = start_time.signed_duration_since(now_utc);
            let formatted_countdown = format_duration(diff);
            lines.push(Line::from(vec![
                Span::styled("⏱  Next: ", Style::default().bold().fg(Color::Yellow)),
                Span::styled(&next_session.name, Style::default().bold().fg(Color::White)),
                Span::styled(format!(" in {}", formatted_countdown), Style::default().bold().fg(Color::Green)),
            ]));
        }
    }

    let block = Block::default()
        .title(format!(" {} Details (Press Esc/Enter/q to close) ", event.event_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(chrono::Duration::days(2) + chrono::Duration::hours(3) + chrono::Duration::minutes(15)), "2d 3h 15m");
        assert_eq!(format_duration(chrono::Duration::hours(4) + chrono::Duration::minutes(20)), "4h 20m");
        assert_eq!(format_duration(chrono::Duration::minutes(35)), "35m");
        assert_eq!(format_duration(chrono::Duration::seconds(20)), "< 1m");
        assert_eq!(format_duration(chrono::Duration::seconds(-10)), "now");
    }
}
