"""Decode Keychron command frames from the usbmon text stream.

Run as root WHILE driving the Launcher in Chrome:
    sudo modprobe usbmon
    sudo python3 capture.py 7        # 7 = dongle bus (usb7)

Then in the Launcher change DPI by one step, then polling rate by one step.
Each OUT report starting 0xB3/0xB5 (host->device command) and the following
0xB4/0xB6 IN report (device reply) are printed with offsets, so the exact
DPI / polling byte positions become visible. Ctrl-C to stop.
"""

import sys

BUS = sys.argv[1] if len(sys.argv) > 1 else "7"
NODE = f"/sys/kernel/debug/usb/usbmon/{BUS}u"

import os

# Set CAPTURE_ALL=1 to log every report (use when hunting an unknown channel,
# e.g. button remap / macros). Default: just the known mouse command reports.
ALL = os.environ.get("CAPTURE_ALL") == "1"
OUT_IDS = {0xB3, 0xB5}   # host -> device command reports
IN_IDS = {0xB4, 0xB6}    # device -> host reply reports
# Live mouse HID input reports (movement/buttons) — noise; skip in ALL mode.
NOISE_IDS = {0x01, 0x02}


def parse_data(tokens):
    """Return list of data bytes from a usbmon line's token list, or None."""
    if "=" not in tokens:
        return None
    words = tokens[tokens.index("=") + 1:]
    hexstr = "".join(words)
    try:
        return [int(hexstr[i:i + 2], 16) for i in range(0, len(hexstr), 2)]
    except ValueError:
        return None


def show(direction, data):
    rid = data[0]
    body = data[1:]
    nonzero = [(i, b) for i, b in enumerate(body) if b]
    ascii_ = "".join(chr(b) if 32 <= b < 127 else "." for b in body[:24])
    print(f"\n{direction} report 0x{rid:02x} ({len(body)}B)")
    print("  hex :", " ".join(f"{b:02x}" for b in body[:32]))
    print("  off :", " ".join(f"{i:02d}" for i in range(min(32, len(body)))))
    print("  ascii:", ascii_)
    print("  nonzero offsets:", [(i, hex(b)) for i, b in nonzero[:16]])


def main():
    print(f"reading {NODE} (bus {BUS}). Drive the Launcher now. Ctrl-C to stop.")
    with open(NODE) as f:
        for line in f:
            tok = line.split()
            # need: tag ts type addr ... ; type 'S'=submit (host out), 'C'=callback (in)
            if len(tok) < 4:
                continue
            urb_type = tok[2]
            addr = tok[3]                 # e.g. 'Io:7:012:2'
            parts = addr.split(":")
            if len(parts) < 2 or parts[1] != BUS:
                continue
            data = parse_data(tok)
            if not data:
                continue
            rid = data[0]
            if ALL and rid in NOISE_IDS:
                continue
            if urb_type == "S" and (ALL or rid in OUT_IDS):
                show("OUT", data)
            elif urb_type == "C" and (ALL or rid in IN_IDS):
                show("IN ", data)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nstopped.")
