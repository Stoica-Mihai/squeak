use super::*;

#[test]
fn click_event_bytes() {
    // left click = press(0x88) + release(0x08), code 1, no delay.
    assert_eq!(click_events(&[1]), vec![0x88, 1, 0, 0, 0x08, 1, 0, 0]);
}

#[test]
fn text_event_bytes() {
    // "a" -> key press(0x81)/release(0x01) of usage 0x04.
    assert_eq!(text_events("a").unwrap(), vec![0x81, 0x04, 0, 0, 0x01, 0x04, 0, 0]);
    assert!(text_events("é").is_err());
}

#[test]
fn frame_layout_single() {
    // one click (2 events) -> length 14, n_events 2.
    let ev = click_events(&[1]);
    let f = build_frame(7, &ev, 1, LOOP_STOP_ON_RELEASE);
    assert_eq!(&f[..12], &[0x54, 7, 0, 14, 0, 1, 1, 0x20, 0, 0, 2, 0]);
    assert_eq!(f.len(), 12 + ev.len());
    assert!(f.len() <= MAX_FRAME);
}

#[test]
fn chunk_seq_formula() {
    // 30 events -> frame 132 bytes -> chunked; seq = 1 + off/16.
    let ev = click_events(&[1; 15]);
    let f = build_frame(0, &ev, 1, LOOP_STOP_ON_RELEASE);
    assert!(f.len() > MAX_FRAME);
    let seqs: Vec<u8> = (0..f.len())
        .step_by(CHUNK_PAYLOAD)
        .map(|off| (1 + off / 16) as u8)
        .collect();
    assert_eq!(seqs, vec![1, 4, 8]); // off 0,59,118 -> 1, 4(59/16=3), 8(118/16=7)
}

/// Live: upload a click macro to a Mouse-type button, then restore its exact
/// original value (opt-in). Skips if no mouse-type button exists:
///   cargo test live_macro_roundtrip -- --ignored --nocapture
#[test]
#[ignore = "writes to a connected device (auto-restores exact value)"]
fn live_macro_roundtrip() {
    use crate::hid::{Device, find_config};
    use crate::proto::buttons::{COUNT, get_all, set_button};

    let info = find_config().expect("config device not found");
    let mut dev = Device::open(&info.node).expect("open hidraw");
    let Some(orig) = get_all(&mut dev, COUNT).unwrap().into_iter().find(|b| b.type_id == 1) else {
        eprintln!("no mouse-type button to test safely; skipping");
        return;
    };
    let id = orig.id;

    let after = set_macro(&mut dev, id, &click_events(&[1])).expect("set macro");
    assert_eq!(after.type_id, 4, "button should become Macro");

    let restored = set_button(&mut dev, id, orig.type_id, orig.data).expect("restore");
    assert_eq!(restored.data, orig.data);
    eprintln!("macro upload+restore OK on button {id}");
}

#[test]
fn frame_length_holds_at_the_event_limit() {
    // 6 + 4*62 = 254, the largest value the len byte can carry.
    let ev = click_events(&[1; MAX_EVENTS / 2]);
    assert_eq!(ev.len() / 4, MAX_EVENTS);
    let f = build_frame(0, &ev, 1, LOOP_STOP_ON_RELEASE);
    assert_eq!(f[3], 254, "len byte must not wrap");
    assert_eq!(f[10], MAX_EVENTS as u8);
}

#[test]
fn set_macro_rejects_past_the_event_limit() {
    use crate::proto::testhid::NoTraffic;
    // One event past the limit; NoTraffic panics if anything reaches the wire.
    let ev = click_events(&[1; MAX_EVENTS / 2 + 1]);
    assert!(ev.len() / 4 > MAX_EVENTS);
    assert!(set_macro(&mut NoTraffic, 0, &ev).is_err());
}

#[test]
fn set_macro_rejects_out_of_range_button_id() {
    use crate::proto::testhid::NoTraffic;
    assert!(set_macro(&mut NoTraffic, 200, &click_events(&[1])).is_err());
}
