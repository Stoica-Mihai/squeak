//! Frame layout: sidebar | content, with a one-line footer (keybinds + status).
//! Overview is wired (M1); other sections are placeholders pending M2+.

mod dpi;
mod overview;
mod polling;
mod sidebar;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, Focus, Screen, StatusLevel};

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
