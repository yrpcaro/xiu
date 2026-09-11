#!/bin/sh
# The FileChooser portal's yazi wrapper, mirroring the contract the
# xdg-desktop-portal-termfilechooser backend passes to every wrapper:
#   $1 multiple (1/0)   $2 directory (1/0)   $3 save (1/0)
#   $4 recommended path (save mode)          $5 output file
# The selection is written to $5, one path per line; empty output cancels.
# yazi's --chooser-file does the writing: open fires the choice, quitting
# writes nothing.
multiple="$1"
directory="$2"
save="$3"
path="$4"
out="$5"

yazi="/usr/bin/yazi"
termcmd="${TERMCMD:-/usr/bin/foot}"

if [ "$save" = "1" ]; then
    # Save mode: navigate to the recommended path's parent, name the file,
    # open it to confirm — the open writes the chosen path to the out file.
    dir=$(dirname -- "$path")
    mkdir -p -- "$dir" 2>/dev/null
    "$termcmd" -- "$yazi" --chooser-file="$out" "$dir"
elif [ "$directory" = "1" ]; then
    # Directory mode: yazi has no dirs-only filter; quitting in a directory
    # selects it through --cwd-file (the fish wrapper's trick).
    tmp=$(mktemp)
    "$termcmd" -- "$yazi" --cwd-file="$tmp" "$path"
    if [ -s "$tmp" ]; then
        cp -- "$tmp" "$out"
    fi
    rm -f -- "$tmp"
elif [ "$multiple" = "1" ]; then
    "$termcmd" -- "$yazi" --chooser-file="$out" "$path"
else
    "$termcmd" -- "$yazi" --chooser-file="$out" "$path"
fi

# Save mode cleanup: an empty out file means the save was cancelled, so the
# recommended file (which we may have created as a navigation anchor in other
# wrappers) must not linger. yazi never creates it here, but the guard keeps
# the contract honest.
if [ "$save" = "1" ] && [ ! -s "$out" ]; then
    rm -f -- "$path" 2>/dev/null
fi
