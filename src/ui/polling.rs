//! Polling screen: single-select list of supported rates. ↑↓ pick, ↵ apply.
//! `●` marks the rate currently set on the device.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::App;
use crate::proto::polling::RATES_HZ;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let active_code = app.settings.as_ref().map(|s| s.polling.levels[0] as usize);

    let items: Vec<ListItem> = RATES_HZ
        .iter()
        .enumerate()
        .map(|(i, hz)| {
            let mark = if Some(i) == active_code { " ● " } else { "   " };
            ListItem::new(Line::raw(format!("{mark}{hz} Hz")))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(th.border))
                .title(" Polling rate "),
        )
        .style(Style::default().fg(th.fg))
        .highlight_style(
            Style::default()
                .fg(th.sel_fg)
                .bg(th.sel_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▌");

    let mut state = ListState::default();
    state.select(Some(app.poll_sel));
    f.render_stateful_widget(list, area, &mut state);
}
