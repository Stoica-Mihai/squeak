//! Sensor screen: mixed toggle/value rows. ↑↓ pick a row, ←→ change, space
//! toggles a boolean, ↵ applies that row (write + verify).

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::{App, SensorRow};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    if app.settings.is_none() {
        f.render_widget(
            Paragraph::new(Line::styled("connecting…", Style::default().fg(th.dim))),
            area,
        );
        return;
    }
    let e = &app.sensor_edit;

    let mut lines = vec![Line::from("")];
    for (i, row) in SensorRow::ALL.iter().enumerate() {
        let selected = i == app.sensor_cursor;
        let (value, color) = match row {
            SensorRow::Lod => (format!("{} mm", e.lod), th.fg),
            SensorRow::ScrollDir => (
                if e.scroll_dir == 1 { "inverted".into() } else { "normal".into() },
                th.fg,
            ),
            SensorRow::Motion => on_off(e.motion == 1, th),
            SensorRow::Angle => {
                if e.angle_on {
                    (format!("on  {}°", e.angle_deg), th.ok)
                } else {
                    ("off".into(), th.dim)
                }
            }
            SensorRow::Ripple => on_off(e.wave == 1, th),
            SensorRow::Sampling => (
                if e.fps20k == 1 { "Competitive".into() } else { "Standard".into() },
                th.fg,
            ),
            SensorRow::Debounce => (format!("{} ms", e.debounce), th.fg),
            SensorRow::Sleep => (format!("{} s", e.sleep), th.fg),
        };

        let cursor = if selected { "▸" } else { " " };
        let label = row.label();
        let (label_style, value_style) = if selected {
            let s = Style::default().fg(th.sel_fg).bg(th.sel_bg).add_modifier(Modifier::BOLD);
            (s, s)
        } else {
            (Style::default().fg(th.dim), Style::default().fg(color))
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {cursor} {label:<20}", label = label), label_style),
            Span::styled(value, value_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  space toggles · ←→ changes · X = factory reset",
        Style::default().fg(th.dim),
    ));
    f.render_widget(Paragraph::new(lines), area);
}

fn on_off(on: bool, th: crate::theme::Theme) -> (String, Color) {
    if on {
        ("● on".into(), th.ok)
    } else {
        ("○ off".into(), th.dim)
    }
}
