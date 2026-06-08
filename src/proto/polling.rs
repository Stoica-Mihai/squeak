//! Polling-rate read/write (cmd 0x06 read, 0x41 write). Port of
//! `keycron/polling.py`. Launcher "Levels: 6": 0=125 1=500 2=1000 3=2000
//! 4=4000 5=8000 (no 250). Every write reads back and confirms.

use crate::hid::{Device, HidError};

pub const RATES_HZ: [u32; 6] = [125, 500, 1000, 2000, 4000, 8000];

const CMD_GET_BLOCK: u8 = 0x06;
const CMD_SET_RATE: u8 = 0x41;
/// Constant tail the Launcher appends to the SET frame.
const TAIL: [u8; 6] = [1, 2, 3, 4, 5, 6];
/// r[3]: high nibble = rate code, low nibble = active profile.
const RATE_OFF: usize = 3;

pub fn hz_from_code(code: u8) -> Option<u32> {
    RATES_HZ.get(code as usize).copied()
}

pub fn code_from_hz(hz: u32) -> Option<u8> {
    RATES_HZ.iter().position(|&h| h == hz).map(|i| i as u8)
}

pub fn get_rate_code(dev: &mut Device) -> Result<u8, HidError> {
    let r = dev.get(CMD_GET_BLOCK, &[])?;
    if r.len() <= RATE_OFF || r[1] != CMD_GET_BLOCK {
        return Err(HidError::BadReply(format!("polling get: unexpected reply {r:02x?}")));
    }
    Ok(r[RATE_OFF] >> 4)
}

/// SET payload (after the cmd byte): `[code, code, 0x00, 1,2,3,4,5,6]`.
fn set_payload(code: u8) -> Vec<u8> {
    let mut payload = vec![code, code, 0x00];
    payload.extend(TAIL);
    payload
}

/// Set polling rate to `hz`. Re-reads to confirm.
pub fn set_rate(dev: &mut Device, hz: u32) -> Result<u32, HidError> {
    let code = code_from_hz(hz)
        .ok_or_else(|| HidError::BadReply(format!("unsupported rate {hz}; choose {RATES_HZ:?}")))?;

    let payload = set_payload(code);
    let (ok, resp) = dev.set(CMD_SET_RATE, &payload)?;
    if !ok {
        return Err(HidError::BadReply(format!("polling set rejected: {resp:02x?}")));
    }

    let after = get_rate_code(dev)?;
    if after != code {
        return Err(HidError::BadReply(format!(
            "polling set unconfirmed: wanted code {code}, read {after}"
        )));
    }
    Ok(hz)
}

#[cfg(test)]
mod tests {
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
}
