//! Left nav: section list with the active section highlighted. The border and
//! selection brighten when the sidebar holds focus.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, area: Rect, app: &App, focused: bool) {
    let th = app.theme();
    let border = if focused { th.accent } else { th.border };
    let items: Vec<ListItem> = Screen::ALL
        .iter()
        .map(|s| ListItem::new(Line::raw(format!(" {}", s.title()))))
        .collect();

    let mut sel = Style::default().fg(th.sel_fg).bg(th.sel_bg);
    if focused {
        sel = sel.add_modifier(Modifier::BOLD);
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(" squeak "),
        )
        .style(Style::default().fg(th.fg).bg(th.bg))
        .highlight_style(sel)
        .highlight_symbol(if focused { "▌" } else { " " });

    let mut state = ListState::default();
    state.select(Some(app.screen_idx));
    f.render_stateful_widget(list, area, &mut state);
}
