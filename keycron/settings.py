"""Short-channel (0xB5) setters for 8k_nordic, decoded from the Launcher.

Sensor/scroll/fps are encoded as `bit ? bit : 2` on write (Launcher `x||2`) and
`+1` for scrollDir/fps20k; we reconstruct the full cmd-0x42 frame from the
current block so unspecified fields are preserved. Every setter re-reads the
block and confirms.
"""

from keycron.block import read_all

CMD_SENSOR = 0x42
CMD_DEBOUNCE = 0x43
CMD_PROFILE_SELECT = 0x45
CMD_SLEEP = 0x0A
CMD_LPM = 0x0B
CMD_RESET = 0x0F

RESET_CATEGORIES = ["key", "dpi", "light", "sensor", "profile", "scroll"]


def _bit_or2(v):
    return v if v else 2


def set_sensor(dev, *, lod=None, scroll_dir=None, motion=None, wave=None,
               line=None, fps20k=None):
    """Set sensor params (cmd 0x42). Unspecified fields keep current values."""
    s = read_all(dev)["sensor"]
    lod = s["lod"] if lod is None else lod
    wave = s["wave"] if wave is None else wave
    line = s["line"] if line is None else line
    motion = s["motion_sync"] if motion is None else motion
    scroll_dir = s["scroll_dir"] if scroll_dir is None else scroll_dir
    fps20k = s["fps20k"] if fps20k is None else fps20k

    payload = [0] * 10
    payload[0] = lod
    payload[1] = _bit_or2(wave)
    payload[2] = _bit_or2(line)
    payload[3] = _bit_or2(motion)
    payload[5] = (scroll_dir & 1) + 1
    payload[7] = (fps20k & 1) + 1
    ok, resp = dev.set(CMD_SENSOR, *payload)
    if not ok:
        raise RuntimeError(f"sensor set rejected: {resp}")
    return read_all(dev)["sensor"]


def set_angle(dev, angle, enable=True):
    """Set angle snapping (cmd 0x42, alt fields)."""
    payload = [0] * 10
    payload[8] = 2 if enable else 0   # buf[9]
    payload[9] = angle & 0xFF          # buf[10]
    ok, resp = dev.set(CMD_SENSOR, *payload)
    if not ok:
        raise RuntimeError(f"angle set rejected: {resp}")
    return read_all(dev)["sensor"]["angle"]


def set_sleep(dev, seconds):
    """Set idle sleep timeout in seconds (cmd 0x0A)."""
    ok, resp = dev.set(CMD_SLEEP, 1, seconds & 0xFF)
    if not ok:
        raise RuntimeError(f"sleep set rejected: {resp}")
    after = read_all(dev)["sleep_s"]
    if after != (seconds & 0xFF):
        raise RuntimeError(f"sleep unconfirmed: wanted {seconds}, read {after}")
    return after


def set_debounce(dev, ms, profile=None):
    """Set debounce time in ms (cmd 0x43)."""
    if profile is None:
        ok, resp = dev.set(CMD_DEBOUNCE, ms)
    else:
        ok, resp = dev.set(CMD_DEBOUNCE, ms, 17, profile, ms)
    if not ok:
        raise RuntimeError(f"debounce set rejected: {resp}")
    after = read_all(dev)["debounce"]["value"]
    if after != ms:
        raise RuntimeError(f"debounce unconfirmed: wanted {ms}, read {after}")
    return after


def set_lpm(dev, value):
    """Set wireless low-power mode (cmd 0x0B)."""
    ok, resp = dev.set(CMD_LPM, 1, value or 17, 0)
    if not ok:
        raise RuntimeError(f"lpm set rejected: {resp}")
    return True


def set_profile_select(dev, indices):
    """Select active DPI profiles/order (cmd 0x45). `indices` = up to 3 ints."""
    idx = (list(indices) + [0, 0, 0])[:3]
    ok, resp = dev.set(CMD_PROFILE_SELECT, *idx)
    if not ok:
        raise RuntimeError(f"profile select rejected: {resp}")
    return read_all(dev)["profile"]


def factory_reset(dev, categories=None, confirm=False):
    """Factory reset (cmd 0x0F). DESTRUCTIVE.

    categories=None resets all; else a subset of RESET_CATEGORIES. Requires
    confirm=True.
    """
    if not confirm:
        raise RuntimeError("factory_reset requires confirm=True")
    if categories is None:
        ok, resp = dev.set(CMD_RESET, 255)
    else:
        mask = 0
        for i, name in enumerate(RESET_CATEGORIES):
            if name in categories:
                mask |= 1 << i
        ok, resp = dev.set(CMD_RESET, 0, mask)
    if not ok:
        raise RuntimeError(f"reset rejected: {resp}")
    return True
