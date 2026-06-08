"""Parse the 8k_nordic settings block (long-channel cmd 0x06).

Field offsets verified live against firmware 0.1.6 and the Launcher's parser
(class `zt`). `b` is the report body with the report id stripped (b[0] = 0x06).
"""

CMD_GET_BLOCK = 0x06


def _le16(b, o):
    return b[o] | (b[o + 1] << 8)


def read_block(dev):
    r = dev.get(CMD_GET_BLOCK)
    if not r or r[1] != CMD_GET_BLOCK:
        raise RuntimeError(f"block read failed: {r}")
    return r[1:]  # strip report id -> Launcher's Ze[] indexing


def parse(b):
    """Return all device settings decoded from the block body `b`."""
    return {
        "profile": {"current": b[1], "count": b[50]},
        "dpi": {
            "active_levels": [b[2] & 15, b[3] & 15, b[4] & 15],
            "presets": [_le16(b, 5), _le16(b, 7), _le16(b, 9), _le16(b, 11), _le16(b, 13)],
            "count": b[16],
            "max": _le16(b, 40),
            "step": b[42] or 50,
        },
        "polling": {
            "levels": [b[2] >> 4, b[3] >> 4, b[4] >> 4],
            "rate_codes": list(b[43:49]),
            "count": b[49] or 6,
        },
        "sensor": {
            "lod": b[15] & 3,
            "wave": (b[15] >> 2) & 1,
            "line": (b[15] >> 3) & 1,
            "motion_sync": (b[15] >> 4) & 1,
            "scroll_dir": (b[15] >> 6) & 1,
            "fps20k": b[52] & 1,
            "angle": b[55] - 256 if b[55] > 90 else b[55],
        },
        "debounce": {"value": b[17], "values": list(b[30:40])},
        "scroll": {"speed": b[27], "inertia": b[28], "spl": b[29]},
        "sleep_s": b[18],
        "battery": {"percent": b[19] & 127, "charging": b[19] >> 7},
        "wake": {
            "key": (b[51] >> 4) & 1, "scroll": (b[51] >> 5) & 1,
            "move": (b[51] >> 6) & 1, "side_scroll": (b[51] >> 7) & 1,
        },
        "support_flags": b[26],
    }


def read_all(dev):
    return parse(read_block(dev))
