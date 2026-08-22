use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

/// Draw the help popup showing all keybindings.
pub fn draw(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 70 / 100).max(64).min(area.width);
    let popup_height = (area.height * 94 / 100).max(26).min(area.height);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();

    // ── Views ──
    lines.push(Line::from(vec![Span::styled(
        " ── Views ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   1                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "List View (All sessions, grouped by date)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   2                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Calendar View (Monthly grid with race days)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   3                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Live Timing & Track Map (Live leaderboard & circuit view)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   4                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Standings View (Driver & Constructor championship tables)",
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Navigation ──
    lines.push(Line::from(vec![Span::styled(
        " ── Navigation ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   j / ↓             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Move down / Next week (calendar) / Next driver (live)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   k / ↑             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Move up / Prev week (calendar) / Prev driver (live)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   h / ←             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Prev day (calendar) / Prev series (standings) / Timing tab (live)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   l / →             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Next day (calendar) / Next series (standings) / Track Map tab (live)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   H / L (calendar)  ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Previous / Next month", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   t / T (calendar)  ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Jump to today's date", Style::default().fg(Color::White)),
    ]));

    lines.push(Line::from(""));

    // ── Live Timing & Track Map ──
    lines.push(Line::from(vec![Span::styled(
        " ── Live Timing & Track Map ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   3                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Switch to Live view / Open session selector",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   h / l (← / →)     ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Switch between Timing Leaderboard and Track Map",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   j / k (↓ / ↑)     ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Navigate drivers on leaderboard or track map",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Enter             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Expand/collapse driver telemetry and tire strategy",
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Event Detail & Results ──
    lines.push(Line::from(vec![Span::styled(
        " ── Event Details & Results ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Tab / h, l        ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Switch detail tabs (Race Results, Qualifying, Sprint, Schedule)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   1 - 5             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Select detail tab directly by number",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   l                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Launch Live Timing session from event details (when active)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   o / O             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Open official stream link in default browser",
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Actions & Filters ──
    lines.push(Line::from(vec![Span::styled(
        " ── Actions & Search ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Enter             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Open event details / Select day event popup",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   f                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Toggle series favorite (with confirmation dialog)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   F                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Open/close filter sidebar",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   /                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Search events (list view) / Search series (standings view)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   r                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Refresh data (re-fetch calendars, results, and standings)",
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Background Daemon & Notifications ──
    lines.push(Line::from(vec![Span::styled(
        " ── Background Daemon & CLI ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   racetui --daemon  ",
            Style::default().bold().fg(Color::Green),
        ),
        Span::styled(
            "Run in background for desktop notifications",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   racetui daemon    ",
            Style::default().bold().fg(Color::Green),
        ),
        Span::styled(
            "Manage daemon: start | stop | status | install-service",
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));

    // ── General ──
    lines.push(Line::from(vec![Span::styled(
        " ── General ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   ?                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Toggle this help popup", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Esc               ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Close popup / Cancel active search",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   q                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Quit (or close open modal popup)",
            Style::default().fg(Color::White),
        ),
    ]));

    let block = Block::default()
        .title(" Keybindings & Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, popup_area);
}
