//! DPI screen: five preset sliders (value · bar). ↑↓ pick, ←→ ±50 (Shift ±500),
//! ↵ apply. Active preset tagged; `*` marks an unsaved edit.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::{App, StatusLevel};
use crate::proto::dpi::DPI_MAX;

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
    // Scale the bar to the sensor's full range (device reports 0 on fw 0.1.6).
    let scale = if s.dpi.max > 0 { s.dpi.max } else { DPI_MAX };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::styled("  ←→ ±50 · ⇧←→ ±500", Style::default().fg(th.dim)),
            Line::from(""),
        ]),
        rows[0],
    );

    let bar_w = (area.width as usize).saturating_sub(24).clamp(8, 44);
    let items: Vec<ListItem> = app
        .dpi_edit
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            let filled = (value as usize * bar_w / scale as usize).min(bar_w);
            let mut spans = vec![
                Span::styled(format!("{}  {value:>5}  ", i + 1), Style::default().fg(th.fg)),
                Span::styled("█".repeat(filled), Style::default().fg(th.accent)),
                Span::styled("─".repeat(bar_w - filled), Style::default().fg(th.border)),
            ];
            if i == active {
                spans.push(Span::styled("  active", Style::default().fg(th.accent)));
            }
            if app.dpi_changed(i) {
                spans.push(Span::styled(" *", Style::default().fg(th.err).add_modifier(Modifier::BOLD)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(th.sel_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.dpi_cursor));
    f.render_stateful_widget(list, rows[1], &mut state);

    f.render_widget(Paragraph::new(footer(app)), rows[2]);
}

/// Bottom line: the last write result if any, else the legend.
fn footer(app: &App) -> Line<'static> {
    let th = app.theme();
    match app.status.level {
        StatusLevel::Ok => Line::styled(format!("  {}", app.status.text), Style::default().fg(th.ok)),
        StatusLevel::Err => Line::styled(format!("  {}", app.status.text), Style::default().fg(th.err)),
        StatusLevel::Info => {
            let max = app.settings.as_ref().map(|s| if s.dpi.max > 0 { s.dpi.max } else { DPI_MAX }).unwrap_or(DPI_MAX);
            Line::styled(format!("  range 50–{max} · step 50 · 5 presets"), Style::default().fg(th.dim))
        }
    }
}
