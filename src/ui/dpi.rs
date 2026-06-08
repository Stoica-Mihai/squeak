//! DPI screen: five preset rows rendered as sliders. ↑↓ pick a row, ←→ adjust
//! ±50 (Shift ±500), ↵ apply (write + verify). `*` marks an unsaved edit.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::App;

/// Display-only full-scale for the bar (most use ≤ this); higher values clamp.
const VISUAL_MAX: u16 = 8000;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let Some(s) = &app.settings else {
        f.render_widget(
            Paragraph::new(Line::styled("connecting…", Style::default().fg(th.dim))),
            area,
        );
        return;
    };
    let active = s.dpi.active_levels[0] as usize;

    let bar_w = (area.width as usize).saturating_sub(30).max(4);
    let mut lines = vec![Line::from("")];
    for i in 0..app.dpi_edit.len() {
        let value = app.dpi_edit[i];
        let selected = i == app.dpi_cursor;
        let filled = (value as usize * bar_w / VISUAL_MAX as usize).min(bar_w);
        let bar: String = "█".repeat(filled);
        let rest: String = "░".repeat(bar_w - filled);

        let cursor = if selected { "▌" } else { " " };
        let row_fg = if selected { th.sel_fg } else { th.fg };
        let mark = if i == active { " ●" } else { "  " };
        let changed = if app.dpi_changed(i) {
            Span::styled(" *", Style::default().fg(th.err).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("  ")
        };

        let mut style = Style::default().fg(row_fg);
        if selected {
            style = style.add_modifier(Modifier::BOLD);
        }
        lines.push(Line::from(vec![
            Span::styled(format!("{cursor} {} ", i + 1), style),
            Span::styled(mark, Style::default().fg(th.accent)),
            Span::raw(" "),
            Span::styled(bar, Style::default().fg(th.accent)),
            Span::styled(rest, Style::default().fg(th.border)),
            Span::styled(format!(" {value:>5} dpi"), style),
            changed,
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  ● = active profile   * = unsaved edit",
        Style::default().fg(th.dim),
    ));
    f.render_widget(Paragraph::new(lines), area);
}
