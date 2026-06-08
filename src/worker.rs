//! Dedicated device-I/O thread (PLAN §7). The UI thread sends `Cmd`, receives
//! `Update`, and never blocks on hidraw. The worker (re)connects lazily so a
//! replug + refresh recovers without restarting.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use crate::hid::{Device, DeviceInfo, find_config};
use crate::proto::block::{self, Settings};
use crate::proto::{self, Variant};

pub enum Cmd {
    ReadAll,
    Shutdown,
}

pub enum Update {
    Connected { name: String, variant: Variant },
    Settings(Box<Settings>),
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
        match cmd {
            Cmd::Shutdown => break,
            Cmd::ReadAll => {
                if dev.is_none() {
                    match connect() {
                        Ok((info, d)) => {
                            let variant = proto::detect(info.usage_page);
                            let name = if info.name.is_empty() {
                                format!("Keychron {:04x}:{:04x}", info.vid, info.pid)
                            } else {
                                info.name
                            };
                            if update_tx.send(Update::Connected { name, variant }).is_err() {
                                break;
                            }
                            dev = Some(d);
                        }
                        Err(e) => {
                            if update_tx.send(Update::Error(e)).is_err() {
                                break;
                            }
                            continue;
                        }
                    }
                }
                let d = dev.as_mut().unwrap();
                match block::read_all(d) {
                    Ok(s) => {
                        if update_tx.send(Update::Settings(Box::new(s))).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        dev = None; // drop; reconnect on next ReadAll
                        if update_tx.send(Update::Error(format!("read failed: {e}"))).is_err() {
                            break;
                        }
                    }
                }
            }
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
