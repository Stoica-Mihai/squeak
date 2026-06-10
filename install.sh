#!/usr/bin/env bash
# squeak installer — build from source + install the udev rule.
#
#   curl -fsSL https://raw.githubusercontent.com/Stoica-Mihai/squeak/main/install.sh | bash
#
# Pick what to install with SQUEAK_TARGET (default: tui):
#   SQUEAK_TARGET=tui      curl ... | bash     # terminal UI only (no webkit)
#   SQUEAK_TARGET=desktop  curl ... | bash     # desktop app + launcher entry
#   SQUEAK_TARGET=both     curl ... | bash
set -euo pipefail

REPO="https://github.com/Stoica-Mihai/squeak"
TARGET="${SQUEAK_TARGET:-tui}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"

say() { printf '\033[1;34m::\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

have cargo || die "cargo not found — install Rust (https://rustup.rs), then re-run."
have git   || die "git not found."
case "$TARGET" in tui|desktop|both) ;; *) die "SQUEAK_TARGET must be tui, desktop, or both";; esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
say "cloning $REPO"
git clone --depth 1 "$REPO" "$work" >/dev/null 2>&1
cd "$work"

if [ "$TARGET" = tui ] || [ "$TARGET" = both ]; then
  say "building + installing the terminal UI (squeak → ~/.cargo/bin)"
  cargo install --path crates/squeak-tui --locked
fi

if [ "$TARGET" = desktop ] || [ "$TARGET" = both ]; then
  pkg-config --exists webkit2gtk-4.1 2>/dev/null \
    || die "webkit2gtk-4.1 missing — install it (Arch: webkit2gtk-4.1, Debian: libwebkit2gtk-4.1-dev) and re-run."
  say "building the desktop app (release)"
  cargo build --release -p squeak-desktop
  say "installing squeak-desktop → /usr/local/bin (sudo)"
  sudo install -Dm755 target/release/squeak-desktop /usr/local/bin/squeak-desktop
  install -Dm644 crates/squeak-desktop/icons/icon.png "$data/icons/hicolor/512x512/apps/squeak-desktop.png"
  install -Dm644 packaging/squeak-desktop.desktop "$data/applications/squeak-desktop.desktop"
  update-desktop-database "$data/applications" 2>/dev/null || true
  gtk-update-icon-cache -f -t "$data/icons/hicolor" 2>/dev/null || true
fi

say "installing udev rule → /etc/udev/rules.d (sudo)"
sudo install -Dm644 packaging/99-keychron.rules /etc/udev/rules.d/99-keychron.rules
sudo udevadm control --reload-rules && sudo udevadm trigger --action=add || true

say "done."
[ "$TARGET" != desktop ] && echo "  terminal UI:  squeak   (ensure ~/.cargo/bin is on PATH)"
[ "$TARGET" != tui ]     && echo "  desktop app:  squeak-desktop   (also in your launcher as 'squeak')"
echo "  replug the device so the udev rule applies."
