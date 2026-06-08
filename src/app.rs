//! App state and the action reducer. Holds the latest device snapshot plus the
//! per-screen edit buffers (DPI presets, polling selection). Device I/O runs on
//! the worker thread; actions that mutate the device send a `Cmd` and the result
//! comes back as an `Update`.

use std::sync::mpsc::Sender;

use crate::event::Action;
use crate::proto::Variant;
use crate::proto::block::Settings;
use crate::proto::dpi::{DPI_MAX, DPI_MIN, NUM_PRESETS};
use crate::proto::polling::RATES_HZ;
use crate::proto::sensor::SensorFields;
use crate::theme::{self, Theme};
use crate::worker::{Cmd, Update};

const LOD_MAX: u8 = 2;
const ANGLE_MAX: u8 = 90;
const ANGLE_STEP: u8 = 5;
const DEBOUNCE_MAX: u8 = 30;
const SLEEP_MAX: u8 = 250;
const SLEEP_STEP: u8 = 10;

/// Left-sidebar sections. Order = display order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    /// Whether the content pane accepts focus / editing (others are read-only).
    pub fn interactive(self) -> bool {
        matches!(self, Screen::Dpi | Screen::Polling | Screen::Sensor)
    }
}

/// Which pane has keyboard focus.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Sidebar,
    Content,
}

/// Editable rows on the Sensor screen.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SensorRow {
    Lod,
    ScrollDir,
    Motion,
    Angle,
    Ripple,
    Sampling,
    Debounce,
    Sleep,
}

impl SensorRow {
    pub const ALL: [SensorRow; 8] = [
        SensorRow::Lod,
        SensorRow::ScrollDir,
        SensorRow::Motion,
        SensorRow::Angle,
        SensorRow::Ripple,
        SensorRow::Sampling,
        SensorRow::Debounce,
        SensorRow::Sleep,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SensorRow::Lod => "Lift-off distance",
            SensorRow::ScrollDir => "Scroll direction",
            SensorRow::Motion => "Motion sync",
            SensorRow::Angle => "Angle snapping",
            SensorRow::Ripple => "Ripple control",
            SensorRow::Sampling => "Sampling mode",
            SensorRow::Debounce => "Debounce",
            SensorRow::Sleep => "Sleep",
        }
    }
}

/// Working copy of the Sensor screen values (seeded from the device snapshot).
#[derive(Clone, Copy, Default)]
pub struct SensorEdit {
    pub lod: u8,
    pub scroll_dir: u8,
    pub motion: u8,
    pub wave: u8,
    pub fps20k: u8,
    pub angle_on: bool,
    pub angle_deg: u8,
    pub debounce: u8,
    pub sleep: u8,
}

/// Active modal overlay.
pub enum Modal {
    ConfirmReset,
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
    pub focus: Focus,

    // DPI editor
    pub dpi_cursor: usize,
    pub dpi_edit: [u16; NUM_PRESETS],
    pub dpi_dirty: bool,

    // Polling editor (index into RATES_HZ)
    pub poll_sel: usize,

    // Sensor editor
    pub sensor_cursor: usize,
    pub sensor_edit: SensorEdit,
    pub sensor_dirty: bool,

    pub modal: Option<Modal>,

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
            focus: Focus::Sidebar,
            dpi_cursor: 0,
            dpi_edit: [0; NUM_PRESETS],
            dpi_dirty: false,
            poll_sel: 0,
            sensor_cursor: 0,
            sensor_edit: SensorEdit::default(),
            sensor_dirty: false,
            modal: None,
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

    /// Has the DPI cursor's preset been edited away from the device value?
    pub fn dpi_changed(&self, index: usize) -> bool {
        match &self.settings {
            Some(s) => self.dpi_edit[index] != s.dpi.presets[index],
            None => false,
        }
    }

    pub fn update(&mut self, action: Action) {
        if self.modal.is_some() {
            self.update_modal(action);
            return;
        }
        match action {
            Action::Quit => self.running = false,
            Action::CycleTheme => {
                self.theme_idx = (self.theme_idx + 1) % theme::ALL.len();
                self.set_status(format!("theme: {}", self.theme().name), StatusLevel::Info);
            }
            Action::Refresh => {
                self.request_read();
                self.set_status("refreshing…".into(), StatusLevel::Info);
            }
            Action::ToggleFocus => self.toggle_focus(),
            Action::Back => self.focus = Focus::Sidebar,
            Action::Vertical(d) => match self.focus {
                Focus::Sidebar => self.section(d),
                Focus::Content => self.move_cursor(d),
            },
            Action::Horizontal(d) => match self.focus {
                Focus::Sidebar => {
                    if d > 0 {
                        self.enter_content();
                    }
                }
                Focus::Content => self.adjust(d),
            },
            Action::Toggle => {
                if self.focus == Focus::Content {
                    self.toggle_row();
                }
            }
            Action::Enter => match self.focus {
                Focus::Sidebar => self.enter_content(),
                Focus::Content => self.apply_edit(),
            },
            Action::ResetPrompt => self.modal = Some(Modal::ConfirmReset),
            Action::Confirm | Action::Cancel | Action::None => {}
        }
    }

    fn update_modal(&mut self, action: Action) {
        match action {
            Action::Confirm | Action::Enter => {
                let _ = self.cmd_tx.send(Cmd::FactoryReset);
                self.set_status("factory reset…".into(), StatusLevel::Info);
                self.modal = None;
            }
            Action::Cancel | Action::Back => self.modal = None,
            Action::Quit => self.running = false,
            _ => {}
        }
    }

    fn section(&mut self, delta: i32) {
        let len = Screen::ALL.len() as i32;
        self.screen_idx = (self.screen_idx as i32 + delta).rem_euclid(len) as usize;
    }

    fn enter_content(&mut self) {
        if self.screen().interactive() {
            self.focus = Focus::Content;
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sidebar if self.screen().interactive() => Focus::Content,
            _ => Focus::Sidebar,
        };
    }

    fn move_cursor(&mut self, delta: i32) {
        match self.screen() {
            Screen::Dpi => {
                self.dpi_cursor = clamp_idx(self.dpi_cursor, delta, NUM_PRESETS);
            }
            Screen::Polling => {
                self.poll_sel = clamp_idx(self.poll_sel, delta, RATES_HZ.len());
            }
            Screen::Sensor => {
                self.sensor_cursor = clamp_idx(self.sensor_cursor, delta, SensorRow::ALL.len());
            }
            _ => {}
        }
    }

    fn adjust(&mut self, delta: i32) {
        match self.screen() {
            Screen::Dpi => {
                let v = (self.dpi_edit[self.dpi_cursor] as i32 + delta)
                    .clamp(DPI_MIN as i32, DPI_MAX as i32) as u16;
                self.dpi_edit[self.dpi_cursor] = v;
                self.dpi_dirty = true;
            }
            Screen::Sensor => self.adjust_sensor(delta.signum()),
            _ => {}
        }
    }

    fn adjust_sensor(&mut self, dir: i32) {
        let e = &mut self.sensor_edit;
        match SensorRow::ALL[self.sensor_cursor] {
            SensorRow::Lod => e.lod = step_clamp(e.lod, dir, 0, LOD_MAX, 1),
            SensorRow::ScrollDir => e.scroll_dir ^= 1,
            SensorRow::Motion => e.motion ^= 1,
            SensorRow::Ripple => e.wave ^= 1,
            SensorRow::Sampling => e.fps20k ^= 1,
            SensorRow::Angle => {
                e.angle_deg = step_clamp(e.angle_deg, dir, 0, ANGLE_MAX, ANGLE_STEP);
                e.angle_on = e.angle_deg > 0;
            }
            SensorRow::Debounce => e.debounce = step_clamp(e.debounce, dir, 0, DEBOUNCE_MAX, 1),
            SensorRow::Sleep => e.sleep = step_clamp(e.sleep, dir, 0, SLEEP_MAX, SLEEP_STEP),
        }
        self.sensor_dirty = true;
    }

    fn toggle_row(&mut self) {
        if self.screen() != Screen::Sensor {
            return;
        }
        let e = &mut self.sensor_edit;
        match SensorRow::ALL[self.sensor_cursor] {
            SensorRow::ScrollDir => e.scroll_dir ^= 1,
            SensorRow::Motion => e.motion ^= 1,
            SensorRow::Ripple => e.wave ^= 1,
            SensorRow::Sampling => e.fps20k ^= 1,
            SensorRow::Angle => e.angle_on = !e.angle_on,
            _ => return,
        }
        self.sensor_dirty = true;
    }

    fn apply_edit(&mut self) {
        match self.screen() {
            Screen::Dpi => {
                let _ = self.cmd_tx.send(Cmd::SetDpi {
                    index: self.dpi_cursor,
                    value: self.dpi_edit[self.dpi_cursor],
                });
                self.set_status("applying DPI…".into(), StatusLevel::Info);
            }
            Screen::Polling => {
                let _ = self.cmd_tx.send(Cmd::SetRate {
                    hz: RATES_HZ[self.poll_sel],
                });
                self.set_status("applying polling rate…".into(), StatusLevel::Info);
            }
            Screen::Sensor => self.apply_sensor_row(),
            _ => {}
        }
    }

    fn apply_sensor_row(&mut self) {
        let e = self.sensor_edit;
        let cmd = match SensorRow::ALL[self.sensor_cursor] {
            SensorRow::Lod => Cmd::SetSensor(SensorFields { lod: Some(e.lod), ..Default::default() }),
            SensorRow::ScrollDir => {
                Cmd::SetSensor(SensorFields { scroll_dir: Some(e.scroll_dir), ..Default::default() })
            }
            SensorRow::Motion => {
                Cmd::SetSensor(SensorFields { motion: Some(e.motion), ..Default::default() })
            }
            SensorRow::Ripple => {
                Cmd::SetSensor(SensorFields { wave: Some(e.wave), ..Default::default() })
            }
            SensorRow::Sampling => {
                Cmd::SetSensor(SensorFields { fps20k: Some(e.fps20k), ..Default::default() })
            }
            SensorRow::Angle => Cmd::SetAngle { degrees: e.angle_deg, enable: e.angle_on },
            SensorRow::Debounce => Cmd::SetDebounce(e.debounce),
            SensorRow::Sleep => Cmd::SetSleep(e.sleep),
        };
        let _ = self.cmd_tx.send(cmd);
        self.set_status("applying…".into(), StatusLevel::Info);
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
                // Reseed editors from the device unless the user has unsaved edits.
                if !self.dpi_dirty {
                    self.dpi_edit = s.dpi.presets;
                }
                let code = s.polling.levels[0] as usize;
                if code < RATES_HZ.len() {
                    self.poll_sel = code;
                }
                if !self.sensor_dirty {
                    self.sensor_edit = seed_sensor(&s);
                }
                self.settings = Some(*s);
            }
            Update::Written { ok, msg } => {
                self.set_status(msg, if ok { StatusLevel::Ok } else { StatusLevel::Err });
                if ok {
                    // next Settings reseeds the editors
                    self.dpi_dirty = false;
                    self.sensor_dirty = false;
                }
            }
            Update::Error(e) => {
                self.settings = None;
                self.set_status(
                    e.lines().next().unwrap_or("error").to_string(),
                    StatusLevel::Err,
                );
                self.conn = Conn::Down(e);
            }
        }
    }

    fn set_status(&mut self, text: String, level: StatusLevel) {
        self.status = Status { text, level };
    }
}

/// Move an index by `delta`, clamped to `0..len`.
fn clamp_idx(cur: usize, delta: i32, len: usize) -> usize {
    (cur as i32 + delta).clamp(0, len as i32 - 1) as usize
}

/// Step a value by `dir * step`, clamped to `min..=max`.
fn step_clamp(cur: u8, dir: i32, min: u8, max: u8, step: u8) -> u8 {
    (cur as i32 + dir.signum() * step as i32).clamp(min as i32, max as i32) as u8
}

/// Seed the Sensor editor from a device snapshot.
fn seed_sensor(s: &Settings) -> SensorEdit {
    SensorEdit {
        lod: s.sensor.lod,
        scroll_dir: s.sensor.scroll_dir,
        motion: s.sensor.motion_sync,
        wave: s.sensor.wave,
        fps20k: s.sensor.fps20k,
        angle_on: s.sensor.angle != 0,
        angle_deg: s.sensor.angle.unsigned_abs().min(ANGLE_MAX as u16) as u8,
        debounce: s.debounce.value,
        sleep: s.sleep_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn app() -> App {
        let (tx, _rx) = channel();
        App::new(tx)
    }

    #[test]
    fn vertical_navigates_sections_when_sidebar_focused() {
        let mut a = app();
        assert_eq!(a.screen_idx, 0);
        a.update(Action::Vertical(1));
        assert_eq!(a.screen_idx, 1);
        a.update(Action::Vertical(-1));
        assert_eq!(a.screen_idx, 0);
        a.update(Action::Vertical(-1)); // wraps to last
        assert_eq!(a.screen_idx, Screen::ALL.len() - 1);
    }

    #[test]
    fn enter_focuses_content_only_on_interactive_screen() {
        let mut a = app();
        // Overview (idx 0) is not interactive: Enter stays on sidebar.
        a.update(Action::Enter);
        assert_eq!(a.focus, Focus::Sidebar);

        // Move to DPI (interactive) and focus content.
        a.update(Action::Vertical(1));
        assert_eq!(a.screen(), Screen::Dpi);
        a.update(Action::Enter);
        assert_eq!(a.focus, Focus::Content);
    }

    #[test]
    fn vertical_moves_cursor_when_content_focused() {
        let mut a = app();
        a.update(Action::Vertical(1)); // -> DPI
        a.update(Action::Enter); // focus content
        let section = a.screen_idx;
        a.update(Action::Vertical(1));
        assert_eq!(a.screen_idx, section, "section must not change in content focus");
        assert_eq!(a.dpi_cursor, 1);
        a.update(Action::Back);
        assert_eq!(a.focus, Focus::Sidebar);
    }
}
