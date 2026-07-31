use super::*;

#[test]
fn be24_split() {
    assert_eq!(be24(0x010000), [0x01, 0x00, 0x00]);
    assert_eq!(be24(0x00fe00), [0x00, 0xfe, 0x00]);
}

#[test]
#[ignore = "dumps live button slots (read-only)"]
fn live_dump_buttons() {
    use crate::hid::{Device, find_config};
    let info = find_config().expect("device");
    let mut dev = Device::open(&info.node).expect("open");
    for b in get_all(&mut dev, COUNT).unwrap() {
        eprintln!("id {:2}  type {:2} ({:11})  data 0x{:06x}", b.id, b.type_id, type_name(b.type_id), b.data);
    }
}

#[test]
fn mouse_name_roundtrip() {
    assert_eq!(mouse_name(0x010000), Some("left"));
    assert_eq!(mouse_name(0x00fe00), Some("downScroll"));
    assert_eq!(mouse_name(0x123456), None);
}

/// Live: remap a Mouse-type button then restore its exact original value
/// (opt-in). Skips if no mouse-type button exists, to avoid wiping a custom
/// mapping we can't reproduce:
///   cargo test live_button_roundtrip -- --ignored --nocapture
#[test]
#[ignore = "writes to a connected device (auto-restores exact value)"]
fn live_button_roundtrip() {
    use crate::hid::{Device, find_config};

    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");
    let all = get_all(&mut dev, COUNT).expect("read buttons");

    let Some(orig) = all.into_iter().find(|b| b.type_id == TYPE_MOUSE) else {
        eprintln!("no mouse-type button to test safely; skipping");
        return;
    };
    let id = orig.id;
    let test_action = if orig.label == "left" { "right" } else { "left" };

    let after = set_mouse(&mut dev, id, test_action).expect("set mouse");
    assert_eq!(after.label, test_action);

    let restored = set_button(&mut dev, id, orig.type_id, orig.data).expect("restore");
    assert_eq!(restored.data, orig.data, "restore mismatch");
    eprintln!(
        "button {id} remap+restore OK: {} → {test_action} → {}",
        orig.label, restored.label
    );
}

#[test]
fn check_id_bounds() {
    assert!(check_id(0).is_ok());
    assert!(check_id(COUNT as u8 - 1).is_ok());
    assert!(check_id(COUNT as u8).is_err());
    assert!(check_id(u8::MAX).is_err());
}

#[test]
fn write_paths_reject_out_of_range_id() {
    use crate::proto::testhid::NoTraffic;
    // NoTraffic panics on any transfer, so returning Err proves the id was
    // rejected before a frame was built.
    let bad = COUNT as u8 + 184;
    assert!(set_button(&mut NoTraffic, bad, TYPE_MOUSE, 0x010000).is_err());
    assert!(set_mouse(&mut NoTraffic, bad, "left").is_err());
    assert!(set_media(&mut NoTraffic, bad, "Mute").is_err());
    assert!(disable(&mut NoTraffic, bad).is_err());
    assert!(restore_default(&mut NoTraffic, bad).is_err());
}
