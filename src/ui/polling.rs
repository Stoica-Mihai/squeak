//! Polling screen: single-select rate list. ↑↓ pick, ↵ apply. `●` marks the
//! rate currently set on the device, `○` the others.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::App;
use crate::proto::polling::RATES_HZ;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let active = app.settings.as_ref().map(|s| s.polling.levels[0] as usize);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::styled("  ↵ applies (writes & re-reads to confirm)", Style::default().fg(th.dim)),
            Line::from(""),
        ]),
        rows[0],
    );

    let items: Vec<ListItem> = RATES_HZ
        .iter()
        .enumerate()
        .map(|(i, hz)| {
            let mark = if Some(i) == active { "●" } else { "○" };
            ListItem::new(Line::raw(format!("{mark} {hz} Hz")))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().fg(th.fg))
        .highlight_style(Style::default().bg(th.sel_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.poll_sel));
    f.render_stateful_widget(list, rows[1], &mut state);

    f.render_widget(
        Paragraph::new(Line::styled("  Levels: 6", Style::default().fg(th.dim))),
        rows[2],
    );
}
