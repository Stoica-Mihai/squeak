//! DPI presets read/write (cmd 0x06 read, 0x40 write). Port of `keycron/dpi.py`.
//! Every write reads back and confirms (never trust the ACK alone).

use crate::hid::{Device, HidError};

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
pub fn get_dpi(dev: &mut Device) -> Result<(u8, [u16; NUM_PRESETS]), HidError> {
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
pub fn set_dpi(dev: &mut Device, value: u16, index: usize) -> Result<[u16; NUM_PRESETS], HidError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_payload_layout() {
        // active=4, presets 400/800/1600/4250/6400 (LE16), count=5.
        let p = set_payload(4, &[400, 800, 1600, 4250, 6400]);
        assert_eq!(
            p,
            vec![
                4, 4, 4, // active ×3
                0x90, 0x01, // 400
                0x20, 0x03, // 800
                0x40, 0x06, // 1600
                0x9a, 0x10, // 4250
                0x00, 0x19, // 6400
                5, // count
            ]
        );
    }

    /// Live write round-trip (opt-in). Changes DPI + polling, confirms the
    /// read-back, then restores the originals:
    ///   cargo test live_write_roundtrip -- --ignored --nocapture
    #[test]
    #[ignore = "writes to a connected Keychron device (auto-restores)"]
    fn live_write_roundtrip() {
        use crate::hid::{Device, find_config};
        use crate::proto::polling;

        let info = find_config().expect("config device not found");
        let mut dev = Device::open(&info.node).expect("open hidraw");

        // DPI preset 0: change, confirm, restore.
        let (_, orig) = get_dpi(&mut dev).expect("get dpi");
        let test_val = if orig[0] == 1600 { 800 } else { 1600 };
        let after = set_dpi(&mut dev, test_val, 0).expect("set dpi");
        assert_eq!(after[0], test_val, "DPI write not confirmed");
        let restored = set_dpi(&mut dev, orig[0], 0).expect("restore dpi");
        assert_eq!(restored[0], orig[0], "DPI restore failed");
        eprintln!("DPI write+restore OK: {} → {test_val} → {}", orig[0], orig[0]);

        // Polling rate: change, confirm, restore.
        let orig_code = polling::get_rate_code(&mut dev).expect("get rate");
        let orig_hz = polling::hz_from_code(orig_code).expect("rate code -> hz");
        let test_hz = if orig_hz == 1000 { 500 } else { 1000 };
        assert_eq!(polling::set_rate(&mut dev, test_hz).unwrap(), test_hz);
        assert_eq!(polling::set_rate(&mut dev, orig_hz).unwrap(), orig_hz);
        eprintln!("polling write+restore OK: {orig_hz} → {test_hz} → {orig_hz}");
    }
}
