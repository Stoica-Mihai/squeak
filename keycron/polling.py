"""Polling-rate read/write for the Ultra-Link 8K dongle (decoded from usbmon)."""

CMD_SET_RATE = 0x41   # short channel
CMD_GET_BLOCK = 0x06  # long channel; rate code = high nibble of resp[3]

# Code -> Hz. Six codes seen over the 2.4 GHz dongle (8K rate is cable-only).
CODE_TO_HZ = {0: 125, 1: 250, 2: 500, 3: 1000, 4: 2000, 5: 4000}
HZ_TO_CODE = {hz: code for code, hz in CODE_TO_HZ.items()}

# Constant tail the Launcher appends to the SET frame.
_TAIL = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06]

RATE_OFF = 3  # resp[3]: high nibble = rate code, low nibble = active profile


def get_rate_code(dev):
    r = dev.get(CMD_GET_BLOCK)
    if not r or r[1] != CMD_GET_BLOCK:
        raise RuntimeError(f"polling get failed: {r}")
    return r[RATE_OFF] >> 4


def get_rate_hz(dev):
    code = get_rate_code(dev)
    return CODE_TO_HZ.get(code, code)


def set_rate(dev, hz):
    """Set polling rate to `hz`. Re-reads to confirm."""
    if hz not in HZ_TO_CODE:
        raise ValueError(f"unsupported rate {hz}; choose from {sorted(HZ_TO_CODE)}")
    code = HZ_TO_CODE[hz]
    ok, resp = dev.set(CMD_SET_RATE, code, code, 0x00, *_TAIL)
    if not ok:
        raise RuntimeError(f"polling set rejected: {resp}")
    after = get_rate_code(dev)
    if after != code:
        raise RuntimeError(f"polling set unconfirmed: wanted code {code}, read {after}")
    return CODE_TO_HZ[code]
