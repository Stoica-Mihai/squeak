//! Dedicated device-I/O thread (PLAN §7). The UI thread sends `Cmd`, receives
//! `Update`, and never blocks on hidraw. The worker (re)connects lazily so a
//! replug + refresh recovers without restarting.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use crate::hid::{Device, DeviceInfo, find_config};
use crate::proto::block::{self, Settings};
use crate::proto::sensor::SensorFields;
use crate::proto::{self, Variant, dpi, polling, sensor, system};

pub enum Cmd {
    ReadAll,
    SetDpi { index: usize, value: u16 },
    SetRate { hz: u32 },
    SetSensor(SensorFields),
    SetAngle { degrees: u8, enable: bool },
    SetDebounce(u8),
    SetSleep(u8),
    FactoryReset,
    Shutdown,
}

pub enum Update {
    Connected { name: String, variant: Variant },
    Settings(Box<Settings>),
    /// Result of a write, after read-back. Drives the ✓/✗ status line.
    Written { ok: bool, msg: String },
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
    while let Ok(cmd) = cmd_rx.recv() {
        if matches!(cmd, Cmd::Shutdown) {
            break;
        }
        // Ensure connected before any device command.
        if dev.is_none() {
            match ensure_connected(&update_tx) {
                Ok(d) => dev = Some(d),
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
            Cmd::ReadAll => {
                let d = dev.as_mut().unwrap();
                match block::read_all(d) {
                    Ok(s) => send(&update_tx, Update::Settings(Box::new(s))),
                    Err(e) => {
                        dev = None; // drop; reconnect on next command
                        send(&update_tx, Update::Error(format!("read failed: {e}")))
                    }
                }
            }
            Cmd::SetDpi { index, value } => {
                let result = dpi::set_dpi(dev.as_mut().unwrap(), value, index)
                    .map(|_| format!("DPI preset {} → {value} ✓ verified", index + 1));
                report_write(&update_tx, &mut dev, result)
            }
            Cmd::SetRate { hz } => {
                let result = polling::set_rate(dev.as_mut().unwrap(), hz)
                    .map(|_| format!("polling → {hz} Hz ✓ verified"));
                report_write(&update_tx, &mut dev, result)
            }
            Cmd::SetSensor(fields) => {
                let result = sensor::set_sensor(dev.as_mut().unwrap(), fields)
                    .map(|_| "sensor ✓ verified".to_string());
                report_write(&update_tx, &mut dev, result)
            }
            Cmd::SetAngle { degrees, enable } => {
                let result = sensor::set_angle(dev.as_mut().unwrap(), degrees, enable).map(|a| {
                    if enable {
                        format!("angle snap → {a}° ✓ verified")
                    } else {
                        "angle snap off ✓ verified".to_string()
                    }
                });
                report_write(&update_tx, &mut dev, result)
            }
            Cmd::SetDebounce(ms) => {
                let result = system::set_debounce(dev.as_mut().unwrap(), ms)
                    .map(|v| format!("debounce → {v} ms ✓ verified"));
                report_write(&update_tx, &mut dev, result)
            }
            Cmd::SetSleep(secs) => {
                let result = system::set_sleep(dev.as_mut().unwrap(), secs)
                    .map(|v| format!("sleep → {v} s ✓ verified"));
                report_write(&update_tx, &mut dev, result)
            }
            Cmd::FactoryReset => {
                let result = system::factory_reset(dev.as_mut().unwrap())
                    .map(|_| "factory reset sent".to_string());
                report_write(&update_tx, &mut dev, result)
            }
        };
        if stop {
            break;
        }
    }
}

fn send(tx: &Sender<Update>, u: Update) -> bool {
    tx.send(u).is_err()
}

/// Connect, announcing it. Err(true) = channel closed (stop); Err(false) = retry later.
fn ensure_connected(tx: &Sender<Update>) -> Result<Device, bool> {
    match connect() {
        Ok((info, d)) => {
            let variant = proto::detect(info.usage_page);
            let name = if info.name.is_empty() {
                format!("Keychron {:04x}:{:04x}", info.vid, info.pid)
            } else {
                info.name
            };
            if send(tx, Update::Connected { name, variant }) {
                return Err(true);
            }
            Ok(d)
        }
        Err(e) => Err(send(tx, Update::Error(e))),
    }
}

/// Emit a Written status from a write result, then refresh the snapshot. On a
/// transport error, drop the device so the next command reconnects.
fn report_write(
    tx: &Sender<Update>,
    dev_slot: &mut Option<Device>,
    result: Result<String, crate::hid::HidError>,
) -> bool {
    let written = match &result {
        Ok(msg) => Update::Written { ok: true, msg: msg.clone() },
        Err(e) => Update::Written { ok: false, msg: e.to_string() },
    };
    if send(tx, written) {
        return true;
    }
    // Refresh from the device so the UI reflects the committed state.
    match block::read_all(dev_slot.as_mut().unwrap()) {
        Ok(s) => send(tx, Update::Settings(Box::new(s))),
        Err(e) => {
            *dev_slot = None;
            send(tx, Update::Error(format!("read failed: {e}")))
        }
    }
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
