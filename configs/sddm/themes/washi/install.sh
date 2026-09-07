#!/usr/bin/env bash
set -euo pipefail

THEME_NAME="washi"
SRC_DIR="$(cd "$(dirname "$0")" && pwd)"
DEST_DIR="/usr/share/sddm/themes/${THEME_NAME}"
CONF_DIR="/etc/sddm.conf.d"
CONF_FILE="${CONF_DIR}/10-theme.conf"

echo ":: Installing SDDM theme '${THEME_NAME}'"
echo "   source : ${SRC_DIR}"
echo "   target : ${DEST_DIR}"

echo ":: Copying theme files (sudo)"
sudo install -d -m 0755 "${DEST_DIR}/current"
sudo cp -aT "${SRC_DIR}" "${DEST_DIR}"
sudo rm -f "${DEST_DIR}/install.sh" "${DEST_DIR}/Xsetup-washi.sh"

echo ":: Installing X11 setup script (primary output + cursor warp)"
sudo install -D -m 0755 "${SRC_DIR}/Xsetup-washi.sh" /etc/sddm/Xsetup-washi.sh
sudo tee "${CONF_DIR}/20-xsetup.conf" >/dev/null <<EOF
[X11]
DisplayCommand=/etc/sddm/Xsetup-washi.sh
EOF

echo ":: Seeding the current wallpaper and face, when they exist"
# The greeter runs as root and reads current/wallpaper + current/face; until
# the next wallpaper change runs sddm_sync, seed them from the user's state.
STATE="${XDG_STATE_HOME:-$HOME/.local/state}/ricelin-wallpaper"
[ -f "$STATE" ] && sudo install -m 0644 "$(cat "$STATE")" "${DEST_DIR}/current/wallpaper" 2>/dev/null || true
[ -f "$HOME/.face" ] && sudo install -m 0644 "$HOME/.face" "${DEST_DIR}/current/face" 2>/dev/null || true

echo ":: Writing ${CONF_FILE} (sudo)"
sudo install -d -m 0755 "${CONF_DIR}"
sudo tee "${CONF_FILE}" >/dev/null <<EOF
[Theme]
Current=${THEME_NAME}
EOF

echo ":: Disabling SDDM on-screen virtual keyboard"
sudo tee "${CONF_DIR}/virtualkeyboard.conf" >/dev/null <<EOF
[General]
InputMethod=
EOF

echo ":: Done. Theme installed and selected."
echo "   Test without logging out:"
echo "     sddm-greeter-qt6 --test-mode --theme ${DEST_DIR}"
echo "   The live wallpaper + palette land in ${DEST_DIR}/current on the next"
echo "   wallpaper change (sddm_sync in hypr/scripts/wallpaper.sh); until then"
echo "   the greeter uses the static washi fallback colors from theme.conf."
echo "   The sddm service was NOT touched; (re)start it yourself when ready."
