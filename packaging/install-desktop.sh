#!/usr/bin/env bash
# Install squeak-desktop system-wide + a launcher entry. Run from the repo.
set -euo pipefail
cd "$(dirname "$0")/.."

data="${XDG_DATA_HOME:-$HOME/.local/share}"

echo "building release…"
cargo build --release -p squeak-desktop

echo "installing binary to /usr/local/bin (needs sudo)…"
sudo install -Dm755 target/release/squeak-desktop /usr/local/bin/squeak-desktop

echo "installing icons + launcher entry…"
icons="$data/icons/hicolor"

# Install every size we ship at its exact dimensions. hicolor advertises each
# size directory for a narrow request range (512x512 covers ~510-514px), so a
# lone 512 is invisible to any shell that honours those ranges without falling
# back to the nearest size — the icon silently renders blank.
install -Dm644 crates/squeak-desktop/icons/32.png "$icons/32x32/apps/squeak-desktop.png"
install -Dm644 crates/squeak-desktop/icons/128.png "$icons/128x128/apps/squeak-desktop.png"
install -Dm644 crates/squeak-desktop/icons/icon.png "$icons/512x512/apps/squeak-desktop.png"

# Bars and docks ask for the small sizes; derive the ones we don't ship.
derived=(16 22 24 48 64 256)
if im=$(command -v magick || command -v convert); then
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  for s in "${derived[@]}"; do
    "$im" crates/squeak-desktop/icons/icon.png -resize "${s}x${s}" -strip "PNG:$tmp/$s.png"
    install -Dm644 "$tmp/$s.png" "$icons/${s}x${s}/apps/squeak-desktop.png"
  done
else
  echo "  note: ImageMagick not found — installed 32/128/512 only."
  echo "  a shell that requests a size we didn't install may show no icon."
fi

install -Dm644 packaging/squeak-desktop.desktop \
  "$data/applications/squeak-desktop.desktop"

update-desktop-database "$data/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "$icons" 2>/dev/null || true

echo "done — 'squeak' should appear in your launcher (run squeak-desktop to launch)."
echo "a running shell may cache icon lookups; restart it if the icon is missing."
echo "uninstall: sudo rm /usr/local/bin/squeak-desktop"
echo "           rm $data/applications/squeak-desktop.desktop"
echo "           rm $icons/*/apps/squeak-desktop.png"
