//! Frame layout: sidebar | content, with a one-line footer (keybinds + status).
//! Overview is wired (M1); other sections are placeholders pending M2+.

mod buttons;
mod dpi;
mod macros;
mod overview;
mod polling;
mod sensor;
mod sidebar;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::app::{App, Focus, Modal, PICK_TYPES, PickKind, PickerCol, Screen, StatusLevel};
use crate::proto::buttons::MOUSE_ACTIONS;

pub fn render(f: &mut Frame, app: &App) {
    let th = app.theme();
    f.render_widget(Block::default().style(Style::default().bg(th.bg)), f.area());

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(0)])
        .split(rows[0]);

    sidebar::render(f, cols[0], app, app.focus == Focus::Sidebar);
    render_content(f, cols[1], app);
    render_footer(f, rows[1], app);

    if let Some(modal) = &app.modal {
        render_modal(f, app, modal);
    }
}

fn render_content(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let focused = app.focus == Focus::Content;
    let border = if focused { th.accent } else { th.border };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(format!(" {} ", app.screen().title()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    match app.screen() {
        Screen::Overview => overview::render(f, inner, app),
        Screen::Dpi => dpi::render(f, inner, app),
        Screen::Polling => polling::render(f, inner, app),
        Screen::Sensor => sensor::render(f, inner, app),
        Screen::Buttons => buttons::render(f, inner, app),
        Screen::Macros => macros::render(f, inner, app),
        _ => f.render_widget(
            Paragraph::new(Line::styled(
                "not yet wired — coming in a later milestone",
                Style::default().fg(th.dim),
            )),
            inner,
        ),
    }
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let status_color = match app.status.level {
        StatusLevel::Info => th.dim,
        StatusLevel::Ok => th.ok,
        StatusLevel::Err => th.err,
    };

    let key = |k: &str| {
        Span::styled(
            k.to_string(),
            Style::default().fg(th.accent).add_modifier(Modifier::BOLD),
        )
    };
    let lbl = |t: &str| Span::styled(t.to_string(), Style::default().fg(th.dim));

    let mut spans = Vec::new();
    match app.focus {
        Focus::Sidebar => {
            spans.push(key(" ↑↓ "));
            spans.push(lbl("section  "));
            if app.screen().interactive() {
                spans.push(key("→ "));
                spans.push(lbl("edit  "));
            }
        }
        Focus::Content => {
            match app.screen() {
                Screen::Dpi => {
                    spans.push(key(" ↑↓ "));
                    spans.push(lbl("row  "));
                    spans.push(key("←→ "));
                    spans.push(lbl("±50 (⇧±500)  "));
                    spans.push(key("↵ "));
                    spans.push(lbl("apply  "));
                }
                Screen::Polling => {
                    spans.push(key(" ↑↓ "));
                    spans.push(lbl("pick  "));
                    spans.push(key("↵ "));
                    spans.push(lbl("apply  "));
                }
                Screen::Sensor => {
                    spans.push(key(" ↑↓ "));
                    spans.push(lbl("row  "));
                    spans.push(key("←→ "));
                    spans.push(lbl("change  "));
                    spans.push(key("␣ "));
                    spans.push(lbl("toggle  "));
                    spans.push(key("↵ "));
                    spans.push(lbl("apply  "));
                }
                Screen::Buttons => {
                    spans.push(key(" ↑↓ "));
                    spans.push(lbl("button  "));
                    spans.push(key("↵ "));
                    spans.push(lbl("remap  "));
                    spans.push(key("d "));
                    spans.push(lbl("default  "));
                    spans.push(key("x "));
                    spans.push(lbl("disable  "));
                    spans.push(key("m "));
                    spans.push(lbl("macro  "));
                }
                Screen::Macros => {
                    spans.push(key(" ↑↓ "));
                    spans.push(lbl("click  "));
                    spans.push(key("+ "));
                    spans.push(lbl("add  "));
                    spans.push(key("i "));
                    spans.push(lbl("text  "));
                    spans.push(key("↵ "));
                    spans.push(lbl("upload  "));
                }
                _ => {}
            }
            spans.push(key("⇥ "));
            spans.push(lbl("back  "));
        }
    }
    spans.push(key("r "));
    spans.push(lbl("refresh  "));
    spans.push(key("t "));
    spans.push(lbl("theme  "));
    spans.push(key("X "));
    spans.push(lbl("reset  "));
    spans.push(key("q "));
    spans.push(lbl("quit   "));
    spans.push(Span::styled(
        app.status.text.clone(),
        Style::default().fg(status_color),
    ));

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(th.bg)),
        area,
    );
}

fn render_modal(f: &mut Frame, app: &App, modal: &Modal) {
    let th = app.theme();
    match modal {
        Modal::ConfirmReset => {
            let area = centered(f.area(), 56, 8);
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(th.err).add_modifier(Modifier::BOLD))
                .title(" Factory reset ")
                .style(Style::default().bg(th.bg));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let lines = vec![
                Line::from(""),
                Line::styled(
                    "  Reset ALL settings to factory defaults?",
                    Style::default().fg(th.fg).add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    "  DPI, polling, sensor, buttons and macros.",
                    Style::default().fg(th.dim),
                ),
                Line::from(""),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled("y", Style::default().fg(th.err).add_modifier(Modifier::BOLD)),
                    Span::styled(" confirm     ", Style::default().fg(th.dim)),
                    Span::styled("n/Esc", Style::default().fg(th.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(" cancel", Style::default().fg(th.dim)),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        Modal::ButtonPicker(p) => {
            let area = centered(f.area(), 52, 14);
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(th.accent))
                .title(format!(" Assign button {} ", p.id))
                .style(Style::default().bg(th.bg));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let cols = Layout::horizontal([Constraint::Length(14), Constraint::Min(0)]).split(inner);

            // Type column
            let mut types = vec![Line::styled("Type", Style::default().fg(th.dim))];
            for (i, (name, _)) in PICK_TYPES.iter().enumerate() {
                let on = p.col == PickerCol::Type && i == p.type_idx;
                types.push(row_line(name, on, th));
            }
            f.render_widget(Paragraph::new(types), cols[0]);

            // Value column (mouse actions when the Mouse type is selected)
            let mut values = vec![Line::styled("Action", Style::default().fg(th.dim))];
            if PICK_TYPES[p.type_idx].1 == PickKind::Mouse {
                for (i, (name, _)) in MOUSE_ACTIONS.iter().enumerate() {
                    let on = p.col == PickerCol::Value && i == p.value_idx;
                    values.push(row_line(name, on, th));
                }
            } else {
                values.push(Line::styled("  ↵ to apply", Style::default().fg(th.dim)));
            }
            f.render_widget(Paragraph::new(values), cols[1]);

            f.render_widget(
                Paragraph::new(Line::styled(
                    " ↑↓ pick · → values · ↵ assign · esc cancel",
                    Style::default().fg(th.dim),
                )),
                Rect { x: inner.x, y: inner.bottom().saturating_sub(1), width: inner.width, height: 1 },
            );
        }
        Modal::MacroText => {
            let area = centered(f.area(), 52, 7);
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(th.accent))
                .title(" Macro text ")
                .style(Style::default().bg(th.bg));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let lines = vec![
                Line::styled("  type a string (a–z 0–9 space - =):", Style::default().fg(th.dim)),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{}_", app.text_buf), Style::default().fg(th.fg)),
                ]),
                Line::from(""),
                Line::styled("  ↵ upload · esc cancel", Style::default().fg(th.dim)),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
    }
}

/// One selectable row in the picker.
fn row_line(label: &str, selected: bool, th: crate::theme::Theme) -> Line<'static> {
    let marker = if selected { "▸ " } else { "  " };
    let style = if selected {
        Style::default().fg(th.sel_fg).bg(th.sel_bg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(th.fg)
    };
    Line::styled(format!("{marker}{label}"), style)
}

/// Center a `w`×`h` rect within `area`.
fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let v = Layout::vertical([Constraint::Length(h)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .split(v[0])[0]
}
