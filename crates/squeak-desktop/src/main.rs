//! squeak desktop — Tauri shell over `squeak-core`. The frontend (dist/) calls
//! the `overview` command, which connects to the dongle and returns a snapshot.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use squeak_core::hid::{Device, find_config};
use squeak_core::proto::{block, info, polling};

#[derive(Serialize)]
struct Overview {
    name: String,
    transport: String,
    firmware: String,
    battery: u8,
    charging: bool,
    dpi: Vec<u16>,
    dpi_active: usize,
    polling_hz: u32,
    lod: u8,
    scroll_inverted: bool,
    motion: bool,
    angle: i16,
    debounce: u8,
    sleep_min: u8,
}

/// Connect, read the 0x06 block + firmware, and return an Overview snapshot.
#[tauri::command]
fn overview() -> Result<Overview, String> {
    let info = find_config().ok_or("Keychron config device not found — plug in the dongle.")?;
    let mut dev = Device::open(&info.node).map_err(|e| e.to_string())?;
    let s = block::read_all(&mut dev).map_err(|e| e.to_string())?;
    let firmware = info::read_version(&mut dev).unwrap_or_else(|_| "?".into());

    let transport = if info.pid >= 0xD000 { "2.4 GHz" } else { "wired" };
    Ok(Overview {
        name: dedupe_words(&info.name),
        transport: transport.into(),
        firmware,
        battery: s.battery.percent.min(100),
        charging: s.battery.charging,
        dpi: s.dpi.presets.to_vec(),
        dpi_active: s.dpi.active_levels[0] as usize,
        polling_hz: polling::hz_from_code(s.polling.levels[0]).unwrap_or(0),
        lod: s.sensor.lod,
        scroll_inverted: s.sensor.scroll_dir == 1,
        motion: s.sensor.motion_sync == 1,
        angle: s.sensor.angle,
        debounce: s.debounce.value,
        sleep_min: s.sleep_min,
    })
}

/// Collapse consecutive duplicate words ("Keychron Keychron …").
fn dedupe_words(s: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for w in s.split_whitespace() {
        if out.last() != Some(&w) {
            out.push(w);
        }
    }
    out.join(" ")
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![overview])
        .run(tauri::generate_context!())
        .expect("error while running squeak desktop");
}
