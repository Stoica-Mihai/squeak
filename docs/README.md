# Keychron mouse protocol maps

Reverse-engineered from the Keychron Launcher bundle
`main.4b620e6634d704dd.js` (and verified live for `8k_nordic`). The Launcher
talks to mice over WebHID; these docs capture the raw HID command set so a CLI
can replicate it.

## Devices group by protocol, not by PID

The Launcher does not match a fixed PID table for config — it connects to any
`VID 0x3434` mouse and **detects the protocol at runtime** via a handshake
(reply bytes decide `1k` / `4k` / `8k` / `8k_nordic`). Each protocol has its own
command classes; named products map onto a protocol.

| Device | productID | dongle PID | Protocol | Doc |
|---|---|---|---|---|
| **Ultra-Link 8K dongle** (our unit) | — | `0xD028` | **8k_nordic** | [8k-nordic.md](8k-nordic.md) ✅ verified |
| M6 4K | 0x0624 | 0xD046 | 4k | [4k.md](4k.md) |
| M3mini4K (alu) | 0x0622 | 0xd041 | 4k | [4k.md](4k.md) |
| M3M2 4K | 0x0623 | 0xd045 | 4k | [4k.md](4k.md) |
| M3mini4K | 0x0620 | 0xd037 | 4k | [4k.md](4k.md) |
| M4 4K | 0x0621 | 0xd040 | 4k | [4k.md](4k.md) |
| M3 4K | 0x07a0 | 0xd03c | 4k | [4k.md](4k.md) |
| (1K variants) | — | — | 1k | [1k.md](1k.md) |
| (8K plain) | — | — | 8k | [8k.md](8k.md) |

The `productID`/`PID` table above is the Launcher's firmware-version lookup list;
it is the only hardcoded device list in the bundle. Our `0xD028` is not listed —
it is handled purely by runtime `8k_nordic` detection.

## Channel model (per protocol)

| Protocol | settings READ | settings WRITE | report id |
|---|---|---|---|
| 1k | cmd `7` | short cmds | `81` (0x51) |
| 4k | long cmd `4` | NAPE `167`/`0xA7` | long + 0xFF60 |
| 8k | long cmd `4`/`68` | NAPE `167` / short | long + short |
| 8k_nordic | long cmd `6` | short cmds | `181` (0xB5) / `179` |

Common: short replies marked `228` (`0xE4`), status at `[1]`. 64-byte long NAPE
frames carry a `161 - sum(buf[0:63])` checksum (`0xA1` seed).

## Verification status

- **8k_nordic** — verified live on hardware (firmware 0.1.6). Trust it.
- **1k / 4k / 8k** — extracted from the JS only; no hardware on hand. Offsets
  should be confirmed with a usbmon capture before trusting writes.
