//! squeak desktop — Tauri shell over `squeak-core`.
//!
//! The real `squeak_core::worker` runs on its own thread (same verified
//! connect / write / read-back / reconnect logic as the TUI). Frontend commands
//! push `Cmd`s; the worker's `Update`s are bridged to Tauri events the web UI
//! listens for. So every write goes through the already-tested dispatch.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::mpsc::Sender;

use serde_json::json;
use tauri::{Emitter, Manager, State};

use squeak_core::proto::buttons::{
    MEDIA_ACTIONS, MOUSE_ACTIONS, friendly_name, is_present, type_name,
};
use squeak_core::proto::polling::RATES_HZ;
use squeak_core::proto::sensor::SensorFields;
use squeak_core::worker::{Cmd, Update, Worker};

struct AppState {
    tx: Sender<Cmd>,
}

impl AppState {
    fn send(&self, cmd: Cmd) -> Result<(), String> {
        self.tx.send(cmd).map_err(|_| "device worker is gone".to_string())
    }
}

// ---- commands (frontend → worker) -----------------------------------------

#[tauri::command]
fn read_all(s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::ReadAll)
}
#[tauri::command]
fn read_buttons(s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::ReadButtons)
}
#[tauri::command]
fn set_dpi(index: usize, value: u16, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetDpi { index, value })
}
#[tauri::command]
fn set_active_dpi(index: usize, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetActiveDpi(index))
}
#[tauri::command]
fn set_rate(hz: u32, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetRate { hz })
}
#[tauri::command]
fn set_lod(value: u8, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetSensor(SensorFields { lod: Some(value), ..Default::default() }))
}
#[tauri::command]
fn set_scroll(inverted: bool, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetSensor(SensorFields { scroll_dir: Some(inverted as u8), ..Default::default() }))
}
#[tauri::command]
fn set_motion(on: bool, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetSensor(SensorFields { motion: Some(on as u8), ..Default::default() }))
}
#[tauri::command]
fn set_fps20k(on: bool, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetSensor(SensorFields { fps20k: Some(on as u8), ..Default::default() }))
}
#[tauri::command]
fn set_angle(degrees: u8, enable: bool, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetAngle { degrees, enable })
}
#[tauri::command]
fn set_debounce(ms: u8, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetDebounce(ms))
}
#[tauri::command]
fn set_sleep(minutes: u8, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetSleep(minutes))
}
#[tauri::command]
fn set_profile(index: u8, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetProfile(index))
}
#[tauri::command]
fn set_button_mouse(id: u8, action: String, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetButtonMouse { id, action })
}
#[tauri::command]
fn set_button_media(id: u8, action: String, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetButtonMedia { id, action })
}
#[tauri::command]
fn set_button_disable(id: u8, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetButtonDisable(id))
}
#[tauri::command]
fn set_button_default(id: u8, s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::SetButtonDefault(id))
}
#[tauri::command]
fn check_update(s: State<AppState>) -> Result<(), String> {
    s.send(Cmd::CheckUpdate)
}

/// Static option lists for the button remap picker.
#[tauri::command]
fn palettes() -> serde_json::Value {
    json!({
        "mouse": MOUSE_ACTIONS.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        "media": MEDIA_ACTIONS.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        "rates": RATES_HZ,
    })
}

// ---- bridge (worker Update → Tauri event) ----------------------------------

fn emit_update(app: &tauri::AppHandle, u: Update) {
    match u {
        Update::Connected { name, firmware, transport, .. } => {
            let _ = app.emit("connected", json!({ "name": name, "firmware": firmware, "transport": transport }));
        }
        Update::Settings(s) => {
            let dto = json!({
                "profile": { "current": s.profile.current, "count": s.profile.count },
                "dpi": {
                    "presets": s.dpi.presets,
                    "active": s.dpi.active_levels[0],
                    "count": s.dpi.count,
                    "max": s.dpi.max,
                },
                "pollingHz": squeak_core::proto::polling::hz_from_code(s.polling.levels[0]).unwrap_or(0),
                "sensor": {
                    "lod": s.sensor.lod,
                    "scrollInverted": s.sensor.scroll_dir == 1,
                    "motion": s.sensor.motion_sync == 1,
                    "fps20k": s.sensor.fps20k == 1,
                    "angle": s.sensor.angle,
                },
                "debounce": s.debounce.value,
                "sleepMin": s.sleep_min,
                "battery": { "percent": s.battery.percent.min(100), "charging": s.battery.charging },
            });
            let _ = app.emit("settings", dto);
        }
        Update::Buttons(list) => {
            let dto: Vec<_> = list
                .iter()
                .map(|b| {
                    json!({
                        "id": b.id,
                        "friendly": friendly_name(b.id).unwrap_or(""),
                        "typeId": b.type_id,
                        "typeName": type_name(b.type_id),
                        "label": b.label,
                        "present": is_present(b),
                    })
                })
                .collect();
            let _ = app.emit("buttons", dto);
        }
        Update::Written { ok, msg } => {
            let _ = app.emit("written", json!({ "ok": ok, "msg": msg }));
        }
        Update::Firmware { latest } => {
            let _ = app.emit("firmware", json!({ "latest": latest }));
        }
        Update::Error(e) => {
            let _ = app.emit("error", json!({ "message": e }));
        }
    }
}

/// Route WebKit's frames through shared memory on Wayland, unless already set.
///
/// The DMA-BUF renderer gives Mesa an EGL surface on the GTK toplevel, and Mesa
/// opts that `wl_surface` into explicit sync. GTK 3 cannot set acquire points,
/// so its next cairo commit is a `no_acquire_point` error and the compositor
/// disconnects us before the window draws. No EGL surface, no opt-in.
fn force_shm_frames_on_wayland() {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return;
    }
    if std::env::var_os("WEBKIT_DMABUF_RENDERER_FORCE_SHM").is_none() {
        // SAFETY: single-threaded — nothing else has started yet.
        unsafe { std::env::set_var("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1") };
    }
}

fn main() {
    force_shm_frames_on_wayland();
    tauri::Builder::default()
        .setup(|app| {
            let worker = Worker::spawn();
            app.manage(AppState { tx: worker.cmd_tx.clone() });
            squeak_core::watch::spawn(worker.cmd_tx.clone()); // auto-refresh on plug/unplug

            let handle = app.handle().clone();
            // Drain worker updates → Tauri events. Worker is moved in and lives
            // for the process lifetime (its own cmd_tx keeps the channel open).
            std::thread::spawn(move || {
                while let Ok(update) = worker.update_rx.recv() {
                    emit_update(&handle, update);
                }
            });
            // Kick the initial snapshot once the frontend is listening.
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_all,
            read_buttons,
            set_dpi,
            set_active_dpi,
            set_rate,
            set_lod,
            set_scroll,
            set_motion,
            set_fps20k,
            set_angle,
            set_debounce,
            set_sleep,
            set_profile,
            set_button_mouse,
            set_button_media,
            set_button_disable,
            set_button_default,
            check_update,
            palettes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running squeak desktop");
}
