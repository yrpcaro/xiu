#!/usr/bin/env python3
"""Xiu browser native messaging host.

Firefox and Zen speak native messaging over stdio: each message is a
4-byte little-endian length followed by UTF-8 JSON. This host emits the
current xiu palette (~/.cache/ricelin/colors.json, rewritten by the
wallpaper pipeline) once on connect and again on every change, so the
browser's theme tracks the desktop live. Firefox closes our stdin when
the extension goes away, which is the shutdown signal.
"""
import json
import struct
import sys
import threading
import time
from pathlib import Path

COLORS = Path.home() / ".cache" / "ricelin" / "colors.json"


def send(obj):
    data = json.dumps(obj).encode()
    sys.stdout.buffer.write(struct.pack("<I", len(data)))
    sys.stdout.buffer.write(data)
    sys.stdout.buffer.flush()


def read_colors():
    try:
        data = json.loads(COLORS.read_text())
        if "primary" in data and "surface" in data:
            return data
    except (OSError, ValueError):
        pass
    return None


def watch_stdin():
    """Firefox keeps stdin open for the extension's lifetime."""
    sys.stdin.buffer.read(1)


def main():
    threading.Thread(target=watch_stdin, daemon=True).start()
    seen = None
    while True:
        colors = read_colors()
        if colors and colors != seen:
            seen = colors
            send(colors)
        time.sleep(1)


if __name__ == "__main__":
    main()
