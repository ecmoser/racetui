pub mod list_view;
pub mod status_bar;

use ratatui::prelude::*;
use crate::app::{App, ViewMode};

/// Main draw function — dispatches to the appropriate view.
pub fn draw(frame: &mut Frame, app: &App) {
    // Layout: main content area + status bar at the bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),     // Main content
            Constraint::Length(1),  // Status bar
        ])
        .split(frame.area());

    // Draw the main content based on view mode
    match app.view_mode {
        ViewMode::List => list_view::draw(frame, app, chunks[0]),
        ViewMode::Calendar => {
            // Placeholder for calendar view (Step 12)
            let placeholder = ratatui::widgets::Paragraph::new("Calendar view coming soon. Press Tab to switch back.")
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, chunks[0]);
        }
    }

    // Draw the status bar
    status_bar::draw(frame, app, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;

    #[test]
    fn test_draw_list_view() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(HashMap::new(), UserConfig::default());

        terminal.draw(|f| {
            draw(f, &app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        // Check that Race Calendar and status hints are rendered
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Race Calendar"));
        assert!(content.contains("Quit"));
    }

    #[test]
    fn test_draw_calendar_placeholder() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(HashMap::new(), UserConfig::default());
        app.view_mode = ViewMode::Calendar;

        terminal.draw(|f| {
            draw(f, &app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Calendar view coming soon"));
    }
}
