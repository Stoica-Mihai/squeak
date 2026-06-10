#!/usr/bin/env bash
# squeak uninstaller — removes everything install.sh placed.
#
#   curl -fsSL https://raw.githubusercontent.com/Stoica-Mihai/squeak/main/uninstall.sh | bash
#
# Keeps the udev rule with KEEP_UDEV=1 (it's harmless and shared with the
# browser Launcher).
set -euo pipefail

data="${XDG_DATA_HOME:-$HOME/.local/share}"
cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
say() { printf '\033[1;34m::\033[0m %s\n' "$*"; }

say "removing terminal UI"
cargo uninstall squeak-tui >/dev/null 2>&1 || rm -f "$cargo_bin/squeak"

say "removing desktop app + launcher entry"
[ -e /usr/local/bin/squeak-desktop ] && sudo rm -f /usr/local/bin/squeak-desktop
rm -f "$data/applications/squeak-desktop.desktop"
rm -f "$data/icons/hicolor/512x512/apps/squeak.png"
update-desktop-database "$data/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "$data/icons/hicolor" 2>/dev/null || true

if [ "${KEEP_UDEV:-0}" != 1 ] && [ -e /etc/udev/rules.d/99-keychron.rules ]; then
  say "removing udev rule (set KEEP_UDEV=1 to keep)"
  sudo rm -f /etc/udev/rules.d/99-keychron.rules
  sudo udevadm control --reload-rules 2>/dev/null || true
fi

say "done — squeak removed."
