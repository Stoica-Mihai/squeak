"""Keychron M6 / Ultra-Link 8K device access over hidraw (cython-hidapi)."""

import hid

VID = 0x3434

# Known config-collection identifiers. Device in hand = Ultra-Link 8K dongle.
PID_M6_4K = 0x0624          # direct USB cable (per FINDINGS, not the unit in hand)
PID_ULTRALINK_8K = 0xD028   # 2.4 GHz dongle, config on usage_page 0xFFC1
USAGE_PAGE_CONFIG = 0xFFC1

# Report-descriptor channels (from hidraw6 report descriptor):
#   long  : 0xB3 OUTPUT 63B -> reply 0xB4 INPUT 63B   (used for READ/GET)
#   short : 0xB5 OUTPUT 20B -> reply 0xB6 INPUT 20B   (used for WRITE/SET)
LONG_OUT, LONG_IN, LONG_LEN = 0xB3, 0xB4, 63
SHORT_OUT, SHORT_IN, SHORT_LEN = 0xB5, 0xB6, 20

# Short-channel reply: [0xB6, 0xE4, status, cmd_echo, ...]; status 0=OK, 7=fail.
SHORT_REPLY_MARK = 0xE4
STATUS_OK = 0x00


def find(pid=None, usage_page=USAGE_PAGE_CONFIG):
    """Return the hidraw path for the config collection (any M6 PID), or None.

    Matches the 0xFFC1 config collection across both the dongle (0xD028) and the
    wired mouse (0xD049, etc). Pass `pid` to pin a specific device.
    """
    for d in hid.enumerate(VID, pid or 0):
        if d["usage_page"] == usage_page:
            return d["path"]
    return None


class Device:
    def __init__(self, path=None):
        self.path = path or find()
        if self.path is None:
            raise RuntimeError("Keychron config collection not found (check udev + dongle)")
        self._h = hid.device()

    def __enter__(self):
        self._h.open_path(self.path)
        return self

    def __exit__(self, *exc):
        self._h.close()

    def _frame(self, report_id, payload_len, payload):
        buf = [report_id, *payload]
        buf += [0] * (1 + payload_len - len(buf))
        return buf

    def _write(self, report_id, payload_len, payload):
        n = self._h.write(self._frame(report_id, payload_len, payload))
        if n < 0:
            raise RuntimeError(f"write failed: {self._h.error()}")

    def _read(self, want_id, payload_len, timeout_ms):
        """Read input reports until one with report id == want_id arrives."""
        left = timeout_ms
        while left > 0:
            resp = self._h.read(1 + payload_len, 200)
            if resp and resp[0] == want_id:
                return resp
            left -= 200
        return []

    def get(self, cmd, *payload, timeout_ms=1000):
        """Long-channel READ: 0xB3 -> 0xB4. Returns reply (incl. report id)."""
        self._write(LONG_OUT, LONG_LEN, [cmd, *payload])
        return self._read(LONG_IN, LONG_LEN, timeout_ms)

    def set(self, cmd, *payload, timeout_ms=1000):
        """Short-channel WRITE: 0xB5 -> 0xB6. Returns (ok, reply)."""
        self._write(SHORT_OUT, SHORT_LEN, [cmd, *payload])
        resp = self._read(SHORT_IN, SHORT_LEN, timeout_ms)
        ok = len(resp) >= 4 and resp[1] == SHORT_REPLY_MARK and resp[2] == STATUS_OK
        return ok, resp

    def long_set(self, cmd, *payload, timeout_ms=1000):
        """WRITE on long channel (0xB3), ack on short (0xB6). Used by buttons."""
        self._write(LONG_OUT, LONG_LEN, [cmd, *payload])
        resp = self._read(SHORT_IN, SHORT_LEN, timeout_ms)
        ok = (len(resp) >= 4 and resp[1] == SHORT_REPLY_MARK
              and resp[2] == STATUS_OK and resp[3] == cmd)
        return ok, resp

    def long_raw(self, *payload, timeout_ms=1000):
        """WRITE arbitrary payload on long channel (0xB3), return short reply."""
        self._write(LONG_OUT, LONG_LEN, list(payload))
        return self._read(SHORT_IN, SHORT_LEN, timeout_ms)
