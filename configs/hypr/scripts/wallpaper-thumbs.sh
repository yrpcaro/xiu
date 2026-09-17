#!/bin/sh
MAGICK_CONFIGURE_PATH="$(dirname "$0")/magick-policy"
export MAGICK_CONFIGURE_PATH

flags="${XDG_STATE_HOME:-$HOME/.local/state}/ricelin/flags.json"
wpdir=$(jq -r '.wallpaperDir // ""' "$flags" 2>/dev/null || echo "")
[ -n "$wpdir" ] || wpdir=$(cat "${XDG_STATE_HOME:-$HOME/.local/state}/ricelin-wallpaper-dir" 2>/dev/null || true)
[ -n "$wpdir" ] || wpdir="$HOME/Pictures/xiu/wallpapers"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/ricelin-wp-thumbs"
mkdir -p "$cache"

if [ -d "$wpdir" ]; then
    find "$wpdir" -type f 2>/dev/null | awk -v cache="$cache" '
        {
            sub(".*/", "", $0)
            if ($0 != "") valid[$0] = 1
        }
        END {
            cmd = "find \"" cache "\" -maxdepth 1 -name \"*.png\""
            while ((cmd | getline f) > 0) {
                n = split(f, parts, "/")
                fname = parts[n]
                sub(/\.png$/, "", fname)
                if (fname != "" && !(fname in valid)) {
                    print f
                }
            }
            close(cmd)
        }
    ' | xargs -r rm -f
fi

find "$wpdir" -type f \( -iname '*.jpg' -o -iname '*.png' -o -iname '*.gif' -o -iname '*.webp' -o -iname '*.mp4' -o -iname '*.webm' -o -iname '*.mkv' -o -iname '*.mov' \) 2>/dev/null | while IFS= read -r src; do
    base=${src##*/}
    thumb="$cache/$base.png"
    if [ ! -s "$thumb" ] || [ "$src" -nt "$thumb" ]; then
        case "$src" in
            *.[Mm][Pp]4|*.[Ww][Ee][Bb][Mm]|*.[Mm][Kk][Vv]|*.[Mm][Oo][Vv])
                ffmpeg -y -loglevel quiet -i "$src" -frames:v 1 -vf 'scale=512:-2' -f image2 -c:v png "$thumb.tmp" 2>/dev/null
                ;;
            *)
                magick "${src}[0]" -strip -resize 512x "png:$thumb.tmp" 2>/dev/null
                ;;
        esac
        if [ -s "$thumb.tmp" ]; then
            mv "$thumb.tmp" "$thumb"
        else
            rm -f "$thumb.tmp"
        fi
    fi
done
