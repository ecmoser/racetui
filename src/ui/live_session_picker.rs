use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::app::App;

/// Draw the live session picker popup when multiple sessions are concurrently live.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let sessions = app.get_live_sessions();
    if sessions.is_empty() {
        return;
    }

    let area = frame.area();
    let popup_width = (area.width * 60 / 100).max(45).min(area.width);
    let popup_height = ((sessions.len() as u16 + 3) * 2)
        .max(6)
        .min(area.height * 70 / 100);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let list_items: Vec<ListItem> = sessions
        .iter()
        .map(|(series_id, short_name, session_name)| {
            let series_color = app
                .series_registry
                .get(series_id)
                .map(|s| Color::Rgb(s.color.0, s.color.1, s.color.2))
                .unwrap_or(Color::Red);

            let is_fav = app.config.favorites.contains(series_id);
            let star = if is_fav { "★ " } else { "  " };

            let line = Line::from(vec![
                Span::styled(star, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:<10} ", short_name),
                    Style::default().bold().fg(series_color),
                ),
                Span::styled(
                    session_name.clone(),
                    Style::default().bold().fg(Color::White),
                ),
                Span::raw("  "),
                Span::styled("● LIVE", Style::default().bold().fg(Color::LightRed)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let block = Block::default()
        .title(" 🔴 Select Live Session (Enter: Watch, j/k: Select, Esc: Close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightRed));

    let list = List::new(list_items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 20, 20))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, popup_area, &mut app.live_session_picker_state);
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
    fn test_draw_live_session_picker() {
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

        registry.insert(
            "nascar_cup".to_string(),
            Series {
                id: "nascar_cup".to_string(),
                name: "NASCAR Cup Series".to_string(),
                short_name: "NASCAR".to_string(),
                car_style: CarStyle::StockCar,
                color: (0, 0, 255),
                region: "USA".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, UserConfig::default());
        let now = Utc::now();

        let event_f1 = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain GP".to_string(),
            circuit_name: "Bahrain".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            round: Some(1),
            sessions: vec![Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: Some(now - chrono::Duration::minutes(10)),
                end_time: Some(now + chrono::Duration::minutes(50)),
            }],
            stream_links: vec![],
            status: EventStatus::Live,
        };

        let event_nascar = RaceEvent {
            series_id: "nascar_cup".to_string(),
            event_name: "Daytona 500".to_string(),
            circuit_name: "Daytona".to_string(),
            location: "Daytona Beach".to_string(),
            country: "USA".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(),
            round: Some(1),
            sessions: vec![Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: Some(now - chrono::Duration::minutes(15)),
                end_time: Some(now + chrono::Duration::minutes(45)),
            }],
            stream_links: vec![],
            status: EventStatus::Live,
        };

        app.update_series_data("f1".to_string(), vec![event_f1]);
        app.update_series_data("nascar_cup".to_string(), vec![event_nascar]);
        app.show_live_session_picker = true;

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Select Live Session"));
        assert!(content.contains("F1"));
        assert!(content.contains("NASCAR"));
        assert!(content.contains("● LIVE"));
    }
}
