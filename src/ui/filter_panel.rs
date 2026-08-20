use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::app::{App, FilterItem, FilterOption};
use crate::data::models::FetchStatus;

/// Draw the filter panel overlay.
pub fn draw(frame: &mut Frame, app: &mut App) {
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
                ListItem::new(Line::from(vec![Span::styled(*title, style)]))
            }
            FilterItem::Entry { label, option } => {
                let is_active = app.is_filter_option_active(option);
                let (prefix, text_style) = if is_active {
                    (" ● ", Style::default().fg(Color::Cyan).bold())
                } else {
                    (" ○ ", Style::default().fg(Color::White))
                };

                let is_error = match option {
                    FilterOption::Series(series_id) => {
                        matches!(app.fetch_status.get(series_id), Some(FetchStatus::Error(_)))
                    }
                    _ => false,
                };

                let mut spans = vec![
                    Span::styled(prefix, text_style),
                    Span::styled(label.as_str(), text_style),
                ];

                if is_error {
                    spans.push(Span::styled(" ⚠", Style::default().fg(Color::Red).bold()));
                }

                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    let block = Block::default()
        .title(" Filters (Space: Toggle, a: Reset, Enter/Esc: Close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(list_items).block(block).highlight_style(
        Style::default()
            .bg(Color::Rgb(40, 40, 60))
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, panel_area, &mut app.filter_list_state);
}
