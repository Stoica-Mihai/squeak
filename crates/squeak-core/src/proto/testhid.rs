//! Hid stubs for guard tests: prove a rejection happens before the wire, and
//! that a device replying with too few bytes is an error rather than a panic.

use crate::hid::{Hid, HidError};

/// Panics on any transfer — assert a guard rejected the call before this ran.
pub(crate) struct NoTraffic;

impl Hid for NoTraffic {
    fn get(&mut self, cmd: u8, _payload: &[u8]) -> Result<Vec<u8>, HidError> {
        panic!("guard let a get(0x{cmd:02x}) reach the device");
    }
    fn set(&mut self, cmd: u8, _payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        panic!("guard let a set(0x{cmd:02x}) reach the device");
    }
    fn long_set(&mut self, cmd: u8, _payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        panic!("guard let a long_set(0x{cmd:02x}) reach the device");
    }
    fn long_raw(&mut self, _payload: &[u8]) -> Result<Vec<u8>, HidError> {
        panic!("guard let a long_raw reach the device");
    }
}

/// Answers the first `full_reads` gets with a well-formed DPI block, then
/// truncates every later reply to `short_len`. Models the device that passes the
/// initial read and only short-answers a later confirm read.
pub(crate) struct ShortAfter {
    pub full_reads: usize,
    pub short_len: usize,
    pub seen: usize,
}

impl Hid for ShortAfter {
    fn get(&mut self, cmd: u8, _payload: &[u8]) -> Result<Vec<u8>, HidError> {
        self.seen += 1;
        if self.seen <= self.full_reads {
            // 0xB4, cmd echo, active at [5], five LE16 presets from [6].
            let mut r = vec![0u8; 20];
            r[0] = 0xB4;
            r[1] = cmd;
            return Ok(r);
        }
        let mut r = vec![0u8; self.short_len];
        if self.short_len > 0 {
            r[0] = 0xB4;
        }
        if self.short_len > 1 {
            r[1] = cmd;
        }
        Ok(r)
    }
    fn set(&mut self, cmd: u8, _payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        Ok((true, vec![0xB6, 0xE4, 0, cmd]))
    }
    fn long_set(&mut self, cmd: u8, _payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        Ok((true, vec![0x00, 0xE4, 0, cmd]))
    }
    fn long_raw(&mut self, _payload: &[u8]) -> Result<Vec<u8>, HidError> {
        Ok(vec![0x00, 0xE4, 0x00])
    }
}
