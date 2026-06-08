# Protocol: 8k_nordic  (OUR DEVICE — verified live)

Runtime-detected variant (handshake reply bytes `[3],[4] == 0x34,0x34`). The
Ultra-Link 8K dongle (`VID 0x3434`, `PID 0xD028`) enumerates here. All offsets
below are **verified live against firmware 0.1.6** unless marked.

## Transport

Config collection: HID usage page `0xFFC1` (`/dev/hidraw6`).

| Report ID | Dir | Bytes | Role |
|---|---|---|---|
| `0xB3` (179) | OUT | 63 | long command (READ block) |
| `0xB4` (180) | IN  | 63 | long reply |
| `0xB5` (181) | OUT | 20 | short command (WRITE settings) |
| `0xB6` (182) | IN  | 20 | short reply |

- Long command: `[0xB3, cmd, payload…]` → reply `0xB4`.
- Short command: `[0xB5, cmd, payload…]` → reply `0xB6 = [E4, status, cmd, …]`,
  `status 0x00 = OK`, `0x07 = fail`. (`E4` = 228 short-reply marker.)
- No checksum on short frames. The `161 - sum(buf[0:63])` (`0xA1` seed) checksum
  applies only to 64-byte **long** NAPE frames.

## READ — settings block (long cmd `0x06`)

`[0xB3, 0x06]` → `0xB4`. Body `b[]` (report id stripped, `b[0]=0x06`). Verified
field map (Launcher class `zt`):

| Field | Offset / formula | Notes |
|---|---|---|
| profile.current | `b[1]` | |
| profile.count | `b[50]` | =5 |
| dpi.active_levels | `b[2]&15, b[3]&15, b[4]&15` | low nibble = active profile idx |
| dpi.presets[5] | LE16 @ `b[5,7,9,11,13]` | e.g. 400/800/1600/4250/5000 |
| dpi.count | `b[16]` | =5 |
| dpi.max | LE16 @ `b[40]` | (0 on fw 0.1.6) |
| dpi.step | `b[42] or 50` | |
| polling.levels | `b[2]>>4, b[3]>>4, b[4]>>4` | high nibble = rate code |
| polling.rate_codes | `b[43:49]` | available codes `[0..5]` |
| polling.count | `b[49] or 6` | |
| sensor.lod | `b[15] & 3` | lift-off distance |
| sensor.wave | `b[15]>>2 & 1` | ripple control |
| sensor.line | `b[15]>>3 & 1` | angle straightening |
| sensor.motion_sync | `b[15]>>4 & 1` | |
| sensor.scroll_dir | `b[15]>>6 & 1` | invert scroll |
| sensor.fps20k | `b[52] & 1` | |
| sensor.angle | signed `b[55]` | angle snap degrees |
| debounce.value | `b[17]` | ms |
| debounce.values | `b[30:40]` | per-button table |
| scroll.speed/inertia/spl | `b[27], b[28], b[29]` | |
| sleep_s | `b[18]` | idle sleep seconds |
| battery.percent | `b[19] & 127` | **battery %** |
| battery.charging | `b[19] >> 7` | |
| wake.key/scroll/move/side_scroll | `b[51]` bits 4/5/6/7 | wake sources |
| support_flags | `b[26]` | bit0 scroll, 1 debounce, 3 max/step, 4 pollingGears, 5 profile |

Battery, version, vid/pid additionally come via the dongle-info reads (long cmd
`0x04`/`0x01` with `b[2]=129`), but battery % is already in the `0x06` block.

## WRITE — short channel (report `0xB5`), `buf[0]=cmd`

| cmd | hex | feature | payload (buf indices after cmd) |
|---|---|---|---|
| `64` | 0x40 | **DPI presets** ✅ | `[1..3]=active`, LE16 presets `@[4..13]`, `[14]=count` |
| `65` | 0x41 | **polling rate** ✅ | `[1,2]=level`, `levelsVal @[3..]`, `[9]=levelNum` |
| `66` | 0x42 | **sensor** | `[1]=lod, [2]=wave||2, [3]=line||2, [4]=motion||2, [6]=scrollDir+1, [8]=fps20k+1` |
| `66` | 0x42 | **angle snap** | `[9]=enable?2:0, [10]=angle` |
| `67` | 0x43 | **debounce** | `[1]=value`; if profile≥0: `[2]=17,[3]=profile,[4]=value` |
| `69` | 0x45 | **DPI profile select** | `[1],[2],[3]` = profile indices/order |
| `36` | 0x24 | DPI indicator colors | `[1..6]` |
| `10` | 0x0A | **sleep timeout** | get `[1]=2`; set `[1]=1,[2]=secs||255` |
| `11` | 0x0B | **wireless low-power mode** | get `[1]=2`; set `[1]=1,[2]=v||17,[3]=` |
| `14` | 0x0E | aux / report-rate-separate | `[1],[2]` |
| `15` | 0x0F | **factory reset (selective)** | `all`→`[1]=255`; else `[2]=idx,[3]=bitmask` of {key,dpi,light,sensor,profile,scroll} |
| `3`  | 0x03 | receiver/connection state | get |
| `74` | 0x4A | save/commit | no args |
| `75` | 0x4B | save profile | no args |

`setPollingRate` applies `pollingGearsSupport ? Us(level) : level`; our device
reports `pollingGearsSupport=false`, so the level is sent raw.

Polling code → Hz (over dongle): `0=125, 1=250, 2=500, 3=1000, 4=2000, 5=4000`
(8000 cable-only).

## Buttons — config channel, cmd `0x52`/`0x62` (VERIFIED LIVE)

Button remap does **not** use the VIA/`0xFF60` node (silent over the dongle, see
below). It rides the same config collection (Launcher class `Si`):

- **SET**: write long `0xB3`, `[0x52, id, 0x00, type, d_hi, d_mid, d_lo]`;
  ack on short `0xB6` = `e4 00 52`.
- **GET**: write long `0xB3`, `[0x62, id]` → `0xB4` =
  `[62, id, status, type, d_hi, d_mid, d_lo]`.

Data is a 24-bit big-endian value; meaning depends on `type`.

**Action type** (enum `S`): `0 Remove, 1 Mouse, 2 Keyboard, 3 Media, 4 Macro,
5 Dpi, 6 Light, 7 Game, 8 ShortCut, 9 Disable, 10 Profile, 13 PollingRate`.

**Mouse action data** (enum `_`, the 24-bit value):
`left 0x010000, right 0x020000, middle 0x040000, forward 0x080000,
backward 0x100000, leftDouble 0x800000, upScroll 0x000200,
downScroll 0x00fe00, leftScroll 0x0000fe, rightScroll 0x000002`.

**Keyboard**: data `= (modifiers << 8) | HID_usage`. Modifier bits (enum `b`):
`ctrl 1, shift 2, alt 4, gui 8`.

Live: ~16 button ids. Observed ids 5/10 = Media, 6/11 = Mouse (ids mirror across
two layers). Verified set/get on id 11 (e.g. side-scroll-up → left). Implemented
in `keycron/buttons.py`; CLI `keycron buttons` / `keycron button <id> …`.

## Macros — config channel, cmd `0x54` (VERIFIED LIVE)

Like buttons, macros ride the config channel (not the NAPE/`0xFF60` node the JS
suggests). One `0x54` frame uploads the macro AND binds it to the button id
(the button's type becomes Macro=4).

- **SET**: write long `0xB3`,
  `[0x54, id, 0x00, len, 0x00, loopCount, loopType, 0x20, 0x00, 0x00, n_events, 0x00, <events>]`;
  ack short `0xB6` = `e4 00 54`. `len = 6 + 4*n_events` (counts from byte 6).
  Invalid/non-physical button id → `e4 05 54` (status 5).
- `loopType 1` = "Stop on Release".

**Event** = 4 bytes `[flag, code, delay_lo, delay_hi]`, `flag = press(0x80) | class`:

| class | meaning | code | press / release |
|---|---|---|---|
| 1 | keyboard key | HID usage | `0x81` / `0x01` |
| 2 | modifier | bitmask (ctrl1 shift2 alt4 gui8, R+16/32/64/128) | `0x82` / `0x02` |
| 8 | mouse button | 1 left, 2 right, 3 middle, 4 back, 5 fwd | `0x88` / `0x08` |

`delay` LE16 ms (0 when delay disabled). All three classes verified: the mouse
capture (Ldown/Lup/Rdown/Lup) and a keyboard capture cross-checked against the
Launcher's export JSON (LShift, KC_1, Ctrl+A, …) decode exactly.

**Long macros — chunked via cmd `0x71` (VERIFIED LIVE).** When the `0x54` frame
exceeds one report (~12 events), split it into 59-byte slices, each sent as
`[0x71, seq, slice_len, <slice>]` on `0xB3`, acked `0xB6 0x72 00`. The reassembled
slices equal the full `0x54` frame. **`seq = 1 + (bytes_sent_before // 16)`** —
the device validates seq against its own running byte count (floor, not ceil;
ceil is silently rejected). The macro commits when the declared length is
received; readback length confirms.

Implemented in `keycron/macro.py` (mouse + keyboard + modifier, single + chunked).
CLI: `keycron macro <id> click left right` / `keycron macro <id> text "hello"`.

## Gestures / tap-holds / combos — NAPE group (`0xA7`=167)

Gestures, tap-holds, combos still go through the NAPE group. The Launcher targets
the QMK-raw collection (usage `0xFF60`) with `buf[0]=167, buf[1]=sub` and the
`161-sum` checksum on 64-byte frames. Sub-commands:

| sub | name |
|---|---|
| 33/34 | NAPE get/set DPI index |
| 35 | NAPE set DPI value (`[2]=idx,[3]=lo,[4]=hi`) |
| 36 | NAPE get DPI value |
| 54/55 | get/set custom DPI value |
| — | NAPE set layer, get/set orientation, get/set layer-ori |
| — | NAPE get/set/del tap-holds, get/set/del combos, get/set gesture |
| — | NAPE get bat report / bat report |
| WL.* | DYNAMIC_KEYMAP get/set keycode, get buffer, clear-all (key remap) |
| WL.* | DYNAMIC_KEYMAP macro get count / buffer-size / reset (macros) |
| WL.* | DYNAMIC_KEYMAP get/set encoder (scroll-wheel actions) |

Implementing these requires opening the `0xFF60` node and the VIA keymap-matrix
protocol — larger scope than the short-channel settings above.

### Live status: VIA/keymap node UNRESPONSIVE over the 2.4 GHz dongle

`0xFF60` enumerates as `/dev/hidraw4` (32-byte reports, no report id; `write =
[0x00, cmd, …]`). The Launcher's keymap channel is `write(0, …)` =
`sendReport(0, …)` = report id 0 = this node. Probed read-only with VIA
`protocol_version` (1), `get_keyboard_value` (2), `GET_LAYER_COUNT` (17),
`CURRENT_LAYER` (163), and NAPE `167` subs (battery 49, dpi 33): **writes
succeed (n=33) but the device sends zero replies.** `hidraw5` (`0x8C`, report ids
`0xB1/0xB2`) is silent too.

Conclusion: button remap / macros / encoder are **not serviced on these nodes
over the wireless dongle**. To implement them, first determine the real routing
via a usbmon capture of the Launcher performing a button remap (it may tunnel
through the mouse channel, or require the wired USB connection / PID `0x0624`).
Do NOT write blindly to the keymap node — a wrong frame can corrupt button maps.

VIA command numbers (for when the node is reachable, e.g. wired):
GET/SET_KEYCODE `4`/`5`, GET/SET_BUFFER `18`/`19`, GET/SET_ENCODER `20`/`21`,
MACRO count/size/get/set/reset `12`/`13`/`14`/`15`/`16`, GET_LAYER_COUNT `17`,
CURRENT_LAYER `163`, NAPE group `167`, HE `169`.
NAPE subs: ori `32`/`52`, dpi `33`/`34`/`35`/`36`, taphold `37`/`38`/`47`,
combos `39`/`40`/`46`, gesture `41`/`42`, profile `43`/`44`, layer `45`,
battery `48`/`49`, layer-ori `56`/`57`.
