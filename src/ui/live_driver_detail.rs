use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::live::event::LiveDriverEntry;

/// Draw the expanded driver detail panel in the Live view.
pub fn draw(frame: &mut Frame, _app: &mut App, area: Rect, driver: &LiveDriverEntry) {
    let num_str = driver
        .driver_number
        .map(|n| format!("#{}", n))
        .unwrap_or_default();

    let title = format!(
        " 🏎️ Driver Detail: {} {} — {} (Enter/Esc to collapse) ",
        num_str, driver.driver_name, driver.team_name
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area horizontally into 3 columns:
    // Col 1: Lap Times & Pace
    // Col 2: Sector Times & Speeds
    // Col 3: Tires, Pits & Strategy
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34), // Pace
            Constraint::Percentage(33), // Sectors
            Constraint::Percentage(33), // Tires & Pits
        ])
        .split(inner_area);

    // Column 1: Pace & Position
    let mut col1_lines = vec![
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                format!("P{}", driver.position),
                Style::default().bold().fg(Color::White),
            ),
            Span::raw("  ("),
            Span::styled(
                &driver.status,
                if driver.status == "On Track" {
                    Style::default().fg(Color::Green).bold()
                } else {
                    Style::default().fg(Color::Yellow).bold()
                },
            ),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::styled("Gap to Leader: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&driver.gap_to_leader, Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Interval Ahead: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&driver.interval, Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Last Lap: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                driver.last_lap_time.as_deref().unwrap_or("-"),
                Style::default().bold().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Best Lap: ", Style::default().fg(Color::DarkGray)),
            if driver.fastest_lap {
                Span::styled(
                    format!("{} [FL]", driver.best_lap_time.as_deref().unwrap_or("-")),
                    Style::default().bold().fg(Color::Magenta),
                )
            } else {
                Span::styled(
                    driver.best_lap_time.as_deref().unwrap_or("-"),
                    Style::default().bold().fg(Color::Green),
                )
            },
        ]),
    ];

    if col1_lines.len() > cols[0].height as usize {
        col1_lines.truncate(cols[0].height as usize);
    }
    frame.render_widget(Paragraph::new(col1_lines), cols[0]);

    // Column 2: Sectors
    let s1_style = if driver.sectors.s1_fastest {
        Style::default().fg(Color::Magenta).bold()
    } else {
        Style::default().fg(Color::Green)
    };
    let s2_style = if driver.sectors.s2_fastest {
        Style::default().fg(Color::Magenta).bold()
    } else {
        Style::default().fg(Color::Green)
    };
    let s3_style = if driver.sectors.s3_fastest {
        Style::default().fg(Color::Magenta).bold()
    } else {
        Style::default().fg(Color::Green)
    };

    let mut col2_lines = vec![
        Line::from(Span::styled("⏱️ Sector Times", Style::default().fg(Color::Yellow).bold())),
        Line::from(vec![
            Span::styled("Sector 1: ", Style::default().fg(Color::DarkGray)),
            Span::styled(driver.sectors.s1_str.as_deref().unwrap_or("-"), s1_style),
        ]),
        Line::from(vec![
            Span::styled("Sector 2: ", Style::default().fg(Color::DarkGray)),
            Span::styled(driver.sectors.s2_str.as_deref().unwrap_or("-"), s2_style),
        ]),
        Line::from(vec![
            Span::styled("Sector 3: ", Style::default().fg(Color::DarkGray)),
            Span::styled(driver.sectors.s3_str.as_deref().unwrap_or("-"), s3_style),
        ]),
        Line::from(vec![
            Span::styled("Laps Completed: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", driver.laps_completed), Style::default().bold().fg(Color::White)),
        ]),
    ];

    if col2_lines.len() > cols[1].height as usize {
        col2_lines.truncate(cols[1].height as usize);
    }
    frame.render_widget(Paragraph::new(col2_lines), cols[1]);

    // Column 3: Tires & Pit Strategy
    let tire_info = if let Some(ref t) = driver.tire {
        let compound_color = match t.compound.to_lowercase().as_str() {
            "soft" => Color::LightRed,
            "medium" => Color::Yellow,
            "hard" => Color::White,
            "intermediate" | "inter" => Color::Green,
            "wet" => Color::LightBlue,
            _ => Color::Gray,
        };
        let age = format!("{} laps", t.laps);
        let new_used = if t.is_new { "New" } else { "Used" };
        (t.compound.clone(), compound_color, age, new_used)
    } else {
        ("Unknown".to_string(), Color::DarkGray, "-".to_string(), "-")
    };

    let pit_duration_str = driver
        .pits
        .last_stop_duration_secs
        .map(|d| format!("{:.1}s", d))
        .unwrap_or_else(|| "-".to_string());

    let last_stop_lap_str = driver
        .pits
        .last_stop_lap
        .map(|l| format!("Lap {}", l))
        .unwrap_or_else(|| "-".to_string());

    let mut col3_lines = vec![
        Line::from(Span::styled("🛞 Tires & Pit Strategy", Style::default().fg(Color::Yellow).bold())),
        Line::from(vec![
            Span::styled("Compound: ", Style::default().fg(Color::DarkGray)),
            Span::styled(tire_info.0, Style::default().fg(tire_info.1).bold()),
            Span::raw(format!(" ({}, {})", tire_info.3, tire_info.2)),
        ]),
        Line::from(vec![
            Span::styled("Pit Stops: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", driver.pits.stops_count), Style::default().bold().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Last Stop: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ({})", last_stop_lap_str, pit_duration_str), Style::default().bold().fg(Color::White)),
        ]),
    ];

    if col3_lines.len() > cols[2].height as usize {
        col3_lines.truncate(cols[2].height as usize);
    }
    frame.render_widget(Paragraph::new(col3_lines), cols[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use crate::live::event::{PitInfo, SectorTimes, TireInfo};
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_live_driver_detail() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());

        let driver = LiveDriverEntry {
            position: 1,
            driver_number: Some(1),
            driver_name: "Max Verstappen".to_string(),
            driver_code: Some("VER".to_string()),
            team_name: "Red Bull Racing".to_string(),
            team_color: Some("#3671C6".to_string()),
            gap_to_leader: "LEADER".to_string(),
            interval: "-".to_string(),
            last_lap_time: Some("1:20.123".to_string()),
            best_lap_time: Some("1:19.876".to_string()),
            sectors: SectorTimes {
                s1_ms: Some(28120),
                s2_ms: Some(30450),
                s3_ms: Some(21553),
                s1_str: Some("28.120".to_string()),
                s2_str: Some("30.450".to_string()),
                s3_str: Some("21.553".to_string()),
                s1_fastest: true,
                s2_fastest: false,
                s3_fastest: true,
            },
            tire: Some(TireInfo {
                compound: "Soft".to_string(),
                laps: 12,
                is_new: true,
            }),
            pits: PitInfo {
                stops_count: 1,
                last_stop_lap: Some(18),
                last_stop_duration_secs: Some(2.4),
                in_pit: false,
            },
            laps_completed: 25,
            status: "On Track".to_string(),
            current_position: None,
            fastest_lap: true,
        };

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area(), &driver);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Max Verstappen"));
        assert!(content.contains("Red Bull Racing"));
        assert!(content.contains("Sector Times"));
        assert!(content.contains("28.120"));
        assert!(content.contains("Tires & Pit Strategy"));
        assert!(content.contains("Soft"));
        assert!(content.contains("1:19.876 [FL]"));
    }
}
