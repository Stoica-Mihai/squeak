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
