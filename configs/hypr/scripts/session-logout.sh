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

# Graceful period for quickshell and daemons to clean up before SIGKILL (up to 500ms)
grace=0
while [ "$grace" -lt 10 ]; do
    if ! pgrep -x quickshell >/dev/null 2>&1 && ! pgrep -x qs >/dev/null 2>&1; then
        break
    fi
    sleep 0.05
    grace=$((grace + 1))
done

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
    uwsm stop 2>/dev/null || true
elif command -v hyprctl >/dev/null 2>&1; then
    hyprctl dispatch exit 2>/dev/null || true
fi

# Wait up to 1.5s for Hyprland to terminate
waited=0
while pgrep -x Hyprland >/dev/null 2>&1 && [ "$waited" -lt 15 ]; do
    sleep 0.1
    waited=$((waited + 1))
done

# Fallback: if Hyprland hangs on Xwayland or DRM master release, escalate with SIGTERM then SIGKILL
if pgrep -x Hyprland >/dev/null 2>&1; then
    pkill -TERM -x Hyprland 2>/dev/null || true
    term_waited=0
    while pgrep -x Hyprland >/dev/null 2>&1 && [ "$term_waited" -lt 5 ]; do
        sleep 0.1
        term_waited=$((term_waited + 1))
    done
    if pgrep -x Hyprland >/dev/null 2>&1; then
        pkill -KILL -x Hyprland 2>/dev/null || true
    fi
    pkill -KILL -x Xwayland 2>/dev/null || true
fi
