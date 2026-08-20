use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

/// Draw the favorite/unfavorite confirmation dialog popup.
pub fn draw(frame: &mut Frame, app: &App) {
    let series_id = match &app.pending_favorite_toggle {
        Some(id) => id,
        None => return,
    };

    let series_name = app
        .series_registry
        .get(series_id)
        .map(|s| s.name.as_str())
        .unwrap_or(series_id.as_str());

    let is_fav = app.config.favorites.contains(series_id);

    let area = frame.area();
    let popup_width = 54.min(area.width);
    let popup_height = 8.min(area.height);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let action_verb = if is_fav { "remove" } else { "add" };
    let preposition = if is_fav { "from" } else { "to" };
    let title = if is_fav {
        " Remove Favorite "
    } else {
        " Add Favorite "
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(" Are you sure you want to "),
            Span::styled(
                action_verb,
                Style::default()
                    .bold()
                    .fg(if is_fav { Color::Red } else { Color::Green }),
            ),
            Span::styled(
                format!(" {}", series_name),
                Style::default().bold().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![Span::raw(format!(" {} your favorites?", preposition))]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y/Enter] Yes", Style::default().bold().fg(Color::Green)),
            Span::raw("     "),
            Span::styled(" [n/Esc] No", Style::default().bold().fg(Color::Red)),
        ]),
    ];

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}
