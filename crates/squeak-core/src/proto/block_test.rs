use super::*;

fn sample() -> Vec<u8> {
    sample_block()
}

#[test]
fn parses_known_offsets() {
    let s = parse(&sample()).unwrap();
    assert_eq!(s.profile.current, 2);
    assert_eq!(s.profile.count, 5);
    assert_eq!(s.dpi.presets, [400, 800, 1600, 4250, 5000]);
    assert_eq!(s.dpi.active_levels, [3, 4, 0]);
    assert_eq!(s.dpi.count, 5);
    assert_eq!(s.dpi.step, 50);
    assert_eq!(s.polling.levels, [1, 0, 0]);
    assert_eq!(s.polling.rate_codes, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(s.sensor.lod, 2);
    assert_eq!(s.sensor.motion_sync, 1);
    assert_eq!(s.sensor.scroll_dir, 1);
    assert_eq!(s.sensor.fps20k, 1);
    assert_eq!(s.sensor.angle, 10);
    assert_eq!(s.debounce.value, 10);
    assert_eq!(s.sleep_min, 30);
    assert_eq!(s.battery.percent, 85);
    assert!(s.battery.charging);
    assert!(s.wake.key && s.wake.mv && s.wake.side_scroll && !s.wake.scroll);
}

#[test]
fn rejects_short_body() {
    assert!(parse(&[0x06, 0x00]).is_err());
}

#[test]
fn polling_hz_maps() {
    assert_eq!(crate::proto::polling::hz_from_code(5), Some(8000));
    assert_eq!(crate::proto::polling::hz_from_code(1), Some(500));
    assert_eq!(crate::proto::polling::hz_from_code(9), None);
}

/// Live round-trip against real hardware (opt-in):
///   cargo test live_read -- --ignored --nocapture
#[test]
#[ignore = "requires a connected Keychron device"]
fn live_read() {
    use crate::hid::{Device, find_config};
    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");
    let s = read_all(&mut dev).expect("read block");
    assert!(s.battery.percent <= 100, "battery {} > 100", s.battery.percent);
    assert!(s.dpi.presets.iter().all(|&p| p <= 26000), "preset out of range: {:?}", s.dpi.presets);
    let fw = crate::proto::info::read_version(&mut dev).unwrap_or_else(|_| "?".into());
    eprintln!("live device: {} ({:04x}:{:04x}) fw {fw}\n{s:#?}", info.name, info.vid, info.pid);
}
