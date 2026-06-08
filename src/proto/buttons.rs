//! Button remap (cmd 0x52 set / 0x62 get). Port of `keycron/buttons.py`.
//! Action data is a 24-bit big-endian value whose meaning depends on `type`.
//! Type 0 = DEFAULT hardware function (not "off"); type 9 = Disable. Every
//! write reads back and confirms.

use crate::hid::{Device, HidError};

const CMD_SET_BUTTON: u8 = 0x52;
const CMD_GET_BUTTON: u8 = 0x62;

pub const COUNT: usize = 16;

const TYPE_MOUSE: u8 = 1;
const TYPE_MEDIA: u8 = 3;
const TYPE_DISABLE: u8 = 9;
const TYPE_DEFAULT: u8 = 0;

/// A Default-type slot with data 0 is an empty / non-physical slot; a real
/// default button carries non-zero data (0xffffff untouched, or a device
/// default code after a restore).
const EMPTY_DEFAULT_DATA: u32 = 0;

/// Action type enum `S` from the Launcher.
pub fn type_name(t: u8) -> &'static str {
    match t {
        0 => "Default",
        1 => "Mouse",
        2 => "Keyboard",
        3 => "Media",
        4 => "Macro",
        5 => "DPI",
        6 => "Light",
        7 => "Game",
        8 => "ShortCut",
        9 => "Disable",
        10 => "Profile",
        13 => "PollingRate",
        _ => "?",
    }
}

/// Mouse actions (enum `_`) as the 24-bit value, display order.
pub const MOUSE_ACTIONS: [(&str, u32); 10] = [
    ("left", 0x010000),
    ("right", 0x020000),
    ("middle", 0x040000),
    ("forward", 0x080000),
    ("backward", 0x100000),
    ("leftDouble", 0x800000),
    ("upScroll", 0x000200),
    ("downScroll", 0x00fe00),
    ("leftScroll", 0x0000fe),
    ("rightScroll", 0x000002),
];

fn mouse_name(data: u32) -> Option<&'static str> {
    MOUSE_ACTIONS.iter().find(|(_, v)| *v == data).map(|(n, _)| *n)
}

/// Media (Consumer Control) action from the high byte of the 24-bit data.
fn media_name(data: u32) -> Option<&'static str> {
    Some(match data >> 16 {
        0xe9 => "Vol +",
        0xea => "Vol -",
        0xe2 => "Mute",
        0xcd => "Play/Pause",
        0xb5 => "Next",
        0xb6 => "Prev",
        0xb7 => "Stop",
        _ => return None,
    })
}

#[derive(Clone, Debug)]
pub struct ButtonInfo {
    pub id: u8,
    pub type_id: u8,
    pub data: u32,
    /// Human label for the assignment (mouse action name, or the type name).
    pub label: String,
}

fn be24(v: u32) -> [u8; 3] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

pub fn get_button(dev: &mut Device, id: u8) -> Result<ButtonInfo, HidError> {
    let r = dev.get(CMD_GET_BUTTON, &[id])?;
    if r.len() < 8 || r[1] != CMD_GET_BUTTON {
        return Err(HidError::BadReply(format!("button get: unexpected reply {r:02x?}")));
    }
    let type_id = r[4];
    let data = ((r[5] as u32) << 16) | ((r[6] as u32) << 8) | r[7] as u32;
    Ok(ButtonInfo { id, type_id, data, label: label_for(type_id, data) })
}

fn label_for(type_id: u8, data: u32) -> String {
    match type_id {
        TYPE_MOUSE => mouse_name(data)
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("mouse 0x{data:06x}")),
        TYPE_MEDIA => media_name(data)
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("media 0x{:02x}", data >> 16)),
        TYPE_DISABLE => "disabled".to_string(),
        TYPE_DEFAULT if data == EMPTY_DEFAULT_DATA => "—".to_string(), // empty slot
        TYPE_DEFAULT => "default".to_string(),
        t => type_name(t).to_string(),
    }
}

/// Whether this slot is a real, configurable button (not an empty slot).
pub fn is_present(b: &ButtonInfo) -> bool {
    !(b.type_id == TYPE_DEFAULT && b.data == EMPTY_DEFAULT_DATA)
}

/// Verified physical name for a slot id, or None if not yet mapped.
///
/// The earlier inferred order (left,right,middle,…) was DISPROVEN live: id 1 is
/// the middle button, not right. Names are now only filled in as each id is
/// empirically confirmed (assign a macro, press the physical button, observe).
pub fn friendly_name(id: u8) -> Option<&'static str> {
    match id {
        0 => Some("Left"),   // confirmed live
        1 => Some("Middle"), // confirmed live
        2 => Some("Right"),  // confirmed live
        3 => Some("Forward"),  // confirmed: front thumb button (left side)
        4 => Some("Backward"), // confirmed live
        5 => Some("Side ↑"),   // confirmed: side-scroll up (default Vol +)
        6 => Some("Side ↓"),   // confirmed: side-scroll down (default Vol -)
        _ => None,
    }
}

pub fn get_all(dev: &mut Device, count: usize) -> Result<Vec<ButtonInfo>, HidError> {
    (0..count as u8).map(|id| get_button(dev, id)).collect()
}

pub fn set_button(dev: &mut Device, id: u8, type_id: u8, data: u32) -> Result<ButtonInfo, HidError> {
    let d = be24(data);
    let (ok, resp) = dev.long_set(CMD_SET_BUTTON, &[id, 0, type_id, d[0], d[1], d[2]])?;
    if !ok {
        return Err(HidError::BadReply(format!("button set rejected: {resp:02x?}")));
    }
    let after = get_button(dev, id)?;
    if after.type_id != type_id || after.data != data {
        return Err(HidError::BadReply(format!("button set unconfirmed: {after:?}")));
    }
    Ok(after)
}

/// Assign a mouse action by name. Returns the confirmed button state.
pub fn set_mouse(dev: &mut Device, id: u8, action: &str) -> Result<ButtonInfo, HidError> {
    let data = MOUSE_ACTIONS
        .iter()
        .find(|(n, _)| *n == action)
        .map(|(_, v)| *v)
        .ok_or_else(|| HidError::BadReply(format!("unknown mouse action {action}")))?;
    set_button(dev, id, TYPE_MOUSE, data)
}

/// Turn a button OFF (no action).
pub fn disable(dev: &mut Device, id: u8) -> Result<ButtonInfo, HidError> {
    set_button(dev, id, TYPE_DISABLE, 0)
}

/// Restore a button's default hardware function (type 0).
pub fn restore_default(dev: &mut Device, id: u8) -> Result<ButtonInfo, HidError> {
    let (ok, resp) = dev.long_set(CMD_SET_BUTTON, &[id, 0, 0, 0, 0, 0])?;
    if !ok {
        return Err(HidError::BadReply(format!("restore rejected: {resp:02x?}")));
    }
    get_button(dev, id)
}

#[cfg(test)]
mod tests {
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
}
