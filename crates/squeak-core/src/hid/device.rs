//! Open a hidraw node and exchange Keychron command frames. Port of
//! `keycron/device.py`. Two channels:
//!   long  0xB3 -> 0xB4 (63B)  READ/GET
//!   short 0xB5 -> 0xB6 (20B)  WRITE/SET; ack = [E4, status, cmd, …]
//!
//! The full transport is here; the write path (set/long_set/long_raw + short
//! constants) is exercised from M2 onward.
#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, poll};

use crate::hid::{Hid, HidError};

pub const VID: u16 = 0x3434;

const LONG_OUT: u8 = 0xB3;
const LONG_IN: u8 = 0xB4;
const LONG_LEN: usize = 63;
const SHORT_OUT: u8 = 0xB5;
const SHORT_IN: u8 = 0xB6;
const SHORT_LEN: usize = 20;

const SHORT_REPLY_MARK: u8 = 0xE4;
const STATUS_OK: u8 = 0x00;

const TIMEOUT: Duration = Duration::from_millis(1000);

pub struct Device {
    file: File,
}

impl Device {
    pub fn open(path: &str) -> Result<Self, HidError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| HidError::Open(path.to_string(), e))?;
        Ok(Device { file })
    }

    /// `[report_id, payload…]` zero-padded to `1 + payload_len`.
    fn frame(report_id: u8, payload_len: usize, payload: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + payload_len);
        buf.push(report_id);
        buf.extend_from_slice(payload);
        buf.resize(1 + payload_len, 0);
        buf
    }

    fn write_frame(&mut self, report_id: u8, payload_len: usize, payload: &[u8]) -> Result<(), HidError> {
        let buf = Self::frame(report_id, payload_len, payload);
        let n = self.file.write(&buf)?;
        if n != buf.len() {
            return Err(HidError::ShortWrite(n, buf.len()));
        }
        Ok(())
    }

    /// Read input reports until one with id == `want_id`, discarding noise
    /// (live mouse-input reports) in between. Bounded by `TIMEOUT`.
    fn read_until(&mut self, want_id: u8, payload_len: usize) -> Result<Vec<u8>, HidError> {
        let mut rbuf = vec![0u8; 1 + payload_len];
        let start = Instant::now();
        while start.elapsed() < TIMEOUT {
            let remaining = (TIMEOUT - start.elapsed()).as_millis().min(200) as i32;
            let mut fds = [PollFd::new(&self.file, PollFlags::IN)];
            let ready = poll(&mut fds, remaining).map_err(HidError::Poll)?;
            if ready == 0 {
                continue;
            }
            let n = self.file.read(&mut rbuf)?;
            if n > 0 && rbuf[0] == want_id {
                return Ok(rbuf[..n].to_vec());
            }
        }
        Err(HidError::Timeout(want_id))
    }

}

impl Hid for Device {
    fn get(&mut self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>, HidError> {
        let mut p = Vec::with_capacity(1 + payload.len());
        p.push(cmd);
        p.extend_from_slice(payload);
        self.write_frame(LONG_OUT, LONG_LEN, &p)?;
        self.read_until(LONG_IN, LONG_LEN)
    }

    fn set(&mut self, cmd: u8, payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        let mut p = Vec::with_capacity(1 + payload.len());
        p.push(cmd);
        p.extend_from_slice(payload);
        self.write_frame(SHORT_OUT, SHORT_LEN, &p)?;
        let resp = self.read_until(SHORT_IN, SHORT_LEN)?;
        let ok = resp.len() >= 4 && resp[1] == SHORT_REPLY_MARK && resp[2] == STATUS_OK;
        Ok((ok, resp))
    }

    fn long_set(&mut self, cmd: u8, payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        let mut p = Vec::with_capacity(1 + payload.len());
        p.push(cmd);
        p.extend_from_slice(payload);
        self.write_frame(LONG_OUT, LONG_LEN, &p)?;
        let resp = self.read_until(SHORT_IN, SHORT_LEN)?;
        let ok = resp.len() >= 4
            && resp[1] == SHORT_REPLY_MARK
            && resp[2] == STATUS_OK
            && resp[3] == cmd;
        Ok((ok, resp))
    }

    fn long_raw(&mut self, payload: &[u8]) -> Result<Vec<u8>, HidError> {
        self.write_frame(LONG_OUT, LONG_LEN, payload)?;
        self.read_until(SHORT_IN, SHORT_LEN)
    }
}
