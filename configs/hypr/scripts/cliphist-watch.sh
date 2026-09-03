#!/bin/sh
# clipvault keeps one watcher for every type: text, images and other binary
# data are all stored byte-for-byte, so a single wl-paste watch covers what
# cliphist needed two of.
pgrep -f "wl-paste --watch clipvault" >/dev/null || wl-paste --watch clipvault store &
