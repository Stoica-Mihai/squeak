# squeak — implementation plan

A fast, professional TUI to configure Keychron mice on Linux. Single static
binary, no runtime deps. Reimplements the reverse-engineered Keychron Launcher
HID protocol (verified live on the M6 8K / Ultra-Link 8K).

> Protocol source of truth: `docs/` (per-variant maps; `8k-nordic.md` is verified
> live) and `FINDINGS.md` (the RE log). The Python reference impl + CLI is in
> `keycron/`. squeak is a Rust port of that verified behavior.

---

## 1. Goals & scope

- **Configure Keychron mice from a terminal** — DPI, polling, sensor, buttons,
  macros, profiles — with live read-back confirmation on every write.
- **Single static binary**, runs on any Linux (x86_64 + aarch64), no Python,
  no `libhidapi`, no glibc-version coupling.
- **Professional TUI** — variant B (sidebar nav + detail pane), themeable.
- **Multi-variant**: detect `1k / 4k / 8k / 8k_nordic` at runtime; M6 8K
  (`8k_nordic`) is the first-class, fully-verified target. Others are
  best-effort from the JS-derived docs, clearly flagged until hardware-verified.

Non-goals (v1): Windows/macOS, RGB (M6 has none), gestures/tap-holds/combos
(not supported by M6), GUI.

---

## 2. Tech stack (decided, validated)

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust 2021 | single static binary; toolchain present (1.96) |
| TUI | `ratatui` + `crossterm` | mature, professional (gitui/bottom/yazi) |
| HID | **pure-std `/dev/hidraw` + sysfs** | no C dep → fully static; proven in `keycron-cli/rust-poc` |
| Errors | `anyhow` (app) + `thiserror` (lib) | ergonomic, small |
| Static build | `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` | glibc dynamic as fallback |

Dependency budget kept tiny (ratatui, crossterm, anyhow, thiserror). No `hidapi`,
no `serde` in v1.

Validated facts (from `keycron-cli/rust-poc`): pure-Rust `std::fs::File` on
`/dev/hidrawN` reads/writes reports; `/sys/class/hidraw/*/device/{uevent,
report_descriptor}` give VID/PID + usage page for enumeration; glibc binary links
only libc/libgcc (no libhidapi).

---

## 3. Repository layout

```
squeak/
├── Cargo.toml
├── README.md
├── LICENSE
├── PLAN.md                      # this file
├── packaging/
│   ├── 99-keychron.rules        # udev: GROUP="input" for VID 3434
│   └── squeak.1                 # man page
├── docs/                        # symlink/copy of protocol maps (reference)
└── src/
    ├── main.rs                  # terminal lifecycle, event loop
    ├── app.rs                   # App state, screens, status line
    ├── event.rs                 # key handling -> actions
    ├── theme.rs                 # palettes (Mocha default, Gruvbox, Nord…)
    ├── hid/
    │   ├── mod.rs
    │   ├── enumerate.rs         # sysfs scan -> DeviceInfo {path, vid, pid, usage, variant}
    │   └── device.rs            # open + get/set/long_set/long_raw/chunk
    ├── proto/
    │   ├── mod.rs               # Variant enum + detection
    │   ├── block.rs             # cmd 0x06 block parse -> Settings struct
    │   ├── dpi.rs               # get/set DPI (cmd 0x40)
    │   ├── polling.rs           # get/set rate (cmd 0x41); code<->Hz
    │   ├── sensor.rs            # cmd 0x42: lod/scroll/motion/angle/ripple/sampling
    │   ├── system.rs            # sleep 0x0A, lpm 0x0B, debounce 0x43, reset 0x0F
    │   ├── buttons.rs           # get 0x62 / set 0x52; type+action enums
    │   └── macro.rs             # set 0x54 + 0x71 chunking; event encoding
    └── ui/
        ├── mod.rs               # frame layout: sidebar | content | footer
        ├── sidebar.rs           # section List
        ├── overview.rs          # gauge + summary
        ├── dpi.rs               # preset sliders
        ├── polling.rs           # single-select list
        ├── sensor.rs            # toggle/value rows
        ├── buttons.rs           # table + action-picker modal
        ├── macros.rs            # step recorder + text mode
        └── widgets.rs           # shared: slider bar, toggle, modal helper
```

---

## 4. HID layer (`src/hid`)

### Enumeration (`enumerate.rs`)
- Scan `/sys/class/hidraw/`. For each `hidrawN`:
  - read `device/uevent` → parse `HID_ID=0003:0000XXXX:0000YYYY` (bus:vid:pid)
    and `HID_NAME`.
  - filter `vid == 0x3434`.
  - read `device/report_descriptor`; detect usage page (first `06 LO HI`).
    Config collection = usage page `0xFFC1`.
- Return `DeviceInfo { node: "/dev/hidrawN", vid, pid, name, usage_page }`.
- Pick the `0xFFC1` node as the config endpoint. (NAPE node `0xFF60` recorded
  but unused — M6 buttons/macros ride the config channel.)

### Device (`device.rs`)
Open `/dev/hidrawN` `O_RDWR` via `OpenOptions`. Reports (8k_nordic):

| ID | dir | bytes | role |
|---|---|---|---|
| `0xB3` 179 | OUT | 63 | long command |
| `0xB4` 180 | IN | 63 | long reply (data) |
| `0xB5` 181 | OUT | 20 | short command |
| `0xB6` 182 | IN | 20 | short reply / ack |

Methods (mirror `keycron/device.py`):
- `get(cmd, args…) -> [u8]` — write `0xB3`, read until `0xB4`.
- `set(cmd, payload…) -> (ok, reply)` — write `0xB5`, read `0xB6`; ok =
  `reply[1]==0xE4 && reply[2]==0x00`.
- `long_set(cmd, payload…)` — write `0xB3`, ack on `0xB6` (`E4 00 cmd`). buttons.
- `long_raw(payload…) -> reply` — write `0xB3`, read `0xB6`. macro chunks (`0x71`).
- `_read(want_id, timeout)` — loop reads, skip noise (mouse input `0x01`/`0x0C`),
  return the report whose id matches. Use a short poll (`rustix`/`nix` poll or
  blocking read with O_NONBLOCK + manual timeout) so the UI never hangs.

All device I/O runs **off the UI thread** (see §7).

---

## 5. Protocol port (`src/proto`) — verified command map

All offsets verified live (8k_nordic, fw 0.1.6). Mirrors `docs/8k-nordic.md`.

### Read block — `get(0x06)` → body `b[]` (`b[0]=0x06`)
Parse into `Settings`:
- `profile { current: b[1], count: b[50] }`
- `dpi { presets: LE16 @ b[5,7,9,11,13], active: b[4]&15? (use levels), count: b[16], max: LE16@b[40], step: b[42] or 50 }`
- `polling { code: b[2]>>4, codes: b[43..49], count: b[49] }`
- `sensor { lod: b[15]&3, ripple(wave): b[15]>>2&1, line: b[15]>>3&1, motion: b[15]>>4&1, scroll_dir: b[15]>>6&1, sampling(fps20k): b[52]&1, angle: signed b[55] }`
- `debounce: b[17]` · `sleep_s: b[18]`
- `battery { percent: b[19]&0x7f, charging: b[19]>>7 }`
- `wake: b[51] bits 4/5/6/7` · `support_flags: b[26]`

### Writes (short channel `0xB5` unless noted)
| Feature | cmd | payload |
|---|---|---|
| DPI presets | `0x40` | `[active,active,active, 5×LE16 presets, count]` |
| Polling | `0x41` | `[code, code, 0,1,2,3,4,5, levelNum@9]` (replay frame; code at [1],[2]) |
| Sensor | `0x42` | `[lod, wave?1:2, line?1:2, motion?1:2, 0, scroll_dir+1, 0, fps20k+1]` |
| Angle snap | `0x42` | `[…, enable?2:0 @8, angle @9]` |
| Debounce | `0x43` | `[ms]` (or `[ms,17,profile,ms]`) |
| Sleep | `0x0A` | `[1, secs]` |
| LPM | `0x0B` | `[1, v|17, 0]` |
| Factory reset | `0x0F` | all→`[255]`; else `[0, mask{key,dpi,light,sensor,profile,scroll}]` |
| Profile select | `0x45` | `[i,i,i]` |

Polling code→Hz: `0=125,1=500,2=1000,3=2000,4=4000,5=8000` (Levels: 6).
Sampling mode = `fps20k`: 0 Standard, 1 Competitive (≥20K scan).

### Buttons (`buttons.rs`)
- GET `get(0x62, id)` → `[62,id,status,type, d_hi,d_mid,d_lo]`.
- SET `long_set(0x52, [id,0,type, be24(data)])`, ack `E4 00 52`.
- **Type 0 = DEFAULT function** (not empty). Type 9 = Disable. (Display "Default".)
- Type enum `S`: 0 Default,1 Mouse,2 Keyboard,3 Media,4 Macro,5 Dpi,6 Light,
  7 Game,8 ShortCut,9 Disable,10 Profile,13 PollingRate.
- Mouse actions (24-bit): left 0x010000, right 0x020000, middle 0x040000,
  forward 0x080000, backward 0x100000, leftDouble 0x800000, upScroll 0x000200,
  downScroll 0x00fe00, leftScroll 0x0000fe, rightScroll 0x000002.
- Keyboard: `(mods<<8)|hid_usage`; mods ctrl1/shift2/alt4/gui8.
- ~16 ids; non-physical ids reject (status 5). Side-scroll ids are remappable;
  middle scroll + free-wheel button are firmware-only (no id, no USB).

### Macros (`macro.rs`)
- Frame = `[0x54, id,0,len,0,loopCount,loopType,0x20,0,0,n_events,0, events…]`,
  `len = 6 + 4*n`.
- Event = `[flag, code, delay_lo, delay_hi]`, `flag = press(0x80)|class`;
  class 1=key (HID usage), 2=modifier (mask), 8=mouse (button).
- ≤63 B → single `long_set(0x54,…)` (ack `E4 00 54`).
- >63 B → chunk: `long_raw(0x71, seq, slice_len, slice…)` per 59-byte slice,
  ack `72 00`, `seq = 1 + bytes_sent/16`. Verify committed length on read-back.

### Variant detection (`mod.rs`)
- M6 8K dongle PID `0xD028`, wired `0xD049` → both `8k_nordic` (usage `0xFFC1`,
  reports B3–B6). Treat usage `0xFFC1` + these report ids as 8k_nordic.
- 1k/4k/8k paths stubbed behind a `Variant` enum, returning
  `Unsupported(variant)` until hardware-verified (per docs).

---

## 6. UI (`src/ui`) — variant B

Frame layout (ratatui `Layout`):
```
┌ sidebar (16) ┬ content (rest) ┐
│  List        │  per-section    │
│  device foot │  widget         │
└──────────────┴─────────────────┘
 footer: contextual keybinds (one line)
```

Sections & widgets:
- **Overview** — `Gauge` (battery) + summary lines.
- **DPI** — 5 rows, each a custom slider bar (`widgets::slider`), active marker;
  `←/→` ±50, `Shift+←/→` ±500, `a` set active, `↵` apply.
- **Polling** — single-select `List` (125…8000), `↵` apply.
- **Sensor** — rows: LOD (value), Scroll dir (enum), Motion (toggle), Angle
  (toggle+value), Ripple (toggle), **Sampling Mode** (Standard/Competitive),
  Debounce, Sleep. `←/→` change, `space` toggle, `↵` apply.
- **Buttons** — `Table` of id→assignment; `↵` opens **action-picker modal**
  (type column → value column); `d` default, `x` disable, `m` macro.
- **Macros** — step list (down/up events) + `t` text mode; `↵` upload.
- **Profiles** — list of 5 slots, active marker, `↵` activate.

Cross-cutting:
- **Status line**: every write shows `✓ written & verified` / `✗ error` from the
  read-back result (mirrors backend guarantee).
- **Theme** (`theme.rs`): Catppuccin Mocha default; `t` cycles
  Mocha/Gruvbox/Nord/Tokyo-Night. Palette = struct of ratatui `Color`s.
- Keybinds global: `↑↓` nav, `q` quit, `r` refresh, `?` help overlay, `t` theme.

State machine (`app.rs`): `Screen` enum + per-screen cursor/selection + optional
`Modal` (action-picker / confirm-reset / help). Reducer pattern: `Action` →
`App::update` → maybe a `proto` call (async) → status update → re-render.

---

## 7. Concurrency / refresh model

- Single async-ish loop without a runtime: dedicated **I/O thread** owning the
  `Device`; UI thread sends `Cmd` over an `mpsc` channel, receives `Update`
  (Settings/Buttons/Status) back. Keeps the UI responsive during HID round-trips
  and avoids blocking on `read`.
- Battery/Settings auto-refresh every N seconds (timer event) + manual `r`.
- crossterm event stream (keys, resize) merged with channel updates via a small
  select loop (poll with timeout).

---

## 8. Permissions

- hidraw needs group access (uaccess is unreliable here — see keycron-cli memory).
  Ship `packaging/99-keychron.rules`:
  ```
  SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3434", MODE="0660", GROUP="input"
  ```
  (covers all Keychron PIDs: dongle d028, wired d049, others).
- First-run check: if the config node exists but `open` fails with EACCES, show a
  clear panel with the udev fix + `sudo` commands instead of crashing.

---

## 9. Build & distribution

- **Dev**: `cargo run`.
- **Release static**: `cargo build --release --target x86_64-unknown-linux-musl`
  (one-time `rustup target add …`). Produces a ~1–2 MB fully static binary.
- **Cross**: aarch64 musl for ARM SBCs.
- **Dist artifacts**: GitHub Releases with `squeak-x86_64`, `squeak-aarch64`,
  the udev rule, and the man page. Optional AUR `PKGBUILD`.
- `--help`/`--version` via a tiny hand-rolled arg parse (no clap needed) so a
  headless `squeak --read` JSON dump is also possible for scripting.

---

## 10. Testing

- **Unit (no device)**: byte-builders & parsers — feed known frames, assert
  decoded `Settings`/`Button`/macro bytes match the captured reference frames in
  `docs/`. These are the highest-value tests (lock the protocol).
- **Golden frames**: embed the real usbmon-captured frames (DPI/polling/sensor/
  button/macro) as fixtures; assert our encoders reproduce them byte-for-byte.
- **Live (feature-gated, opt-in)**: `cargo test --features live` round-trips each
  setter on real hardware with restore (mirrors the Python validation we ran).
- **UI**: ratatui `TestBackend` — render each screen to a buffer, snapshot-assert
  layout; drive key events through `App::update`.

---

## 11. Milestones

1. **M0 — skeleton**: Cargo project, deps, terminal lifecycle, empty sidebar +
   footer, theme, quit. (proves ratatui scaffold)
2. **M1 — HID + read**: `hid::enumerate` + `Device`, `proto::block`, Overview
   screen showing live battery/DPI/polling/sensor. Read-only, no writes.
3. **M2 — DPI + Polling**: sliders + select, writes with read-back status.
4. **M3 — Sensor + System**: toggles/values (incl. Sampling Mode), debounce,
   sleep; factory-reset behind a confirm modal.
5. **M4 — Buttons**: table + action-picker modal; default/disable.
6. **M5 — Macros**: step recorder + text mode; single-frame then `0x71` chunking.
7. **M6 — polish**: profiles, themes, help overlay, error panels, man page.
8. **M7 — release**: musl static builds, packaging, AUR, README/asciinema.

Each milestone: golden-frame unit tests + a live smoke test before moving on.

---

## 12. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Writing wrong frames corrupts config | Every write reads back & verifies; never trust ACK alone (learned: button type-0 issue) |
| Blocking HID read hangs UI | dedicated I/O thread + timeout reads |
| Volatile reads during live scroll | filter noise report ids; debounce reads |
| Media/0xFF byte not round-tripping | treat device-owned bytes as read-only; verify on length/type not exact echo |
| Other variants unverified | gate 1k/4k/8k behind `Unsupported` until hardware-tested; ship 8k_nordic first |
| musl target missing on user box | provide glibc binary too; document `rustup target add` |

---

## 13. Future

- Config profiles export/import (TOML) — re-add `serde`.
- Other Keychron mice once hardware is available to verify.
- NAPE `0xFF60` features if a future device exposes gestures/taphold/combos.
- Optional daemon mode (battery in status bar via IPC).
