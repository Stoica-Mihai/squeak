//! Overview screen: battery gauge + a live summary of the device snapshot.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

use crate::app::{App, Conn};
use crate::proto::polling;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(th.border))
        .title(" Overview ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(s) = &app.settings else {
        render_no_data(f, inner, app);
        return;
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let pct = s.battery.percent.min(100);
    let charge = if s.battery.charging { " ⚡charging" } else { "" };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(th.accent).bg(th.sel_bg))
        .ratio(pct as f64 / 100.0)
        .label(format!("battery {pct}%{charge}"));
    f.render_widget(gauge, rows[0]);

    let hz = polling::hz_from_code(s.polling.levels[0])
        .map(|h| format!("{h} Hz"))
        .unwrap_or_else(|| format!("code {}", s.polling.levels[0]));

    let mut lines = vec![Line::from("")];
    if let Conn::Up { name, variant } = &app.conn {
        lines.push(kv(app, "Device", format!("{name} ({})", variant.label())));
    }
    lines.extend([
        kv(app, "Profile", format!("{} of {}", s.profile.current, s.profile.count)),
        kv(app, "DPI presets", format!("{:?}", s.dpi.presets)),
        kv(app, "DPI active", format!("levels {:?}", s.dpi.active_levels)),
        kv(app, "Polling", hz),
        kv(app, "LOD", s.sensor.lod.to_string()),
        kv(app, "Scroll dir", invert(s.sensor.scroll_dir)),
        kv(app, "Motion sync", on_off(s.sensor.motion_sync)),
        kv(app, "Angle snap", if s.sensor.angle == 0 { "off".into() } else { format!("{}°", s.sensor.angle) }),
        kv(app, "Sampling", if s.sensor.fps20k == 1 { "Competitive (≥20K)".into() } else { "Standard".into() }),
        kv(app, "Debounce", format!("{} ms", s.debounce.value)),
        kv(app, "Sleep", format!("{} s", s.sleep_s)),
    ]);
    f.render_widget(Paragraph::new(lines), rows[1]);
}

fn kv(app: &App, key: &str, value: String) -> Line<'static> {
    let th = app.theme();
    Line::from(vec![
        Span::styled(format!("  {key:<14}"), Style::default().fg(th.dim)),
        Span::styled(value, Style::default().fg(th.fg)),
    ])
}

fn on_off(v: u8) -> String {
    if v == 1 { "on".into() } else { "off".into() }
}

fn invert(v: u8) -> String {
    if v == 1 { "inverted".into() } else { "normal".into() }
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
