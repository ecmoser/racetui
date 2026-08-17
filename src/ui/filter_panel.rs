use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::app::{App, FilterItem};

/// Draw the filter panel overlay.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let panel_width = (area.width * 35 / 100).max(32).min(area.width);
    let panel_area = Rect::new(0, 0, panel_width, area.height);

    // Clear background behind the panel
    frame.render_widget(Clear, panel_area);

    let items = app.filter_items();
    let list_items: Vec<ListItem> = items
        .iter()
        .map(|item| match item {
            FilterItem::Header(title) => {
                let style = Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                ListItem::new(Line::from(vec![
                    Span::styled(*title, style),
                ]))
            }
            FilterItem::Entry { label, option } => {
                let is_active = app.is_filter_option_active(option);
                let (prefix, text_style) = if is_active {
                    (" ● ", Style::default().fg(Color::Cyan).bold())
                } else {
                    (" ○ ", Style::default().fg(Color::White))
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, text_style),
                    Span::styled(label.as_str(), text_style),
                ]))
            }
        })
        .collect();

    let block = Block::default()
        .title(" Filters (Space: Toggle, a: Reset, Enter/Esc: Close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(list_items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        );

    let mut state = app.filter_list_state.clone();
    frame.render_stateful_widget(list, panel_area, &mut state);
}
