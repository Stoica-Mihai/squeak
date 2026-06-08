//! Find the Keychron config collection by scanning sysfs. Port of the
//! enumeration in `keycron/device.py` + `rust-poc`, pure-std (no libudev).

use std::fs;

use crate::hid::device::VID;

pub const USAGE_PAGE_CONFIG: u16 = 0xFFC1;

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub node: String,
    pub vid: u16,
    pub pid: u16,
    pub name: String,
    pub usage_page: u16,
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
        let Some((vid, pid)) = parse_hid_id(&uevent) else { continue };
        if vid != VID {
            continue;
        }
        let hid_name = parse_hid_name(&uevent);
        let desc = fs::read(format!("{dev}/report_descriptor")).unwrap_or_default();
        let usage_page = first_usage_page(&desc).unwrap_or(0);

        out.push(DeviceInfo {
            node: format!("/dev/{node_name}"),
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
    enumerate()
        .into_iter()
        .find(|d| d.usage_page == USAGE_PAGE_CONFIG)
}

/// `HID_ID=0003:00003434:0000D028` -> (vid, pid).
fn parse_hid_id(uevent: &str) -> Option<(u16, u16)> {
    let line = uevent.lines().find(|l| l.starts_with("HID_ID="))?;
    let value = line.trim_start_matches("HID_ID=");
    let mut parts = value.split(':');
    let _bus = parts.next()?;
    let vid = u32::from_str_radix(parts.next()?, 16).ok()? as u16;
    let pid = u32::from_str_radix(parts.next()?, 16).ok()? as u16;
    Some((vid, pid))
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
mod tests {
    use super::*;

    #[test]
    fn parses_hid_id() {
        let ue = "HID_NAME=Keychron Ultra-Link 8K\nHID_ID=0003:00003434:0000D028\n";
        assert_eq!(parse_hid_id(ue), Some((0x3434, 0xD028)));
        assert_eq!(parse_hid_name(ue), "Keychron Ultra-Link 8K");
    }

    #[test]
    fn finds_usage_page() {
        // 06 C1 FF = Usage Page 0xFFC1, then 09 01 = Usage.
        let desc = [0x06, 0xC1, 0xFF, 0x09, 0x01];
        assert_eq!(first_usage_page(&desc), Some(0xFFC1));
    }
}
