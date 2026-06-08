//! App state and the action reducer. Holds the latest device snapshot; device
//! I/O happens on the worker thread (see `worker`). M1 wires the read path and
//! the Overview screen; writes are M2.

use std::sync::mpsc::Sender;

use crate::event::Action;
use crate::proto::Variant;
use crate::proto::block::Settings;
use crate::theme::{self, Theme};
use crate::worker::{Cmd, Update};

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

/// Connection state to the device.
pub enum Conn {
    Connecting,
    Up { name: String, variant: Variant },
    Down(String),
}

/// Transient status-line message; severity drives its color.
pub struct Status {
    pub text: String,
    pub level: StatusLevel,
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // Ok/Err set by write read-back in M2
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
    pub conn: Conn,
    pub settings: Option<Settings>,
    cmd_tx: Sender<Cmd>,
}

impl App {
    pub fn new(cmd_tx: Sender<Cmd>) -> Self {
        App {
            running: true,
            screen_idx: 0,
            theme_idx: 0,
            status: Status {
                text: "connecting…".into(),
                level: StatusLevel::Info,
            },
            conn: Conn::Connecting,
            settings: None,
            cmd_tx,
        }
    }

    pub fn screen(&self) -> Screen {
        Screen::ALL[self.screen_idx]
    }

    pub fn theme(&self) -> Theme {
        theme::ALL[self.theme_idx]
    }

    pub fn request_read(&self) {
        let _ = self.cmd_tx.send(Cmd::ReadAll);
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
                self.set_status(format!("theme: {}", self.theme().name), StatusLevel::Info);
            }
            Action::Refresh => {
                self.request_read();
                self.set_status("refreshing…".into(), StatusLevel::Info);
            }
            Action::None => {}
        }
    }

    /// Apply a device update from the worker thread.
    pub fn apply(&mut self, update: Update) {
        match update {
            Update::Connected { name, variant } => {
                let warn = if variant == Variant::EightKNordic {
                    ""
                } else {
                    " (unsupported variant — reads may be wrong)"
                };
                self.set_status(format!("connected: {name}{warn}"), StatusLevel::Ok);
                self.conn = Conn::Up { name, variant };
            }
            Update::Settings(s) => {
                self.settings = Some(*s);
            }
            Update::Error(e) => {
                self.settings = None;
                self.set_status(e.lines().next().unwrap_or("error").to_string(), StatusLevel::Err);
                self.conn = Conn::Down(e);
            }
        }
    }

    fn set_status(&mut self, text: String, level: StatusLevel) {
        self.status = Status { text, level };
    }
}
