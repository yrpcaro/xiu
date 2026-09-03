#!/bin/sh
#
# CTRL+SHIFT+ALT+V paste-latest: put the newest clipboard history entry back on
# the clipboard, then type it into whatever had focus, so the key works in apps
# that ignore the clipboard too. dotool's line protocol wants each line of the
# text as a `type` command with a Return between lines, not after the last.

sleep 0.3
entry=$(cliphist list | head -1 | cliphist decode)
[ -n "$entry" ] || exit 0

printf '%s' "$entry" | wl-copy
printf '%s' "$entry" | awk '{ if (NR > 1) print "key Return"; printf "type %s\n", $0 }' | dotool
