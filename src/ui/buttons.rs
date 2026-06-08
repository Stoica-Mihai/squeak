//! Buttons screen: a table of button id → type → assignment. ↑↓ pick a row,
//! ↵ opens the action picker, d restores default, x disables.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::App;
use crate::proto::buttons::{is_present, type_name};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    if app.buttons.is_empty() {
        f.render_widget(
            Paragraph::new(Line::styled("loading buttons…", Style::default().fg(th.dim))),
            area,
        );
        return;
    }

    let mut lines = vec![Line::styled(
        "  id   type         assignment",
        Style::default().fg(th.dim),
    )];
    for (i, b) in app.buttons.iter().enumerate() {
        let selected = i == app.button_cursor;
        let cursor = if selected { "▸" } else { " " };
        let text = format!(
            " {cursor} {id:>2}   {ty:<11}  {label}",
            id = b.id,
            ty = type_name(b.type_id),
            label = b.label,
        );
        let style = if selected {
            Style::default().fg(th.sel_fg).bg(th.sel_bg).add_modifier(Modifier::BOLD)
        } else if is_present(b) {
            Style::default().fg(th.fg)
        } else {
            Style::default().fg(th.dim) // empty / non-physical slot
        };
        lines.push(Line::styled(text, style));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ↵", Style::default().fg(th.accent)),
        Span::styled(" remap   ", Style::default().fg(th.dim)),
        Span::styled("d", Style::default().fg(th.accent)),
        Span::styled(" default   ", Style::default().fg(th.dim)),
        Span::styled("x", Style::default().fg(th.accent)),
        Span::styled(" disable   ", Style::default().fg(th.dim)),
        Span::styled("m", Style::default().fg(th.accent)),
        Span::styled(" macro", Style::default().fg(th.dim)),
    ]));
    f.render_widget(Paragraph::new(lines), area);
}
