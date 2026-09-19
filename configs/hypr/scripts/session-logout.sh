#!/bin/sh
# Clean session teardown for Xiu.
# Stops systemd user units, terminates session daemons, cleans runtime lockfiles,
# and cleanly exits Hyprland or uwsm.

set -u

# 1. Kill watchdogs immediately so nothing can respawn
pkill -KILL -f "watchdog.sh" 2>/dev/null || true

# 2. Dispatch exit to compositor immediately (closes Wayland sockets to all clients)
if [ -n "${UWSM_ID:-}" ] && command -v uwsm >/dev/null 2>&1; then
    uwsm stop 2>/dev/null || true
elif command -v hyprctl >/dev/null 2>&1; then
    hyprctl dispatch exit 2>/dev/null || true
fi

# 3. Stop systemd user session units asynchronously (--no-block avoids waiting)
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user stop --no-block hyprland-session.target hypridle.service hyprpolkitagent.service hyprsunset.service 2>/dev/null || true
fi

# 4. Gracefully terminate session daemons in parallel with combined regex calls
pkill -TERM -f "cliphist-watch\.sh|clipboard-watch\.sh|wallpaper\.sh|launch-guard\.sh|xiu-resizer" 2>/dev/null || true
pkill -TERM -x "clipvault|wl-paste|awww-daemon|swww-daemon|cava|quickshell|qs" 2>/dev/null || true

# 5. Clean up session lockfiles, sockets, and temporary files
rt="${XDG_RUNTIME_DIR:-/tmp}"
rm -f "$rt"/xiu-*.lock "$rt"/*-watchdog.lock "$rt"/launch-guard.* "$rt"/xiu-resizer.sock "$rt"/ricelin-lock-* "$rt"/xiu-lock-* 2>/dev/null || true

# 6. Fast poll for Hyprland to exit (up to 500ms in 20ms steps)
waited=0
while pgrep -x Hyprland >/dev/null 2>&1 && [ "$waited" -lt 25 ]; do
    sleep 0.02
    waited=$((waited + 1))
done

# Fallback: if Hyprland hangs on Xwayland or DRM master release, escalate with SIGTERM then SIGKILL
if pgrep -x Hyprland >/dev/null 2>&1; then
    pkill -TERM -x Hyprland 2>/dev/null || true
    term_waited=0
    while pgrep -x Hyprland >/dev/null 2>&1 && [ "$term_waited" -lt 5 ]; do
        sleep 0.02
        term_waited=$((term_waited + 1))
    done
    if pgrep -x Hyprland >/dev/null 2>&1; then
        pkill -KILL -x Hyprland 2>/dev/null || true
    fi
fi

# Force kill any remaining daemons and Xwayland
pkill -KILL -f "cliphist-watch\.sh|clipboard-watch\.sh|wallpaper\.sh|launch-guard\.sh|xiu-resizer" 2>/dev/null || true
pkill -KILL -x "clipvault|wl-paste|awww-daemon|swww-daemon|cava|quickshell|qs|Xwayland" 2>/dev/null || true
