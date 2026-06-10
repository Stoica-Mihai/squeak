//! Sensor screen: mixed toggle/value rows. ↑↓ pick, ←→ change, space toggles,
//! ↵ applies (write + verify). Selected row shows its adjuster / options.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::{App, SensorRow};
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
        .map(|row| ListItem::new(row_line(app, *row, th)))
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(th.sel_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.sensor_cursor));
    f.render_stateful_widget(list, rows[1], &mut state);

    f.render_widget(Paragraph::new(footer(app)), rows[2]);
}

/// Whether the row's edited value differs from the device's current value.
fn row_changed(app: &App, row: SensorRow) -> bool {
    let Some(s) = &app.settings else { return false };
    let e = &app.sensor_edit;
    let se = &s.sensor;
    match row {
        SensorRow::Lod => e.lod != se.lod,
        SensorRow::ScrollDir => e.scroll_dir != se.scroll_dir,
        SensorRow::Motion => e.motion != se.motion_sync,
        SensorRow::Ripple => e.wave != se.wave,
        SensorRow::Sampling => e.fps20k != se.fps20k,
        SensorRow::Angle => {
            let dev_deg = se.angle.unsigned_abs().min(90) as u8;
            e.angle_on != (se.angle != 0) || (e.angle_on && e.angle_deg != dev_deg)
        }
        SensorRow::Debounce => e.debounce != s.debounce.value,
        SensorRow::Sleep => e.sleep != s.sleep_min,
    }
}

fn row_line(app: &App, row: SensorRow, th: Theme) -> Line<'static> {
    let selected = SensorRow::ALL[app.sensor_cursor] == row;
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
            let mm = lod_mm(e.lod);
            spans.push(val(mm.to_string()));
            if selected {
                spans.push(adj(mm.to_string()));
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
            spans.push(val(format!("{} min", e.sleep)));
            if selected {
                spans.push(adj(format!("{} min", e.sleep)));
            }
        }
    }
    if row_changed(app, row) {
        spans.push(Span::styled(
            "  ✎ unsaved",
            Style::default().fg(th.err).add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}

/// LOD device code (1/2/3) -> the Launcher's millimetre label.
fn lod_mm(v: u8) -> &'static str {
    match v {
        1 => "1.0 mm",
        2 => "2.0 mm",
        3 => "0.7 mm",
        _ => "? mm",
    }
}

fn footer(app: &App) -> Line<'static> {
    let th = app.theme();
    if SensorRow::ALL.iter().any(|r| row_changed(app, *r)) {
        return Line::styled(
            "  ✎ unsaved changes — ↵ to apply",
            Style::default().fg(th.err).add_modifier(Modifier::BOLD),
        );
    }
    // Write results show in the main footer bar; here just the legend.
    Line::styled(
        "  space toggles · ←→ change · X = factory reset",
        Style::default().fg(th.dim),
    )
}
