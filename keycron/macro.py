"""Mouse macro upload for 8k_nordic (decoded from usbmon capture).

SET: long channel cmd 0x54 = [id, 0, len, 0, loopCount, loopType, 0x20, 0, 0,
event_count, 0, <events>]; ack 0xB6 e4 00 54. len counts from byte 6 onward
(= 6 + 4*event_count). Setting a macro also binds it to that button id (the
button's type becomes Macro = 4).

Event = [flag, code, delay_lo, delay_hi]; flag 0x88=press, 0x08=release.
Verified for mouse buttons (left=1, right=2). Other codes best-effort.
"""

CMD_SET_MACRO = 0x54

PRESS, RELEASE = 0x88, 0x08
MOUSE_BTN = {"left": 1, "right": 2, "middle": 3, "backward": 4, "forward": 5}

# loopType (Launcher enum we): seen value 1 = "Stop on Release".
LOOP_STOP_ON_RELEASE = 1


def event(button, press, delay=0):
    """One mouse macro event. `button` name, `press` True=down/False=up."""
    code = MOUSE_BTN[button] if isinstance(button, str) else button
    flag = PRESS if press else RELEASE
    return [flag, code, delay & 0xFF, (delay >> 8) & 0xFF]


def click(button="left", delay=0):
    """A full press+release pair for `button`."""
    return event(button, True, delay) + event(button, False, 0)


def set_macro(dev, button_id, events, loop_count=1, loop_type=LOOP_STOP_ON_RELEASE):
    """Upload a macro (list of 4-byte events) and bind it to `button_id`.

    `events` is a flat list of bytes (use event()/click() to build). Re-reads to
    confirm the button became a Macro.
    """
    n_events = len(events) // 4
    length = 6 + 4 * n_events
    header = [button_id, 0x00, length, 0x00, loop_count, loop_type,
              0x20, 0x00, 0x00, n_events, 0x00]
    ok, resp = dev.long_set(CMD_SET_MACRO, *header, *events)
    if not ok:
        raise RuntimeError(f"macro set rejected: {resp}")

    from keycron.buttons import get_button
    after = get_button(dev, button_id)
    if after["type"] != "Macro":
        raise RuntimeError(f"macro set unconfirmed, button is {after}")
    return after
