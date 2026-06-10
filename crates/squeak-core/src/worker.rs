//! Dedicated device-I/O thread (PLAN §7). The UI thread sends `Cmd`, receives
//! `Update`, and never blocks on hidraw. The worker (re)connects lazily so a
//! replug + refresh recovers without restarting.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use crate::hid::{Device, DeviceInfo, Hid, HidError, find_config};
use crate::proto::block::{self, Settings};
use crate::proto::buttons::{self, ButtonInfo};
use crate::proto::sensor::SensorFields;
use crate::proto::{self, Variant, dpi, info, macros, polling, profile, sensor, system};

#[derive(Debug)]
pub enum Cmd {
    ReadAll,
    ReadButtons,
    SetDpi { index: usize, value: u16 },
    SetActiveDpi(usize),
    SetRate { hz: u32 },
    SetSensor(SensorFields),
    SetAngle { degrees: u8, enable: bool },
    SetDebounce(u8),
    SetSleep(u8),
    SetButtonMouse { id: u8, action: String },
    SetButtonMedia { id: u8, action: String },
    SetButtonDisable(u8),
    SetButtonDefault(u8),
    SetMacro { id: u8, events: Vec<u8> },
    SetProfile(u8),
    FactoryReset,
    CheckUpdate,
    Shutdown,
}

pub enum Update {
    Connected { name: String, variant: Variant, firmware: String, transport: &'static str },
    Settings(Box<Settings>),
    Buttons(Vec<ButtonInfo>),
    /// Result of a write, after read-back. Drives the ✓/✗ status line.
    Written { ok: bool, msg: String },
    /// Opt-in firmware check result; None = lookup failed/offline.
    Firmware { latest: Option<String> },
    Error(String),
}

pub struct Worker {
    pub cmd_tx: Sender<Cmd>,
    pub update_rx: Receiver<Update>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn spawn() -> Worker {
        let (cmd_tx, cmd_rx) = channel::<Cmd>();
        let (update_tx, update_rx) = channel::<Update>();
        let handle = thread::spawn(move || run(cmd_rx, update_tx));
        Worker {
            cmd_tx,
            update_rx,
            handle: Some(handle),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(Cmd::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn run(cmd_rx: Receiver<Cmd>, update_tx: Sender<Update>) {
    let mut dev: Option<Device> = None;
    let mut ids: Option<(u16, u16)> = None; // (vid, pid) for the update check
    while let Ok(cmd) = cmd_rx.recv() {
        if matches!(cmd, Cmd::Shutdown) {
            break;
        }
        // Ensure connected before any device command.
        if dev.is_none() {
            match ensure_connected(&update_tx) {
                Ok((d, vid, pid)) => {
                    dev = Some(d);
                    ids = Some((vid, pid));
                }
                Err(stop) => {
                    if stop {
                        break;
                    }
                    continue;
                }
            }
        }
        let stop = match cmd {
            Cmd::Shutdown => true,
            // Network check on a detached thread — never blocks device I/O.
            Cmd::CheckUpdate => {
                if let Some((vid, pid)) = ids {
                    let tx = update_tx.clone();
                    std::thread::spawn(move || {
                        let latest = crate::update::latest_version(vid, pid).ok();
                        let _ = tx.send(Update::Firmware { latest });
                    });
                }
                false
            }
            other => {
                let mut closed = false;
                let mut drop_dev = false;
                for u in handle(other, dev.as_mut().unwrap()) {
                    drop_dev |= matches!(u, Update::Error(_));
                    if send(&update_tx, u) {
                        closed = true;
                        break;
                    }
                }
                if drop_dev {
                    dev = None; // drop; reconnect on next command
                }
                closed
            }
        };
        if stop {
            break;
        }
    }
}

/// Execute one connected-device command, returning the Updates to emit in
/// order. An `Update::Error` in the result signals the caller to drop and
/// reconnect the device. `CheckUpdate`/`Shutdown` are handled by `run`.
fn handle(cmd: Cmd, dev: &mut dyn Hid) -> Vec<Update> {
    match cmd {
        Cmd::ReadAll => match block::read_all(dev) {
            Ok(s) => vec![Update::Settings(Box::new(s))],
            Err(e) => vec![Update::Error(format!("read failed: {e}"))],
        },
        Cmd::ReadButtons => match buttons::get_all(dev, buttons::COUNT) {
            Ok(v) => vec![Update::Buttons(v)],
            Err(e) => vec![Update::Error(format!("button read failed: {e}"))],
        },
        Cmd::SetDpi { index, value } => {
            let r = dpi::set_dpi(dev, value, index)
                .map(|_| format!("DPI preset {} → {value} ✓ verified", index + 1));
            write_then_settings(dev, r)
        }
        Cmd::SetActiveDpi(index) => {
            let r = dpi::set_active(dev, index)
                .map(|i| format!("active DPI → preset {} ✓ verified", i + 1));
            write_then_settings(dev, r)
        }
        Cmd::SetRate { hz } => {
            let r = polling::set_rate(dev, hz).map(|_| format!("polling → {hz} Hz ✓ verified"));
            write_then_settings(dev, r)
        }
        Cmd::SetSensor(fields) => {
            let r = sensor::set_sensor(dev, fields).map(|_| "sensor ✓ verified".to_string());
            write_then_settings(dev, r)
        }
        Cmd::SetAngle { degrees, enable } => {
            let r = sensor::set_angle(dev, degrees, enable).map(|a| {
                if enable {
                    format!("angle snap → {a}° ✓ verified")
                } else {
                    "angle snap off ✓ verified".to_string()
                }
            });
            write_then_settings(dev, r)
        }
        Cmd::SetDebounce(ms) => {
            let r = system::set_debounce(dev, ms).map(|v| format!("debounce → {v} ms ✓ verified"));
            write_then_settings(dev, r)
        }
        Cmd::SetSleep(minutes) => {
            let r = system::set_sleep(dev, minutes).map(|v| format!("sleep → {v} min ✓ verified"));
            write_then_settings(dev, r)
        }
        Cmd::FactoryReset => {
            let r = system::factory_reset(dev).map(|_| "factory reset sent".to_string());
            write_then_settings(dev, r)
        }
        Cmd::SetProfile(index) => {
            let r = profile::set_profile(dev, index).map(|i| format!("profile → {} ✓ verified", i + 1));
            write_then_settings(dev, r)
        }
        Cmd::SetButtonMouse { id, action } => {
            let r = buttons::set_mouse(dev, id, &action)
                .map(|b| format!("button {id} → {} ✓ verified", b.label));
            write_then_buttons(dev, r)
        }
        Cmd::SetButtonMedia { id, action } => {
            let r = buttons::set_media(dev, id, &action)
                .map(|b| format!("button {id} → {} ✓ verified", b.label));
            write_then_buttons(dev, r)
        }
        Cmd::SetButtonDisable(id) => {
            let r = buttons::disable(dev, id).map(|_| format!("button {id} disabled ✓ verified"));
            write_then_buttons(dev, r)
        }
        Cmd::SetButtonDefault(id) => {
            let r = buttons::restore_default(dev, id)
                .map(|b| format!("button {id} → {} ✓ verified", b.label));
            write_then_buttons(dev, r)
        }
        Cmd::SetMacro { id, events } => {
            let r = macros::set_macro(dev, id, &events)
                .map(|b| format!("macro → button {id} ✓ verified (len {})", b.data));
            write_then_buttons(dev, r)
        }
        Cmd::CheckUpdate | Cmd::Shutdown => vec![],
    }
}

fn send(tx: &Sender<Update>, u: Update) -> bool {
    tx.send(u).is_err()
}

/// Written status from a write result, then a fresh block snapshot. On a write
/// error: just the failed status. On a read-back error: an `Update::Error`.
fn write_then_settings(dev: &mut dyn Hid, result: Result<String, HidError>) -> Vec<Update> {
    let msg = match result {
        Ok(msg) => msg,
        Err(e) => return vec![Update::Written { ok: false, msg: e.to_string() }],
    };
    let mut out = vec![Update::Written { ok: true, msg }];
    match block::read_all(dev) {
        Ok(s) => out.push(Update::Settings(Box::new(s))),
        Err(e) => out.push(Update::Error(format!("read failed: {e}"))),
    }
    out
}

/// Like `write_then_settings`, but refreshes the button table after the write.
fn write_then_buttons(dev: &mut dyn Hid, result: Result<String, HidError>) -> Vec<Update> {
    let msg = match result {
        Ok(msg) => msg,
        Err(e) => return vec![Update::Written { ok: false, msg: e.to_string() }],
    };
    let mut out = vec![Update::Written { ok: true, msg }];
    match buttons::get_all(dev, buttons::COUNT) {
        Ok(v) => out.push(Update::Buttons(v)),
        Err(e) => out.push(Update::Error(format!("button read failed: {e}"))),
    }
    out
}

/// Connect, announcing it. Returns the device + (vid, pid). Err(true) = channel
/// closed (stop); Err(false) = retry later.
fn ensure_connected(tx: &Sender<Update>) -> Result<(Device, u16, u16), bool> {
    match connect() {
        Ok((di, mut d)) => {
            let (vid, pid) = (di.vid, di.pid);
            let variant = proto::detect(di.usage_page);
            let name = if di.name.is_empty() {
                format!("Keychron {vid:04x}:{pid:04x}")
            } else {
                dedupe_words(&di.name)
            };
            // 0xD0xx PIDs are the 2.4 GHz dongle transport; 0x06xx are wired.
            let transport = if pid >= 0xD000 { "2.4 GHz" } else { "wired" };
            let firmware = info::read_version(&mut d).unwrap_or_else(|_| "?".into());
            if send(tx, Update::Connected { name, variant, firmware, transport }) {
                return Err(true);
            }
            Ok((d, vid, pid))
        }
        Err(e) => Err(send(tx, Update::Error(e))),
    }
}

/// Collapse consecutive duplicate words (device HID_NAME repeats "Keychron").
fn dedupe_words(s: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for w in s.split_whitespace() {
        if out.last() != Some(&w) {
            out.push(w);
        }
    }
    out.join(" ")
}

fn connect() -> Result<(DeviceInfo, Device), String> {
    let info = find_config().ok_or_else(|| {
        "Keychron config device not found (VID 3434, usage 0xFFC1). Plug in the dongle.".to_string()
    })?;
    match Device::open(&info.node) {
        Ok(d) => Ok((info, d)),
        Err(e) => Err(format!(
            "cannot open {} ({e}).\nudev fix (then replug the dongle):\n  \
             echo 'SUBSYSTEM==\"hidraw\", ATTRS{{idVendor}}==\"3434\", MODE=\"0660\", GROUP=\"input\"' \
             | sudo tee /etc/udev/rules.d/99-keychron.rules\n  \
             sudo udevadm control --reload-rules && sudo udevadm trigger --action=add",
            info.node
        )),
    }
}

#[cfg(test)]
#[path = "worker_test.rs"]
mod tests;
