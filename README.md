# squeak

A terminal UI to configure Keychron mice on Linux — DPI, polling rate, sensor,
buttons, macros, and profiles — over raw HID, replicating the web Launcher.
Every write is read back and verified.

First-class target: **Keychron M6 8K / Ultra-Link 8K dongle** (`8k_nordic`,
VID `0x3434`), verified live on firmware 0.1.6.

```
╭ squeak ────────╮╭ Overview ──────────────────────────────────────────╮
│▌ Overview      ││Keychron Ultra-Link 8K  ·  2.4 GHz  ·  firmware 0.1.6│
│  DPI           ││                                                     │
│  Polling       ││Battery  ████████████████████████ 100%              │
│  Sensor        ││DPI      400 800 1600 4250 6000                      │
│  Buttons       ││Polling  125 Hz                                      │
│  Profiles      ││Sensor   LOD 1 · normal scroll · angle off           │
│                ││Timing   debounce 4 ms · sleep 7 min                 │
│ ● Ultra-Link 8K││last refreshed 2s ago                                │
│   100%         ││                                                     │
╰────────────────╯╰─────────────────────────────────────────────────────╯
```

## Build

Rust 2024 (toolchain ≥ 1.85). No C dependencies — pure-std `/dev/hidraw` for
device I/O. (The opt-in `u` update check pulls in `ureq`/`rustls`; everything
else is offline.)

```bash
cargo build --release
./target/release/squeak
```

## Permissions

hidraw needs group access (the browser Launcher needs this too). Ship a udev
rule, then replug the dongle:

```
# /etc/udev/rules.d/99-keychron.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3434", MODE="0660", GROUP="input"
```

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger --action=add
# replug the dongle
```

(If a config node exists but `open` fails with EACCES, squeak shows the fix.)

## Usage

```bash
squeak              # launch the TUI
```

### Keys

Focus model: the **sidebar** (section list) and the **content** pane each take
focus.

| Key | Action |
|---|---|
| `Tab` | move focus between sidebar and content |
| `↑ ↓` | sidebar: change section · content: move row/selection |
| `→` / `Enter` | sidebar: enter content · content: apply / open picker |
| `Esc` | back to the sidebar |
| `r` | refresh from device · `t` theme · `u` check firmware update · `?` help · `X` factory reset · `q` quit |

Per screen (content focus):

- **DPI** — `↑↓` pick a preset, `Enter` to type an exact value (50–26000).
- **Polling** — `↑↓` pick a rate, `Enter` apply (`●` = current).
- **Sensor** — `↑↓` row, `←→` change, `Space` toggle; `Enter` shows a diff and
  applies. Edited-but-unapplied rows are marked `✎ unsaved`.
- **Buttons** — `↑↓` a button; `Enter` opens the action picker
  (Mouse / Media / Disable / Default), `d` default, `x` disable, `m` record a
  macro (modal).
- **Profiles** — `↑↓` pick, `Enter` activate (reloads the whole config).

`u` checks for a firmware update — the **only** thing that touches the network,
and only when you press it. It queries the Keychron Launcher API for the latest
version and shows `✓ latest` / `⬆ X available` on the Overview firmware line
(silent if offline). squeak does not flash firmware.

## Features

DPI presets · polling rate · sensor (LOD, scroll dir, motion sync, angle snap,
ripple, sampling mode) · debounce · sleep · button remap (mouse / media /
keyboard via the reference CLI) · macros (click sequences + text, auto-chunked
over `0x71`) · profile switching · factory reset. Themes: Mocha, Gruvbox, Nord,
Tokyo Night (`t`).

Gestures / tap-holds / combos are not supported by the M6 (its capability flags
don't advertise them).

## Protocol & reference implementation

The HID protocol was reverse-engineered from the Keychron Launcher (WebHID) and
usbmon captures. Per-device maps are in [`docs/`](docs/) —
[`docs/8k-nordic.md`](docs/8k-nordic.md) is our verified device.
[`FINDINGS.md`](FINDINGS.md) is the RE log; [`capture.py`](capture.py) is the
usbmon decoder (`RAW=1` logs every report id).

[`keycron/`](keycron/) is the verified Python reference implementation + CLI
that squeak ports. [`PLAN.md`](PLAN.md) is the implementation plan.

## Status

8k_nordic is verified live (DPI, polling, sensor, buttons, macros, profile
switch — all read-back-confirmed on the Ultra-Link 8K dongle). Other Keychron
variants (1k / 4k / 8k) are documented from the Launcher JS but unverified.
