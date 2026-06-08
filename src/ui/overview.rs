//! Overview screen: a compact live summary — header (device · transport ·
//! firmware), battery bar, and grouped DPI / Polling / Sensor / Timing rows.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use crate::app::{App, Conn};
use crate::proto::polling;

const LABEL_W: usize = 9;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let Some(s) = &app.settings else {
        render_no_data(f, area, app);
        return;
    };

    let mut lines = Vec::new();

    // Header: name · transport · firmware
    if let Conn::Up { name, transport, firmware, .. } = &app.conn {
        lines.push(Line::from(vec![
            Span::styled(name.clone(), Style::default().fg(th.fg).add_modifier(Modifier::BOLD)),
            Span::styled("  ·  ", Style::default().fg(th.dim)),
            Span::styled(transport.to_string(), Style::default().fg(th.fg)),
            Span::styled("  ·  firmware ", Style::default().fg(th.dim)),
            Span::styled(firmware.clone(), Style::default().fg(th.accent)),
        ]));
    }
    lines.push(Line::from(""));

    // Battery: green bar + right-side label.
    let pct = s.battery.percent.min(100);
    let bar_w = (area.width as usize).saturating_sub(LABEL_W + 12).clamp(8, 28);
    let filled = pct as usize * bar_w / 100;
    let charge = if s.battery.charging { " ⚡" } else { "" };
    lines.push(Line::from(vec![
        label(th, "Battery"),
        Span::styled("█".repeat(filled), Style::default().fg(th.ok)),
        Span::styled("░".repeat(bar_w - filled), Style::default().fg(th.border)),
        Span::styled(
            format!(" {pct}%{charge}"),
            Style::default().fg(th.ok).add_modifier(Modifier::BOLD),
        ),
    ]));

    // DPI presets, active one highlighted.
    let active = s.dpi.active_levels[0] as usize;
    let mut dpi_spans = vec![label(th, "DPI")];
    for (i, dpi) in s.dpi.presets.iter().enumerate() {
        let style = if i == active {
            Style::default().fg(th.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(th.dim)
        };
        dpi_spans.push(Span::styled(format!("{dpi} "), style));
    }
    lines.push(Line::from(dpi_spans));

    // Polling
    let hz = polling::hz_from_code(s.polling.levels[0])
        .map(|h| format!("{h} Hz"))
        .unwrap_or_else(|| format!("code {}", s.polling.levels[0]));
    lines.push(Line::from(vec![
        label(th, "Polling"),
        Span::styled(hz, Style::default().fg(th.accent).add_modifier(Modifier::BOLD)),
    ]));

    // Sensor (grouped)
    let scroll = if s.sensor.scroll_dir == 1 { "inverted" } else { "normal" };
    let angle = if s.sensor.angle == 0 { "off".to_string() } else { format!("{}°", s.sensor.angle) };
    lines.push(Line::from(vec![
        label(th, "Sensor"),
        Span::styled(
            format!("LOD {} · {scroll} scroll · angle {angle}", s.sensor.lod),
            Style::default().fg(th.fg),
        ),
    ]));

    // Timing (grouped)
    lines.push(Line::from(vec![
        label(th, "Timing"),
        Span::styled(
            format!("debounce {} ms · sleep {} min", s.debounce.value, s.sleep_min),
            Style::default().fg(th.fg),
        ),
    ]));

    if let Some(t) = app.last_update {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!("last refreshed {}s ago", t.elapsed().as_secs()),
            Style::default().fg(th.dim),
        ));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn label(th: crate::theme::Theme, text: &str) -> Span<'static> {
    Span::styled(format!("{text:<LABEL_W$}"), Style::default().fg(th.dim))
}

fn render_no_data(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let lines: Vec<Line> = match &app.conn {
        Conn::Connecting => vec![Line::styled("connecting to device…", Style::default().fg(th.dim))],
        Conn::Up { .. } => vec![Line::styled("reading…", Style::default().fg(th.dim))],
        Conn::Down(msg) => {
            let mut lines = vec![Line::styled(
                "device unavailable",
                Style::default().fg(th.err).add_modifier(Modifier::BOLD),
            )];
            for l in msg.lines() {
                lines.push(Line::styled(l.to_string(), Style::default().fg(th.fg)));
            }
            lines
        }
    };
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}
