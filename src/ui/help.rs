use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

/// Draw the help popup showing all keybindings.
pub fn draw(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 60 / 100).max(56).min(area.width);
    let popup_height = (area.height * 90 / 100).max(24).min(area.height);
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
            "List View (All sessions)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   2                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Calendar View (Monthly grid)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   3                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Live Timing View", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   4                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Standings View (Driver & Constructor)",
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
            "Move down / Next week (calendar)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   k / ↑             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Move up / Prev week (calendar)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   h / ←             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Prev day (calendar) / Prev series (standings)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   l / →             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Next day (calendar) / Next series (standings)",
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
        Span::styled("Jump to today", Style::default().fg(Color::White)),
    ]));

    lines.push(Line::from(""));

    // ── Live Timing View ──
    lines.push(Line::from(vec![Span::styled(
        " ── Live Timing View ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   3                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Switch to Live view / Open session picker",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   h / l (← / →)     ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Switch sub-tab (Timing / Track Map)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   j / k (↓ / ↑)     ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Navigate driver leaderboard rows",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Enter             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Expand/collapse driver telemetry & strategy",
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Detail & Standings ──
    lines.push(Line::from(vec![Span::styled(
        " ── Detail View & Popups ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Tab / h, l        ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Switch detail tabs (Race, Qual, Sprint, Sched)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   1 - 5             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Select detail tab directly",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   o / O             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Open stream link in default browser",
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Actions ──
    lines.push(Line::from(vec![Span::styled(
        " ── Actions ──",
        Style::default().bold().fg(Color::Cyan),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Enter             ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Open event details / Select day event",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   f                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Toggle series favorite (with confirmation)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   F                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Toggle filter panel", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   /                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Search events (list) / Search series (standings)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   r                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Refresh data & reload schedules/standings",
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
            "Close popup / Cancel search",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   q                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Quit (or close popup if open)",
            Style::default().fg(Color::White),
        ),
    ]));

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, popup_area);
}
