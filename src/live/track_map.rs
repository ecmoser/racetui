use anyhow::{Context, Result};
use ratatui::prelude::*;
use ratatui::symbols::Marker;
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine, Points as CanvasPoints};
use ratatui::widgets::{Block, Borders, Paragraph};
use serde::{Deserialize, Serialize};

use crate::live::event::LiveTimingData;
use crate::scraper::fetcher::create_http_client;

pub const MULTIVIEWER_CIRCUITS_BASE_URL: &str = "https://api.multiviewer.app/api/v1/circuits";

/// A single coordinate point on the circuit track outline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackPoint {
    /// X coordinate (e.g., in meters or normalized units)
    pub x: f64,
    /// Y coordinate (e.g., in meters or normalized units)
    pub y: f64,
}

/// A driver entry positioned on the track map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackDriver {
    /// Current leaderboard position (1-based)
    pub position: u32,
    /// Driver number
    pub driver_number: Option<u32>,
    /// Driver code (e.g., "VER")
    pub driver_code: Option<String>,
    /// Driver full name
    pub driver_name: String,
    /// Team name
    pub team_name: String,
    /// Team color hex code (e.g., "#3671C6")
    pub team_color: Option<String>,
    /// X coordinate on the track
    pub x: f64,
    /// Y coordinate on the track
    pub y: f64,
}

/// Complete dataset for rendering a circuit track map and driver locations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TrackMapData {
    /// Circuit or track name
    pub circuit_name: String,
    /// Outline coordinates of the circuit
    pub points: Vec<TrackPoint>,
    /// Current drivers on track with their coordinate positions
    pub drivers: Vec<TrackDriver>,
}

impl TrackMapData {
    /// Create a new TrackMapData instance.
    pub fn new(
        circuit_name: impl Into<String>,
        points: Vec<TrackPoint>,
        drivers: Vec<TrackDriver>,
    ) -> Self {
        Self {
            circuit_name: circuit_name.into(),
            points,
            drivers,
        }
    }

    /// Build TrackMapData from circuit points and live timing data.
    pub fn from_live_timing(
        circuit_name: &str,
        points: &[TrackPoint],
        live_data: &LiveTimingData,
    ) -> Self {
        let mut drivers = Vec::new();
        for d in &live_data.drivers {
            if let Some(ref pos) = d.current_position {
                drivers.push(TrackDriver {
                    position: d.position,
                    driver_number: d.driver_number,
                    driver_code: d.driver_code.clone(),
                    driver_name: d.driver_name.clone(),
                    team_name: d.team_name.clone(),
                    team_color: d.team_color.clone(),
                    x: pos.x,
                    y: pos.y,
                });
            }
        }

        Self {
            circuit_name: circuit_name.to_string(),
            points: points.to_vec(),
            drivers,
        }
    }

    /// Returns true if there are no track points.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Calculate x and y bounds with a 5% margin around the circuit and drivers.
    pub fn bounds(&self) -> Option<([f64; 2], [f64; 2])> {
        if self.points.is_empty() && self.drivers.is_empty() {
            return None;
        }

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for p in &self.points {
            if p.x < min_x {
                min_x = p.x;
            }
            if p.x > max_x {
                max_x = p.x;
            }
            if p.y < min_y {
                min_y = p.y;
            }
            if p.y > max_y {
                max_y = p.y;
            }
        }

        for d in &self.drivers {
            if d.x < min_x {
                min_x = d.x;
            }
            if d.x > max_x {
                max_x = d.x;
            }
            if d.y < min_y {
                min_y = d.y;
            }
            if d.y > max_y {
                max_y = d.y;
            }
        }

        if min_x.is_infinite() || max_x.is_infinite() || min_y.is_infinite() || max_y.is_infinite()
        {
            return None;
        }

        let span_x = (max_x - min_x).abs();
        let span_y = (max_y - min_y).abs();
        let pad_x = if span_x < 1e-6 { 1.0 } else { span_x * 0.05 };
        let pad_y = if span_y < 1e-6 { 1.0 } else { span_y * 0.05 };

        Some((
            [min_x - pad_x, max_x + pad_x],
            [min_y - pad_y, max_y + pad_y],
        ))
    }
}

/// Draw the track map widget using Ratatui's Canvas with braille characters.
pub fn draw_track_map(frame: &mut Frame, area: Rect, data: &TrackMapData) {
    let title = if data.circuit_name.is_empty() {
        " Track Map ".to_string()
    } else {
        format!(" Track Map: {} ", data.circuit_name)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    if data.points.is_empty() {
        let empty_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No circuit geometry loaded",
                Style::default().fg(Color::Yellow).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Circuit layout will appear here once telemetry is available.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty_p, area);
        return;
    }

    let Some((x_bounds, y_bounds)) = data.bounds() else {
        frame.render_widget(block, area);
        return;
    };

    let canvas = Canvas::default()
        .block(block)
        .x_bounds(x_bounds)
        .y_bounds(y_bounds)
        .marker(Marker::Braille)
        .paint(|ctx| {
            // Draw circuit outline lines
            for window in data.points.windows(2) {
                ctx.draw(&CanvasLine {
                    x1: window[0].x,
                    y1: window[0].y,
                    x2: window[1].x,
                    y2: window[1].y,
                    color: Color::White,
                });
            }
            if data.points.len() > 2 {
                let first = &data.points[0];
                let last = &data.points[data.points.len() - 1];
                ctx.draw(&CanvasLine {
                    x1: last.x,
                    y1: last.y,
                    x2: first.x,
                    y2: first.y,
                    color: Color::White,
                });
            }

            // Draw drivers on the track
            for driver in &data.drivers {
                let color = parse_hex_color(driver.team_color.as_deref()).unwrap_or(Color::Yellow);
                ctx.draw(&CanvasPoints {
                    coords: &[(driver.x, driver.y)],
                    color,
                });

                let label = driver
                    .driver_code
                    .clone()
                    .unwrap_or_else(|| driver.driver_name.chars().take(3).collect());
                ctx.print(
                    driver.x,
                    driver.y,
                    Line::from(Span::styled(label, Style::default().fg(color).bold())),
                );
            }
        });

    frame.render_widget(canvas, area);
}

/// Draw the driver legend panel next to the track map.
pub fn draw_driver_legend(frame: &mut Frame, area: Rect, drivers: &[TrackDriver]) {
    let block = Block::default()
        .title(" Drivers ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if drivers.is_empty() {
        let empty_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No active cars on track",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty_p, area);
        return;
    }

    let mut lines = Vec::new();
    for driver in drivers {
        let color = parse_hex_color(driver.team_color.as_deref()).unwrap_or(Color::White);
        let code = driver.driver_code.as_deref().unwrap_or("???");
        let num_str = driver
            .driver_number
            .map(|n| format!("#{:>2}", n))
            .unwrap_or_else(|| "   ".to_string());

        lines.push(Line::from(vec![
            Span::styled("● ", Style::default().fg(color)),
            Span::styled(
                format!("P{:<2} ", driver.position),
                Style::default().fg(Color::Yellow).bold(),
            ),
            Span::styled(format!("{} ", num_str), Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:<3} ", code), Style::default().bold()),
            Span::styled(&driver.driver_name, Style::default().fg(Color::White)),
        ]));
    }

    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, area);
}

/// Parse a hex color string (e.g., "#3671C6" or "3671C6") into a Ratatui Color.
pub fn parse_hex_color(hex: Option<&str>) -> Option<Color> {
    let s = hex?.strip_prefix('#').unwrap_or(hex?);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

/// MultiViewer circuit geometry response format.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MultiViewerCircuitResponse {
    /// Standard MultiViewer format with parallel x and y coordinate vectors
    Coordinates {
        #[serde(default)]
        circuit_key: Option<u64>,
        #[serde(default)]
        circuit_name: Option<String>,
        x: Vec<f64>,
        y: Vec<f64>,
        #[serde(default)]
        rotation: Option<f64>,
    },
    /// List of TrackPoints
    PointsList(Vec<TrackPoint>),
    /// Object containing a list of TrackPoints
    PointsObject { points: Vec<TrackPoint> },
}

/// Parse MultiViewer circuit JSON response into a list of TrackPoints.
pub fn parse_multiviewer_circuit_json(json_str: &str) -> Result<Vec<TrackPoint>> {
    let response: MultiViewerCircuitResponse = serde_json::from_str(json_str)
        .context("Failed to parse MultiViewer circuit geometry JSON")?;

    match response {
        MultiViewerCircuitResponse::Coordinates { x, y, rotation, .. } => {
            let points: Vec<TrackPoint> = x
                .into_iter()
                .zip(y.into_iter())
                .map(|(px, py)| {
                    if let Some(rot_deg) = rotation {
                        if rot_deg.abs() > 1e-6 {
                            let rad = rot_deg.to_radians();
                            let cos_a = rad.cos();
                            let sin_a = rad.sin();
                            let rx = px * cos_a - py * sin_a;
                            let ry = px * sin_a + py * cos_a;
                            return TrackPoint { x: rx, y: ry };
                        }
                    }
                    TrackPoint { x: px, y: py }
                })
                .collect();
            Ok(points)
        }
        MultiViewerCircuitResponse::PointsList(pts) => Ok(pts),
        MultiViewerCircuitResponse::PointsObject { points } => Ok(points),
    }
}

/// Fetch circuit geometry from MultiViewer API for a given circuit key and year.
pub async fn fetch_circuit_geometry(circuit_key: u64, year: u32) -> Result<Vec<TrackPoint>> {
    let client = create_http_client().unwrap_or_else(|_| reqwest::Client::new());
    fetch_circuit_geometry_with_client(&client, circuit_key, year).await
}

/// Fetch circuit geometry with a custom HTTP client.
pub async fn fetch_circuit_geometry_with_client(
    client: &reqwest::Client,
    circuit_key: u64,
    year: u32,
) -> Result<Vec<TrackPoint>> {
    let url = format!("{}/{}/{}", MULTIVIEWER_CIRCUITS_BASE_URL, circuit_key, year);
    fetch_circuit_geometry_from_url(client, &url).await
}

/// Fetch circuit geometry directly from a given URL.
pub async fn fetch_circuit_geometry_from_url(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<TrackPoint>> {
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch circuit geometry from {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!(
            "MultiViewer API returned status {} for {}",
            resp.status(),
            url
        );
    }

    let text = resp
        .text()
        .await
        .with_context(|| format!("Failed to read circuit geometry response from {}", url))?;

    parse_multiviewer_circuit_json(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::event::{DriverPosition, LiveDriverEntry, PitInfo, SectorTimes};
    use chrono::Utc;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_track_point_serialization() {
        let point = TrackPoint {
            x: 123.45,
            y: -67.89,
        };
        let json = serde_json::to_string(&point).expect("serialize TrackPoint");
        let parsed: TrackPoint = serde_json::from_str(&json).expect("deserialize TrackPoint");
        assert_eq!(point, parsed);
    }

    #[test]
    fn test_track_driver_and_track_map_data_serialization() {
        let driver = TrackDriver {
            position: 1,
            driver_number: Some(1),
            driver_code: Some("VER".to_string()),
            driver_name: "Max Verstappen".to_string(),
            team_name: "Red Bull Racing".to_string(),
            team_color: Some("#3671C6".to_string()),
            x: 100.0,
            y: 200.0,
        };
        let map_data = TrackMapData {
            circuit_name: "Monaco".to_string(),
            points: vec![
                TrackPoint { x: 0.0, y: 0.0 },
                TrackPoint { x: 10.0, y: 20.0 },
                TrackPoint { x: 0.0, y: 20.0 },
            ],
            drivers: vec![driver],
        };

        let json = serde_json::to_string(&map_data).expect("serialize TrackMapData");
        let deserialized: TrackMapData =
            serde_json::from_str(&json).expect("deserialize TrackMapData");
        assert_eq!(map_data, deserialized);
    }

    #[test]
    fn test_track_map_bounds_calculation() {
        let empty_data = TrackMapData::default();
        assert_eq!(empty_data.bounds(), None);

        let data = TrackMapData {
            circuit_name: "Test Circuit".to_string(),
            points: vec![
                TrackPoint { x: 100.0, y: 50.0 },
                TrackPoint { x: 200.0, y: 150.0 },
            ],
            drivers: vec![TrackDriver {
                position: 1,
                driver_number: Some(1),
                driver_code: Some("VER".to_string()),
                driver_name: "Verstappen".to_string(),
                team_name: "RBR".to_string(),
                team_color: None,
                x: 250.0,
                y: 30.0,
            }],
        };

        let (x_bounds, y_bounds) = data.bounds().expect("valid bounds");
        // min_x = 100.0, max_x = 250.0, span = 150.0, pad = 7.5 -> [92.5, 257.5]
        assert!((x_bounds[0] - 92.5).abs() < 1e-4);
        assert!((x_bounds[1] - 257.5).abs() < 1e-4);
        // min_y = 30.0, max_y = 150.0, span = 120.0, pad = 6.0 -> [24.0, 156.0]
        assert!((y_bounds[0] - 24.0).abs() < 1e-4);
        assert!((y_bounds[1] - 156.0).abs() < 1e-4);
    }

    #[test]
    fn test_track_map_from_live_timing() {
        let driver_with_pos = LiveDriverEntry {
            position: 1,
            driver_number: Some(1),
            driver_name: "Max Verstappen".to_string(),
            driver_code: Some("VER".to_string()),
            team_name: "Red Bull Racing".to_string(),
            team_color: Some("#3671C6".to_string()),
            gap_to_leader: "LEADER".to_string(),
            interval: "-".to_string(),
            last_lap_time: None,
            best_lap_time: None,
            sectors: SectorTimes::default(),
            tire: None,
            pits: PitInfo::default(),
            laps_completed: 10,
            status: "On Track".to_string(),
            current_position: Some(DriverPosition {
                driver_number: Some(1),
                driver_code: Some("VER".to_string()),
                x: 50.0,
                y: 75.0,
            }),
            fastest_lap: false,
        };

        let driver_without_pos = LiveDriverEntry {
            position: 2,
            driver_number: Some(44),
            driver_name: "Lewis Hamilton".to_string(),
            driver_code: Some("HAM".to_string()),
            team_name: "Ferrari".to_string(),
            team_color: Some("#E8002D".to_string()),
            gap_to_leader: "+1.2".to_string(),
            interval: "+1.2".to_string(),
            last_lap_time: None,
            best_lap_time: None,
            sectors: SectorTimes::default(),
            tire: None,
            pits: PitInfo::default(),
            laps_completed: 10,
            status: "In Pit".to_string(),
            current_position: None,
            fastest_lap: false,
        };

        let live_data = LiveTimingData {
            series_id: "f1".to_string(),
            session_name: "Race".to_string(),
            event_name: "Monaco Grand Prix".to_string(),
            circuit_name: "Circuit de Monaco".to_string(),
            total_laps: Some(78),
            current_lap: Some(10),
            time_remaining: None,
            session_status: "Green".to_string(),
            drivers: vec![driver_with_pos, driver_without_pos],
            weather: None,
            updated_at: Utc::now(),
        };

        let points = vec![
            TrackPoint { x: 0.0, y: 0.0 },
            TrackPoint { x: 100.0, y: 100.0 },
        ];

        let map_data = TrackMapData::from_live_timing("Circuit de Monaco", &points, &live_data);
        assert_eq!(map_data.circuit_name, "Circuit de Monaco");
        assert_eq!(map_data.points.len(), 2);
        assert_eq!(map_data.drivers.len(), 1);
        assert_eq!(map_data.drivers[0].driver_name, "Max Verstappen");
        assert_eq!(map_data.drivers[0].x, 50.0);
        assert_eq!(map_data.drivers[0].y, 75.0);
    }

    #[test]
    fn test_draw_track_map_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let map_data = TrackMapData::default();

        terminal
            .draw(|f| {
                draw_track_map(f, f.area(), &map_data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Track Map"));
        assert!(content.contains("No circuit geometry loaded"));
    }

    #[test]
    fn test_draw_track_map_with_geometry_and_drivers() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let map_data = TrackMapData {
            circuit_name: "Monza".to_string(),
            points: vec![
                TrackPoint { x: 0.0, y: 0.0 },
                TrackPoint { x: 100.0, y: 0.0 },
                TrackPoint { x: 100.0, y: 50.0 },
                TrackPoint { x: 0.0, y: 50.0 },
            ],
            drivers: vec![TrackDriver {
                position: 1,
                driver_number: Some(1),
                driver_code: Some("VER".to_string()),
                driver_name: "Max Verstappen".to_string(),
                team_name: "Red Bull Racing".to_string(),
                team_color: Some("#3671C6".to_string()),
                x: 50.0,
                y: 25.0,
            }],
        };

        terminal
            .draw(|f| {
                draw_track_map(f, f.area(), &map_data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Monza"));
        assert!(content.contains("VER"));
    }

    #[test]
    fn test_draw_driver_legend() {
        let backend = TestBackend::new(40, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let drivers = vec![
            TrackDriver {
                position: 1,
                driver_number: Some(1),
                driver_code: Some("VER".to_string()),
                driver_name: "Max Verstappen".to_string(),
                team_name: "Red Bull Racing".to_string(),
                team_color: Some("#3671C6".to_string()),
                x: 0.0,
                y: 0.0,
            },
            TrackDriver {
                position: 2,
                driver_number: Some(44),
                driver_code: Some("HAM".to_string()),
                driver_name: "Lewis Hamilton".to_string(),
                team_name: "Ferrari".to_string(),
                team_color: Some("#E8002D".to_string()),
                x: 10.0,
                y: 10.0,
            },
        ];

        terminal
            .draw(|f| {
                draw_driver_legend(f, f.area(), &drivers);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Drivers"));
        assert!(content.contains("P1"));
        assert!(content.contains("# 1"));
        assert!(content.contains("VER"));
        assert!(content.contains("P2"));
        assert!(content.contains("#44"));
        assert!(content.contains("HAM"));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(
            parse_hex_color(Some("#3671C6")),
            Some(Color::Rgb(0x36, 0x71, 0xC6))
        );
        assert_eq!(
            parse_hex_color(Some("E8002D")),
            Some(Color::Rgb(0xE8, 0x00, 0x2D))
        );
        assert_eq!(parse_hex_color(Some("invalid")), None);
        assert_eq!(parse_hex_color(None), None);
    }

    #[test]
    fn test_parse_multiviewer_circuit_coordinates_format() {
        let json = r#"{
            "circuit_key": 63,
            "circuit_name": "Bahrain International Circuit",
            "x": [0.0, 100.5, 200.0, 150.0],
            "y": [0.0, 50.2, 100.0, -25.0],
            "rotation": 0.0
        }"#;

        let points = parse_multiviewer_circuit_json(json).expect("parse coordinates format");
        assert_eq!(points.len(), 4);
        assert_eq!(points[0], TrackPoint { x: 0.0, y: 0.0 });
        assert_eq!(points[1], TrackPoint { x: 100.5, y: 50.2 });
        assert_eq!(points[2], TrackPoint { x: 200.0, y: 100.0 });
        assert_eq!(points[3], TrackPoint { x: 150.0, y: -25.0 });
    }

    #[test]
    fn test_parse_multiviewer_circuit_with_rotation() {
        let json = r#"{
            "circuit_key": 63,
            "x": [10.0, 0.0],
            "y": [0.0, 10.0],
            "rotation": 90.0
        }"#;

        let points = parse_multiviewer_circuit_json(json).expect("parse rotated coordinates");
        assert_eq!(points.len(), 2);
        // (10, 0) rotated 90 deg -> (0, 10)
        assert!((points[0].x - 0.0).abs() < 1e-4);
        assert!((points[0].y - 10.0).abs() < 1e-4);
        // (0, 10) rotated 90 deg -> (-10, 0)
        assert!((points[1].x - -10.0).abs() < 1e-4);
        assert!((points[1].y - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_parse_multiviewer_circuit_points_list_format() {
        let json = r#"[
            {"x": 10.0, "y": 20.0},
            {"x": 30.0, "y": 40.0}
        ]"#;

        let points = parse_multiviewer_circuit_json(json).expect("parse points list");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], TrackPoint { x: 10.0, y: 20.0 });
        assert_eq!(points[1], TrackPoint { x: 30.0, y: 40.0 });
    }

    #[test]
    fn test_parse_multiviewer_circuit_points_object_format() {
        let json = r#"{
            "points": [
                {"x": -5.0, "y": 15.0},
                {"x": 25.0, "y": -35.0}
            ]
        }"#;

        let points = parse_multiviewer_circuit_json(json).expect("parse points object");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], TrackPoint { x: -5.0, y: 15.0 });
        assert_eq!(points[1], TrackPoint { x: 25.0, y: -35.0 });
    }

    #[test]
    fn test_parse_multiviewer_circuit_invalid_json() {
        assert!(parse_multiviewer_circuit_json("invalid json").is_err());
    }
}
