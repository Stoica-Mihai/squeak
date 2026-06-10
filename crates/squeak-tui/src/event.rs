//! Key events -> `Action`. Actions are focus-agnostic; `App::update` interprets
//! them against the focused pane (sidebar sections vs. content editing).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use squeak_core::proto::dpi::DPI_STEP;

pub enum Action {
    None,
    Quit,
    CycleTheme,
    Refresh,
    /// Tab: move focus between sidebar and content.
    ToggleFocus,
    /// Esc: return focus to the sidebar.
    Back,
    /// ↑/↓ as -1/+1. Sidebar: change section. Content: move cursor.
    Vertical(i32),
    /// ←/→ as a signed step. Sidebar: enter content (→). Content: adjust value.
    Horizontal(i32),
    /// Enter. Sidebar: focus content. Content: apply edit.
    Enter,
    /// Space: toggle the focused boolean row.
    Toggle,
    /// Open the factory-reset confirm modal.
    ResetPrompt,
    /// Modal: confirm / cancel.
    Confirm,
    Cancel,
    /// Buttons screen: d restore default, x disable, m record macro.
    SetDefault,
    SetDisable,
    RecordMacro,
    /// Macros screen: + add step, Backspace/Del remove, i text input.
    Add,
    Remove,
    TextInput,
    /// Toggle the help overlay.
    Help,
    /// Opt-in: check for a firmware update (network).
    CheckUpdate,
    /// Buttons: toggle the left-click lock (protects the left button).
    ToggleLock,
}

pub fn map_key(key: KeyEvent) -> Action {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let step = if shift { DPI_STEP as i32 * 10 } else { DPI_STEP as i32 };
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Esc => Action::Back,
        KeyCode::Tab | KeyCode::BackTab => Action::ToggleFocus,
        KeyCode::Up | KeyCode::Char('k') => Action::Vertical(-1),
        KeyCode::Down | KeyCode::Char('j') => Action::Vertical(1),
        KeyCode::Left | KeyCode::Char('h') => Action::Horizontal(-step),
        KeyCode::Right | KeyCode::Char('l') => Action::Horizontal(step),
        KeyCode::Enter => Action::Enter,
        KeyCode::Char(' ') => Action::Toggle,
        KeyCode::Char('X') => Action::ResetPrompt,
        KeyCode::Char('y') => Action::Confirm,
        KeyCode::Char('n') => Action::Cancel,
        KeyCode::Char('d') => Action::SetDefault,
        KeyCode::Char('x') => Action::SetDisable,
        KeyCode::Char('m') => Action::RecordMacro,
        KeyCode::Char('+') => Action::Add,
        KeyCode::Backspace | KeyCode::Delete => Action::Remove,
        KeyCode::Char('i') => Action::TextInput,
        KeyCode::Char('?') => Action::Help,
        KeyCode::Char('u') => Action::CheckUpdate,
        KeyCode::Char('t') => Action::CycleTheme,
        KeyCode::Char('r') => Action::Refresh,
        KeyCode::Char('L') => Action::ToggleLock,
        _ => Action::None,
    }
}
