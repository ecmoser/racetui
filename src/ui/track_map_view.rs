use ratatui::prelude::*;

use crate::app::App;
use crate::live::event::LiveTimingData;
use crate::live::track_map::{draw_driver_legend, draw_track_map, TrackMapData};

/// Draw the full Track Map view with circuit canvas and driver legend.
pub fn draw(frame: &mut Frame, app: &App, area: Rect, data: &LiveTimingData) {
    let points = app.track_map_geometry.clone().unwrap_or_default();
    let track_map_data = TrackMapData::from_live_timing(&data.circuit_name, &points, data);

    if area.width < 60 {
        // For narrow terminals, show only the track map
        draw_track_map(frame, area, &track_map_data);
    } else {
        // Split area: 75% track map canvas, 25% driver legend
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75), // Track map canvas
                Constraint::Percentage(25), // Driver legend panel
            ])
            .split(area);

        draw_track_map(frame, chunks[0], &track_map_data);
        draw_driver_legend(frame, chunks[1], &track_map_data.drivers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use crate::live::event::{DriverPosition, LiveDriverEntry, PitInfo, SectorTimes, TireInfo};
    use crate::live::track_map::TrackPoint;
    use chrono::Utc;
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_track_map_view() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.track_map_geometry = Some(vec![
            TrackPoint { x: 0.0, y: 0.0 },
            TrackPoint { x: 100.0, y: 0.0 },
            TrackPoint { x: 100.0, y: 100.0 },
            TrackPoint { x: 0.0, y: 100.0 },
        ]);

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
            current_position: Some(DriverPosition {
                driver_number: Some(1),
                driver_code: Some("VER".to_string()),
                x: 50.0,
                y: 50.0,
            }),
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
                draw(f, &app, f.area(), &timing);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Track Map"));
        assert!(content.contains("Drivers"));
        assert!(content.contains("VER"));
    }

    #[test]
    fn test_draw_track_map_view_narrow() {
        let backend = TestBackend::new(50, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(HashMap::new(), UserConfig::default());

        let timing = LiveTimingData {
            series_id: "f1".to_string(),
            session_name: "Race".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Bahrain International Circuit".to_string(),
            total_laps: Some(57),
            current_lap: Some(25),
            time_remaining: None,
            session_status: "Green".to_string(),
            drivers: vec![],
            weather: None,
            updated_at: Utc::now(),
        };

        terminal
            .draw(|f| {
                draw(f, &app, f.area(), &timing);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Track Map"));
    }
}
