//! Left nav: section list with the active section highlighted.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let items: Vec<ListItem> = Screen::ALL
        .iter()
        .map(|s| ListItem::new(Line::raw(format!(" {}", s.title()))))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(th.border))
                .title(" squeak "),
        )
        .style(Style::default().fg(th.fg).bg(th.bg))
        .highlight_style(
            Style::default()
                .fg(th.sel_fg)
                .bg(th.sel_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▌");

    let mut state = ListState::default();
    state.select(Some(app.screen_idx));
    f.render_stateful_widget(list, area, &mut state);
}
