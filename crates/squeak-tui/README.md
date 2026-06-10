# squeak (terminal UI)

The ratatui frontend — keyboard-driven, SSH-able, tiny. Built on
[`squeak-core`](../squeak-core). See the [root README](../../README.md) for the
project overview, permissions (udev rule), and protocol notes.

<img src="../../docs/screenshots/overview.png" alt="Overview screen" width="820">

## Install

Needs a Rust toolchain ≥ 1.85 (edition 2024).

```bash
cargo install --git https://github.com/Stoica-Mihai/squeak squeak-tui --locked
```

Drops `squeak` in `~/.cargo/bin` (make sure it's on your `PATH`). No C
dependencies — pure-std `/dev/hidraw`; the opt-in `u` firmware check pulls in
`ureq`/`rustls` (it looks up the latest version, never flashes), everything
else is offline. Then set up [permissions](../../README.md#permissions).

From source:

```bash
cargo build --release        # binary at target/release/squeak
./target/release/squeak
```

## Keys

Focus model: the **sidebar** (section list) and the **content** pane each take
focus.

| Key | Action |
|---|---|
| `Tab` | move focus between sidebar and content |
| `↑ ↓` | sidebar: change section · content: move row/selection |
| `→` / `Enter` | sidebar: enter content · content: apply / open picker |
| `Esc` | back to the sidebar |
| `r` refresh · `t` theme · `u` check firmware (online) · `?` help · `X` factory reset · `q` quit | |

Per screen (content focus):

- **DPI** — `↑↓` pick a preset, `Enter` to type an exact value (50–26000).
- **Polling** — `↑↓` pick a rate, `Enter` apply (`●` = current).
- **Sensor** — `↑↓` row, `←→` change, `Space` toggle; `Enter` shows a diff and
  applies. Edited-but-unapplied rows are marked `✎ unsaved`.
- **Buttons** — `↑↓` a button; `Enter` opens the action picker
  (Mouse / Media / Disable / Default), `d` default, `x` disable, `m` macro,
  `L` toggles the left-click lock.
- **Profiles** — `↑↓` pick, `Enter` activate (reloads the whole config).

`t` opens a theme picker — `↑↓` previews each theme live, `↵` confirms, `esc`
reverts. Themes: Mocha, Gruvbox, Nord, Dracula.

`u` queries the Keychron Launcher API for the latest firmware and shows
`✓ latest` / `⬆ X available` on the Overview line (silent if offline). The only
network call, and only when pressed.

## Screens

<details>
<summary><b>DPI</b></summary>
<img src="../../docs/screenshots/dpi.png" alt="DPI screen" width="820">
</details>
<details>
<summary><b>Polling</b></summary>
<img src="../../docs/screenshots/polling.png" alt="Polling screen" width="820">
</details>
<details>
<summary><b>Sensor</b></summary>
<img src="../../docs/screenshots/sensor.png" alt="Sensor screen" width="820">
</details>
<details>
<summary><b>Buttons</b></summary>
<img src="../../docs/screenshots/buttons.png" alt="Buttons screen" width="820">
</details>
<details>
<summary><b>Profiles</b></summary>
<img src="../../docs/screenshots/profiles.png" alt="Profiles screen" width="820">
</details>
<details>
<summary><b>Button picker / Theme picker / Help</b></summary>
<img src="../../docs/screenshots/button-picker.png" alt="Button picker" width="820">
<img src="../../docs/screenshots/theme-picker.png" alt="Theme picker" width="820">
<img src="../../docs/screenshots/help.png" alt="Help overlay" width="820">
</details>
