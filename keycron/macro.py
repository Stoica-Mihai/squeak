"""Macro upload for 8k_nordic (decoded from usbmon + Launcher export JSON).

The macro frame is cmd 0x54 =
[0x54, id, 0, len, 0, loopCount, loopType, 0x20, 0, 0, n_events, 0, <events>];
len = 6 + 4*n_events (from byte 6). Setting also binds the macro to the button
(its type becomes Macro=4). Invalid button -> status 5.

If the frame fits one 0xB3 report it is sent directly (ack 0xB6 e4 00 54).
Otherwise it is chunked: each 59-byte slice of the full 0x54 frame is sent as
[0x71, seq, slice_len, <slice>] on 0xB3, acked 0xB6 0x72 00, where
seq = 1 + (bytes_sent_before // 16) — the device validates this against its own
running byte count. Verified live (readback length matches).

Event = [flag, code, delay_lo, delay_hi]. flag = press_bit(0x80) | class:
  class 1 = keyboard key   (code = HID usage)        press 0x81 / release 0x01
  class 2 = modifier       (code = modifier bitmask) press 0x82 / release 0x02
  class 8 = mouse button   (code = 1 left/2 right..) press 0x88 / release 0x08
delay = LE16 ms (0 when delay disabled). All three classes verified against the
Launcher (mouse capture + keyboard capture cross-checked with its export JSON).

Long macros (> ~12 events) are chunked by the Launcher via cmd 0x71
(`71 <seq> <len> <chunk>` -> ack `72 00`, reassembled = the 0x54 frame). Not yet
implemented here; set_macro raises if the frame exceeds one report.
"""

CMD_SET_MACRO = 0x54
CMD_CHUNK = 0x71        # chunked upload wrapper
CHUNK_ACK = 0x72       # chunk reply marker on 0xB6
MAX_FRAME = 63         # one 0xB3 report payload
CHUNK_PAYLOAD = 59     # bytes of 0x54 stream per chunk (3-byte 0x71 header + 59 = 62)

CLASS_KEY, CLASS_MOD, CLASS_MOUSE = 1, 2, 8
PRESS = 0x80

MOUSE_BTN = {"left": 1, "right": 2, "middle": 3, "backward": 4, "forward": 5}
MOD = {"ctrl": 1, "shift": 2, "alt": 4, "gui": 8,
       "rctrl": 16, "rshift": 32, "ralt": 64, "rgui": 128}

# Minimal HID usage map for the text helper.
_KC = {**{c: 0x04 + i for i, c in enumerate("abcdefghijklmnopqrstuvwxyz")},
       "1": 0x1e, "2": 0x1f, "3": 0x20, "4": 0x21, "5": 0x22, "6": 0x23,
       "7": 0x24, "8": 0x25, "9": 0x26, "0": 0x27,
       " ": 0x2c, "\n": 0x28, "\t": 0x2b, "-": 0x2d, "=": 0x2e}

LOOP_STOP_ON_RELEASE = 1


def _ev(cls, code, press, delay=0):
    return [(PRESS if press else 0) | cls, code, delay & 0xFF, (delay >> 8) & 0xFF]


def mouse(button, press, delay=0):
    code = MOUSE_BTN[button] if isinstance(button, str) else button
    return _ev(CLASS_MOUSE, code, press, delay)


def key(code, press, delay=0):
    return _ev(CLASS_KEY, code, press, delay)


def modifier(mask, press, delay=0):
    return _ev(CLASS_MOD, mask, press, delay)


def click(button="left", delay=0):
    return mouse(button, True, delay) + mouse(button, False, 0)


def tap(code, delay=0):
    return key(code, True, delay) + key(code, False, 0)


def type_text(text):
    """Build events that type `text` (lowercase letters/digits/space only)."""
    out = []
    for ch in text.lower():
        if ch not in _KC:
            raise ValueError(f"no keycode for {ch!r}")
        out += tap(_KC[ch])
    return out


def set_macro(dev, button_id, events, loop_count=1, loop_type=LOOP_STOP_ON_RELEASE):
    """Upload `events` (flat byte list) to `button_id`. Re-reads to confirm."""
    n_events = len(events) // 4
    length = 6 + 4 * n_events
    frame = [CMD_SET_MACRO, button_id, 0x00, length, 0x00, loop_count,
             loop_type, 0x20, 0x00, 0x00, n_events, 0x00] + list(events)

    if len(frame) <= MAX_FRAME:
        ok, resp = dev.long_set(frame[0], *frame[1:])
        if not ok:
            raise RuntimeError(f"macro rejected (status {resp[2] if resp else '?'}): {resp}")
    else:
        # Chunk the 0x54 frame; seq = 1 + floor(bytes_sent/16) (device's own count).
        for off in range(0, len(frame), CHUNK_PAYLOAD):
            chunk = frame[off:off + CHUNK_PAYLOAD]
            seq = 1 + off // 16
            resp = dev.long_raw(CMD_CHUNK, seq, len(chunk), *chunk)
            if not (len(resp) >= 3 and resp[1] == CHUNK_ACK and resp[2] == 0):
                raise RuntimeError(f"macro chunk @{off} rejected: {resp}")

    from keycron.buttons import get_button
    after = get_button(dev, button_id)
    if after["type"] != "Macro" or after["data"] != length:
        raise RuntimeError(f"macro set unconfirmed (want len {length}): {after}")
    return after
