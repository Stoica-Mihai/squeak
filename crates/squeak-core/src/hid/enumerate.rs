//! Find the Keychron config collection by scanning sysfs. Port of the
//! enumeration in `keycron/device.py` + `rust-poc`, pure-std (no libudev).

use std::fs;

use crate::hid::device::VID;

pub const USAGE_PAGE_CONFIG: u16 = 0xFFC1;

/// `bustype` from `HID_ID` (linux/input.h `BUS_*`). The cable and a Bluetooth
/// link report the same PID, so the bus is the only thing that separates them.
pub const BUS_USB: u16 = 0x03;
pub const BUS_BLUETOOTH: u16 = 0x05;

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub node: String,
    pub bus: u16,
    pub vid: u16,
    pub pid: u16,
    pub name: String,
    pub usage_page: u16,
}

impl DeviceInfo {
    /// Human label for how the mouse is attached.
    pub fn transport(&self) -> &'static str {
        match (self.bus, self.pid) {
            (BUS_BLUETOOTH, _) => "Bluetooth",
            // The Ultra-Link dongle is its own product (0xD028 = wireless); the
            // M6 enumerating directly (e.g. 0xD049) is the cable.
            (_, 0xD028) => "2.4 GHz",
            _ => "wired",
        }
    }

    /// Whether this node carries the vendor config collection.
    pub fn is_config(&self) -> bool {
        self.usage_page == USAGE_PAGE_CONFIG
    }
}

/// All VID-0x3434 hidraw nodes, each with its collection's usage page.
pub fn enumerate() -> Vec<DeviceInfo> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/hidraw") else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(node_name) = name.to_str() else { continue };
        let dev = format!("/sys/class/hidraw/{node_name}/device");

        let uevent = fs::read_to_string(format!("{dev}/uevent")).unwrap_or_default();
        let Some((bus, vid, pid)) = parse_hid_id(&uevent) else { continue };
        if vid != VID {
            continue;
        }
        let hid_name = parse_hid_name(&uevent);
        let desc = fs::read(format!("{dev}/report_descriptor")).unwrap_or_default();
        let usage_page = first_usage_page(&desc).unwrap_or(0);

        out.push(DeviceInfo {
            node: format!("/dev/{node_name}"),
            bus,
            vid,
            pid,
            name: hid_name,
            usage_page,
        });
    }
    out
}

/// The config endpoint: VID 0x3434 collection on usage page 0xFFC1.
pub fn find_config() -> Option<DeviceInfo> {
    enumerate().into_iter().find(DeviceInfo::is_config)
}

/// All config-collection candidates (e.g. dongle + wired both present). The
/// caller probes each, since an idle transport's node won't answer.
pub fn find_all_config() -> Vec<DeviceInfo> {
    enumerate().into_iter().filter(DeviceInfo::is_config).collect()
}

/// VID-0x3434 nodes that are present but expose no config collection — the
/// Bluetooth link is the case that matters: the mouse is usable over it, but
/// Keychron only carries the 0xFFC1 collection over the dongle and the cable.
pub fn find_non_config() -> Vec<DeviceInfo> {
    enumerate().into_iter().filter(|d| !d.is_config()).collect()
}

/// `HID_ID=0003:00003434:0000D028` -> (bus, vid, pid).
fn parse_hid_id(uevent: &str) -> Option<(u16, u16, u16)> {
    let line = uevent.lines().find(|l| l.starts_with("HID_ID="))?;
    let value = line.trim_start_matches("HID_ID=");
    let mut parts = value.split(':');
    let bus = u32::from_str_radix(parts.next()?.trim(), 16).ok()? as u16;
    let vid = u32::from_str_radix(parts.next()?, 16).ok()? as u16;
    let pid = u32::from_str_radix(parts.next()?, 16).ok()? as u16;
    Some((bus, vid, pid))
}

/// `HID_NAME=Keychron Ultra-Link 8K` -> the name (empty if absent).
fn parse_hid_name(uevent: &str) -> String {
    uevent
        .lines()
        .find(|l| l.starts_with("HID_NAME="))
        .map(|l| l.trim_start_matches("HID_NAME=").to_string())
        .unwrap_or_default()
}

/// First `06 LO HI` (Usage Page, 2-byte) item in a HID report descriptor.
fn first_usage_page(desc: &[u8]) -> Option<u16> {
    let mut i = 0;
    while i + 2 < desc.len() {
        if desc[i] == 0x06 {
            return Some(desc[i + 1] as u16 | ((desc[i + 2] as u16) << 8));
        }
        i += 1;
    }
    None
}

#[cfg(test)]
#[path = "enumerate_test.rs"]
mod tests;
