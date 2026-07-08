#!/usr/bin/env bash
# Kaydence — Linux injection live demo (P1-P0-3, ADR-0013).
#
# Runs the AT-SPI-detect + uinput-type harness so you can SEE the injection path
# work: focus a text field and it types; focus a password field and it refuses.
# This is the operator-in-the-loop validation (Wayland won't let a script move
# keyboard focus, so a human must focus the target).
#
# Run ON the Linux machine, inside its graphical (GNOME) session:
#   bash scripts/linux-inject-demo.sh
#
# First run installs a udev rule granting the `input` group access to
# /dev/uinput (needs sudo once + re-login for group membership). After that no
# root is needed.
set -euo pipefail
cd "$(dirname "$0")/.."

RULE=/etc/udev/rules.d/99-kaydence-uinput.rules
if [ ! -f "$RULE" ]; then
  echo "[demo] installing udev rule for /dev/uinput (sudo, one time)..."
  echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' \
    | sudo tee "$RULE" >/dev/null
  sudo udevadm control --reload-rules && sudo udevadm trigger /dev/uinput || true
  sudo modprobe uinput || true
  if ! id -nG "$USER" | grep -qw input; then
    sudo usermod -aG input "$USER"
    echo "[demo] added $USER to the 'input' group — LOG OUT and back in, then re-run."
    exit 0
  fi
fi

echo "[demo] enabling GNOME accessibility (needed for detection)..."
gsettings set org.gnome.desktop.interface toolkit-accessibility true || true

echo "[demo] building the harness..."
cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml --bin atspi-selftest

echo
echo "[demo] Ready. A text editor and terminal are good targets."
echo "[demo]   - Focus a text field  -> Kaydence types '[Kaydence] ' via uinput."
echo "[demo]   - Focus a password box -> Kaydence REFUSES (secure-field gate)."
echo "[demo] Ctrl-C to stop."
echo
exec ./target/debug/atspi-selftest
