//! Frame layout: sidebar | content, with a one-line footer (keybinds + status).
//! Per-section content widgets land here in M1+; M0 draws a placeholder pane.

mod sidebar;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, StatusLevel};

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
    let th = app.theme();
    let title = format!(" {} ", app.screen().title());
    let body = Paragraph::new(Line::styled(
        "not yet wired — M0 skeleton",
        Style::default().fg(th.dim),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(th.border))
            .title(title),
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

    let line = Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(th.accent).add_modifier(Modifier::BOLD)),
        Span::styled("nav  ", Style::default().fg(th.dim)),
        Span::styled("r ", Style::default().fg(th.accent).add_modifier(Modifier::BOLD)),
        Span::styled("refresh  ", Style::default().fg(th.dim)),
        Span::styled("t ", Style::default().fg(th.accent).add_modifier(Modifier::BOLD)),
        Span::styled("theme  ", Style::default().fg(th.dim)),
        Span::styled("q ", Style::default().fg(th.accent).add_modifier(Modifier::BOLD)),
        Span::styled("quit   ", Style::default().fg(th.dim)),
        Span::styled(&app.status.text, Style::default().fg(status_color)),
    ]);
    f.render_widget(Paragraph::new(line).style(Style::default().bg(th.bg)), area);
}
