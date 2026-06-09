//! App state and the action reducer. Holds the latest device snapshot plus the
//! per-screen edit buffers (DPI presets, polling selection). Device I/O runs on
//! the worker thread; actions that mutate the device send a `Cmd` and the result
//! comes back as an `Update`.

use std::sync::mpsc::Sender;
use std::time::Instant;

use crate::event::Action;
use crate::proto::Variant;
use crate::proto::block::Settings;
use crate::proto::buttons::{ButtonInfo, MEDIA_ACTIONS, MOUSE_ACTIONS};
use crate::proto::dpi::{DPI_MAX, DPI_MIN, NUM_PRESETS};
use crate::proto::macros::{self, MOUSE_PALETTE};
use crate::proto::polling::RATES_HZ;
use crate::proto::sensor::SensorFields;
use crate::theme::{self, Theme};
use crate::worker::{Cmd, Update};

const LOD_MAX: u8 = 2;
const ANGLE_MAX: u8 = 90;
const ANGLE_STEP: u8 = 5;
const DEBOUNCE_MAX: u8 = 30;
const SLEEP_MIN: u8 = 1;
const SLEEP_MAX: u8 = 240;
const SLEEP_STEP: u8 = 5;

/// Left-sidebar sections. Order = display order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Overview,
    Dpi,
    Polling,
    Sensor,
    Buttons,
    Profiles,
}

impl Screen {
    pub const ALL: [Screen; 6] = [
        Screen::Overview,
        Screen::Dpi,
        Screen::Polling,
        Screen::Sensor,
        Screen::Buttons,
        Screen::Profiles,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Screen::Overview => "Overview",
            Screen::Dpi => "DPI",
            Screen::Polling => "Polling",
            Screen::Sensor => "Sensor",
            Screen::Buttons => "Buttons",
            Screen::Profiles => "Profiles",
        }
    }

    /// Whether the content pane accepts focus / editing (others are read-only).
    pub fn interactive(self) -> bool {
        matches!(
            self,
            Screen::Dpi | Screen::Polling | Screen::Sensor | Screen::Buttons | Screen::Profiles
        )
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

/// Action-picker columns: choose a type, then (for Mouse) a value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickerCol {
    Type,
    Value,
}

/// Action types offered in the picker (M4: the verified set).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickKind {
    Mouse,
    Media,
    Disable,
    Default,
}

pub const PICK_TYPES: [(&str, PickKind); 4] = [
    ("Mouse", PickKind::Mouse),
    ("Media", PickKind::Media),
    ("Disable", PickKind::Disable),
    ("Default", PickKind::Default),
];

impl PickKind {
    /// Number of values offered for this type (0 = applied immediately).
    pub fn value_count(self) -> usize {
        match self {
            PickKind::Mouse => MOUSE_ACTIONS.len(),
            PickKind::Media => MEDIA_ACTIONS.len(),
            _ => 0,
        }
    }

    /// Value label at `idx` for this type.
    pub fn value_label(self, idx: usize) -> &'static str {
        match self {
            PickKind::Mouse => MOUSE_ACTIONS[idx].0,
            PickKind::Media => MEDIA_ACTIONS[idx].0,
            _ => "",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Picker {
    pub id: u8,
    pub col: PickerCol,
    pub type_idx: usize,
    pub value_idx: usize,
}

/// Active modal overlay.
pub enum Modal {
    ConfirmReset,
    ButtonPicker(Picker),
    MacroText,
    /// Numeric entry for the selected DPI preset.
    DpiInput,
    /// Macro builder for the target button (click sequence + text option).
    MacroEdit,
    /// Diff confirmation before applying pending sensor edits.
    ConfirmSensor,
    Help,
}

/// Connection state to the device.
pub enum Conn {
    Connecting,
    Up { name: String, firmware: String, transport: &'static str },
    Down(String),
}

/// Opt-in firmware-update check state (triggered by `u`).
pub enum FwCheck {
    Idle,
    Checking,
    UpToDate,
    Available(String),
    Failed,
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
    pub fw_check: FwCheck,
    pub settings: Option<Settings>,
    pub last_update: Option<Instant>,
    pub focus: Focus,

    // DPI
    pub dpi_cursor: usize,

    // Polling editor (index into RATES_HZ)
    pub poll_sel: usize,

    // Sensor editor
    pub sensor_cursor: usize,
    pub sensor_edit: SensorEdit,
    pub sensor_dirty: bool,

    // Buttons
    pub buttons: Vec<ButtonInfo>,
    pub button_cursor: usize,

    // Profiles
    pub profile_cursor: usize,

    // Macros (bound to macro_target button)
    pub macro_target: Option<u8>,
    pub macro_palette: usize,
    pub macro_seq: Vec<u8>,
    pub text_buf: String,

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
            fw_check: FwCheck::Idle,
            settings: None,
            last_update: None,
            focus: Focus::Sidebar,
            dpi_cursor: 0,
            poll_sel: 0,
            sensor_cursor: 0,
            sensor_edit: SensorEdit::default(),
            sensor_dirty: false,
            buttons: Vec::new(),
            button_cursor: 0,
            profile_cursor: 0,
            macro_target: None,
            macro_palette: 0,
            macro_seq: Vec::new(),
            text_buf: String::new(),
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

    pub fn request_buttons(&self) {
        let _ = self.cmd_tx.send(Cmd::ReadButtons);
    }

    /// Pending sensor edits as (label, old, new) display strings.
    pub fn sensor_diff(&self) -> Vec<(&'static str, String, String)> {
        let Some(s) = &self.settings else { return Vec::new() };
        let e = &self.sensor_edit;
        let d = &s.sensor;
        let mut out = Vec::new();
        if e.lod != d.lod {
            out.push(("Lift-off distance", lod_mm(d.lod).into(), lod_mm(e.lod).into()));
        }
        if e.scroll_dir != d.scroll_dir {
            let f = |v: u8| if v == 1 { "inverted".to_string() } else { "normal".to_string() };
            out.push(("Scroll direction", f(d.scroll_dir), f(e.scroll_dir)));
        }
        if e.motion != d.motion_sync {
            out.push(("Motion sync", on_off(d.motion_sync), on_off(e.motion)));
        }
        if e.wave != d.wave {
            out.push(("Ripple control", on_off(d.wave), on_off(e.wave)));
        }
        if e.fps20k != d.fps20k {
            let f = |v: u8| if v == 1 { "Competitive".to_string() } else { "Standard".to_string() };
            out.push(("Sampling mode", f(d.fps20k), f(e.fps20k)));
        }
        let dev_on = d.angle != 0;
        let dev_deg = d.angle.unsigned_abs().min(90) as u8;
        if e.angle_on != dev_on || (e.angle_on && e.angle_deg != dev_deg) {
            out.push(("Angle snapping", angle_str(dev_on, dev_deg), angle_str(e.angle_on, e.angle_deg)));
        }
        if e.debounce != s.debounce.value {
            out.push(("Debounce", format!("{} ms", s.debounce.value), format!("{} ms", e.debounce)));
        }
        if e.sleep != s.sleep_min {
            out.push(("Sleep", format!("{} min", s.sleep_min), format!("{} min", e.sleep)));
        }
        out
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
                if self.screen() == Screen::Buttons {
                    self.request_buttons();
                }
                self.set_status("refreshing…".into(), StatusLevel::Info);
            }
            Action::ToggleFocus => self.toggle_focus(),
            Action::Back => self.back(),
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
                if self.focus == Focus::Content && self.screen() == Screen::Sensor {
                    self.toggle_row();
                }
            }
            Action::Enter => match self.focus {
                Focus::Sidebar => self.enter_content(),
                Focus::Content => self.apply_edit(),
            },
            Action::ResetPrompt => self.modal = Some(Modal::ConfirmReset),
            Action::CheckUpdate => {
                if matches!(self.conn, Conn::Up { .. }) {
                    let _ = self.cmd_tx.send(Cmd::CheckUpdate);
                    self.fw_check = FwCheck::Checking;
                }
            }
            Action::Help => self.modal = Some(Modal::Help),
            Action::SetDefault => self.button_action(Cmd::SetButtonDefault, "restoring default…"),
            Action::SetDisable => self.button_action(Cmd::SetButtonDisable, "disabling…"),
            Action::RecordMacro => self.start_macro_for_selected_button(),
            Action::TextInput => {
                if self.screen() == Screen::Dpi {
                    self.text_buf.clear();
                    self.modal = Some(Modal::DpiInput);
                }
            }
            Action::Add | Action::Remove | Action::Confirm | Action::Cancel | Action::None => {}
        }
    }

    /// `m` on a button: target it and open the macro editor modal.
    fn start_macro_for_selected_button(&mut self) {
        if !self.on_buttons_content() {
            return;
        }
        self.macro_target = Some(self.buttons[self.button_cursor].id);
        self.macro_seq.clear();
        self.macro_palette = 0;
        self.modal = Some(Modal::MacroEdit);
    }

    fn macro_add(&mut self) {
        self.macro_seq.push(MOUSE_PALETTE[self.macro_palette].1);
    }

    fn macro_remove(&mut self) {
        self.macro_seq.pop();
    }

    fn macro_cursor(&mut self, delta: i32) {
        self.macro_palette = clamp_idx(self.macro_palette, delta, MOUSE_PALETTE.len());
    }

    // Text/number input modals — char capture routed from the event loop.
    pub fn capturing_text(&self) -> bool {
        matches!(self.modal, Some(Modal::MacroText | Modal::DpiInput))
    }

    pub fn input_char(&mut self, c: char) {
        match self.modal {
            Some(Modal::DpiInput) => {
                if c.is_ascii_digit() && self.text_buf.len() < 5 {
                    self.text_buf.push(c);
                }
            }
            Some(Modal::MacroText) if !c.is_control() => self.text_buf.push(c),
            _ => {}
        }
    }

    pub fn input_backspace(&mut self) {
        self.text_buf.pop();
    }

    pub fn input_cancel(&mut self) {
        self.modal = None;
    }

    pub fn input_commit(&mut self) {
        match self.modal.take() {
            Some(Modal::MacroText) => self.commit_macro_text(),
            Some(Modal::DpiInput) => self.commit_dpi_input(),
            _ => {}
        }
    }

    fn commit_macro_text(&mut self) {
        let Some(id) = self.macro_target else { return };
        match macros::text_events(&self.text_buf) {
            Ok(events) if !events.is_empty() => {
                let _ = self.cmd_tx.send(Cmd::SetMacro { id, events });
                self.set_status("uploading text macro…".into(), StatusLevel::Info);
            }
            Ok(_) => self.set_status("empty macro — nothing sent".into(), StatusLevel::Info),
            Err(e) => self.set_status(e.to_string(), StatusLevel::Err),
        }
    }

    fn commit_dpi_input(&mut self) {
        match self.text_buf.parse::<u16>() {
            Ok(v) => {
                let v = v.clamp(DPI_MIN, DPI_MAX);
                let _ = self.cmd_tx.send(Cmd::SetDpi { index: self.dpi_cursor, value: v });
                self.set_status(format!("DPI preset {} → {v}…", self.dpi_cursor + 1), StatusLevel::Info);
            }
            Err(_) => self.set_status("invalid number".into(), StatusLevel::Err),
        }
    }

    fn on_buttons_content(&self) -> bool {
        self.focus == Focus::Content && self.screen() == Screen::Buttons && !self.buttons.is_empty()
    }

    /// Send a per-button command for the selected button (Buttons content only).
    fn button_action(&mut self, make: impl Fn(u8) -> Cmd, status: &str) {
        if !self.on_buttons_content() {
            return;
        }
        let id = self.buttons[self.button_cursor].id;
        let _ = self.cmd_tx.send(make(id));
        self.set_status(status.into(), StatusLevel::Info);
    }

    fn open_button_picker(&mut self) {
        if self.buttons.is_empty() {
            return;
        }
        self.modal = Some(Modal::ButtonPicker(Picker {
            id: self.buttons[self.button_cursor].id,
            col: PickerCol::Type,
            type_idx: 0,
            value_idx: 0,
        }));
    }

    fn update_modal(&mut self, action: Action) {
        if let Action::Quit = action {
            self.running = false;
            return;
        }
        let Some(modal) = self.modal.take() else { return };
        match modal {
            Modal::ConfirmReset => match action {
                Action::Confirm | Action::Enter => {
                    let _ = self.cmd_tx.send(Cmd::FactoryReset);
                    self.set_status("factory reset…".into(), StatusLevel::Info);
                }
                Action::Cancel | Action::Back => {}
                _ => self.modal = Some(Modal::ConfirmReset), // keep open on stray keys
            },
            Modal::ButtonPicker(p) => self.update_picker(p, action),
            // Text/number capture is routed via input_* from the event loop.
            Modal::MacroText => self.modal = Some(Modal::MacroText),
            Modal::DpiInput => self.modal = Some(Modal::DpiInput),
            Modal::MacroEdit => match action {
                Action::Back | Action::Cancel => {} // close
                Action::Enter => self.upload_click_macro(), // upload + close
                Action::TextInput => {
                    self.text_buf.clear();
                    self.modal = Some(Modal::MacroText);
                }
                Action::Vertical(d) => {
                    self.macro_cursor(d);
                    self.modal = Some(Modal::MacroEdit);
                }
                Action::Add | Action::Toggle => {
                    self.macro_add();
                    self.modal = Some(Modal::MacroEdit);
                }
                Action::Remove => {
                    self.macro_remove();
                    self.modal = Some(Modal::MacroEdit);
                }
                _ => self.modal = Some(Modal::MacroEdit),
            },
            Modal::ConfirmSensor => match action {
                Action::Enter => self.apply_sensor_row(),
                Action::Back => {}
                _ => self.modal = Some(Modal::ConfirmSensor),
            },
            Modal::Help => match action {
                Action::Cancel | Action::Back | Action::Help | Action::Enter => {}
                _ => self.modal = Some(Modal::Help),
            },
        }
    }

    fn update_picker(&mut self, mut p: Picker, action: Action) {
        match action {
            Action::Cancel | Action::Back => return, // closed (modal already taken)
            Action::Vertical(d) => match p.col {
                PickerCol::Type => p.type_idx = clamp_idx(p.type_idx, d, PICK_TYPES.len()),
                PickerCol::Value => {
                    let len = PICK_TYPES[p.type_idx].1.value_count();
                    p.value_idx = clamp_idx(p.value_idx, d, len);
                }
            },
            Action::Horizontal(d) => {
                let has_values = PICK_TYPES[p.type_idx].1.value_count() > 0;
                if d > 0 && p.col == PickerCol::Type && has_values {
                    p.col = PickerCol::Value;
                    p.value_idx = 0;
                } else if d < 0 && p.col == PickerCol::Value {
                    p.col = PickerCol::Type;
                }
            }
            // commit_picker returns true when a command was sent -> leave closed
            Action::Enter | Action::Confirm if self.commit_picker(&mut p) => return,
            _ => {}
        }
        self.modal = Some(Modal::ButtonPicker(p)); // keep open
    }

    /// Act on the picker. Returns true if a command was sent (close the modal).
    fn commit_picker(&mut self, p: &mut Picker) -> bool {
        let kind = PICK_TYPES[p.type_idx].1;
        match p.col {
            PickerCol::Type => match kind {
                PickKind::Mouse | PickKind::Media => {
                    p.col = PickerCol::Value; // descend into values, stay open
                    p.value_idx = 0;
                    false
                }
                PickKind::Disable => {
                    let _ = self.cmd_tx.send(Cmd::SetButtonDisable(p.id));
                    self.set_status("disabling…".into(), StatusLevel::Info);
                    true
                }
                PickKind::Default => {
                    let _ = self.cmd_tx.send(Cmd::SetButtonDefault(p.id));
                    self.set_status("restoring default…".into(), StatusLevel::Info);
                    true
                }
            },
            PickerCol::Value => {
                let action = kind.value_label(p.value_idx).to_string();
                let cmd = match kind {
                    PickKind::Media => Cmd::SetButtonMedia { id: p.id, action },
                    _ => Cmd::SetButtonMouse { id: p.id, action },
                };
                let _ = self.cmd_tx.send(cmd);
                self.set_status("applying…".into(), StatusLevel::Info);
                true
            }
        }
    }

    /// Esc: drop focus back to the sidebar.
    fn back(&mut self) {
        self.focus = Focus::Sidebar;
    }

    fn section(&mut self, delta: i32) {
        let len = Screen::ALL.len() as i32;
        self.screen_idx = (self.screen_idx as i32 + delta).rem_euclid(len) as usize;
        if self.screen() == Screen::Buttons {
            self.request_buttons(); // lazy-load the button table on entry
        }
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
            Screen::Buttons if !self.buttons.is_empty() => {
                self.button_cursor = clamp_idx(self.button_cursor, delta, self.buttons.len());
            }
            Screen::Profiles => {
                self.profile_cursor = clamp_idx(self.profile_cursor, delta, self.profile_count());
            }
            _ => {}
        }
    }

    /// Number of device profiles (defaults to 5 before the first read).
    pub fn profile_count(&self) -> usize {
        self.settings
            .as_ref()
            .map(|s| (s.profile.count.max(1)) as usize)
            .unwrap_or(5)
    }

    fn adjust(&mut self, delta: i32) {
        if self.screen() == Screen::Sensor {
            self.adjust_sensor(delta.signum());
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
            SensorRow::Sleep => e.sleep = step_clamp(e.sleep, dir, SLEEP_MIN, SLEEP_MAX, SLEEP_STEP),
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
                self.text_buf.clear();
                self.modal = Some(Modal::DpiInput);
            }
            Screen::Polling => {
                let _ = self.cmd_tx.send(Cmd::SetRate {
                    hz: RATES_HZ[self.poll_sel],
                });
                self.set_status("applying polling rate…".into(), StatusLevel::Info);
            }
            Screen::Sensor => {
                if !self.sensor_diff().is_empty() {
                    self.modal = Some(Modal::ConfirmSensor);
                }
            }
            Screen::Buttons => self.open_button_picker(),
            Screen::Profiles => {
                let _ = self.cmd_tx.send(Cmd::SetProfile(self.profile_cursor as u8));
                self.set_status("switching profile…".into(), StatusLevel::Info);
            }
            _ => {}
        }
    }

    fn upload_click_macro(&mut self) {
        let Some(id) = self.macro_target else {
            self.set_status("no target — press m on a button first".into(), StatusLevel::Info);
            return;
        };
        if self.macro_seq.is_empty() {
            self.set_status("macro empty — add clicks or press i for text".into(), StatusLevel::Info);
            return;
        }
        let events = macros::click_events(&self.macro_seq);
        let _ = self.cmd_tx.send(Cmd::SetMacro { id, events });
        self.set_status("uploading macro…".into(), StatusLevel::Info);
    }

    /// Apply every sensor field that differs from the device (one ↵ commits all
    /// pending edits), then let the post-write refresh reseed cleanly.
    fn apply_sensor_row(&mut self) {
        let Some(s) = self.settings.clone() else { return };
        let e = self.sensor_edit;
        let dev = &s.sensor;

        // Combine all changed sensor-block fields into one cmd 0x42 write.
        let mut f = SensorFields::default();
        let mut any = false;
        if e.lod != dev.lod {
            f.lod = Some(e.lod);
            any = true;
        }
        if e.scroll_dir != dev.scroll_dir {
            f.scroll_dir = Some(e.scroll_dir);
            any = true;
        }
        if e.motion != dev.motion_sync {
            f.motion = Some(e.motion);
            any = true;
        }
        if e.wave != dev.wave {
            f.wave = Some(e.wave);
            any = true;
        }
        if e.fps20k != dev.fps20k {
            f.fps20k = Some(e.fps20k);
            any = true;
        }
        if any {
            let _ = self.cmd_tx.send(Cmd::SetSensor(f));
        }

        let dev_deg = dev.angle.unsigned_abs().min(90) as u8;
        if e.angle_on != (dev.angle != 0) || (e.angle_on && e.angle_deg != dev_deg) {
            let _ = self.cmd_tx.send(Cmd::SetAngle { degrees: e.angle_deg, enable: e.angle_on });
        }
        if e.debounce != s.debounce.value {
            let _ = self.cmd_tx.send(Cmd::SetDebounce(e.debounce));
        }
        if e.sleep != s.sleep_min {
            let _ = self.cmd_tx.send(Cmd::SetSleep(e.sleep));
        }
        self.set_status("applying sensor…".into(), StatusLevel::Info);
    }

    /// Apply a device update from the worker thread.
    pub fn apply(&mut self, update: Update) {
        match update {
            Update::Connected { name, variant, firmware, transport } => {
                let warn = if variant == Variant::EightKNordic {
                    ""
                } else {
                    " (unsupported variant — reads may be wrong)"
                };
                self.set_status(format!("connected: {name}{warn}"), StatusLevel::Ok);
                self.conn = Conn::Up { name, firmware, transport };
                self.fw_check = FwCheck::Idle;
            }
            Update::Settings(s) => {
                // Reseed editors from the device unless the user has unsaved edits.
                let code = s.polling.levels[0] as usize;
                if code < RATES_HZ.len() {
                    self.poll_sel = code;
                }
                if !self.sensor_dirty {
                    self.sensor_edit = seed_sensor(&s);
                }
                self.settings = Some(*s);
                self.last_update = Some(Instant::now());
            }
            Update::Buttons(v) => {
                self.buttons = v;
                if self.button_cursor >= self.buttons.len() {
                    self.button_cursor = self.buttons.len().saturating_sub(1);
                }
            }
            Update::Written { ok, msg } => {
                self.set_status(msg, if ok { StatusLevel::Ok } else { StatusLevel::Err });
                if ok {
                    self.sensor_dirty = false; // next Settings reseeds the sensor editor
                }
            }
            Update::Firmware { latest } => {
                let current = match &self.conn {
                    Conn::Up { firmware, .. } => firmware.clone(),
                    _ => return,
                };
                self.fw_check = match latest {
                    Some(v) if v == current => FwCheck::UpToDate,
                    Some(v) => FwCheck::Available(v),
                    None => FwCheck::Failed,
                };
            }
            Update::Error(e) => {
                self.settings = None;
                self.fw_check = FwCheck::Idle;
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

fn lod_mm(v: u8) -> &'static str {
    match v {
        0 => "0.7 mm",
        1 => "1.0 mm",
        2 => "2.0 mm",
        _ => "?",
    }
}

fn on_off(v: u8) -> String {
    if v == 1 { "on".into() } else { "off".into() }
}

fn angle_str(on: bool, deg: u8) -> String {
    if on { format!("on {deg}°") } else { "off".into() }
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
        sleep: s.sleep_min,
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
