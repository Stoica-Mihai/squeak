//! Profiles screen (read-only): the 5 DPI profile slots with the active one
//! marked. Activation uses an unverified 0x45 frame, so it's not wired yet.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
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
    let active = s.dpi.active_levels[0] as usize;

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  current  ", Style::default().fg(th.dim)),
            Span::styled(
                format!("profile {} of {}", s.profile.current, s.profile.count),
                Style::default().fg(th.fg).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];
    for (i, dpi) in s.dpi.presets.iter().enumerate() {
        let is_active = i == active;
        let mark = if is_active { "●" } else { " " };
        let style = if is_active {
            Style::default().fg(th.ok).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(th.fg)
        };
        let tail = if is_active { "  active" } else { "" };
        lines.push(Line::styled(format!("  {mark} slot {}   {dpi:>5} dpi{tail}", i + 1), style));
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  read-only — activation not yet wired (0x45 frame unverified)",
        Style::default().fg(th.dim),
    ));
    f.render_widget(Paragraph::new(lines), area);
}
