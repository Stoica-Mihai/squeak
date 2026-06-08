//! Sensor screen: mixed toggle/value rows. ↑↓ pick, ←→ change, space toggles,
//! ↵ applies (write + verify). Selected row shows its adjuster / options.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::{App, SensorRow, StatusLevel};
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    if app.settings.is_none() {
        f.render_widget(
            Paragraph::new(Line::styled("connecting…", Style::default().fg(th.dim))),
            area,
        );
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    f.render_widget(Paragraph::new(Line::from("")), rows[0]);

    let items: Vec<ListItem> = SensorRow::ALL
        .iter()
        .enumerate()
        .map(|(i, row)| ListItem::new(row_line(app, *row, i == app.sensor_cursor, th)))
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(th.sel_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.sensor_cursor));
    f.render_stateful_widget(list, rows[1], &mut state);

    f.render_widget(Paragraph::new(footer(app)), rows[2]);
}

fn row_line(app: &App, row: SensorRow, selected: bool, th: Theme) -> Line<'static> {
    let e = &app.sensor_edit;
    let label = Span::styled(format!("{:<18}", row.label()), Style::default().fg(th.dim));
    let val = |s: String| Span::styled(s, Style::default().fg(th.fg));
    let opts = |s: &'static str| Span::styled(s.to_string(), Style::default().fg(th.dim));
    let on = || Span::styled("● on".to_string(), Style::default().fg(th.ok));
    let off = || Span::styled("○ off".to_string(), Style::default().fg(th.dim));
    let adj = |v: String| Span::styled(format!("   ◄ {v} ►"), Style::default().fg(th.accent));

    let mut spans = vec![label];
    match row {
        SensorRow::Lod => {
            spans.push(val(format!("{} mm", e.lod)));
            if selected {
                spans.push(adj(e.lod.to_string()));
            }
        }
        SensorRow::ScrollDir => {
            spans.push(val(if e.scroll_dir == 1 { "inverted".into() } else { "normal".into() }));
            spans.push(opts("  [ normal | inverted ]"));
        }
        SensorRow::Motion => spans.push(if e.motion == 1 { on() } else { off() }),
        SensorRow::Angle => {
            spans.push(if e.angle_on { on() } else { off() });
            spans.push(Span::styled(format!("   {}°", e.angle_deg), Style::default().fg(th.dim)));
        }
        SensorRow::Ripple => spans.push(if e.wave == 1 { on() } else { off() }),
        SensorRow::Sampling => {
            spans.push(val(if e.fps20k == 1 { "Competitive".into() } else { "Standard".into() }));
            spans.push(opts("  [ Std | Competitive ]"));
        }
        SensorRow::Debounce => {
            spans.push(val(format!("{} ms", e.debounce)));
            if selected {
                spans.push(adj(e.debounce.to_string()));
            }
        }
        SensorRow::Sleep => {
            spans.push(val(format!("{} s", e.sleep)));
            if selected {
                spans.push(adj(e.sleep.to_string()));
            }
        }
    }
    Line::from(spans)
}

fn footer(app: &App) -> Line<'static> {
    let th = app.theme();
    match app.status.level {
        StatusLevel::Ok => Line::styled(format!("  {}", app.status.text), Style::default().fg(th.ok)),
        StatusLevel::Err => Line::styled(format!("  {}", app.status.text), Style::default().fg(th.err)),
        StatusLevel::Info => Line::styled(
            "  space toggles · ←→ change · X = factory reset",
            Style::default().fg(th.dim),
        ),
    }
}
