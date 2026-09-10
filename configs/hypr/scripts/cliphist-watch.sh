#!/bin/sh
# clipvault keeps one watcher for every type: text, images and other binary
# data are all stored byte-for-byte, so a single wl-paste watch covers what
# cliphist needed two of (clipvault's README documents exactly this line).
# The pill's Cliphist singleton re-runs this script as a heartbeat; a box
# without clipvault starts nothing.
#
# flock, not pgrep: the heartbeat can fire twice inside pgrep's race window
# and the pgrep guard would pass both times, leaving two watchers storing
# every copy twice. An exclusive lock on the script's own runtime file makes
# the second instance exit before spawning, whichever order they arrive in.
command -v clipvault >/dev/null 2>&1 || exit 0
lock="${XDG_RUNTIME_DIR:-/tmp}/xiu-clipvault-watch.lock"
exec 9>"$lock"
flock -n 9 || exit 0
exec wl-paste --watch clipvault store
