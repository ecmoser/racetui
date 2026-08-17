use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::app::App;
use crate::ui::calendar_view::month_name;

/// Draw the day events selection popup (when multiple races occur on a selected calendar day).
pub fn draw(frame: &mut Frame, app: &App) {
    let events = app.events_on_selected_calendar_day();
    if events.is_empty() {
        return;
    }

    let area = frame.area();
    let popup_width = (area.width * 60 / 100).max(45).min(area.width);
    let popup_height = ((events.len() as u16 + 4) * 2).max(8).min(area.height * 70 / 100);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let list_items: Vec<ListItem> = events
        .iter()
        .map(|event| {
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

            let time_str = event.format_local_time();
            let is_fav = app.config.favorites.contains(&event.series_id);
            let star = if is_fav { "★" } else { " " };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<9} ", time_str), Style::default().fg(Color::Gray)),
                Span::styled(format!("{}{:<8} ", star, short_name), Style::default().bold().fg(series_color)),
                Span::styled(&event.event_name, Style::default().bold().fg(Color::White)),
                Span::styled(format!(" · {}", event.circuit_name), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let title = format!(
        " Races on {} {}, {} (Enter: View Details, j/k: Select, Esc: Close) ",
        month_name(app.calendar_month),
        app.calendar_selected_day,
        app.calendar_year
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(list_items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        );

    let mut state = app.day_events_state.clone();
    frame.render_stateful_widget(list, popup_area, &mut state);
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
    fn test_draw_day_events_popup() {
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
        app.calendar_year = 2026;
        app.calendar_month = 5;
        app.calendar_selected_day = 24;

        let event1 = RaceEvent {
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

        app.update_series_data("f1".to_string(), vec![event1]);
        app.show_day_events = true;

        terminal.draw(|f| {
            draw(f, &app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Monaco Grand Prix"));
    }
}
