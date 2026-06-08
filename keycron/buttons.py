"""Button remap for 8k_nordic (decoded from Launcher class Si + usbmon).

SET: long channel cmd 0x52 = [id, 0, type, data_be24]; ack 0xB6 e4 00 52.
GET: long cmd 0x62 [id] -> 0xB4 [62, id, status, type, d_hi, d_mid, d_lo].
Action data is a 24-bit big-endian value whose meaning depends on `type`.
"""

CMD_SET_BUTTON = 0x52
CMD_GET_BUTTON = 0x62

# Action type (Launcher enum S).
TYPE = {0: "Remove", 1: "Mouse", 2: "Keyboard", 3: "Media", 4: "Macro",
        5: "Dpi", 6: "Light", 7: "Game", 8: "ShortCut", 9: "Disable",
        10: "Profile", 13: "PollingRate"}
TYPE_ID = {v: k for k, v in TYPE.items()}

# Mouse action data (Launcher enum _), as the 24-bit value.
MOUSE = {
    "left": 0x010000, "right": 0x020000, "middle": 0x040000,
    "forward": 0x080000, "backward": 0x100000, "leftDouble": 0x800000,
    "upScroll": 0x000200, "downScroll": 0x00fe00, "leftScroll": 0x0000fe,
    "rightScroll": 0x000002,
}
MOUSE_NAME = {v: k for k, v in MOUSE.items()}

# Keyboard modifier bits (Launcher enum b).
MOD = {"ctrl": 1, "shift": 2, "alt": 4, "gui": 8}


def _be24(v):
    return [(v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF]


def get_button(dev, button_id):
    r = dev.get(CMD_GET_BUTTON, button_id)
    if not r or r[1] != CMD_GET_BUTTON:
        raise RuntimeError(f"button get failed: {r}")
    b = r[1:]  # [62, id, status, type, d_hi, d_mid, d_lo]
    typ = b[3]
    data = (b[4] << 16) | (b[5] << 8) | b[6]
    return {"id": button_id, "type": TYPE.get(typ, typ), "type_id": typ,
            "data": data, "name": MOUSE_NAME.get(data) if typ == 1 else None}


def get_all(dev, count=16):
    return [get_button(dev, i) for i in range(count)]


def set_button(dev, button_id, type_id, data):
    """Set a button to (type_id, 24-bit data). Re-reads to confirm."""
    ok, resp = dev.long_set(CMD_SET_BUTTON, button_id, 0, type_id, *_be24(data))
    if not ok:
        raise RuntimeError(f"button set rejected: {resp}")
    after = get_button(dev, button_id)
    if after["type_id"] != type_id or after["data"] != data:
        raise RuntimeError(f"button set unconfirmed: {after}")
    return after


def set_mouse(dev, button_id, action):
    """Assign a mouse action by name (left/right/middle/forward/... )."""
    if action not in MOUSE:
        raise ValueError(f"unknown mouse action {action}; choose {sorted(MOUSE)}")
    return set_button(dev, button_id, TYPE_ID["Mouse"], MOUSE[action])


def set_keyboard(dev, button_id, keycode, mods=0):
    """Assign a keyboard key: HID usage `keycode` with modifier bitmask `mods`."""
    return set_button(dev, button_id, TYPE_ID["Keyboard"], (mods << 8) | keycode)


def disable(dev, button_id):
    return set_button(dev, button_id, TYPE_ID["Disable"], 0)
