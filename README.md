# keycron-cli

Configure a Keychron M6 mouse (Ultra-Link 8K dongle) from the Linux CLI —
DPI and polling rate — replicating the web Launcher over raw HID.

## Setup

Grant hidraw access (the Launcher needs this too):

```
# /etc/udev/rules.d/99-keychron-m6.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3434", ATTRS{idProduct}=="d028", MODE="0660", GROUP="input"
```

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger --action=add
# replug the dongle
```

Requires the `hid` Python package (cython-hidapi).

## Usage

```bash
python3 -m keycron info                 # all settings (add --json for raw)
python3 -m keycron battery              # battery percent

python3 -m keycron dpi                  # read DPI presets
python3 -m keycron dpi 1600             # set active profile's DPI (50..5000)
python3 -m keycron dpi 3200 --index 2   # set a specific preset slot

python3 -m keycron polling              # read polling rate
python3 -m keycron polling 1000         # 125/250/500/1000/2000/4000 Hz

python3 -m keycron sensor               # read sensor params
python3 -m keycron sensor --scroll-dir 1 --lod 2 --motion 1   # set sensor
python3 -m keycron angle 15             # angle snap degrees (0 = off)
python3 -m keycron debounce 8           # debounce ms
python3 -m keycron sleep 60             # idle sleep seconds
python3 -m keycron profile 5 6 7        # select DPI profiles

python3 -m keycron buttons              # list button assignments
python3 -m keycron button 11            # read one button
python3 -m keycron button 11 mouse right        # assign a mouse action
python3 -m keycron button 11 key 0x04 --mods ctrl   # assign Ctrl+A
python3 -m keycron button 11 disable

python3 -m keycron macro 11 click left right   # mouse-click macro onto a button
python3 -m keycron macro 11 text "hi"          # keyboard macro (types text)

python3 -m keycron reset --yes                       # factory reset (all)
python3 -m keycron reset --categories dpi sensor --yes
```

Every write re-reads the device and confirms the value took. All commands above
are verified live on the 8k_nordic device.

Macros support mouse / keyboard / modifier events but only **short** ones (one
HID report, ~12 events) — long macros use a `0x71` chunking scheme that isn't
implemented yet. Also documented-but-not-implemented (in `docs/`): scroll-wheel
**encoder** and gestures/tap-holds/combos (NAPE `0xA7` group).

## Protocol

Per-device protocol maps are in [`docs/`](docs/) — start with
[`docs/README.md`](docs/README.md). Our device is
[`docs/8k-nordic.md`](docs/8k-nordic.md) (verified live). `FINDINGS.md` is the
original reverse-engineering log; `capture.py` is the usbmon decoder used to
bootstrap it before the Launcher JS was mapped.
