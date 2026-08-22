use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table,
};

use crate::app::App;
use crate::live::event::{LiveTimingData, TireInfo};

/// Draw the live timing leaderboard table.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect, data: &LiveTimingData) {
    let block = Block::default()
        .title(" Leaderboard ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if data.drivers.is_empty() {
        let empty_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Waiting for driver telemetry...",
                Style::default().fg(Color::Yellow),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty_p, area);
        return;
    }

    let has_numbers = data.drivers.iter().any(|d| d.driver_number.is_some());
    let has_sectors = data.drivers.iter().any(|d| {
        d.sectors.s1_str.is_some()
            || d.sectors.s2_str.is_some()
            || d.sectors.s3_str.is_some()
            || d.sectors.s1_ms.is_some()
            || d.sectors.s2_ms.is_some()
            || d.sectors.s3_ms.is_some()
    });
    let has_tires = data.drivers.iter().any(|d| d.tire.is_some());
    let has_pits = data
        .drivers
        .iter()
        .any(|d| d.pits.stops_count > 0 || d.pits.in_pit || d.status == "In Pit");

    let mut header_cells = vec![Cell::from("Pos").style(Style::default().fg(Color::Yellow).bold())];
    let mut widths = vec![
        Constraint::Length(4), // Pos
    ];

    if has_numbers {
        header_cells.push(Cell::from("#").style(Style::default().fg(Color::Yellow).bold()));
        widths.push(Constraint::Length(4));
    }

    header_cells.push(Cell::from("Driver").style(Style::default().fg(Color::Yellow).bold()));
    widths.push(Constraint::Min(16));

    header_cells.push(Cell::from("Team").style(Style::default().fg(Color::Yellow).bold()));
    widths.push(Constraint::Min(14));

    header_cells.push(Cell::from("Gap").style(Style::default().fg(Color::Yellow).bold()));
    widths.push(Constraint::Length(10));

    header_cells.push(Cell::from("Int").style(Style::default().fg(Color::Yellow).bold()));
    widths.push(Constraint::Length(9));

    header_cells.push(Cell::from("Last Lap").style(Style::default().fg(Color::Yellow).bold()));
    widths.push(Constraint::Length(15)); // Accommodate times with " [FL]"

    if has_sectors {
        header_cells.push(Cell::from("S1").style(Style::default().fg(Color::Yellow).bold()));
        widths.push(Constraint::Length(8));
        header_cells.push(Cell::from("S2").style(Style::default().fg(Color::Yellow).bold()));
        widths.push(Constraint::Length(8));
        header_cells.push(Cell::from("S3").style(Style::default().fg(Color::Yellow).bold()));
        widths.push(Constraint::Length(8));
    }

    if has_tires {
        header_cells.push(Cell::from("Tire").style(Style::default().fg(Color::Yellow).bold()));
        widths.push(Constraint::Length(14));
    }

    if has_pits {
        header_cells.push(Cell::from("Pits").style(Style::default().fg(Color::Yellow).bold()));
        widths.push(Constraint::Length(9));
    }

    let header = Row::new(header_cells)
        .height(1)
        .bottom_margin(0)
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));

    let rows: Vec<Row> = data
        .drivers
        .iter()
        .enumerate()
        .map(|(idx, d)| {
            let row_bg = if idx % 2 == 0 {
                Color::Reset
            } else {
                Color::Rgb(20, 20, 20)
            };

            let pos_str = format!("{:>2}", d.position);
            let num_str = d
                .driver_number
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".to_string());

            let driver_span = if let Some(ref code) = d.driver_code {
                format!("{} ({})", d.driver_name, code)
            } else {
                d.driver_name.clone()
            };

            let team_color = parse_hex_color(d.team_color.as_deref()).unwrap_or(Color::White);

            let last_lap_span = if let Some(ref ll) = d.last_lap_time {
                if d.fastest_lap {
                    Span::styled(
                        format!("{} [FL]", ll),
                        Style::default().fg(Color::Magenta).bold(),
                    )
                } else {
                    Span::raw(ll.clone())
                }
            } else {
                Span::styled("-", Style::default().fg(Color::DarkGray))
            };

            let mut cells = vec![Cell::from(pos_str).style(Style::default().bold())];

            if has_numbers {
                cells.push(Cell::from(num_str).style(Style::default().fg(Color::Cyan)));
            }

            cells.push(Cell::from(driver_span).style(Style::default().bold()));
            cells.push(Cell::from(d.team_name.clone()).style(Style::default().fg(team_color)));
            cells.push(Cell::from(d.gap_to_leader.clone()));
            cells.push(Cell::from(d.interval.clone()));
            cells.push(Cell::from(last_lap_span));

            if has_sectors {
                cells.push(format_sector_cell(&d.sectors.s1_str, d.sectors.s1_fastest));
                cells.push(format_sector_cell(&d.sectors.s2_str, d.sectors.s2_fastest));
                cells.push(format_sector_cell(&d.sectors.s3_str, d.sectors.s3_fastest));
            }

            if has_tires {
                cells.push(format_tire_cell(d.tire.as_ref()));
            }

            if has_pits {
                let pit_str = if d.status == "In Pit" {
                    Span::styled("IN PIT", Style::default().fg(Color::Yellow).bold())
                } else if d.pits.stops_count > 0 {
                    if let Some(lap) = d.pits.last_stop_lap {
                        Span::raw(format!("{} (L{})", d.pits.stops_count, lap))
                    } else {
                        Span::raw(format!("{}", d.pits.stops_count))
                    }
                } else {
                    Span::styled("0", Style::default().fg(Color::DarkGray))
                };
                cells.push(Cell::from(pit_str));
            }

            Row::new(cells).style(Style::default().bg(row_bg))
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 50, 80))
                .fg(Color::White)
                .bold(),
        )
        .highlight_spacing(HighlightSpacing::Always);

    frame.render_stateful_widget(table, area, &mut app.live_timing_table_state);

    // Render scrollbar if there are more drivers than visible height
    let total_drivers = data.drivers.len();
    if total_drivers > area.height.saturating_sub(3) as usize {
        let mut scrollbar_state = ScrollbarState::new(total_drivers)
            .position(app.live_timing_table_state.selected().unwrap_or(0));
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn format_sector_cell(sec_str: &Option<String>, is_fastest: bool) -> Cell<'static> {
    if let Some(ref s) = sec_str {
        if is_fastest {
            Cell::from(Span::styled(
                s.clone(),
                Style::default().fg(Color::Magenta).bold(),
            ))
        } else {
            Cell::from(Span::styled(s.clone(), Style::default().fg(Color::Green)))
        }
    } else {
        Cell::from(Span::styled("-", Style::default().fg(Color::DarkGray)))
    }
}

fn format_tire_cell(tire: Option<&TireInfo>) -> Cell<'static> {
    if let Some(t) = tire {
        let compound_color = match t.compound.to_lowercase().as_str() {
            "soft" => Color::LightRed,
            "medium" => Color::Yellow,
            "hard" => Color::White,
            "intermediate" | "inter" => Color::Green,
            "wet" => Color::LightBlue,
            _ => Color::Gray,
        };

        let label = format!("{} ({}L)", t.compound, t.laps);
        Cell::from(Span::styled(
            label,
            Style::default().fg(compound_color).bold(),
        ))
    } else {
        Cell::from(Span::styled("-", Style::default().fg(Color::DarkGray)))
    }
}

fn parse_hex_color(hex: Option<&str>) -> Option<Color> {
    let s = hex?.strip_prefix('#').unwrap_or(hex?);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use crate::live::event::{LiveDriverEntry, PitInfo, SectorTimes};
    use chrono::Utc;
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_live_timing_table() {
        let backend = TestBackend::new(140, 24);
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
            pits: PitInfo::default(),
            laps_completed: 25,
            status: "On Track".to_string(),
            current_position: None,
            fastest_lap: true,
        };

        let timing = LiveTimingData {
            series_id: "f1".to_string(),
            session_name: "Race".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            total_laps: Some(57),
            current_lap: Some(25),
            time_remaining: None,
            session_status: "Green".to_string(),
            drivers: vec![driver],
            weather: None,
            updated_at: Utc::now(),
        };

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area(), &timing);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Pos"));
        assert!(content.contains("Max Verstappen"));
        assert!(content.contains("Red Bull Racing"));
        assert!(content.contains("Soft (12L)"));
        assert!(content.contains("LEADER"));
        assert!(content.contains("[FL]"));
    }

    #[test]
    fn test_draw_live_timing_table_omits_missing_columns() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());

        let driver = LiveDriverEntry {
            position: 1,
            driver_number: Some(24),
            driver_name: "William Byron".to_string(),
            driver_code: None,
            team_name: "Hendrick Motorsports".to_string(),
            team_color: None,
            gap_to_leader: "LEADER".to_string(),
            interval: "-".to_string(),
            last_lap_time: Some("28.456".to_string()),
            best_lap_time: Some("28.123".to_string()),
            sectors: SectorTimes::default(), // No sector times
            tire: None,                      // No tire data
            pits: PitInfo::default(),        // No pits
            laps_completed: 100,
            status: "On Track".to_string(),
            current_position: None,
            fastest_lap: true,
        };

        let timing = LiveTimingData {
            series_id: "nascar_cup".to_string(),
            session_name: "Race".to_string(),
            event_name: "Daytona 500".to_string(),
            circuit_name: "Daytona International Speedway".to_string(),
            total_laps: Some(200),
            current_lap: Some(100),
            time_remaining: None,
            session_status: "Green".to_string(),
            drivers: vec![driver],
            weather: None,
            updated_at: Utc::now(),
        };

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area(), &timing);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Pos"));
        assert!(content.contains("William Byron"));
        assert!(content.contains("28.456 [FL]"));
        // Verify S1, S2, S3, Tire, Pits headers are NOT present
        assert!(!content.contains("S1"));
        assert!(!content.contains("S2"));
        assert!(!content.contains("S3"));
        assert!(!content.contains("Tire"));
        assert!(!content.contains("Pits"));
    }
}
