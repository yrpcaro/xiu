#!/bin/sh
# Mount or unmount an Android phone over MTP under ~/mnt/phone, the one
# place yazi's phone opener and every file manager look. simple-mtpfs is the
# FUSE tool (AUR); jmtpfs is an acceptable stand-in if that is what's
# installed. Usage: mount-phone.sh [up|down] — no argument toggles.
MNT="$HOME/mnt/phone"

case "${1:-toggle}" in
    up|down|toggle) ;;
    *) echo "usage: $0 [up|down]" >&2; exit 2 ;;
esac

if grep -qs "simple-mtpfs\|jmtpfs" /proc/mounts && [ "${1:-toggle}" != "up" ]; then
    fusermount -u "$MNT" && echo "phone unmounted"
    exit 0
fi

FS=$(command -v simple-mtpfs || command -v jmtpfs)
[ -n "$FS" ] || { echo "no MTP filesystem tool (install simple-mtpfs)" >&2; exit 1; }

# Only the first MTP device; list with `simple-mtpfs -l` when several.
mkdir -p "$MNT"
if grep -qs "simple-mtpfs\|jmtpfs" /proc/mounts; then
    echo "already mounted at $MNT"
    exit 0
fi
"$FS" "$MNT" && echo "phone mounted at $MNT"
