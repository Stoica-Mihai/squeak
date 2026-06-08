//! Device profile switch (cmd 0x0E, short channel). Decoded from a usbmon
//! capture over the 2.4 GHz dongle: `[0x0E, index, count]`, index 0-based
//! (Launcher "profile 2" = index 1), count = number of profiles. Read-back
//! confirms via the 0x06 block's `profile.current`.

use crate::hid::{Device, HidError};
use crate::proto::block;

const CMD_PROFILE: u8 = 0x0E;

/// Switch to profile `index` (0-based). Re-reads to confirm.
pub fn set_profile(dev: &mut Device, index: u8) -> Result<u8, HidError> {
    let p = block::read_all(dev)?.profile;
    let count = if p.count == 0 { 5 } else { p.count };
    if index >= count {
        return Err(HidError::BadReply(format!("profile {index} out of range 0..{count}")));
    }

    let (ok, resp) = dev.set(CMD_PROFILE, &[index, count])?;
    if !ok {
        return Err(HidError::BadReply(format!("profile set rejected: {resp:02x?}")));
    }

    let after = block::read_all(dev)?.profile.current;
    if after != index {
        return Err(HidError::BadReply(format!(
            "profile set unconfirmed: wanted {index}, read {after}"
        )));
    }
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::{Device, find_config};

    /// Live: switch to another profile, confirm, switch back (opt-in):
    ///   cargo test live_profile_roundtrip -- --ignored --nocapture
    #[test]
    #[ignore = "switches the active device profile (restores)"]
    fn live_profile_roundtrip() {
        let info = find_config().expect("config device not found");
        let mut dev = Device::open(&info.node).expect("open hidraw");
        let orig = block::read_all(&mut dev).unwrap().profile.current;
        let other = if orig == 0 { 1 } else { 0 };

        assert_eq!(set_profile(&mut dev, other).unwrap(), other);
        assert_eq!(set_profile(&mut dev, orig).unwrap(), orig);
        eprintln!("profile {orig} → {other} → {orig} OK");
    }
}
