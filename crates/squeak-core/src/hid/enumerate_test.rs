use super::*;

#[test]
fn parses_hid_id() {
    let ue = "HID_NAME=Keychron Ultra-Link 8K\nHID_ID=0003:00003434:0000D028\n";
    assert_eq!(parse_hid_id(ue), Some((BUS_USB, 0x3434, 0xD028)));
    assert_eq!(parse_hid_name(ue), "Keychron Ultra-Link 8K");
}

#[test]
fn parses_bluetooth_hid_id() {
    let ue = "HID_NAME=Keychron M6 8K\nHID_ID=0005:00003434:0000D049\n";
    assert_eq!(parse_hid_id(ue), Some((BUS_BLUETOOTH, 0x3434, 0xD049)));
}

#[test]
fn finds_usage_page() {
    // 06 C1 FF = Usage Page 0xFFC1, then 09 01 = Usage.
    let desc = [0x06, 0xC1, 0xFF, 0x09, 0x01];
    assert_eq!(first_usage_page(&desc), Some(0xFFC1));
}

fn info(bus: u16, pid: u16, usage_page: u16) -> DeviceInfo {
    DeviceInfo {
        node: "/dev/hidraw0".into(),
        bus,
        vid: 0x3434,
        pid,
        name: "Keychron".into(),
        usage_page,
    }
}

#[test]
fn transport_separates_bluetooth_from_the_cable() {
    // The cable and the Bluetooth link report the same PID, so only the bus
    // distinguishes them.
    assert_eq!(info(BUS_USB, 0xD049, 0).transport(), "wired");
    assert_eq!(info(BUS_BLUETOOTH, 0xD049, 0).transport(), "Bluetooth");
    assert_eq!(info(BUS_USB, 0xD028, 0).transport(), "2.4 GHz");
    assert_eq!(info(BUS_BLUETOOTH, 0xD028, 0).transport(), "Bluetooth");
}

#[test]
fn is_config_requires_the_vendor_usage_page() {
    assert!(info(BUS_USB, 0xD028, USAGE_PAGE_CONFIG).is_config());
    assert!(!info(BUS_USB, 0xD028, 0x0001).is_config());
    // The Bluetooth node carries no config collection.
    assert!(!info(BUS_BLUETOOTH, 0xD049, 0x0001).is_config());
}

/// Live: print every VID-0x3434 node with its bus and derived transport, split
/// by whether it carries the config collection (opt-in):
///   cargo test live_transports -- --ignored --nocapture
#[test]
#[ignore = "reads /sys/class/hidraw on the host"]
fn live_transports() {
    for (label, set) in [("config-capable", find_all_config()), ("no config", find_non_config())] {
        println!("--- {label} ---");
        for d in set {
            println!("  {} bus={:#04x} pid={:04X} {}", d.node, d.bus, d.pid, d.transport());
        }
    }
}
