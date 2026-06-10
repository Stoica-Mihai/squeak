//! Buttons screen: a table of button id → name → type → assignment. ↑↓ pick a
//! row, ↵ opens the action picker, d restores default, x disables, m macro.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::App;
use squeak_core::proto::buttons::{friendly_name, is_present, type_name};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    if app.buttons.is_empty() {
        f.render_widget(
            Paragraph::new(Line::styled("loading buttons…", Style::default().fg(th.dim))),
            area,
        );
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::styled(
            "   id  button     type         assignment",
            Style::default().fg(th.dim),
        )),
        rows[0],
    );

    let items: Vec<ListItem> = app
        .buttons
        .iter()
        .map(|b| {
            let text = format!(
                "{id:>2}  {name:<9}  {ty:<11}  {label}",
                id = b.id,
                name = friendly_name(b.id).unwrap_or(""),
                ty = type_name(b.type_id),
                label = b.label,
            );
            let fg = if is_present(b) { th.fg } else { th.dim };
            ListItem::new(Line::styled(text, Style::default().fg(fg)))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(th.sel_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.button_cursor));
    f.render_stateful_widget(list, rows[1], &mut state);

    let hint = Line::from(vec![
        Span::styled("  ↵", Style::default().fg(th.accent)),
        Span::styled(" remap   ", Style::default().fg(th.dim)),
        Span::styled("d", Style::default().fg(th.accent)),
        Span::styled(" default   ", Style::default().fg(th.dim)),
        Span::styled("x", Style::default().fg(th.accent)),
        Span::styled(" disable   ", Style::default().fg(th.dim)),
        Span::styled("m", Style::default().fg(th.accent)),
        Span::styled(" macro", Style::default().fg(th.dim)),
    ]);
    f.render_widget(Paragraph::new(hint), rows[2]);
}
