//! Button remap (cmd 0x52 set / 0x62 get). Port of `keycron/buttons.py`.
//! Action data is a 24-bit big-endian value whose meaning depends on `type`.
//! Type 0 = DEFAULT hardware function (not "off"); type 9 = Disable. Every
//! write reads back and confirms.

use crate::hid::{Hid, HidError};

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

/// Assignable Media (Consumer Control) actions: name -> consumer code.
pub const MEDIA_ACTIONS: [(&str, u8); 7] = [
    ("Vol +", 0xe9),
    ("Vol -", 0xea),
    ("Mute", 0xe2),
    ("Play/Pause", 0xcd),
    ("Next", 0xb5),
    ("Prev", 0xb6),
    ("Stop", 0xb7),
];

/// Media (Consumer Control) action from the high byte of the 24-bit data.
fn media_name(data: u32) -> Option<&'static str> {
    MEDIA_ACTIONS.iter().find(|(_, c)| *c as u32 == data >> 16).map(|(n, _)| *n)
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

pub fn get_button(dev: &mut dyn Hid, id: u8) -> Result<ButtonInfo, HidError> {
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
        // Firmware mirrors the side wheel into a second slot pair (editing 5/6
        // updates 10/11 and vice-versa) — same physical button.
        10 => Some("Side ↑ ⧉"),
        11 => Some("Side ↓ ⧉"),
        _ => None,
    }
}

pub fn get_all(dev: &mut dyn Hid, count: usize) -> Result<Vec<ButtonInfo>, HidError> {
    (0..count as u8).map(|id| get_button(dev, id)).collect()
}

/// Reject ids outside the device's button table. Every write path calls this
/// before a frame reaches the wire — firmware behaviour past `COUNT` is undefined.
pub fn check_id(id: u8) -> Result<(), HidError> {
    if id as usize >= COUNT {
        return Err(HidError::BadReply(format!(
            "button id {id} out of range 0..{COUNT}"
        )));
    }
    Ok(())
}

pub fn set_button(dev: &mut dyn Hid, id: u8, type_id: u8, data: u32) -> Result<ButtonInfo, HidError> {
    check_id(id)?;
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
pub fn set_mouse(dev: &mut dyn Hid, id: u8, action: &str) -> Result<ButtonInfo, HidError> {
    let data = MOUSE_ACTIONS
        .iter()
        .find(|(n, _)| *n == action)
        .map(|(_, v)| *v)
        .ok_or_else(|| HidError::BadReply(format!("unknown mouse action {action}")))?;
    set_button(dev, id, TYPE_MOUSE, data)
}

/// Assign a media action by name. Data = `(consumer_code << 16) | 0x00FF`
/// (matches the device's stored format, e.g. Vol+ = 0xe900ff).
pub fn set_media(dev: &mut dyn Hid, id: u8, action: &str) -> Result<ButtonInfo, HidError> {
    let code = MEDIA_ACTIONS
        .iter()
        .find(|(n, _)| *n == action)
        .map(|(_, c)| *c)
        .ok_or_else(|| HidError::BadReply(format!("unknown media action {action}")))?;
    set_button(dev, id, TYPE_MEDIA, ((code as u32) << 16) | 0xff)
}

/// Turn a button OFF (no action).
pub fn disable(dev: &mut dyn Hid, id: u8) -> Result<ButtonInfo, HidError> {
    set_button(dev, id, TYPE_DISABLE, 0)
}

/// Restore a button's default hardware function (type 0).
pub fn restore_default(dev: &mut dyn Hid, id: u8) -> Result<ButtonInfo, HidError> {
    check_id(id)?;
    let (ok, resp) = dev.long_set(CMD_SET_BUTTON, &[id, 0, 0, 0, 0, 0])?;
    if !ok {
        return Err(HidError::BadReply(format!("restore rejected: {resp:02x?}")));
    }
    get_button(dev, id)
}

#[cfg(test)]
#[path = "buttons_test.rs"]
mod tests;
