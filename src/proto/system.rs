//! System params: debounce (0x43), sleep (0x0A), factory reset (0x0F). Port of
//! `keycron/settings.py`. Debounce/sleep read back and confirm.

use crate::hid::{Device, HidError};
use crate::proto::block::read_all;

const CMD_SLEEP: u8 = 0x0A;
const CMD_DEBOUNCE: u8 = 0x43;
const CMD_RESET: u8 = 0x0F;

pub fn set_debounce(dev: &mut Device, ms: u8) -> Result<u8, HidError> {
    let (ok, resp) = dev.set(CMD_DEBOUNCE, &[ms])?;
    if !ok {
        return Err(HidError::BadReply(format!("debounce set rejected: {resp:02x?}")));
    }
    let after = read_all(dev)?.debounce.value;
    if after != ms {
        return Err(HidError::BadReply(format!("debounce unconfirmed: wanted {ms}, read {after}")));
    }
    Ok(after)
}

/// Idle sleep timeout in MINUTES (Launcher range 1–240). Re-reads to confirm.
pub fn set_sleep(dev: &mut Device, minutes: u8) -> Result<u8, HidError> {
    let (ok, resp) = dev.set(CMD_SLEEP, &[1, minutes])?;
    if !ok {
        return Err(HidError::BadReply(format!("sleep set rejected: {resp:02x?}")));
    }
    let after = read_all(dev)?.sleep_min;
    if after != minutes {
        return Err(HidError::BadReply(format!("sleep unconfirmed: wanted {minutes}, read {after}")));
    }
    Ok(after)
}

/// Factory reset, all categories (DESTRUCTIVE). Payload `[255]`.
pub fn factory_reset(dev: &mut Device) -> Result<(), HidError> {
    let (ok, resp) = dev.set(CMD_RESET, &[255])?;
    if !ok {
        return Err(HidError::BadReply(format!("factory reset rejected: {resp:02x?}")));
    }
    Ok(())
}
