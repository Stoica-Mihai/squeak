"""CLI: Keychron M6 / Ultra-Link 8K (8k_nordic) configuration."""

import argparse
import json

from keycron.device import Device
from keycron import dpi as dpi_mod
from keycron import polling as poll_mod
from keycron import block as block_mod
from keycron import settings as S
from keycron import buttons as B
from keycron import macro as Mac


def cmd_info(dev, args):
    data = block_mod.read_all(dev)
    if args.json:
        print(json.dumps(data, indent=2))
        return
    p = poll_mod.CODE_TO_HZ
    b = data
    print(f"battery     : {b['battery']['percent']}%{' (charging)' if b['battery']['charging'] else ''}")
    print(f"profile     : current {b['profile']['current']} of {b['profile']['count']}")
    print(f"DPI presets : {b['dpi']['presets']} (active levels {b['dpi']['active_levels']})")
    print(f"polling     : {p.get(b['polling']['levels'][0], '?')} Hz")
    print(f"sensor      : LOD={b['sensor']['lod']} scrollDir={b['sensor']['scroll_dir']} "
          f"motion={b['sensor']['motion_sync']} wave={b['sensor']['wave']} "
          f"line={b['sensor']['line']} fps20k={b['sensor']['fps20k']} angle={b['sensor']['angle']}")
    print(f"debounce    : {b['debounce']['value']} ms")
    print(f"sleep       : {b['sleep_s']} s")


def cmd_battery(dev, _):
    bt = block_mod.read_all(dev)["battery"]
    print(f"{bt['percent']}%{' (charging)' if bt['charging'] else ''}")


def cmd_dpi(dev, args):
    if args.value is None:
        a, p = dpi_mod.get_dpi(dev)
        print(f"presets {p}, active {a} = {p[a]} DPI")
    else:
        print(f"DPI -> {dpi_mod.set_dpi(dev, args.value, index=args.index)}")


def cmd_polling(dev, args):
    if args.hz is None:
        print(f"{poll_mod.get_rate_hz(dev)} Hz")
    else:
        print(f"polling -> {poll_mod.set_rate(dev, args.hz)} Hz")


def cmd_sensor(dev, args):
    kw = {k: v for k, v in dict(lod=args.lod, scroll_dir=args.scroll_dir,
          motion=args.motion, wave=args.wave, line=args.line,
          fps20k=args.fps20k).items() if v is not None}
    if not kw:
        print(json.dumps(block_mod.read_all(dev)["sensor"]))
    else:
        print(f"sensor -> {S.set_sensor(dev, **kw)}")


def cmd_angle(dev, args):
    if args.degrees is None:
        print(block_mod.read_all(dev)["sensor"]["angle"])
    else:
        print(f"angle -> {S.set_angle(dev, args.degrees, enable=args.degrees != 0)}")


def cmd_debounce(dev, args):
    if args.ms is None:
        print(f"{block_mod.read_all(dev)['debounce']['value']} ms")
    else:
        print(f"debounce -> {S.set_debounce(dev, args.ms)} ms")


def cmd_sleep(dev, args):
    if args.seconds is None:
        print(f"{block_mod.read_all(dev)['sleep_s']} s")
    else:
        print(f"sleep -> {S.set_sleep(dev, args.seconds)} s")


def cmd_profile(dev, args):
    if not args.indices:
        print(json.dumps(block_mod.read_all(dev)["profile"]))
    else:
        print(f"profile -> {S.set_profile_select(dev, args.indices)}")


def cmd_buttons(dev, args):
    for b in B.get_all(dev, args.count):
        label = b["name"] or (f"0x{b['data']:06x}" if b["type_id"] not in (0, 9) else "")
        print(f"  id {b['id']:2d}: {b['type']:9s} {label}")


def cmd_button(dev, args):
    if args.action is None:
        print(json.dumps(B.get_button(dev, args.id)))
    elif args.action == "mouse":
        print(B.set_mouse(dev, args.id, args.value))
    elif args.action == "key":
        print(B.set_keyboard(dev, args.id, int(args.value, 0),
                             sum(B.MOD[m] for m in args.mods)))
    elif args.action == "disable":
        print(B.disable(dev, args.id))
    elif args.action == "default":
        print(B.restore_default(dev, args.id))


def cmd_macro(dev, args):
    if args.mode == "click":
        events = []
        for c in args.args:
            events += Mac.click(c)
    else:  # text
        events = Mac.type_text(" ".join(args.args))
    print(Mac.set_macro(dev, args.id, events))


def cmd_reset(dev, args):
    S.factory_reset(dev, categories=args.categories or None, confirm=args.yes)
    print("factory reset sent" + (f" ({','.join(args.categories)})" if args.categories else " (all)"))


def main():
    p = argparse.ArgumentParser(prog="keycron", description="Keychron M6 (8k_nordic) config")
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("info", help="show all settings")
    s.add_argument("--json", action="store_true")
    s.set_defaults(fn=cmd_info)

    sub.add_parser("battery", help="battery percent").set_defaults(fn=cmd_battery)

    s = sub.add_parser("dpi", help="get/set DPI")
    s.add_argument("value", nargs="?", type=int)
    s.add_argument("--index", type=int, default=None)
    s.set_defaults(fn=cmd_dpi)

    s = sub.add_parser("polling", help="get/set polling Hz")
    s.add_argument("hz", nargs="?", type=int)
    s.set_defaults(fn=cmd_polling)

    s = sub.add_parser("sensor", help="get/set sensor params")
    for f in ("lod", "scroll-dir", "motion", "wave", "line", "fps20k"):
        s.add_argument(f"--{f}", dest=f.replace("-", "_"), type=int, default=None)
    s.set_defaults(fn=cmd_sensor)

    s = sub.add_parser("angle", help="get/set angle snap degrees (0=off)")
    s.add_argument("degrees", nargs="?", type=int)
    s.set_defaults(fn=cmd_angle)

    s = sub.add_parser("debounce", help="get/set debounce ms")
    s.add_argument("ms", nargs="?", type=int)
    s.set_defaults(fn=cmd_debounce)

    s = sub.add_parser("sleep", help="get/set idle sleep seconds")
    s.add_argument("seconds", nargs="?", type=int)
    s.set_defaults(fn=cmd_sleep)

    s = sub.add_parser("profile", help="get/select DPI profiles")
    s.add_argument("indices", nargs="*", type=int)
    s.set_defaults(fn=cmd_profile)

    s = sub.add_parser("buttons", help="list button assignments")
    s.add_argument("--count", type=int, default=16)
    s.set_defaults(fn=cmd_buttons)

    s = sub.add_parser("button", help="get/set one button")
    s.add_argument("id", type=int)
    s.add_argument("action", nargs="?", choices=["mouse", "key", "disable", "default"])
    s.add_argument("value", nargs="?", help="mouse action name, or key HID code")
    s.add_argument("--mods", nargs="*", default=[], choices=list(B.MOD))
    s.set_defaults(fn=cmd_button)

    s = sub.add_parser("macro", help="record a macro onto a button (short macros only)")
    s.add_argument("id", type=int)
    s.add_argument("mode", choices=["click", "text"])
    s.add_argument("args", nargs="+",
                   help="click: button names (left right ..); text: a string to type")
    s.set_defaults(fn=cmd_macro)

    s = sub.add_parser("reset", help="factory reset (needs --yes)")
    s.add_argument("--categories", nargs="*", choices=S.RESET_CATEGORIES)
    s.add_argument("--yes", action="store_true", help="confirm destructive reset")
    s.set_defaults(fn=cmd_reset)

    args = p.parse_args()
    with Device() as dev:
        args.fn(dev, args)


if __name__ == "__main__":
    main()
