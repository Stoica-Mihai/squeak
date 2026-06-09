use super::*;
use crate::hid::{Device, find_config};

/// Live: switch to another profile, confirm, switch back (opt-in):
///   cargo test live_profile_roundtrip -- --ignored --nocapture
#[test]
#[ignore = "switches the active device profile (restores)"]
fn live_profile_roundtrip() {
    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");
    let orig = block::read_all(&mut dev).unwrap().profile.current;
    let other = if orig == 0 { 1 } else { 0 };

    assert_eq!(set_profile(&mut dev, other).unwrap(), other);
    assert_eq!(set_profile(&mut dev, orig).unwrap(), orig);
    eprintln!("profile {orig} → {other} → {orig} OK");
}
