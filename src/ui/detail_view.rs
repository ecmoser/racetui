use chrono::{Local, Utc};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Clear, HighlightSpacing, Paragraph, Row, Table};

use crate::app::App;
use crate::data::models::{EventStatus, StreamAccess};
use crate::data::results::RaceResults;

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

/// Draw the race results table widget with a custom title.
pub fn draw_results_with_title(
    frame: &mut Frame,
    results: &RaceResults,
    title: &str,
    area: Rect,
) {
    let header = Row::new(vec![
        Cell::from("Pos").style(Style::default().bold().fg(Color::White)),
        Cell::from("#").style(Style::default().bold().fg(Color::White)),
        Cell::from("Driver").style(Style::default().bold().fg(Color::White)),
        Cell::from("Team").style(Style::default().bold().fg(Color::White)),
        Cell::from("Gap").style(Style::default().bold().fg(Color::White)),
        Cell::from("Grid").style(Style::default().bold().fg(Color::White)),
        Cell::from("+/-").style(Style::default().bold().fg(Color::White)),
        Cell::from("Pts").style(Style::default().bold().fg(Color::White)),
        Cell::from("Status").style(Style::default().bold().fg(Color::White)),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = results
        .results
        .iter()
        .map(|r| {
            let pos_str = r
                .position
                .map(|p| format!("{}", p))
                .unwrap_or_else(|| "-".to_string());
            let num_str = r
                .driver_number
                .map(|n| format!("{}", n))
                .unwrap_or_else(|| "-".to_string());

            let name_code = match &r.driver_code {
                Some(code) => format!("{} ({})", r.driver_name, code),
                None => r.driver_name.clone(),
            };

            let driver_cell_content = if r.fastest_lap {
                Line::from(vec![
                    Span::styled(name_code, Style::default().bold().fg(Color::White)),
                    Span::styled(" [FL]", Style::default().bold().fg(Color::Magenta)),
                ])
            } else {
                Line::from(vec![Span::styled(
                    name_code,
                    Style::default().bold().fg(Color::White),
                )])
            };

            let gap_str = if r.gap_to_leader.is_empty() {
                "-".to_string()
            } else {
                r.gap_to_leader.clone()
            };

            let grid_str = r
                .grid_position
                .map(|g| format!("{}", g))
                .unwrap_or_else(|| "-".to_string());

            let (gained_str, gained_style) = match r.positions_gained() {
                Some(diff) if diff > 0 => (
                    format!("▲{}", diff),
                    Style::default().bold().fg(Color::Green),
                ),
                Some(diff) if diff < 0 => (
                    format!("▼{}", -diff),
                    Style::default().bold().fg(Color::Red),
                ),
                Some(_) => ("=".to_string(), Style::default().fg(Color::DarkGray)),
                None => ("-".to_string(), Style::default().fg(Color::DarkGray)),
            };

            let pts_str = if r.points.fract() == 0.0 {
                format!("{:.0}", r.points)
            } else {
                format!("{:.1}", r.points)
            };

            let status_str = if let Some(ref penalty) = r.penalty {
                format!("{} ({})", r.status, penalty)
            } else {
                r.status.clone()
            };

            let status_style = match r.status.to_uppercase().as_str() {
                "FINISHED" => Style::default().fg(Color::DarkGray),
                "DNF" | "RETIRED" => Style::default().bold().fg(Color::Red),
                "DNS" | "DSQ" => Style::default().bold().fg(Color::Magenta),
                _ => Style::default().fg(Color::Yellow),
            };

            Row::new(vec![
                Cell::from(pos_str).style(Style::default().bold().fg(Color::Yellow)),
                Cell::from(num_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(driver_cell_content),
                Cell::from(r.team.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(gap_str).style(Style::default().fg(Color::White)),
                Cell::from(grid_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(gained_str).style(gained_style),
                Cell::from(pts_str).style(Style::default().bold().fg(Color::Green)),
                Cell::from(status_str).style(status_style),
            ])
        })
        .collect();

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let widths = [
        Constraint::Length(5),  // Pos
        Constraint::Length(4),  // #
        Constraint::Min(26),    // Driver
        Constraint::Length(20), // Team
        Constraint::Length(14), // Gap
        Constraint::Length(5),  // Grid
        Constraint::Length(5),  // +/-
        Constraint::Length(5),  // Pts
        Constraint::Max(15),    // Status
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .highlight_spacing(HighlightSpacing::Always);

    frame.render_widget(table, area);
}

/// Draw the race results table widget.
pub fn draw_results(frame: &mut Frame, results: &RaceResults, area: Rect) {
    draw_results_with_title(frame, results, "Official Race Results", area);
}

/// Draw the sprint results table widget.
pub fn draw_sprint_results(frame: &mut Frame, results: &RaceResults, area: Rect) {
    draw_results_with_title(frame, results, "Official Sprint Results", area);
}

/// Draw the sprint starting grid table widget based on Sprint Qualifying / Shootout results.
pub fn draw_sprint_grid(frame: &mut Frame, sprint_results: &RaceResults, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Grid").style(Style::default().bold().fg(Color::White)),
        Cell::from("#").style(Style::default().bold().fg(Color::White)),
        Cell::from("Driver").style(Style::default().bold().fg(Color::White)),
        Cell::from("Team").style(Style::default().bold().fg(Color::White)),
    ])
    .height(1)
    .bottom_margin(1);

    let mut sorted_drivers = sprint_results.results.clone();
    sorted_drivers.sort_by_key(|r| r.grid_position.unwrap_or(999));

    let rows: Vec<Row> = sorted_drivers
        .iter()
        .map(|r| {
            let grid_str = r
                .grid_position
                .map(|g| format!("{}", g))
                .unwrap_or_else(|| "-".to_string());
            let num_str = r
                .driver_number
                .map(|n| format!("{}", n))
                .unwrap_or_else(|| "-".to_string());

            let name_code = match &r.driver_code {
                Some(code) => format!("{} ({})", r.driver_name, code),
                None => r.driver_name.clone(),
            };

            let pos_style = match r.grid_position {
                Some(1..=3) => Style::default().bold().fg(Color::Yellow),
                Some(4..=10) => Style::default().bold().fg(Color::Green),
                _ => Style::default().fg(Color::White),
            };

            Row::new(vec![
                Cell::from(grid_str).style(pos_style),
                Cell::from(num_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(name_code).style(Style::default().bold().fg(Color::White)),
                Cell::from(r.team.clone()).style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Sprint Starting Grid (Sprint Shootout) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let widths = [
        Constraint::Length(6),  // Grid
        Constraint::Length(4),  // #
        Constraint::Min(26),    // Driver
        Constraint::Length(25), // Team
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .highlight_spacing(HighlightSpacing::Always);

    frame.render_widget(table, area);
}

/// Draw the qualifying results table widget.
pub fn draw_qualifying_results(
    frame: &mut Frame,
    results: &crate::data::results::QualifyingResults,
    area: Rect,
) {
    let header = Row::new(vec![
        Cell::from("Pos").style(Style::default().bold().fg(Color::White)),
        Cell::from("#").style(Style::default().bold().fg(Color::White)),
        Cell::from("Driver").style(Style::default().bold().fg(Color::White)),
        Cell::from("Team").style(Style::default().bold().fg(Color::White)),
        Cell::from("Q1").style(Style::default().bold().fg(Color::White)),
        Cell::from("Q2").style(Style::default().bold().fg(Color::White)),
        Cell::from("Q3").style(Style::default().bold().fg(Color::White)),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = results
        .results
        .iter()
        .map(|r| {
            let pos_str = format!("{}", r.position);
            let num_str = r
                .driver_number
                .map(|n| format!("{}", n))
                .unwrap_or_else(|| "-".to_string());

            let driver_text = match &r.driver_code {
                Some(code) => format!("{} ({})", r.driver_name, code),
                None => r.driver_name.clone(),
            };

            let q1_str = r.q1.as_deref().unwrap_or("-").to_string();
            let q2_str = r.q2.as_deref().unwrap_or("-").to_string();
            let q3_str = r.q3.as_deref().unwrap_or("-").to_string();

            let pos_style = if r.position <= 3 {
                Style::default().bold().fg(Color::Yellow)
            } else if r.position <= 10 {
                Style::default().bold().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            Row::new(vec![
                Cell::from(pos_str).style(pos_style),
                Cell::from(num_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(driver_text).style(Style::default().bold().fg(Color::White)),
                Cell::from(r.team.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(q1_str).style(Style::default().fg(Color::White)),
                Cell::from(q2_str).style(Style::default().fg(Color::White)),
                Cell::from(q3_str).style(Style::default().bold().fg(Color::Yellow)),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Official Qualifying Results ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let widths = [
        Constraint::Length(5),  // Pos
        Constraint::Length(4),  // #
        Constraint::Min(26),    // Driver
        Constraint::Length(20), // Team
        Constraint::Length(12), // Q1
        Constraint::Length(12), // Q2
        Constraint::Length(12), // Q3
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .highlight_spacing(HighlightSpacing::Always);

    frame.render_widget(table, area);
}

fn draw_loading(frame: &mut Frame, msg: &str, area: Rect) {
    let block = Block::default()
        .title(" Results ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(msg, Style::default().bold().fg(Color::Yellow))),
    ])
    .block(block)
    .alignment(Alignment::Center);
    frame.render_widget(para, area);
}

fn draw_empty_tab(frame: &mut Frame, msg: &str, area: Rect) {
    let block = Block::default()
        .title(" Results ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(msg, Style::default().fg(Color::DarkGray))),
    ])
    .block(block)
    .alignment(Alignment::Center);
    frame.render_widget(para, area);
}

/// Draw the schedule and stream links tab content.
pub fn draw_schedule(frame: &mut Frame, event: &crate::data::models::RaceEvent, area: Rect) {
    let now_utc = Utc::now();
    let mut lines = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        "── Sessions Schedule ──",
        Style::default().bold().fg(Color::Yellow),
    )]));

    if event.sessions.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No session times available",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        let next_session_time = event.next_session_time();
        for session in &event.sessions {
            let (date_str, time_str, is_past, is_live, is_next) = match session.start_time {
                Some(start) => {
                    let local_dt = start.with_timezone(&Local);
                    let d = local_dt.format("%a, %b %d").to_string();
                    let t = local_dt.format("%l:%M %p").to_string();

                    let is_past = session.is_completed(&event.series_id);
                    let is_live = session.is_live(&event.series_id);
                    let is_next = Some(start) == next_session_time && !is_live && !is_past;
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
                Span::styled(" [LIVE] ", Style::default().bold().fg(Color::Red))
            } else if is_next {
                Span::styled(" [NEXT] ", Style::default().bold().fg(Color::Cyan))
            } else if is_past {
                Span::styled(" [Done] ", Style::default().fg(Color::DarkGray))
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
                Span::styled(format!("{:<24}", session.name), row_style),
                Span::styled(format!(" {:<16}", date_str), row_style),
                Span::styled(format!(" {:<12}", time_str), row_style),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "── Stream & Watch Links ──",
        Style::default().bold().fg(Color::Yellow),
    )]));

    if event.stream_links.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No streaming links available",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for (i, link) in event.stream_links.iter().enumerate() {
            let access_badge = match link.access {
                StreamAccess::Free => {
                    Span::styled(" [Free] ", Style::default().bold().fg(Color::Green))
                }
                StreamAccess::Paid => {
                    Span::styled(" [Paid $] ", Style::default().bold().fg(Color::Yellow))
                }
                StreamAccess::Mixed => Span::styled(" [Mixed] ", Style::default().fg(Color::Cyan)),
                StreamAccess::Unknown => {
                    Span::styled(" [Link] ", Style::default().fg(Color::DarkGray))
                }
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  [{}] ", i + 1),
                    Style::default().bold().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:<15}", link.platform),
                    Style::default().bold().fg(Color::White),
                ),
                access_badge,
                Span::styled(&link.url, Style::default().fg(Color::Blue)),
            ]));
        }
    }

    if let Some(next_session) = event.next_session() {
        if let Some(start_time) = next_session.start_time {
            let diff = start_time.signed_duration_since(now_utc);
            let formatted_countdown = format_duration(diff);
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Next: ", Style::default().bold().fg(Color::Yellow)),
                Span::styled(&next_session.name, Style::default().bold().fg(Color::White)),
                Span::styled(
                    format!(" in {}", formatted_countdown),
                    Style::default().bold().fg(Color::Green),
                ),
            ]));
        }
    }

    let block = Block::default()
        .title(" Event Schedule ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Draw the event detail view popup.
pub fn draw(frame: &mut Frame, app: &App) {
    let event = match app.selected_event() {
        Some(e) => e,
        None => return,
    };

    let area = frame.area();
    let popup_width = (area.width * 85 / 100).max(75).min(area.width);
    let popup_height = (area.height * 85 / 100).max(22).min(area.height);

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

    let round_opt = event.round;
    let round_display = round_opt
        .map(|r| format!(" · Round {}", r))
        .unwrap_or_default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header with tabs
            Constraint::Min(1),    // Tab content area
        ])
        .split(popup_area);

    let status_span = match event.current_status() {
        EventStatus::Completed => {
            Span::styled(" [Completed] ", Style::default().bold().fg(Color::Green))
        }
        EventStatus::Live => Span::styled(" [LIVE] ", Style::default().bold().fg(Color::Red)),
        EventStatus::Upcoming => {
            Span::styled(" [Upcoming] ", Style::default().bold().fg(Color::Cyan))
        }
        EventStatus::Cancelled => {
            Span::styled(" [Cancelled] ", Style::default().bold().fg(Color::DarkGray))
        }
    };

    let mut summary_lines = Vec::new();
    summary_lines.push(Line::from(vec![
        Span::styled(
            &event.event_name,
            Style::default()
                .bold()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            format!(" · {}{}", series_name, round_display),
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::raw("  "),
        status_span,
    ]));

    summary_lines.push(Line::from(vec![
        Span::styled(
            format!(
                "Circuit: {} · {}, {}",
                event.circuit_name, event.location, event.country
            ),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("  Date: {}", event.end_date.format("%a, %b %d, %Y")),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Dynamic Tab buttons
    let available_tabs = app.available_detail_tabs(event);
    let mut tab_spans = Vec::new();
    tab_spans.push(Span::styled(
        "Tabs: ",
        Style::default().bold().fg(Color::DarkGray),
    ));
    for (tab_type, label) in &available_tabs {
        if app.detail_tab == *tab_type {
            tab_spans.push(Span::styled(
                format!(" [ {} ] ", label),
                Style::default().bold().fg(Color::Black).bg(Color::Cyan),
            ));
        } else {
            tab_spans.push(Span::styled(
                format!(" [ {} ] ", label),
                Style::default().bold().fg(Color::DarkGray),
            ));
        }
        tab_spans.push(Span::raw(" "));
    }
    summary_lines.push(Line::from(""));
    summary_lines.push(Line::from(tab_spans));

    let top_block = Block::default()
        .title(format!(
            " {} (Esc/q to close, Tab / 1-{} / h,l to switch tabs) ",
            event.event_name,
            available_tabs.len()
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let top_paragraph = Paragraph::new(summary_lines)
        .block(top_block)
        .alignment(Alignment::Left);
    frame.render_widget(top_paragraph, chunks[0]);

    // 2. Render active tab
    match app.detail_tab {
        crate::app::DetailTab::Race => {
            let r_res = round_opt.and_then(|r| app.results.get(&(event.series_id.clone(), r)));
            let is_fetching = round_opt.map_or(false, |r| {
                app.results_fetching
                    .contains(&(event.series_id.clone(), r, "race".to_string()))
            });
            if let Some(results) = r_res {
                draw_results(frame, results, chunks[1]);
            } else if is_fetching {
                draw_loading(frame, "Fetching official race results...", chunks[1]);
            } else {
                draw_empty_tab(
                    frame,
                    "No race results available for this round. (Press Tab or 1-4 to switch)",
                    chunks[1],
                );
            }
        }
        crate::app::DetailTab::Qualifying => {
            let q_res =
                round_opt.and_then(|r| app.qualifying_results.get(&(event.series_id.clone(), r)));
            let is_fetching = round_opt.map_or(false, |r| {
                app.results_fetching.contains(&(
                    event.series_id.clone(),
                    r,
                    "qualifying".to_string(),
                ))
            });
            if let Some(results) = q_res {
                draw_qualifying_results(frame, results, chunks[1]);
            } else if is_fetching {
                draw_loading(frame, "Fetching official qualifying results...", chunks[1]);
            } else {
                draw_empty_tab(
                    frame,
                    "No qualifying results available for this round. (Press Tab or 1-4 to switch)",
                    chunks[1],
                );
            }
        }
        crate::app::DetailTab::Sprint => {
            let s_res =
                round_opt.and_then(|r| app.sprint_results.get(&(event.series_id.clone(), r)));
            let is_fetching = round_opt.map_or(false, |r| {
                app.results_fetching
                    .contains(&(event.series_id.clone(), r, "sprint".to_string()))
            });
            if let Some(results) = s_res {
                draw_sprint_results(frame, results, chunks[1]);
            } else if is_fetching {
                draw_loading(frame, "Fetching official sprint results...", chunks[1]);
            } else {
                draw_empty_tab(
                    frame,
                    "No sprint results available for this round. (Press Tab or 1-5 to switch)",
                    chunks[1],
                );
            }
        }
        crate::app::DetailTab::SprintQualifying => {
            let s_res =
                round_opt.and_then(|r| app.sprint_results.get(&(event.series_id.clone(), r)));
            let is_fetching = round_opt.map_or(false, |r| {
                app.results_fetching.contains(&(
                    event.series_id.clone(),
                    r,
                    "sprint".to_string(),
                ))
            });
            if let Some(results) = s_res {
                draw_sprint_grid(frame, results, chunks[1]);
            } else if is_fetching {
                draw_loading(
                    frame,
                    "Fetching official sprint qualifying / starting grid...",
                    chunks[1],
                );
            } else {
                draw_empty_tab(
                    frame,
                    "Sprint qualifying results are not published separately (see Sprint starting grid)",
                    chunks[1],
                );
            }
        }
        crate::app::DetailTab::Schedule => {
            draw_schedule(frame, event, chunks[1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(
            format_duration(
                chrono::Duration::days(2)
                    + chrono::Duration::hours(3)
                    + chrono::Duration::minutes(15)
            ),
            "2d 3h 15m"
        );
        assert_eq!(
            format_duration(chrono::Duration::hours(4) + chrono::Duration::minutes(20)),
            "4h 20m"
        );
        assert_eq!(format_duration(chrono::Duration::minutes(35)), "35m");
        assert_eq!(format_duration(chrono::Duration::seconds(20)), "< 1m");
        assert_eq!(format_duration(chrono::Duration::seconds(-10)), "now");
    }

    #[test]
    fn test_draw_results_table() {
        use crate::data::results::{DriverResult, RaceResults};
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let results = RaceResults {
            series_id: "f1".to_string(),
            round: 1,
            event_name: "Bahrain GP".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            race_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            results: vec![
                DriverResult {
                    position: Some(1),
                    driver_name: "Max Verstappen".to_string(),
                    driver_code: Some("VER".to_string()),
                    driver_number: Some(1),
                    team: "Red Bull".to_string(),
                    gap_to_leader: "1:31:44.742".to_string(),
                    gap_to_ahead: "".to_string(),
                    grid_position: Some(1),
                    points: 26.0,
                    fastest_lap: true,
                    penalty: None,
                    status: "Finished".to_string(),
                },
                DriverResult {
                    position: Some(2),
                    driver_name: "Lando Norris".to_string(),
                    driver_code: Some("NOR".to_string()),
                    driver_number: Some(4),
                    team: "McLaren".to_string(),
                    gap_to_leader: "+22.457".to_string(),
                    gap_to_ahead: "+22.457".to_string(),
                    grid_position: Some(3),
                    points: 18.0,
                    fastest_lap: false,
                    penalty: None,
                    status: "Finished".to_string(),
                },
            ],
            fetched_at: chrono::Utc::now(),
        };

        terminal
            .draw(|f| {
                draw_results(f, &results, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Official Race Results"));
        assert!(content.contains("Max Verstappen"));
        assert!(content.contains("Lando Norris"));
        assert!(content.contains("▲1")); // Positions gained
    }

    #[test]
    fn test_draw_detail_view_with_results() {
        use crate::config::UserConfig;
        use crate::data::models::{CarStyle, RaceEvent, Series};
        use crate::data::results::{DriverResult, RaceResults};
        use ratatui::backend::TestBackend;
        use std::collections::HashMap;

        let backend = TestBackend::new(100, 30);
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
        let completed_event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Completed,
        };

        app.update_series_data("f1".to_string(), vec![completed_event]);
        app.active_filters.statuses.insert(EventStatus::Completed);
        app.table_state.select(Some(1));
        app.show_detail = true;

        let results = RaceResults {
            series_id: "f1".to_string(),
            round: 1,
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            race_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            results: vec![DriverResult {
                position: Some(1),
                driver_name: "Max Verstappen".to_string(),
                driver_code: Some("VER".to_string()),
                driver_number: Some(1),
                team: "Red Bull".to_string(),
                gap_to_leader: "1:31:44.742".to_string(),
                gap_to_ahead: "".to_string(),
                grid_position: Some(1),
                points: 26.0,
                fastest_lap: true,
                penalty: None,
                status: "Finished".to_string(),
            }],
            fetched_at: chrono::Utc::now(),
        };

        app.results.insert(("f1".to_string(), 1), results);

        terminal
            .draw(|f| {
                draw(f, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Bahrain Grand Prix"));
        assert!(content.contains("Official Race Results"));
        assert!(content.contains("Max Verstappen"));
    }

    #[test]
    fn test_draw_qualifying_results_table() {
        use crate::data::results::{QualifyingDriverResult, QualifyingResults};
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let results = QualifyingResults {
            series_id: "f1".to_string(),
            round: 1,
            event_name: "Bahrain GP".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            race_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            results: vec![QualifyingDriverResult {
                position: 1,
                driver_name: "George Russell".to_string(),
                driver_code: Some("RUS".to_string()),
                driver_number: Some(63),
                team: "Mercedes".to_string(),
                q1: Some("1:19.507".to_string()),
                q2: Some("1:18.934".to_string()),
                q3: Some("1:18.518".to_string()),
            }],
            fetched_at: chrono::Utc::now(),
        };

        terminal
            .draw(|f| {
                draw_qualifying_results(f, &results, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Official Qualifying Results"));
        assert!(content.contains("George Russell"));
        assert!(content.contains("1:18.518"));
    }

    #[test]
    fn test_draw_detail_view_loading_state() {
        use crate::config::UserConfig;
        use crate::data::models::{CarStyle, RaceEvent, Series};
        use ratatui::backend::TestBackend;
        use std::collections::HashMap;

        let backend = TestBackend::new(100, 30);
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
        let completed_event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Monaco Grand Prix".to_string(),
            circuit_name: "Circuit de Monaco".to_string(),
            location: "Monte Carlo".to_string(),
            country: "Monaco".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            round: Some(8),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Completed,
        };

        app.update_series_data("f1".to_string(), vec![completed_event]);
        app.active_filters.statuses.insert(EventStatus::Completed);
        app.table_state.select(Some(1));
        app.results_fetching
            .insert(("f1".to_string(), 8, "race".to_string()));
        app.show_detail = true;

        terminal
            .draw(|f| {
                draw(f, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Monaco Grand Prix"));
        assert!(content.contains("Fetching official"));
    }

    #[test]
    fn test_draw_detail_view_qualifying_tab() {
        use crate::config::UserConfig;
        use crate::data::models::{CarStyle, RaceEvent, Series};
        use crate::data::results::{QualifyingDriverResult, QualifyingResults};
        use ratatui::backend::TestBackend;
        use std::collections::HashMap;

        let backend = TestBackend::new(100, 30);
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
        let completed_event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            round: Some(1),
            sessions: vec![],
            stream_links: vec![],
            status: EventStatus::Completed,
        };

        app.update_series_data("f1".to_string(), vec![completed_event]);
        app.active_filters.statuses.insert(EventStatus::Completed);
        app.table_state.select(Some(1));
        app.detail_tab = crate::app::DetailTab::Qualifying;
        app.show_detail = true;

        let q_results = QualifyingResults {
            series_id: "f1".to_string(),
            round: 1,
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            race_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            results: vec![QualifyingDriverResult {
                position: 1,
                driver_name: "George Russell".to_string(),
                driver_code: Some("RUS".to_string()),
                driver_number: Some(63),
                team: "Mercedes".to_string(),
                q1: Some("1:19.507".to_string()),
                q2: Some("1:18.934".to_string()),
                q3: Some("1:18.518".to_string()),
            }],
            fetched_at: chrono::Utc::now(),
        };
        app.qualifying_results
            .insert(("f1".to_string(), 1), q_results);

        terminal
            .draw(|f| {
                draw(f, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Bahrain Grand Prix"));
        assert!(content.contains("Official Qualifying Results"));
        assert!(content.contains("George Russell"));
    }
}
