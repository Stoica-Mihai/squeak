//! Frame layout: sidebar | content, with a one-line footer (keybinds + status).
//! Overview is wired (M1); other sections are placeholders pending M2+.

mod overview;
mod sidebar;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, Screen, StatusLevel};

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

    sidebar::render(f, cols[0], app);
    render_content(f, cols[1], app);
    render_footer(f, rows[1], app);
}

fn render_content(f: &mut Frame, area: Rect, app: &App) {
    match app.screen() {
        Screen::Overview => overview::render(f, area, app),
        other => render_placeholder(f, area, app, other),
    }
}

fn render_placeholder(f: &mut Frame, area: Rect, app: &App, screen: Screen) {
    let th = app.theme();
    let body = Paragraph::new(Line::styled(
        "not yet wired — coming in a later milestone",
        Style::default().fg(th.dim),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(th.border))
            .title(format!(" {} ", screen.title())),
    )
    .style(Style::default().fg(th.fg).bg(th.bg));
    f.render_widget(body, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let th = app.theme();
    let status_color = match app.status.level {
        StatusLevel::Info => th.dim,
        StatusLevel::Ok => th.ok,
        StatusLevel::Err => th.err,
    };

    let key = |k: &'static str| {
        Span::styled(k, Style::default().fg(th.accent).add_modifier(Modifier::BOLD))
    };
    let lbl = |t: &'static str| Span::styled(t, Style::default().fg(th.dim));

    let line = Line::from(vec![
        key(" ↑↓ "),
        lbl("nav  "),
        key("r "),
        lbl("refresh  "),
        key("t "),
        lbl("theme  "),
        key("q "),
        lbl("quit   "),
        Span::styled(app.status.text.clone(), Style::default().fg(status_color)),
    ]);
    f.render_widget(Paragraph::new(line).style(Style::default().bg(th.bg)), area);
}
