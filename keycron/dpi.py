"""DPI read/write for the Ultra-Link 8K dongle (decoded from usbmon capture)."""

CMD_GET_DPI = 0x06    # long channel: returns the DPI block
CMD_SET_DPI = 0x40    # short channel: sets the 5-preset DPI table

NUM_PRESETS = 5
DPI_MIN, DPI_MAX = 50, 5000

# GET reply layout (resp[0]=0xB4 report id):
#   [1]=0x06 echo  [5]=active profile index  [6,8,10,12,14]=presets LE16
ACTIVE_OFF = 5
PRESET_OFF = 6


def _le16(lo, hi):
    return lo | (hi << 8)


def get_dpi(dev):
    """Return (active_index, [preset0..preset4]) from the device."""
    r = dev.get(CMD_GET_DPI)
    if not r or r[1] != CMD_GET_DPI:
        raise RuntimeError(f"DPI get failed: {r}")
    active = r[ACTIVE_OFF]
    presets = [_le16(r[PRESET_OFF + 2 * i], r[PRESET_OFF + 2 * i + 1])
               for i in range(NUM_PRESETS)]
    return active, presets


def set_dpi(dev, value, index=None):
    """Set the DPI of preset `index` (default: active profile) to `value`.

    Replays the Launcher's SET frame: [0x40, 0x03, active, active, <5 LE16
    presets>, 0x05]. Re-reads to confirm the write took.
    """
    if not (DPI_MIN <= value <= DPI_MAX):
        raise ValueError(f"DPI {value} out of range {DPI_MIN}..{DPI_MAX}")

    active, presets = get_dpi(dev)
    slot = active if index is None else index
    if not (0 <= slot < NUM_PRESETS):
        raise ValueError(f"preset index {slot} out of range 0..{NUM_PRESETS - 1}")
    presets[slot] = value

    # Launcher (8k_nordic): buf[1..3]=active, presets LE16 @ buf[4..], buf[14]=count.
    table = []
    for p in presets:
        table += [p & 0xFF, (p >> 8) & 0xFF]
    payload = [active, active, active, *table, NUM_PRESETS]

    ok, resp = dev.set(CMD_SET_DPI, *payload)
    if not ok:
        raise RuntimeError(f"DPI set rejected (status!=OK): {resp}")

    _, after = get_dpi(dev)
    if after[slot] != value:
        raise RuntimeError(f"DPI set unconfirmed: wanted {value}, read {after[slot]}")
    return after
