//! Key events -> `Action`. Section nav is Tab/BackTab; ↑↓/←→/↵ act within the
//! focused screen. `App::update` interprets each action against the current
//! screen.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::proto::dpi::DPI_STEP;

pub enum Action {
    None,
    Quit,
    CycleTheme,
    Refresh,
    NextSection,
    PrevSection,
    CursorUp,
    CursorDown,
    /// Signed value delta (DPI editing).
    Adjust(i32),
    Apply,
}

pub fn map_key(key: KeyEvent) -> Action {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let big = DPI_STEP as i32 * 10;
    let step = DPI_STEP as i32;
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Tab => Action::NextSection,
        KeyCode::BackTab => Action::PrevSection,
        KeyCode::Up | KeyCode::Char('k') => Action::CursorUp,
        KeyCode::Down | KeyCode::Char('j') => Action::CursorDown,
        KeyCode::Left | KeyCode::Char('h') => Action::Adjust(if shift { -big } else { -step }),
        KeyCode::Right | KeyCode::Char('l') => Action::Adjust(if shift { big } else { step }),
        KeyCode::Enter => Action::Apply,
        KeyCode::Char('t') => Action::CycleTheme,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::None,
    }
}
