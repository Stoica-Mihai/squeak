use super::*;

/// Live: switch the active DPI stage, confirm, switch back (opt-in):
///   cargo test live_set_active -- --ignored --nocapture
#[test]
#[ignore = "switches the active DPI stage on a connected device (restores)"]
fn live_set_active() {
    use crate::hid::{Device, find_config};
    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");
    let (orig, _) = get_dpi(&mut dev).unwrap();
    let other = if orig == 0 { 1 } else { 0 };
    assert_eq!(set_active(&mut dev, other).unwrap(), other as u8);
    assert_eq!(set_active(&mut dev, orig as usize).unwrap(), orig);
    eprintln!("active stage {orig} → {other} → {orig} OK");
}

#[test]
fn set_payload_layout() {
    // active=4, presets 400/800/1600/4250/6400 (LE16), count=5.
    let p = set_payload(4, &[400, 800, 1600, 4250, 6400]);
    assert_eq!(
        p,
        vec![
            4, 4, 4, // active ×3
            0x90, 0x01, // 400
            0x20, 0x03, // 800
            0x40, 0x06, // 1600
            0x9a, 0x10, // 4250
            0x00, 0x19, // 6400
            5, // count
        ]
    );
}

/// Live write round-trip (opt-in). Changes DPI + polling, confirms the
/// read-back, then restores the originals:
///   cargo test live_write_roundtrip -- --ignored --nocapture
#[test]
#[ignore = "writes to a connected Keychron device (auto-restores)"]
fn live_write_roundtrip() {
    use crate::hid::{Device, find_config};
    use crate::proto::polling;

    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");

    // DPI preset 0: change, confirm, restore.
    let (_, orig) = get_dpi(&mut dev).expect("get dpi");
    let test_val = if orig[0] == 1600 { 800 } else { 1600 };
    let after = set_dpi(&mut dev, test_val, 0).expect("set dpi");
    assert_eq!(after[0], test_val, "DPI write not confirmed");
    let restored = set_dpi(&mut dev, orig[0], 0).expect("restore dpi");
    assert_eq!(restored[0], orig[0], "DPI restore failed");
    eprintln!("DPI write+restore OK: {} → {test_val} → {}", orig[0], orig[0]);

    // Polling rate: change, confirm, restore.
    let orig_code = polling::get_rate_code(&mut dev).expect("get rate");
    let orig_hz = polling::hz_from_code(orig_code).expect("rate code -> hz");
    let test_hz = if orig_hz == 1000 { 500 } else { 1000 };
    assert_eq!(polling::set_rate(&mut dev, test_hz).unwrap(), test_hz);
    assert_eq!(polling::set_rate(&mut dev, orig_hz).unwrap(), orig_hz);
    eprintln!("polling write+restore OK: {orig_hz} → {test_hz} → {orig_hz}");
}
