//! Key events -> `Action`. Keeps the keymap in one place so `App::update` is a
//! pure reducer over `Action`.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Action {
    None,
    Quit,
    NavUp,
    NavDown,
    CycleTheme,
    Refresh,
}

pub fn map_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Up | KeyCode::Char('k') => Action::NavUp,
        KeyCode::Down | KeyCode::Char('j') => Action::NavDown,
        KeyCode::Char('t') => Action::CycleTheme,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::None,
    }
}
