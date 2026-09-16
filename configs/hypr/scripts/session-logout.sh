#!/bin/sh
# Clean session teardown for Xiu.
# Stops systemd user units, terminates session daemons, cleans runtime lockfiles,
# and cleanly exits Hyprland or uwsm.

set -u

# 1. Stop systemd user session units
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user stop hyprland-session.target 2>/dev/null || true
    systemctl --user stop hypridle.service hyprpolkitagent.service hyprsunset.service 2>/dev/null || true
fi

# 2. Stop watchdogs first to prevent respawning surfaces
pkill -TERM -f "watchdog.sh" 2>/dev/null || true

# 3. Gracefully terminate session processes and scripts
pkill -TERM -f "cliphist-watch.sh" 2>/dev/null || true
pkill -TERM -f "clipboard-watch.sh" 2>/dev/null || true
pkill -TERM -f "wallpaper.sh" 2>/dev/null || true
pkill -TERM -f "launch-guard.sh" 2>/dev/null || true
pkill -TERM -x clipvault 2>/dev/null || true
pkill -TERM -x wl-paste 2>/dev/null || true
pkill -TERM -f "xiu-resizer" 2>/dev/null || true
pkill -TERM -x awww-daemon 2>/dev/null || true
pkill -TERM -x swww-daemon 2>/dev/null || true
pkill -TERM -x cava 2>/dev/null || true
pkill -TERM -x quickshell 2>/dev/null || true
pkill -TERM -x qs 2>/dev/null || true

# Brief grace period for clean exit
sleep 0.15

# Force kill any lingering daemons
pkill -KILL -f "watchdog.sh" 2>/dev/null || true
pkill -KILL -f "cliphist-watch.sh" 2>/dev/null || true
pkill -KILL -f "clipboard-watch.sh" 2>/dev/null || true
pkill -KILL -f "wallpaper.sh" 2>/dev/null || true
pkill -KILL -f "launch-guard.sh" 2>/dev/null || true
pkill -KILL -x clipvault 2>/dev/null || true
pkill -KILL -x wl-paste 2>/dev/null || true
pkill -KILL -f "xiu-resizer" 2>/dev/null || true
pkill -KILL -x awww-daemon 2>/dev/null || true
pkill -KILL -x swww-daemon 2>/dev/null || true
pkill -KILL -x cava 2>/dev/null || true
pkill -KILL -x quickshell 2>/dev/null || true
pkill -KILL -x qs 2>/dev/null || true

# 4. Clean up session lockfiles, sockets, and temporary files
rt="${XDG_RUNTIME_DIR:-/tmp}"
rm -f "$rt"/xiu-*.lock "$rt"/*-watchdog.lock "$rt"/launch-guard.* "$rt"/xiu-resizer.sock "$rt"/ricelin-lock-* "$rt"/xiu-lock-* 2>/dev/null || true

# 5. Exit compositor
if [ -n "${UWSM_ID:-}" ] && command -v uwsm >/dev/null 2>&1; then
    exec uwsm stop
elif command -v hyprctl >/dev/null 2>&1; then
    exec hyprctl dispatch exit
fi
