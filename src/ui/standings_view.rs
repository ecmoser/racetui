use chrono::Datelike;
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table,
};

use crate::app::App;

/// Draw the championship standings view.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Championship Standings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let series_ids = app.standings_series_list();

    if series_ids.is_empty() {
        let msg = if let Some(ref q) = app.search_query {
            format!("No series found matching \"{}\".", q)
        } else {
            "No racing series registered.".to_string()
        };
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(msg, Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            Line::from(Span::styled(
                "Press Backspace/Esc to clear search or 'r' to refresh.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let empty_p = Paragraph::new(empty_text)
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(empty_p, area);
        return;
    }

    // Auto-select first favorited series (or first in list) if none selected or selection not in filtered list
    if app.standings_selected_series.is_none()
        || !app
            .standings_selected_series
            .as_ref()
            .map_or(false, |id| series_ids.contains(id))
    {
        let default_series = series_ids
            .iter()
            .find(|id| app.config.favorites.contains(*id))
            .or_else(|| series_ids.first())
            .cloned();
        if let Some(ref id) = default_series {
            app.standings_selected_series = Some(id.clone());
            if let Some(idx) = series_ids.iter().position(|s| s == id) {
                app.standings_series_index = idx;
            }
        }
    }

    let selected_id = match app.standings_selected_series.as_ref() {
        Some(id) => id,
        None => return,
    };

    let series_name = app
        .series_registry
        .get(selected_id)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| selected_id.clone());

    let series_color = app
        .series_registry
        .get(selected_id)
        .map(|s| Color::Rgb(s.color.0, s.color.1, s.color.2))
        .unwrap_or(Color::Yellow);

    let standings = app.standings.get(selected_id);
    let has_drivers = standings.map_or(false, |s| !s.drivers.is_empty());
    let has_constructors = standings.map_or(false, |s| s.has_constructor_standings());

    // Inner area for rendering content
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Layout: Selector bar + Driver table + (Optional Constructor table)
    let chunks = if has_drivers && has_constructors {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),      // Series selector / search bar
                Constraint::Percentage(60), // Driver standings table
                Constraint::Percentage(40), // Constructor standings table
            ])
            .split(inner_area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Series selector / search bar
                Constraint::Min(1),    // Driver standings table or empty state
            ])
            .split(inner_area)
    };

    // 1. Series Selector / Search Bar
    let selector_line = if app.search_active || app.search_query.is_some() {
        let query = app.search_query.as_deref().unwrap_or("");
        let cursor = if app.search_active { "_" } else { "" };
        Line::from(vec![
            Span::styled(" Search: ", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                format!("{}{}", query, cursor),
                Style::default().fg(Color::White).bold(),
            ),
            Span::styled(
                format!(
                    "  ({} match{})  Series: ◀ {} ▶",
                    series_ids.len(),
                    if series_ids.len() == 1 { "" } else { "es" },
                    series_name
                ),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                " (Esc: clear, Enter: done)",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" Series: ", Style::default().fg(Color::DarkGray).bold()),
            Span::styled("◀ ", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                format!(" {} ", series_name),
                Style::default().fg(series_color).bold(),
            ),
            Span::styled(" ▶", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                "  (/ to search, ←/→ or h/l to switch series)",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    };
    let selector_widget = Paragraph::new(selector_line);
    frame.render_widget(selector_widget, chunks[0]);

    // If no standings data for this series, render a friendly empty message
    if !has_drivers {
        let current_year = chrono::Utc::now().year();
        let empty_lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                format!(
                    "No championship standings data available yet for {} ({}).",
                    series_name, current_year
                ),
                Style::default().fg(Color::White).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Standings will populate when published. Press 'r' to refresh, or '/' to search series.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let empty_p = Paragraph::new(empty_lines).alignment(Alignment::Center);
        frame.render_widget(empty_p, chunks[1]);
        return;
    }

    let standings = standings.unwrap();

    // 2. Driver Standings Table
    let driver_header = Row::new(vec![
        Cell::from("Pos").style(Style::default().bold().fg(Color::White)),
        Cell::from("#").style(Style::default().bold().fg(Color::White)),
        Cell::from("Driver").style(Style::default().bold().fg(Color::White)),
        Cell::from("Team").style(Style::default().bold().fg(Color::White)),
        Cell::from("Wins").style(Style::default().bold().fg(Color::White)),
        Cell::from("Pts").style(Style::default().bold().fg(Color::White)),
    ])
    .height(1)
    .bottom_margin(1);

    let driver_rows: Vec<Row> = standings
        .drivers
        .iter()
        .map(|d| {
            let pos_str = format!("{}", d.position);
            let num_str = d
                .driver_number
                .map(|n| format!("{}", n))
                .unwrap_or_else(|| "-".to_string());
            let name_str = match &d.driver_code {
                Some(code) => format!("{} ({})", d.driver_name, code),
                None => d.driver_name.clone(),
            };
            let wins_str = format!("{}", d.wins);
            let pts_str = if d.points.fract() == 0.0 {
                format!("{:.0}", d.points)
            } else {
                format!("{:.1}", d.points)
            };

            Row::new(vec![
                Cell::from(pos_str).style(Style::default().fg(Color::Yellow)),
                Cell::from(num_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(name_str).style(Style::default().fg(Color::White).bold()),
                Cell::from(d.team.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(wins_str).style(Style::default().fg(Color::White)),
                Cell::from(pts_str).style(Style::default().fg(Color::Green).bold()),
            ])
        })
        .collect();

    let driver_block = Block::default()
        .title(" Drivers ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let driver_widths = [
        Constraint::Length(5), // Pos
        Constraint::Length(4), // #
        Constraint::Max(30),   // Driver
        Constraint::Max(25),   // Team
        Constraint::Length(6), // Wins
        Constraint::Length(8), // Pts
    ];

    let driver_items_count = driver_rows.len();
    let driver_table = Table::new(driver_rows, driver_widths)
        .header(driver_header)
        .block(driver_block)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always);

    frame.render_stateful_widget(driver_table, chunks[1], &mut app.standings_table_state);

    if driver_items_count > chunks[1].height as usize {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(driver_items_count)
            .position(app.standings_table_state.selected().unwrap_or(0));
        frame.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
    }

    // 3. Constructor Standings Table (if applicable)
    if has_constructors {
        let constr_header = Row::new(vec![
            Cell::from("Pos").style(Style::default().bold().fg(Color::White)),
            Cell::from("Constructor").style(Style::default().bold().fg(Color::White)),
            Cell::from("Wins").style(Style::default().bold().fg(Color::White)),
            Cell::from("Pts").style(Style::default().bold().fg(Color::White)),
        ])
        .height(1)
        .bottom_margin(1);

        let constr_rows: Vec<Row> = standings
            .constructors
            .iter()
            .map(|c| {
                let pos_str = format!("{}", c.position);
                let wins_str = format!("{}", c.wins);
                let pts_str = if c.points.fract() == 0.0 {
                    format!("{:.0}", c.points)
                } else {
                    format!("{:.1}", c.points)
                };

                Row::new(vec![
                    Cell::from(pos_str).style(Style::default().fg(Color::Yellow)),
                    Cell::from(c.name.clone()).style(Style::default().fg(Color::White).bold()),
                    Cell::from(wins_str).style(Style::default().fg(Color::White)),
                    Cell::from(pts_str).style(Style::default().fg(Color::Green).bold()),
                ])
            })
            .collect();

        let constr_block = Block::default()
            .title(" Constructors ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let constr_widths = [
            Constraint::Length(5), // Pos
            Constraint::Max(40),   // Constructor
            Constraint::Length(6), // Wins
            Constraint::Length(8), // Pts
        ];

        let constr_table = Table::new(constr_rows, constr_widths)
            .header(constr_header)
            .block(constr_block);

        frame.render_widget(constr_table, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use crate::data::models::{CarStyle, Series};
    use crate::data::standings::{ConstructorStanding, DriverStanding, SeasonStandings};
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_standings_view_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("No racing series registered"));
    }

    #[test]
    fn test_draw_standings_view_unpopulated_series() {
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
                color: (235, 0, 0),
                region: "Global".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, UserConfig::default());

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Formula 1"));
        assert!(content.contains("No championship standings data available yet for Formula 1"));
    }

    #[test]
    fn test_draw_standings_view_with_data() {
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
                color: (235, 0, 0),
                region: "Global".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, UserConfig::default());
        app.standings.insert(
            "f1".to_string(),
            SeasonStandings {
                series_id: "f1".to_string(),
                season: 2026,
                drivers: vec![DriverStanding {
                    position: 1,
                    driver_name: "Max Verstappen".to_string(),
                    driver_code: Some("VER".to_string()),
                    driver_number: Some(1),
                    team: "Red Bull Racing".to_string(),
                    points: 575.0,
                    wins: 19,
                }],
                constructors: vec![ConstructorStanding {
                    position: 1,
                    name: "Red Bull Racing".to_string(),
                    points: 860.0,
                    wins: 21,
                }],
                fetched_at: chrono::Utc::now(),
            },
        );

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Formula 1"));
        assert!(content.contains("Max Verstappen"));
        assert!(content.contains("Red Bull Racing"));
    }

    #[test]
    fn test_draw_standings_view_search() {
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
                color: (235, 0, 0),
                region: "Global".to_string(),
                calendar_url: "https://example.com".to_string(),
                requires_js: false,
            },
        );

        let mut app = App::new(registry, UserConfig::default());
        app.search_active = true;
        app.search_query = Some("f1".to_string());

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Search:"));
        assert!(content.contains("f1"));
    }
}
