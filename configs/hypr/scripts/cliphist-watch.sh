#!/bin/sh
# clipvault keeps one watcher for every type: text, images and other binary
# data are all stored byte-for-byte, so a single wl-paste watch covers what
# cliphist needed two of. The pill's Cliphist singleton re-runs this script
# as a heartbeat; the pgrep guard keeps it to one watcher no matter how often
# it runs, and a box without clipvault starts nothing.
command -v clipvault >/dev/null 2>&1 || exit 0
pgrep -f "wl-paste --watch clipvault" >/dev/null || wl-paste --watch clipvault store &
