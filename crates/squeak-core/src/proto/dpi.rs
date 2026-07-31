//! DPI presets read/write (cmd 0x06 read, 0x40 write). Port of `keycron/dpi.py`.
//! Every write reads back and confirms (never trust the ACK alone).

use crate::hid::{Hid, HidError};

const CMD_GET_DPI: u8 = 0x06;
const CMD_SET_DPI: u8 = 0x40;

pub const NUM_PRESETS: usize = 5;
pub const DPI_MIN: u16 = 50;
/// Client-side ceiling (8K sensor range). The device read-back is the real
/// guard — an out-of-range value is rejected and surfaced as an error.
pub const DPI_MAX: u16 = 26000;
pub const DPI_STEP: u16 = 50;

// GET reply (r[0]=0xB4): r[1]=0x06 echo, r[5]=active, presets LE16 @ r[6,8,10,12,14].
const ACTIVE_OFF: usize = 5;
const PRESET_OFF: usize = 6;

fn le16(lo: u8, hi: u8) -> u16 {
    lo as u16 | ((hi as u16) << 8)
}

/// Returns (active byte, 5 presets).
pub fn get_dpi(dev: &mut dyn Hid) -> Result<(u8, [u16; NUM_PRESETS]), HidError> {
    let r = dev.get(CMD_GET_DPI, &[])?;
    if r.len() < 2 || r[1] != CMD_GET_DPI {
        return Err(HidError::BadReply(format!("DPI get: unexpected reply {r:02x?}")));
    }
    if r.len() < PRESET_OFF + 2 * NUM_PRESETS {
        return Err(HidError::BadReply(format!("DPI get: short reply ({} bytes)", r.len())));
    }
    let active = r[ACTIVE_OFF];
    let mut presets = [0u16; NUM_PRESETS];
    for (i, p) in presets.iter_mut().enumerate() {
        *p = le16(r[PRESET_OFF + 2 * i], r[PRESET_OFF + 2 * i + 1]);
    }
    Ok((active, presets))
}

/// SET payload (after the cmd byte): `[active, active, active, 5×LE16, count]`.
fn set_payload(active: u8, presets: &[u16; NUM_PRESETS]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(3 + 2 * NUM_PRESETS + 1);
    payload.extend([active, active, active]);
    for &p in presets {
        payload.push((p & 0xff) as u8);
        payload.push((p >> 8) as u8);
    }
    payload.push(NUM_PRESETS as u8);
    payload
}

/// Set preset `index` to `value`, preserving the others. Re-reads to confirm.
/// Frame (Launcher 8k_nordic): `[0x40, active, active, active, 5×LE16, count]`.
pub fn set_dpi(dev: &mut dyn Hid, value: u16, index: usize) -> Result<[u16; NUM_PRESETS], HidError> {
    if !(DPI_MIN..=DPI_MAX).contains(&value) {
        return Err(HidError::BadReply(format!("DPI {value} out of range {DPI_MIN}..{DPI_MAX}")));
    }
    if index >= NUM_PRESETS {
        return Err(HidError::BadReply(format!("preset index {index} out of range")));
    }

    let (active, mut presets) = get_dpi(dev)?;
    presets[index] = value;

    let payload = set_payload(active, &presets);
    let (ok, resp) = dev.set(CMD_SET_DPI, &payload)?;
    if !ok {
        return Err(HidError::BadReply(format!("DPI set rejected: {resp:02x?}")));
    }

    let (_, after) = get_dpi(dev)?;
    if after[index] != value {
        return Err(HidError::BadReply(format!(
            "DPI set unconfirmed: wanted {value}, read {}",
            after[index]
        )));
    }
    Ok(after)
}

/// Switch the active DPI stage to preset `index` (preserving preset values).
/// The active byte is part of the 0x40 write; re-reads to confirm.
pub fn set_active(dev: &mut dyn Hid, index: usize) -> Result<u8, HidError> {
    if index >= NUM_PRESETS {
        return Err(HidError::BadReply(format!("preset index {index} out of range")));
    }
    let (_, presets) = get_dpi(dev)?;
    let payload = set_payload(index as u8, &presets);
    let (ok, resp) = dev.set(CMD_SET_DPI, &payload)?;
    if !ok {
        return Err(HidError::BadReply(format!("DPI active set rejected: {resp:02x?}")));
    }
    let (active, _) = get_dpi(dev)?;
    if active as usize != index {
        return Err(HidError::BadReply(format!(
            "DPI active unconfirmed: wanted {index}, read {active}"
        )));
    }
    Ok(active)
}

#[cfg(test)]
#[path = "dpi_test.rs"]
mod tests;
