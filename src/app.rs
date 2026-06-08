//! App state and the action reducer. Device I/O is not wired yet (M1); M0 holds
//! navigation, theme cycling, status line, and quit.

use crate::event::Action;
use crate::theme::{self, Theme};

/// Left-sidebar sections. Order = display order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Overview,
    Dpi,
    Polling,
    Sensor,
    Buttons,
    Macros,
    Profiles,
}

impl Screen {
    pub const ALL: [Screen; 7] = [
        Screen::Overview,
        Screen::Dpi,
        Screen::Polling,
        Screen::Sensor,
        Screen::Buttons,
        Screen::Macros,
        Screen::Profiles,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Screen::Overview => "Overview",
            Screen::Dpi => "DPI",
            Screen::Polling => "Polling",
            Screen::Sensor => "Sensor",
            Screen::Buttons => "Buttons",
            Screen::Macros => "Macros",
            Screen::Profiles => "Profiles",
        }
    }
}

/// Transient status-line message; severity drives its color.
pub struct Status {
    pub text: String,
    pub level: StatusLevel,
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // Ok/Err set by write read-back in M1
pub enum StatusLevel {
    Info,
    Ok,
    Err,
}

pub struct App {
    pub running: bool,
    pub screen_idx: usize,
    pub theme_idx: usize,
    pub status: Status,
}

impl Default for App {
    fn default() -> Self {
        App {
            running: true,
            screen_idx: 0,
            theme_idx: 0,
            status: Status {
                text: "no device wired yet (M0 skeleton)".into(),
                level: StatusLevel::Info,
            },
        }
    }
}

impl App {
    pub fn screen(&self) -> Screen {
        Screen::ALL[self.screen_idx]
    }

    pub fn theme(&self) -> Theme {
        theme::ALL[self.theme_idx]
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::NavUp => {
                self.screen_idx =
                    (self.screen_idx + Screen::ALL.len() - 1) % Screen::ALL.len();
            }
            Action::NavDown => {
                self.screen_idx = (self.screen_idx + 1) % Screen::ALL.len();
            }
            Action::CycleTheme => {
                self.theme_idx = (self.theme_idx + 1) % theme::ALL.len();
                self.status = Status {
                    text: format!("theme: {}", self.theme().name),
                    level: StatusLevel::Info,
                };
            }
            Action::Refresh => {
                self.status = Status {
                    text: "refresh: no device wired yet (M1)".into(),
                    level: StatusLevel::Info,
                };
            }
            Action::None => {}
        }
    }
}
