//! Parse the 8k_nordic settings block (long cmd 0x06). Port of
//! `keycron/block.py`; offsets verified live (fw 0.1.6), see docs/8k-nordic.md.
//! `b` is the reply body with the report id stripped, so `b[0] == 0x06`.
//!
//! The whole block is decoded here; the Sensor/Profiles/DPI screens (M2+)
//! surface the fields the Overview doesn't yet read.
#![allow(dead_code)]

use crate::hid::{Hid, HidError};

pub const CMD_GET_BLOCK: u8 = 0x06;

/// Smallest body we can fully parse (highest offset read is b[55]).
const MIN_BODY: usize = 56;

#[derive(Clone, Debug)]
pub struct Settings {
    pub profile: Profile,
    pub dpi: Dpi,
    pub polling: Polling,
    pub sensor: Sensor,
    pub debounce: Debounce,
    pub scroll: Scroll,
    pub sleep_min: u8,
    pub battery: Battery,
    pub wake: Wake,
    pub support_flags: u8,
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub current: u8,
    pub count: u8,
}

#[derive(Clone, Debug)]
pub struct Dpi {
    pub active_levels: [u8; 3],
    pub presets: [u16; 5],
    pub count: u8,
    pub max: u16,
    pub step: u8,
}

#[derive(Clone, Debug)]
pub struct Polling {
    pub levels: [u8; 3],
    pub rate_codes: Vec<u8>,
    pub count: u8,
}

#[derive(Clone, Debug)]
pub struct Sensor {
    pub lod: u8,
    pub wave: u8,
    pub line: u8,
    pub motion_sync: u8,
    pub scroll_dir: u8,
    pub fps20k: u8,
    pub angle: i16,
}

#[derive(Clone, Debug)]
pub struct Debounce {
    pub value: u8,
    pub values: [u8; 10],
}

#[derive(Clone, Debug)]
pub struct Scroll {
    pub speed: u8,
    pub inertia: u8,
    pub spl: u8,
}

#[derive(Clone, Debug)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Clone, Debug)]
pub struct Wake {
    pub key: bool,
    pub scroll: bool,
    pub mv: bool,
    pub side_scroll: bool,
}

fn le16(b: &[u8], o: usize) -> u16 {
    b[o] as u16 | ((b[o + 1] as u16) << 8)
}

pub fn parse(b: &[u8]) -> Result<Settings, HidError> {
    if b.len() < MIN_BODY {
        return Err(HidError::BadReply(format!(
            "block too short: {} bytes (need {MIN_BODY})",
            b.len()
        )));
    }
    let angle = if b[55] > 90 {
        b[55] as i16 - 256
    } else {
        b[55] as i16
    };
    Ok(Settings {
        profile: Profile {
            current: b[1],
            count: b[50],
        },
        dpi: Dpi {
            active_levels: [b[2] & 15, b[3] & 15, b[4] & 15],
            presets: [
                le16(b, 5),
                le16(b, 7),
                le16(b, 9),
                le16(b, 11),
                le16(b, 13),
            ],
            count: b[16],
            max: le16(b, 40),
            step: if b[42] == 0 { 50 } else { b[42] },
        },
        polling: Polling {
            levels: [b[2] >> 4, b[3] >> 4, b[4] >> 4],
            rate_codes: b[43..49].to_vec(),
            count: if b[49] == 0 { 6 } else { b[49] },
        },
        sensor: Sensor {
            lod: b[15] & 3,
            wave: (b[15] >> 2) & 1,
            line: (b[15] >> 3) & 1,
            motion_sync: (b[15] >> 4) & 1,
            scroll_dir: (b[15] >> 6) & 1,
            fps20k: b[52] & 1,
            angle,
        },
        debounce: Debounce {
            value: b[17],
            values: b[30..40].try_into().unwrap(),
        },
        scroll: Scroll {
            speed: b[27],
            inertia: b[28],
            spl: b[29],
        },
        sleep_min: b[18],
        battery: Battery {
            percent: b[19] & 127,
            charging: b[19] >> 7 == 1,
        },
        wake: Wake {
            key: (b[51] >> 4) & 1 == 1,
            scroll: (b[51] >> 5) & 1 == 1,
            mv: (b[51] >> 6) & 1 == 1,
            side_scroll: (b[51] >> 7) & 1 == 1,
        },
        support_flags: b[26],
    })
}

pub fn read_all(dev: &mut dyn Hid) -> Result<Settings, HidError> {
    let r = dev.get(CMD_GET_BLOCK, &[])?;
    if r.len() < 2 || r[1] != CMD_GET_BLOCK {
        return Err(HidError::BadReply(format!("block read: unexpected reply {r:02x?}")));
    }
    parse(&r[1..])
}

/// Synthetic 0x06 body matching docs/8k-nordic.md offsets. Not a captured frame
/// — locks the offset map and is reused as a fixture by app/ui tests.
#[cfg(test)]
pub(crate) fn sample_block() -> Vec<u8> {
    let mut b = vec![0u8; 63];
    b[0] = 0x06;
    b[1] = 2; // profile.current
    b[50] = 5; // profile.count
    b[2] = 0x13; // active level0 = 3, polling code = 1 (500 Hz)
    b[3] = 0x04; // active level1 = 4, polling code = 0
    b[4] = 0x00;
    // DPI presets 400/800/1600/4250/5000 (LE16)
    for (i, v) in [400u16, 800, 1600, 4250, 5000].iter().enumerate() {
        b[5 + 2 * i] = (*v & 0xff) as u8;
        b[6 + 2 * i] = (*v >> 8) as u8;
    }
    b[16] = 5; // dpi.count
    // sensor byte: lod=2, motion(bit4)=1, scroll_dir(bit6)=1
    b[15] = 0b0101_0010;
    b[17] = 10; // debounce
    b[18] = 30; // sleep_min
    b[19] = 0x80 | 85; // battery 85%, charging
    b[26] = 0x2f; // support flags
    b[42] = 0; // step -> defaults to 50
    b[43..49].copy_from_slice(&[0, 1, 2, 3, 4, 5]);
    b[49] = 6; // polling count
    b[51] = 0b1101_0000; // wake bits 4,6,7 set: key, mv, side_scroll (not scroll)
    b[52] = 1; // fps20k
    b[55] = 10; // angle
    b
}

/// Parsed fixture `Settings` for tests in other modules.
#[cfg(test)]
pub(crate) fn sample_settings() -> Settings {
    parse(&sample_block()).unwrap()
}

#[cfg(test)]
#[path = "block_test.rs"]
mod tests;
