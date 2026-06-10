use super::*;

#[test]
fn set_payload_layout() {
    assert_eq!(set_payload(2), vec![2, 2, 0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn code_hz_roundtrip() {
    for (code, &hz) in RATES_HZ.iter().enumerate() {
        assert_eq!(code_from_hz(hz), Some(code as u8));
        assert_eq!(hz_from_code(code as u8), Some(hz));
    }
    assert_eq!(code_from_hz(250), None);
}
