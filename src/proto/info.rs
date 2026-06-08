//! Dongle/firmware info. `[0xB3, 0x04]` -> `0xB4` reply carrying the firmware
//! version string (e.g. "0.1.6"), per FINDINGS.

use crate::hid::{Device, HidError};

const CMD_VERSION: u8 = 0x04;

/// Best-effort firmware version string ("?" if it can't be parsed).
pub fn read_version(dev: &mut Device) -> Result<String, HidError> {
    let r = dev.get(CMD_VERSION, &[])?;
    let v = match r.iter().position(|b| b.is_ascii_digit()) {
        Some(i) => r[i..]
            .iter()
            .take_while(|&&b| b.is_ascii_digit() || b == b'.')
            .map(|&b| b as char)
            .collect(),
        None => "?".to_string(),
    };
    Ok(v)
}
