#!/bin/sh
MAGICK_CONFIGURE_PATH="$(dirname "$0")/magick-policy"
export MAGICK_CONFIGURE_PATH

cache="${XDG_CACHE_HOME:-$HOME/.cache}/clipvault-thumbs"
mkdir -p "$cache"
chmod 700 "$cache"

tab=$(printf '\t')
snapshot=$(clipvault list 2>/dev/null) || exit 0
[ -n "$snapshot" ] || exit 0

# Prune stale thumbnails in one batch without per-file subprocess spawns
printf '%s\n' "$snapshot" | awk -v cache="$cache" '
    BEGIN { FS="\t" }
    NF { valid[$1] = 1 }
    END {
        cmd = "find \"" cache "\" -maxdepth 1 -name \"*.png\""
        while ((cmd | getline f) > 0) {
            n = split(f, parts, "/")
            fname = parts[n]
            sub(/\.png$/, "", fname)
            if (fname ~ /^[0-9]+$/ && !(fname in valid)) {
                print f
            }
        }
        close(cmd)
    }
' | xargs -r rm -f

printf '%s\n' "$snapshot" | while IFS= read -r line; do
    # clipvault's metadata always carries dimensions after the mime
    # ("image/png 400x300 ]]"), so the pattern must not pin the mime against
    # the closing bracket.
    case "$line" in
        *"$tab[[ binary data"*image/*"]]"*)
            id=${line%%$tab*}
            thumb="$cache/$id.png"
            if [ ! -s "$thumb" ]; then
                raw="$cache/.raw.$id"
                printf '%s' "$line" | clipvault get > "$raw" 2>/dev/null
                magick "${raw}[0]" -resize '256x256>' "png:$thumb.tmp" 2>/dev/null
                if [ -s "$thumb.tmp" ]; then
                    mv "$thumb.tmp" "$thumb"
                else
                    rm -f "$thumb.tmp"
                fi
                rm -f "$raw"
            fi
            ;;
    esac
done
