use super::*;

#[test]
fn sensor_payload_encoding() {
    // lod=1, wave=on, line=on, motion=on, scroll=normal(0), fps=standard(0)
    assert_eq!(sensor_payload(1, 1, 1, 1, 0, 0), [1, 1, 1, 1, 0, 1, 0, 1, 0, 0]);
    // wave/line/motion off -> 2; scroll inverted(1)->2; fps competitive(1)->2
    assert_eq!(sensor_payload(2, 0, 0, 0, 1, 1), [2, 2, 2, 2, 0, 2, 0, 2, 0, 0]);
}

#[test]
fn angle_payload_encoding() {
    assert_eq!(angle_payload(5, true), [0, 0, 0, 0, 0, 0, 0, 0, 2, 5]);
    // off = angle 0 (p[8] stays the 2 "edit angle" marker)
    assert_eq!(angle_payload(0, false), [0, 0, 0, 0, 0, 0, 0, 0, 2, 0]);
}

/// Live: enable angle snapping, confirm, turn off, restore (opt-in).
#[test]
#[ignore = "writes to a connected device (auto-restores)"]
fn live_angle_roundtrip() {
    use crate::hid::{Device, find_config};
    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");
    let orig = read_all(&mut dev).unwrap().sensor.angle;

    assert_eq!(set_angle(&mut dev, 15, true).unwrap(), 15, "enable 15° failed");
    assert_eq!(set_angle(&mut dev, 0, false).unwrap(), 0, "turn off failed");
    if orig != 0 {
        set_angle(&mut dev, orig.unsigned_abs().min(90) as u8, true).unwrap();
    }
    eprintln!("angle 15° → off → restore({orig}) OK");
}

/// Live: toggle each boolean sensor field and restore (opt-in). Does NOT
/// touch debounce/sleep (device-specific valid sets) or factory reset:
///   cargo test live_sensor_roundtrip -- --ignored --nocapture
#[test]
#[ignore = "writes to a connected device (auto-restores; never resets)"]
fn live_sensor_roundtrip() {
    use crate::hid::{Device, find_config};

    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");
    let s0 = read_all(&mut dev).unwrap().sensor;

    // motion_sync
    set_sensor(&mut dev, SensorFields { motion: Some(1 - s0.motion_sync), ..Default::default() }).unwrap();
    assert_eq!(read_all(&mut dev).unwrap().sensor.motion_sync, 1 - s0.motion_sync);
    set_sensor(&mut dev, SensorFields { motion: Some(s0.motion_sync), ..Default::default() }).unwrap();

    // scroll_dir
    set_sensor(&mut dev, SensorFields { scroll_dir: Some(1 - s0.scroll_dir), ..Default::default() }).unwrap();
    assert_eq!(read_all(&mut dev).unwrap().sensor.scroll_dir, 1 - s0.scroll_dir);
    set_sensor(&mut dev, SensorFields { scroll_dir: Some(s0.scroll_dir), ..Default::default() }).unwrap();

    // fps20k (sampling mode)
    set_sensor(&mut dev, SensorFields { fps20k: Some(1 - s0.fps20k), ..Default::default() }).unwrap();
    assert_eq!(read_all(&mut dev).unwrap().sensor.fps20k, 1 - s0.fps20k);
    set_sensor(&mut dev, SensorFields { fps20k: Some(s0.fps20k), ..Default::default() }).unwrap();

    let restored = read_all(&mut dev).unwrap().sensor;
    assert_eq!(restored.motion_sync, s0.motion_sync);
    assert_eq!(restored.scroll_dir, s0.scroll_dir);
    assert_eq!(restored.fps20k, s0.fps20k);
    eprintln!(
        "sensor write+restore OK (motion {}, scroll_dir {}, fps20k {})",
        s0.motion_sync, s0.scroll_dir, s0.fps20k
    );
}
