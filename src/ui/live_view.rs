use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, LiveSubTab};
use crate::ui::{live_driver_detail, live_timing_table};

/// Draw the Live view UI (header + weather bar + sub-tabs + timing table / track map).
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" 🔴 Live Timing ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightRed));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // If no live timing data is loaded yet, show placeholder / active live sessions info
    if app.live_timing_data.is_none() {
        draw_empty_live_view(frame, app, inner_area);
        return;
    }

    let timing_data = app.live_timing_data.clone().unwrap();

    // Layout:
    // 1. Session Header (1 line)
    // 2. Weather bar (1 line)
    // 3. Sub-tab bar (1 line)
    // 4. Main content (Timing table or Track Map)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Length(1), // Weather
            Constraint::Length(1), // Sub-tabs
            Constraint::Min(5),    // Content
        ])
        .split(inner_area);

    // 1. Session Header
    draw_session_header(frame, app, chunks[0], &timing_data);

    // 2. Weather Bar
    draw_weather_bar(frame, chunks[1], &timing_data);

    // 3. Sub-tabs bar
    draw_sub_tabs(frame, app, chunks[2]);

    // 4. Content Area based on active sub-tab
    match app.live_sub_tab {
        LiveSubTab::Timing => {
            if app.live_driver_detail_open {
                let timing_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(60), // Leaderboard table
                        Constraint::Percentage(40), // Driver detail panel
                    ])
                    .split(chunks[3]);

                live_timing_table::draw(frame, app, timing_chunks[0], &timing_data);

                let selected_idx = app.live_timing_table_state.selected().unwrap_or(0);
                if let Some(driver) = timing_data.drivers.get(selected_idx) {
                    live_driver_detail::draw(frame, app, timing_chunks[1], driver);
                }
            } else {
                live_timing_table::draw(frame, app, chunks[3], &timing_data);
            }
        }
        LiveSubTab::TrackMap => {
            draw_track_map_placeholder(frame, chunks[3]);
        }
    }
}

fn draw_session_header(frame: &mut Frame, app: &App, area: Rect, data: &crate::live::event::LiveTimingData) {
    let series_name = app
        .series_registry
        .get(&data.series_id)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| data.series_id.to_uppercase());

    let series_color = app
        .series_registry
        .get(&data.series_id)
        .map(|s| Color::Rgb(s.color.0, s.color.1, s.color.2))
        .unwrap_or(Color::Red);

    let status_style = match data.session_status.to_lowercase().as_str() {
        "green" => Style::default().fg(Color::Green).bold(),
        "yellow" | "caution" | "sc" | "vsc" => Style::default().fg(Color::Yellow).bold(),
        "red" => Style::default().fg(Color::Red).bold(),
        "finished" | "checkered" => Style::default().fg(Color::Cyan).bold(),
        _ => Style::default().fg(Color::White).bold(),
    };

    let status_badge = format!(" [{}] ", data.session_status.to_uppercase());

    let lap_info = match (data.current_lap, data.total_laps) {
        (Some(cur), Some(tot)) => format!("  Lap {}/{}", cur, tot),
        (Some(cur), None) => format!("  Lap {}", cur),
        _ => String::new(),
    };

    let header_spans = vec![
        Span::styled(format!(" {} ", series_name), Style::default().fg(Color::Black).bg(series_color).bold()),
        Span::raw(" "),
        Span::styled(&data.event_name, Style::default().bold()),
        Span::styled(format!(" — {}", data.session_name), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" ({})", data.circuit_name), Style::default().fg(Color::DarkGray)),
        Span::styled(lap_info, Style::default().fg(Color::Cyan).bold()),
        Span::raw("  "),
        Span::styled(status_badge, status_style),
    ];

    frame.render_widget(Paragraph::new(Line::from(header_spans)), area);
}

fn draw_weather_bar(frame: &mut Frame, area: Rect, data: &crate::live::event::LiveTimingData) {
    let weather_str = if let Some(ref w) = data.weather {
        w.display_string()
    } else {
        "☀️ Track Conditions: Dry / Normal  |  Weather feed connecting...".to_string()
    };

    let line = Line::from(vec![
        Span::styled(" ⛅ Weather: ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(weather_str, Style::default().fg(Color::White)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn draw_sub_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let is_timing = app.live_sub_tab == LiveSubTab::Timing;
    let is_track_map = app.live_sub_tab == LiveSubTab::TrackMap;

    let timing_style = if is_timing {
        Style::default().fg(Color::Black).bg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let track_map_style = if is_track_map {
        Style::default().fg(Color::Black).bg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let line = Line::from(vec![
        Span::styled(" [ 1. Timing Leaderboard ] ", timing_style),
        Span::raw(" "),
        Span::styled(" [ 2. Track Map ] ", track_map_style),
        Span::raw("  "),
        Span::styled("(←/→ or h/l to switch sub-tabs)", Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn draw_track_map_placeholder(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Track Map ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled("🗺️ Track Map View", Style::default().fg(Color::Cyan).bold())),
        Line::from(""),
        Line::from(Span::styled(
            "Track map braille rendering engine will be implemented in Phase 3.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Left Arrow (← or 'h') to switch back to the Timing Leaderboard.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(text).block(block).alignment(Alignment::Center);
    frame.render_widget(p, area);
}

fn draw_empty_live_view(frame: &mut Frame, app: &App, area: Rect) {
    let live_sessions = app.get_live_sessions();

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "🔴 No Live Session Selected",
            Style::default().fg(Color::LightRed).bold(),
        )),
        Line::from(""),
    ];

    if live_sessions.is_empty() {
        lines.push(Line::from(Span::styled(
            "There are currently no live racing sessions detected.",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press '1' for List View, '2' for Calendar View, or '4' for Standings View.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Active Live Sessions:",
            Style::default().fg(Color::Green).bold(),
        )));
        lines.push(Line::from(""));
        for (sid, sname, sess) in &live_sessions {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Green)),
                Span::styled(sname, Style::default().bold()),
                Span::styled(format!(" — {}", sess), Style::default().fg(Color::Yellow)),
                Span::styled(format!(" ({})", sid), Style::default().fg(Color::DarkGray)),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press '3' to select an active live session to watch.",
            Style::default().fg(Color::Cyan),
        )));
    }

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(p, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use crate::live::event::{LiveDriverEntry, LiveTimingData, PitInfo, SectorTimes, TireInfo, WeatherInfo};
    use chrono::Utc;
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_live_view_empty() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.view_mode = crate::app::ViewMode::Live;

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Live Timing"));
        assert!(content.contains("No Live Session Selected"));
    }

    #[test]
    fn test_draw_live_view_with_data() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.view_mode = crate::app::ViewMode::Live;

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
            sectors: SectorTimes::default(),
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

        let weather = WeatherInfo {
            air_temp_c: Some(24.5),
            track_temp_c: Some(38.0),
            humidity_pct: Some(45.0),
            wind_speed_kmh: Some(10.5),
            wind_direction_deg: Some(180.0),
            rainfall: false,
            rain_intensity: None,
            description: Some("Sunny".to_string()),
        };

        app.live_timing_data = Some(LiveTimingData {
            series_id: "f1".to_string(),
            session_name: "Race".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            total_laps: Some(57),
            current_lap: Some(25),
            time_remaining: None,
            session_status: "Green".to_string(),
            drivers: vec![driver],
            weather: Some(weather),
            updated_at: Utc::now(),
        });

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Bahrain Grand Prix"));
        assert!(content.contains("Race"));
        assert!(content.contains("Weather"));
        assert!(content.contains("Air: 24.5°C"));
        assert!(content.contains("Timing Leaderboard"));
        assert!(content.contains("Max Verstappen"));
    }

    #[test]
    fn test_draw_live_view_with_driver_detail_expanded() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.view_mode = crate::app::ViewMode::Live;
        app.live_driver_detail_open = true;

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
            sectors: SectorTimes::default(),
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

        app.live_timing_data = Some(LiveTimingData {
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
        });

        terminal
            .draw(|f| {
                draw(f, &mut app, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Driver Detail:"));
        assert!(content.contains("Max Verstappen"));
        assert!(content.contains("Red Bull Racing"));
    }
}
