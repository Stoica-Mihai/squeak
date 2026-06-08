//! Macros screen: build a click sequence for the target button (chosen with `m`
//! on the Buttons screen), or `i` for a text macro. ↵ uploads (auto-chunks).

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::App;
use crate::proto::macros::MOUSE_PALETTE;

fn name_for(code: u8) -> &'static str {
    MOUSE_PALETTE.iter().find(|(_, c)| *c == code).map(|(n, _)| *n).unwrap_or("?")
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let Some(id) = app.macro_target else {
        f.render_widget(
            Paragraph::new(Line::styled(
                "Press m on a button (Buttons screen) to record a macro for it.",
                Style::default().fg(th.dim),
            )),
            area,
        );
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  target  ", Style::default().fg(th.dim)),
            Span::styled(format!("button {id}"), Style::default().fg(th.fg).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::styled("  add a mouse click:", Style::default().fg(th.dim)),
    ];

    for (i, (name, _)) in MOUSE_PALETTE.iter().enumerate() {
        let selected = i == app.macro_palette;
        let cursor = if selected { "▸" } else { " " };
        let style = if selected {
            Style::default().fg(th.sel_fg).bg(th.sel_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(th.fg)
        };
        lines.push(Line::styled(format!("   {cursor} {name}"), style));
    }

    lines.push(Line::from(""));
    let seq: Vec<&str> = app.macro_seq.iter().map(|c| name_for(*c)).collect();
    let seq_text = if seq.is_empty() { "(empty)".to_string() } else { seq.join(", ") };
    lines.push(Line::from(vec![
        Span::styled(format!("  sequence ({})  ", app.macro_seq.len()), Style::default().fg(th.dim)),
        Span::styled(seq_text, Style::default().fg(th.accent)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  +/␣ add · ⌫ remove · i text macro · ↵ upload",
        Style::default().fg(th.dim),
    ));
    f.render_widget(Paragraph::new(lines), area);
}
