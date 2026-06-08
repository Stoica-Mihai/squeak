//! Sensor params (cmd 0x42). Port of `keycron/settings.py` set_sensor/set_angle.
//! Encoding: each field 0 = "leave unchanged", on = 1, off = 2 (Launcher `x||2`);
//! scroll_dir/fps20k are `bit+1`. We resolve unspecified fields from the current
//! block so a single-field change preserves the rest, then read back to confirm.

use crate::hid::{Device, HidError};
use crate::proto::block::read_all;

const CMD_SENSOR: u8 = 0x42;

/// Optional sensor fields; `None` = keep the device's current value.
#[derive(Default, Clone, Copy)]
pub struct SensorFields {
    pub lod: Option<u8>,
    pub scroll_dir: Option<u8>,
    pub motion: Option<u8>,
    pub wave: Option<u8>,
    pub line: Option<u8>,
    pub fps20k: Option<u8>,
}

fn bit_or2(v: u8) -> u8 {
    if v != 0 { v } else { 2 }
}

/// 10-byte SET payload for the resolved (fully-specified) sensor fields.
fn sensor_payload(lod: u8, wave: u8, line: u8, motion: u8, scroll_dir: u8, fps20k: u8) -> [u8; 10] {
    let mut p = [0u8; 10];
    p[0] = lod;
    p[1] = bit_or2(wave);
    p[2] = bit_or2(line);
    p[3] = bit_or2(motion);
    p[5] = (scroll_dir & 1) + 1;
    p[7] = (fps20k & 1) + 1;
    p
}

pub fn set_sensor(dev: &mut Device, f: SensorFields) -> Result<(), HidError> {
    let s = read_all(dev)?.sensor;
    let lod = f.lod.unwrap_or(s.lod);
    let wave = f.wave.unwrap_or(s.wave);
    let line = f.line.unwrap_or(s.line);
    let motion = f.motion.unwrap_or(s.motion_sync);
    let scroll_dir = f.scroll_dir.unwrap_or(s.scroll_dir);
    let fps20k = f.fps20k.unwrap_or(s.fps20k);

    let payload = sensor_payload(lod, wave, line, motion, scroll_dir, fps20k);
    let (ok, resp) = dev.set(CMD_SENSOR, &payload)?;
    if !ok {
        return Err(HidError::BadReply(format!("sensor set rejected: {resp:02x?}")));
    }

    let after = read_all(dev)?.sensor;
    if after.lod != lod
        || after.wave != wave
        || after.line != line
        || after.motion_sync != motion
        || after.scroll_dir != scroll_dir
        || after.fps20k != fps20k
    {
        return Err(HidError::BadReply("sensor set unconfirmed on read-back".into()));
    }
    Ok(())
}

/// Angle snapping (cmd 0x42, alt fields). Returns the read-back angle.
fn angle_payload(angle: u8, enable: bool) -> [u8; 10] {
    let mut p = [0u8; 10];
    p[8] = if enable { 2 } else { 0 };
    p[9] = angle;
    p
}

pub fn set_angle(dev: &mut Device, angle: u8, enable: bool) -> Result<i16, HidError> {
    let payload = angle_payload(angle, enable);
    let (ok, resp) = dev.set(CMD_SENSOR, &payload)?;
    if !ok {
        return Err(HidError::BadReply(format!("angle set rejected: {resp:02x?}")));
    }
    Ok(read_all(dev)?.sensor.angle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensor_payload_encoding() {
        // lod=1, wave=on, line=on, motion=on, scroll=normal(0), fps=standard(0)
        assert_eq!(sensor_payload(1, 1, 1, 1, 0, 0), [1, 1, 1, 1, 0, 1, 0, 1, 0, 0]);
        // wave/line/motion off -> 2; scroll inverted(1)->2; fps competitive(1)->2
        assert_eq!(sensor_payload(2, 0, 0, 0, 1, 1), [2, 2, 2, 2, 0, 2, 0, 2, 0, 0]);
    }

    #[test]
    fn angle_payload_encoding() {
        assert_eq!(angle_payload(5, true), [0, 0, 0, 0, 0, 0, 0, 0, 2, 5]);
        assert_eq!(angle_payload(0, false), [0; 10]);
    }

    /// Live: toggle each boolean sensor field and restore (opt-in). Does NOT
    /// touch debounce/sleep (device-specific valid sets) or factory reset:
    ///   cargo test live_sensor_roundtrip -- --ignored --nocapture
    #[test]
    #[ignore = "writes to a connected device (auto-restores; never resets)"]
    fn live_sensor_roundtrip() {
        use crate::hid::{Device, find_config};

        let info = find_config().expect("config device not found");
        let mut dev = Device::open(&info.node).expect("open hidraw");
        let s0 = read_all(&mut dev).unwrap().sensor;

        // motion_sync
        set_sensor(&mut dev, SensorFields { motion: Some(1 - s0.motion_sync), ..Default::default() }).unwrap();
        assert_eq!(read_all(&mut dev).unwrap().sensor.motion_sync, 1 - s0.motion_sync);
        set_sensor(&mut dev, SensorFields { motion: Some(s0.motion_sync), ..Default::default() }).unwrap();

        // scroll_dir
        set_sensor(&mut dev, SensorFields { scroll_dir: Some(1 - s0.scroll_dir), ..Default::default() }).unwrap();
        assert_eq!(read_all(&mut dev).unwrap().sensor.scroll_dir, 1 - s0.scroll_dir);
        set_sensor(&mut dev, SensorFields { scroll_dir: Some(s0.scroll_dir), ..Default::default() }).unwrap();

        // fps20k (sampling mode)
        set_sensor(&mut dev, SensorFields { fps20k: Some(1 - s0.fps20k), ..Default::default() }).unwrap();
        assert_eq!(read_all(&mut dev).unwrap().sensor.fps20k, 1 - s0.fps20k);
        set_sensor(&mut dev, SensorFields { fps20k: Some(s0.fps20k), ..Default::default() }).unwrap();

        let restored = read_all(&mut dev).unwrap().sensor;
        assert_eq!(restored.motion_sync, s0.motion_sync);
        assert_eq!(restored.scroll_dir, s0.scroll_dir);
        assert_eq!(restored.fps20k, s0.fps20k);
        eprintln!(
            "sensor write+restore OK (motion {}, scroll_dir {}, fps20k {})",
            s0.motion_sync, s0.scroll_dir, s0.fps20k
        );
    }
}
