use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

/// Draw the help popup showing all keybindings.
pub fn draw(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 55 / 100).max(52).min(area.width);
    let popup_height = (area.height * 85 / 100).max(22).min(area.height);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();

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
    lines.push(Line::from(vec![
        Span::styled(
            "   1 / 2 / 3 / 4     ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Switch view (1:List, 2:Calendar, 3:Live, 4:Standings)",
            Style::default().fg(Color::White),
        ),
    ]));

    // Separator
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
        Span::styled("Show event details", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   f                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Toggle favorite", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   F                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Open filter panel", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   /                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Search events / series (standings)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   o                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Open stream link (in detail view)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   1-9               ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled(
            "Open Nth stream link (in detail view)",
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   r                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Refresh all data", Style::default().fg(Color::White)),
    ]));

    // Separator
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
        Span::styled("Toggle this help", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   Esc               ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Close popup / cancel", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "   q                 ",
            Style::default().bold().fg(Color::Yellow),
        ),
        Span::styled("Quit", Style::default().fg(Color::White)),
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
