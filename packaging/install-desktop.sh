#!/usr/bin/env bash
# Install squeak-desktop system-wide + a launcher entry. Run from the repo.
set -euo pipefail
cd "$(dirname "$0")/.."

data="${XDG_DATA_HOME:-$HOME/.local/share}"

echo "building release…"
cargo build --release -p squeak-desktop

echo "installing binary to /usr/local/bin (needs sudo)…"
sudo install -Dm755 target/release/squeak-desktop /usr/local/bin/squeak-desktop

echo "installing icon + launcher entry…"
install -Dm644 crates/squeak-desktop/icons/icon.png \
  "$data/icons/hicolor/512x512/apps/squeak.png"
install -Dm644 packaging/squeak-desktop.desktop \
  "$data/applications/squeak-desktop.desktop"

update-desktop-database "$data/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "$data/icons/hicolor" 2>/dev/null || true

echo "done — 'squeak' should appear in your launcher (run squeak-desktop to launch)."
echo "uninstall: sudo rm /usr/local/bin/squeak-desktop; rm $data/applications/squeak-desktop.desktop $data/icons/hicolor/512x512/apps/squeak.png"
