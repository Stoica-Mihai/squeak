# Keychron M6 — Linux CLI configuration: research findings

Goal: configure the Keychron M6 mouse (DPI, polling rate, etc.) from the Linux
CLI, replicating what the official browser configurator does. This doc captures
everything needed to build the tool once the mouse is physically available for
live testing.

## TL;DR

- The official configurator ("Keychron Launcher") is a **web page using the
  WebHID API** — it is NOT a webserver running on the mouse. JS in a
  Chromium-based browser talks to the mouse over raw USB HID. Any HID-capable
  CLI program can do the same.
- **No existing Linux CLI tool fits.** `libratbag`/Piper does not list the M6 in
  its device database; openrazer is Razer-only. Only Keychron-specific Linux
  code found is a battery-only reader (no DPI/polling).
- The HID protocol has been reverse-engineered (community repo, verified against
  the decompiled Launcher JS). Command IDs are known. Exact payload byte offsets
  for the SET commands still need one `usbmon` capture to confirm.
- Build path: Python + the `hid` package (hidapi) — both already installed on
  this machine.

## How the official configurator works

- Keychron Launcher: <https://launcher.keychron.com/>
- WebHID exclusively — no WebBluetooth, no WebSerial. Source: decompiled Angular
  bundle, documented in `Tymon3310/keychron-vial`.
- Implication: replicating it from the CLI = open the same hidraw device and
  send/receive the same HID reports.
- Browser caveat: WebHID is Chromium-only (Chrome/Edge/Opera/Brave). Firefox
  does NOT support WebHID. This box currently has no Chromium-family browser
  installed — install one before attempting a usbmon capture of the Launcher.

## Device identification

- Vendor ID (all Keychron): `0x3434` (13364)
- M6 4K product ID: `0x0624`, its 2.4 GHz dongle PID: `0xD046`
- There is also an **M6 1K** variant (max 1000 Hz) that uses a different PID and
  the 1K protocol. Confirm which variant is in hand by enumerating (below).

### Actual hardware in hand (enumerated 2026-06-08)

Device is **"Keychron Ultra-Link 8K" dongle**, NOT the PIDs assumed above:

- PID `0xD028` (8K wireless dongle, `D0xx` = via 2.4 GHz dongle transport).
- Config collection on usage page `0xFFC1`, exposed as `/dev/hidraw6`.
- Other collections: `0x01`, `0x0C`, `0xFF60`, `0x8C`.
- `usage_page 0xFFC1` here but product reports 8K — table below (1K=0xFFC1)
  is approximate; treat `0xD028` as the 8K dongle and run protocol detection
  + apply the 8K checksum path.

| Variant | HID usage page | Report size | Notes |
|---|---|---|---|
| M6 1K | `0xFFC1` (65473) | 32 bytes | PixArt 3395, 125/500/1000 Hz |
| M6 4K/8K | `0xFF0A` (65290) | 64 bytes | PID `0x0624`; adds 2000/4000/8000 Hz; 8K variant adds a checksum |

Report ID for mouse reports: `0xB5` (181).

### Step 1 — enumerate when the mouse is connected

```bash
python3 -c "import hid; [print(hex(d['vendor_id']), hex(d['product_id']), 'usage_page', hex(d['usage_page']), d['product_string']) for d in hid.enumerate(0x3434)]"
```

- `usage_page == 0xffc1` → 1K variant (32-byte reports)
- `usage_page == 0xff0a` → 4K/8K variant (64-byte reports)
- Note the `product_id`: low `0x06xx` = direct USB cable; `0xD0xx` = via 2.4 GHz
  dongle (different transport, see below).

## Protocol

Reverse-engineered and documented here (verified against decompiled Launcher
bundle `main.67e8841912a834d8.js`):

- Mouse protocol: <https://github.com/Tymon3310/keychron-vial/blob/main/docs/launcher/mouse-protocol.md>
- Launcher overview: <https://github.com/Tymon3310/keychron-vial/blob/main/docs/launcher/overview.md>
- Command map: <https://github.com/Tymon3310/keychron-vial/blob/main/docs/launcher/command-map.md>
- Misc command framing (`0xA7`): <https://github.com/Tymon3310/keychron-vial/blob/main/docs/firmware/misc-commands.md>

### Misc command group `0xA7` framing

Request: `data[0] = 0xA7`, `data[1] = sub_command`, rest = payload.
Response:

```
Byte  Field         Description
 0    0xA7          command echo
 1    sub_command   sub-command echo
 2    status        0 = success, 1 = fail
 3..  payload       sub-command-specific
```

Over hidraw (`hid` package), the buffer's first byte is the report ID, so a
write looks like: `[0xB5, 0xA7, sub_command, ...]`.

### Relevant sub-commands

DPI (NAPE sub-commands, under `0xA7`):

| Sub-cmd | Hex | Dir | Description |
|---|---|---|---|
| `NAPE_GET_DPI` | `0x21` | read | get DPI profile settings |
| `NAPE_SET_DPI` | `0x22` | write | set active/selected DPI profile |
| `NAPE_SET_DPI_VALUE` | `0x23` | write | set DPI value for a profile |
| `NAPE_GET_DPI_VALUE` | `0x24` | read | get DPI value per profile |
| `NAPE_GET_PROFILE` | `0x2C` | read | active profile |
| `NAPE_SET_PROFILE` | `0x2B` | write | set active profile |
| `NAPE_GET_BAT_REPORT` | `0x31` | read | battery |
| `NAPE_BAT_REPORT` | `0x30` | write | battery report config |

Polling / report rate (misc sub-commands, under `0xA7`):

| Sub-cmd | Hex | Dir | Description |
|---|---|---|---|
| `REPORT_RATE_GET` | `0x0D` | read | get USB report rate |
| `REPORT_RATE_SET` | `0x0E` | write | set USB report rate |
| `DEBOUNCE_GET/SET` | `0x05`/`0x06` | r/w | button debounce |
| `WIRELESS_LPM_GET/SET` | `0x0B`/`0x0C` | r/w | wireless low-power mode |
| `FACTORY_RESET` | `0x11` | write | reset settings to defaults |

Full NAPE map: `0x20`–`0x34` (orientation, tap-holds, combos, gestures, layer,
profile, battery, force-gesture-scroll). See command-map.md.

### Base info (device settings block)

Request: `[0x04, 0x00, 0x83, 0x03]` (distinct framing from the `0xA7` group).
Response fields by offset:

| Offset | Field | Description |
|---|---|---|
| 0-1 | systemSleepTime | idle sleep timeout, LE16 seconds |
| 2-3 | bleSystemSleepTime | BLE idle sleep, LE16 seconds |
| 4 | reportRate | current polling rate |
| 5 | debounceTime | button debounce, ms |
| 6 | quickResponse | quick response mode |
| 7 | buttonWakeupEnable | wake on button |
| 8 | moveWakeupEnable | wake on movement |
| 9 | wheelWakeupEnable | wake on scroll |
| 10 | wheelReverse | invert scroll direction |
| 14 | rfPowerMode | RF transmit power |
| 15 | bleNum | Bluetooth host slot |
| 16 | rptFlag | supported polling rates flag |
| 17-22 | rptTable | 6-byte array of available polling rates |

Dongle version query: request `[0x00, 0x00, 0x81, 0x00]`; response byte 8 =
minor/patch nibbles, byte 9 = major.

### 8K checksum (only the 8K/Nordic variant)

`buf[63] = (161 - sum(buf[0:63])) & 0xFF` (seed `0xA1`). 4K and 1K do not need
this.

### Transport: direct USB vs dongle

- `workMode == 0` (direct USB cable): simple `sendReport` or feature reports.
- `workMode == 1` (via 2.4 GHz dongle): "queued with retry" — send, wait for
  ACK, up to 3 attempts, with timeout. Build retry logic for the dongle path.

### Protocol detection (4K vs 8K)

Send `[1, 0, 129, 1, ..., checksum]` to the 4K collection. If response bytes
[6,7] == `[0x34,0x34]` or `[0x2D,0x36]` → "8k_nordic", else "4k".

## Live probe results (Ultra-Link 8K dongle, 2026-06-08)

Probed `/dev/hidraw6` (usage_page `0xFFC1`) directly. Concrete findings:

**Report descriptor (the real channels — FINDINGS' `0xB5`/64-byte assumption was wrong):**

| Report ID | Dir | Payload | Role |
|---|---|---|---|
| `0xB3` | OUTPUT | 63 B | command channel (write here) |
| `0xB4` | INPUT | 63 B | data response |
| `0xB5` | OUTPUT | 20 B | short output (unused so far) |
| `0xB6` | INPUT | 20 B | NAK/short response |

**Framing:** `[0xB3, <command>, <payload…>]`. The first payload byte is the
**command selector**. Replies arrive on `0xB4` (real data) or `0xB6` (NAK).

**The documented `0xA7` NAPE/misc group is SILENT on this device** — no response
on either channel. This dongle uses a different command set.

**Confirmed reads:**
- `[0xB3, 0x04, …]` → `0xB4: 04 06 "0.1.6"` — firmware version string.

**NAK signature:** unknown commands reply `0xB6: e4 07 <cmd-echo> 00…`. Commands
`0x00,0x01,0x80,0x82,0x84,0x85,0x86` all NAK'd — blind scanning is low-yield.

**Binding note:** the installed `hid` package is cython-hidapi (`hid.device()` +
`open_path()`), NOT pyhidapi (`hid.Device`). `write()` first byte = report ID;
`write()` returns -1 on failure (check it). FINDINGS' `hid.Device(path=...)`
reference pattern does not work here.

→ Next: usbmon-capture the Launcher to learn the real DPI/polling command IDs +
payload offsets. Blind probing already hit the NAK wall.

### DECODED from usbmon capture (DPI fully working, verified live)

Two channels by purpose:
- **READ/GET** → long: write `0xB3`, reply `0xB4` (63 B).
- **WRITE/SET** → short: write `0xB5`, reply `0xB6` (20 B).

Short reply format: `[0xB6, 0xE4, status, cmd_echo, …]`. `status 0x00 = OK`,
`0x07 = fail`. (`0xE4` = short-reply marker.)

**DPI GET** — `[0xB3, 0x06]` → `0xB4` block:
```
B4 06 00 23 23 │AA│ p0lo p0hi │ p1 │ p2 │ p3 │ p4 …
            active(resp[5])  └ 5 presets, LE16 at resp[6,8,10,12,14] ┘
```
Presets observed `[400, 800, 1600, <edited>, 5000]`. DPI = raw LE16, step 50,
max 5000 (0x1388).

**DPI SET** — `0xB5` report, cmd `0x40`:
```
40 03 <active> <active> │ <5 presets LE16> │ 05
```
Send all 5 presets (change only the target slot). **No 8K checksum** — frame
ends after `0x05`. Verified: set→readback confirms.

Implemented in `keycron/dpi.py` (`get_dpi`, `set_dpi`), both confirmed on the
real dongle (active profile idx 3 and 4).

**Polling rate — DECODED (verified live):**

- **SET** — `0xB5` report, cmd `0x41`: `41 <code> <code> 00 01 02 03 04 05 06`
  (`01..06` tail constant). Reply `e4 00 41` = OK.
- **GET** — in the `0x06` block: `resp[3]` packs `(code << 4) | active_profile`.
  So rate code = high nibble of `resp[3]`; low nibble = active profile index
  (this is why `resp[2..3]` shifted `23↔24` between earlier sessions — that was
  the active profile changing, not the rate).
- Codes over the 2.4 GHz dongle: `0=125, 1=250, 2=500, 3=1000, 4=2000, 5=4000`
  Hz. (8000 Hz is cable-only and not offered on the dongle.)

The old FINDINGS `0x0D`/`0x0E` "REPORT_RATE_GET/SET" map does NOT apply to this
dongle — those `0x62`-prefixed reads returned no rate.

Implemented in `keycron/polling.py` (`get_rate_hz`, `set_rate`).

### Full command map (from Launcher bundle main.4b620e6634d704dd.js)

Device is **8k_nordic** → mouse settings use the short channel, report id `181`
(`0xB5`) out / `182` (`0xB6`) in. (1K variant uses report id `81`; 4K/8K-plain
use the NAPE `167`/`0xA7` group on the long/QMK channel instead.) Short reply
marker `228` (`0xE4`), status byte at `[1]` (`0`=OK). `numToHighLow` = LE16.

Short-channel (`181`) commands, `buf[0]=cmd`:

| cmd | hex | feature | payload |
|----|----|----|----|
| 3  | 0x03 | receiver/connection state | get |
| 10 | 0x0A | **sleep / idle timeout** | get `[1]=2`; set `[1]=1,[2]=secs(\|255)`; reply time at `[2]` |
| 11 | 0x0B | **wireless low-power mode** | get `[1]=2`; set `[1]=1,[2]=v\|17,[3]=`|
| 14 | 0x0E | (report-rate-separate / aux) | `[1]=ze,[2]=Ye` |
| 15 | 0x0F | **factory reset (selective)** | `[1 or 3]=bitmask{key,dpi,light,sensor,profile,scroll}`, `all=255` |
| 36 | 0x24 | DPI-indicator colors (6 vals) | `[1..6]` |
| 64 | 0x40 | **DPI presets** ✅ | `[1..3]=active,[4..]=LE16 presets,[14]=count` |
| 65 | 0x41 | **polling rate** ✅ | `[1,2]=level,[3..]=levelsVal,[9]=levelNum` |
| 66 | 0x42 | **sensor**: LOD, motion, scroll dir, 20k FPS | `[1]=lod,[2..4]=sensor,[6]=scrollDir+1,[8]=fps20k+1` |
| 66 | 0x42 | **angle snapping** | `[9]=enable?2:0,[10]=angle` |
| 67 | 0x43 | **debounce** (system param) | `[1]=ze,[2]=17,[3]=Ye,[4]=ze` |
| 69 | 0x45 | DPI profile select (which/order) | `[1,2,3]=profile idxs` |
| 74 | 0x4A | save/commit (no args) | — |
| 75 | 0x4B | save profile / reset (no args) | — |

Long channel NAPE (`167`/`0xA7`, QMK-raw collection = usage `0xFF60`,
likely `/dev/hidraw4`): keymap/button remap, macros, encoder (wheel), gestures,
tap-holds, combos, orientation/layer, **battery** (`NAPE_GET_BAT_REPORT`),
DPI-value sub-cmds (34/35/36/55). These need the other hidraw node + the
`161-sum` 8K checksum on 64-byte frames.

### Status: DPI + polling COMPLETE

`python3 -m keycron {info | dpi [VALUE] | polling [HZ]}` — all verified on the
real dongle. `capture.py` retained as the usbmon decoder for future RE
(battery, debounce, sleep, wheel-reverse still un-decoded).

## The one gap: exact SET payload offsets

Command IDs are documented; the exact byte position of the DPI value / polling
level inside the SET frames is not fully spelled out. Confirm with a single
`usbmon` capture of the Launcher performing the change.

### usbmon capture recipe

```bash
sudo modprobe usbmon
lsusb | grep -i 3434        # note the "Bus 00X" number → X
# Open launcher.keychron.com in a Chromium-based browser, connect the mouse,
# then change DPI by ONE step (and separately, polling rate by one step).
sudo cat /sys/kernel/debug/usb/usbmon/Xu   # X = bus number from lsusb
# (or capture with Wireshark on the usbmonX interface for easier decoding)
```

The captured OUT transfers are the exact HID reports the Launcher sends. Decode
those to lock the SET byte layout, then the writes become deterministic.

## Permissions (udev)

Non-root hidraw access needs a udev rule. This is also what blocks the **browser
Launcher** — Chrome/Chromium runs as the user and silently fails to open the
hidraw node (connect attempt drops to `#/not-connect`) until the user can open it.

**`TAG+="uaccess"` did NOT work on this machine** even with an active local
Wayland seat0 session — logind never wrote the `user:<name>:rw` ACL, even after
forcing a real `add` event. Use a **group-based rule** instead (`input` group,
which the user is already a member of):

```
# /etc/udev/rules.d/99-keychron-m6.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3434", ATTRS{idProduct}=="d028", MODE="0660", GROUP="input"
```

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add
# then REPLUG the dongle — cold add event applies the rule reliably
ls -l /dev/hidraw6   # want: crw-rw---- root input
```

Confirmed working: group `input` + user in `input` → browser Launcher connects.
Add the direct-cable PID `0x0624` / other dongle PIDs the same way if used.

## Local environment (already verified on this machine)

- `hid` Python package: installed (`/usr/lib/python3.14/site-packages/hid...so`)
- `hidapi` 0.15.0: installed
- `usbmon`: available, load with `sudo modprobe usbmon`
- Chromium-family browser: NONE installed — required for the Launcher capture
  (install chromium/brave first, or skip the capture and build reads-only).

## Reference HID access pattern (Python `hid`)

Working pattern, adapted from `byte-bandit/keychron-m3-linux`
(<https://github.com/byte-bandit/keychron-m3-linux>):

```python
import hid

VID = 0x3434

def find_device(product_id, usage_page):
    for d in hid.enumerate():
        if (d["vendor_id"] == VID
                and d["product_id"] == product_id
                and d["usage_page"] == usage_page):
            return d
    return None

info = find_device(0x0624, 0xFF0A)        # M6 4K, direct USB
with hid.Device(path=info["path"]) as dev:
    data = dev.read(64, timeout=1000)     # 64-byte reports for 4K variant
    # writes: dev.write(bytes([0xB5, 0xA7, sub_cmd, ...]))
```

That M3 tool reads battery by passively listening to input reports; the M6 CLI
additionally needs the command/response path above (write `0xA7` request, read
the echoed response).

## Build plan (when the mouse arrives)

1. Enumerate → confirm variant (1K vs 4K/8K), PID, transport (USB vs dongle).
2. Install the udev rule with the real PID(s).
3. Implement READ commands first (well documented, low risk): base-info dump
   (polling rate, debounce, sleep, wheel-reverse), DPI get, battery.
4. usbmon-capture the Launcher changing DPI + polling rate; decode exact frames.
5. Implement WRITE commands (DPI value, active DPI profile, polling rate) using
   the verified frames; add the dongle retry/ACK logic if wireless; add the 8K
   checksum if it is the 8K variant.
6. Test each write live and re-read to confirm it took.

## Sources

- Keychron Launcher: <https://launcher.keychron.com/>
- Reverse-engineered protocol (mouse): <https://github.com/Tymon3310/keychron-vial/blob/main/docs/launcher/mouse-protocol.md>
- Launcher overview: <https://github.com/Tymon3310/keychron-vial/blob/main/docs/launcher/overview.md>
- Command map: <https://github.com/Tymon3310/keychron-vial/blob/main/docs/launcher/command-map.md>
- Misc commands framing: <https://github.com/Tymon3310/keychron-vial/blob/main/docs/firmware/misc-commands.md>
- M3 Linux battery reader (HID access reference): <https://github.com/byte-bandit/keychron-m3-linux>
- libratbag (no M6 support): <https://github.com/libratbag/libratbag>
- Keychron open-source mouse firmware (Zephyr): <https://github.com/Keychron/zgm>
