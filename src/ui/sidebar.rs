//! Left nav: rounded box with the section list on top and a device-status block
//! at the bottom (connection dot, name, battery). Border/selection brighten when
//! the sidebar holds focus.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::{App, Conn, Screen};

pub fn render(f: &mut Frame, area: Rect, app: &App, focused: bool) {
    let th = app.theme();
    let border = if focused { th.accent } else { th.border };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(" squeak ")
        .style(Style::default().bg(th.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    let items: Vec<ListItem> = Screen::ALL
        .iter()
        .map(|s| ListItem::new(Line::raw(format!(" {}", s.title()))))
        .collect();
    let mut sel = Style::default().fg(th.sel_fg).bg(th.sel_bg);
    if focused {
        sel = sel.add_modifier(Modifier::BOLD);
    }
    let list = List::new(items)
        .style(Style::default().fg(th.fg))
        .highlight_style(sel)
        .highlight_symbol(if focused { "▌" } else { " " });
    let mut state = ListState::default();
    state.select(Some(app.screen_idx));
    f.render_stateful_widget(list, rows[0], &mut state);

    f.render_widget(Paragraph::new(status_lines(app)), rows[1]);
}

fn status_lines(app: &App) -> Vec<Line<'static>> {
    let th = app.theme();
    let (dot, dot_color, label) = match &app.conn {
        Conn::Connecting => ("…", th.dim, "connecting".to_string()),
        Conn::Up { name, .. } => ("●", th.ok, short_name(name)),
        Conn::Down(_) => ("○", th.err, "offline".to_string()),
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(format!(" {dot} "), Style::default().fg(dot_color)),
        Span::styled(label, Style::default().fg(th.dim)),
    ])];
    let battery = match &app.settings {
        Some(s) => {
            let charge = if s.battery.charging { " ⚡" } else { "" };
            Line::from(vec![
                Span::raw("   "),
                Span::styled(
                    format!("{}%{charge}", s.battery.percent.min(100)),
                    Style::default().fg(th.ok).add_modifier(Modifier::BOLD),
                ),
            ])
        }
        None => Line::from(""),
    };
    lines.push(battery);
    lines
}

/// Trim a long product string to fit the narrow sidebar.
fn short_name(name: &str) -> String {
    name.rsplit(' ').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" ")
}
