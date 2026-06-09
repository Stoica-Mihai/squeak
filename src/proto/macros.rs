//! Macro upload (cmd 0x54, chunked via 0x71). Port of `keycron/macro.py`.
//! One 0x54 frame uploads the macro AND binds it to the button (type -> Macro).
//! Event = [flag, code, delay_lo, delay_hi], flag = press(0x80) | class.

use crate::hid::{Hid, HidError};
use crate::proto::buttons::{ButtonInfo, get_button};

const CMD_SET_MACRO: u8 = 0x54;
const CMD_CHUNK: u8 = 0x71;
const CHUNK_ACK: u8 = 0x72;
/// One 0xB3 report payload.
const MAX_FRAME: usize = 63;
/// Bytes of the 0x54 stream per chunk (3-byte 0x71 header + 59 = 62).
const CHUNK_PAYLOAD: usize = 59;

const CLASS_KEY: u8 = 1;
const CLASS_MOUSE: u8 = 8;
const PRESS: u8 = 0x80;
const TYPE_MACRO: u8 = 4;
const LOOP_STOP_ON_RELEASE: u8 = 1;

/// Mouse buttons for click macros (name, event code).
pub const MOUSE_PALETTE: [(&str, u8); 5] = [
    ("left", 1),
    ("right", 2),
    ("middle", 3),
    ("backward", 4),
    ("forward", 5),
];

// Minimal HID usage map for the text helper (lowercase letters/digits/space…).
fn keycode(ch: char) -> Option<u8> {
    Some(match ch {
        'a'..='z' => 0x04 + (ch as u8 - b'a'),
        '1'..='9' => 0x1e + (ch as u8 - b'1'),
        '0' => 0x27,
        ' ' => 0x2c,
        '\n' => 0x28,
        '\t' => 0x2b,
        '-' => 0x2d,
        '=' => 0x2e,
        _ => return None,
    })
}

fn event(class: u8, code: u8, press: bool, delay: u16) -> [u8; 4] {
    let flag = if press { PRESS | class } else { class };
    [flag, code, (delay & 0xff) as u8, (delay >> 8) as u8]
}

/// Click events (press + release) for each mouse button code.
pub fn click_events(codes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(codes.len() * 8);
    for &code in codes {
        out.extend(event(CLASS_MOUSE, code, true, 0));
        out.extend(event(CLASS_MOUSE, code, false, 0));
    }
    out
}

/// Tap events for a typed string. Errors on an unsupported character.
pub fn text_events(text: &str) -> Result<Vec<u8>, HidError> {
    let mut out = Vec::new();
    for ch in text.to_lowercase().chars() {
        let kc = keycode(ch)
            .ok_or_else(|| HidError::BadReply(format!("no keycode for {ch:?}")))?;
        out.extend(event(CLASS_KEY, kc, true, 0));
        out.extend(event(CLASS_KEY, kc, false, 0));
    }
    Ok(out)
}

/// The 0x54 frame: `[0x54, id, 0, len, 0, loopCount, loopType, 0x20, 0, 0,
/// n_events, 0, <events>]`, `len = 6 + 4*n_events`.
fn build_frame(id: u8, events: &[u8], loop_count: u8, loop_type: u8) -> Vec<u8> {
    let n = (events.len() / 4) as u8;
    let length = 6 + 4 * n;
    let mut f = vec![
        CMD_SET_MACRO, id, 0x00, length, 0x00, loop_count, loop_type, 0x20, 0x00, 0x00, n, 0x00,
    ];
    f.extend_from_slice(events);
    f
}

/// Upload `events` to `button_id` (binds the button to the macro). Re-reads to
/// confirm. Long frames are chunked via 0x71 (`seq = 1 + bytes_sent/16`).
pub fn set_macro(dev: &mut dyn Hid, button_id: u8, events: &[u8]) -> Result<ButtonInfo, HidError> {
    let frame = build_frame(button_id, events, 1, LOOP_STOP_ON_RELEASE);
    let length = frame[3];

    if frame.len() <= MAX_FRAME {
        let (ok, resp) = dev.long_set(frame[0], &frame[1..])?;
        if !ok {
            return Err(HidError::BadReply(format!("macro rejected: {resp:02x?}")));
        }
    } else {
        let mut off = 0;
        while off < frame.len() {
            let end = (off + CHUNK_PAYLOAD).min(frame.len());
            let chunk = &frame[off..end];
            let seq = (1 + off / 16) as u8;
            let mut payload = vec![CMD_CHUNK, seq, chunk.len() as u8];
            payload.extend_from_slice(chunk);
            let resp = dev.long_raw(&payload)?;
            if resp.len() < 3 || resp[1] != CHUNK_ACK || resp[2] != 0 {
                return Err(HidError::BadReply(format!("macro chunk @{off} rejected: {resp:02x?}")));
            }
            off += CHUNK_PAYLOAD;
        }
    }

    let after = get_button(dev, button_id)?;
    if after.type_id != TYPE_MACRO || after.data != length as u32 {
        return Err(HidError::BadReply(format!(
            "macro set unconfirmed (want len {length}): {after:?}"
        )));
    }
    Ok(after)
}

#[cfg(test)]
#[path = "macros_test.rs"]
mod tests;
