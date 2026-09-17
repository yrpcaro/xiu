#!/usr/bin/env python3
"""Xiu browser native messaging host.

Firefox and Zen speak native messaging over stdio: each message is a
4-byte little-endian length followed by UTF-8 JSON. This host emits the
current xiu palette (~/.cache/xiu/colors.json or ricelin/colors.json,
rewritten by the wallpaper pipeline) once on connect and again on every
change, so the browser's theme tracks the desktop live. Firefox closes our
stdin when the extension goes away, which is the shutdown signal.
"""
import json
import os
import struct
import sys
import threading
import time
from pathlib import Path

COLORS_XIU = Path.home() / ".cache" / "xiu" / "colors.json"
COLORS_RICELIN = Path.home() / ".cache" / "ricelin" / "colors.json"


def get_colors_path():
    if COLORS_XIU.is_file():
        return COLORS_XIU
    return COLORS_RICELIN


def send(obj):
    data = json.dumps(obj).encode()
    sys.stdout.buffer.write(struct.pack("<I", len(data)))
    sys.stdout.buffer.write(data)
    sys.stdout.buffer.flush()


def read_colors(path=None):
    if path is None:
        path = get_colors_path()
    try:
        data = json.loads(path.read_text())
        if "primary" in data and "surface" in data:
            return data
    except (OSError, ValueError):
        pass
    return None


def watch_stdin():
    """Firefox keeps stdin open for the extension's lifetime.
    When Firefox closes stdin or exits, read() returns EOF (b''),
    signaling immediate termination to avoid zombie processes.
    """
    try:
        while True:
            chunk = sys.stdin.buffer.read(1024)
            if not chunk:
                break
    except Exception:
        pass
    os._exit(0)


def main():
    threading.Thread(target=watch_stdin, daemon=True).start()
    seen_colors = None
    last_mtime = None

    while True:
        path = get_colors_path()
        try:
            mtime = path.stat().st_mtime_ns
        except OSError:
            mtime = None

        if mtime != last_mtime:
            last_mtime = mtime
            colors = read_colors(path)
            if colors and colors != seen_colors:
                seen_colors = colors
                send(colors)

        time.sleep(2)


if __name__ == "__main__":
    main()

