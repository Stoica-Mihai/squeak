//! Profiles screen: the device's hardware profiles. ↑↓ pick, ↵ activate.
//! `●` marks the profile currently active on the device.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let Some(s) = &app.settings else {
        f.render_widget(
            Paragraph::new(Line::styled("connecting…", Style::default().fg(th.dim))),
            area,
        );
        return;
    };
    let current = s.profile.current as usize;
    let count = app.profile_count();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::styled("  ↵ activates a profile (full button/DPI/sensor set)", Style::default().fg(th.dim)),
            Line::from(""),
        ]),
        rows[0],
    );

    let items: Vec<ListItem> = (0..count)
        .map(|i| {
            let mark = if i == current { "●" } else { "○" };
            let active = if i == current { "   (active)" } else { "" };
            ListItem::new(Line::raw(format!("{mark} Profile {}{active}", i + 1)))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().fg(th.fg))
        .highlight_style(Style::default().bg(th.sel_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.profile_cursor.min(count.saturating_sub(1))));
    f.render_stateful_widget(list, rows[1], &mut state);

    f.render_widget(
        Paragraph::new(Line::styled(
            "  switching reloads all settings from the new profile",
            Style::default().fg(th.dim),
        )),
        rows[2],
    );
}
