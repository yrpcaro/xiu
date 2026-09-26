#!/usr/bin/env python3
"""
Synchronize default applications selected in the shell with ~/.config/xiu/vars.lua
and reload Hyprland.
"""
import os
import re
import subprocess
import sys
from pathlib import Path

VAR_MAP = {
    "x-scheme-handler/http": "browser",
    "x-scheme-handler/terminal": "terminal",
    "inode/directory": "fileManager",
}

WELL_KNOWN = {
    "org.kde.dolphin.desktop": "dolphin",
    "thunar.desktop": "thunar",
    "xiu-yazi.desktop": "yazi",
    "nautilus.desktop": "nautilus",
    "foot.desktop": "foot",
    "footclient.desktop": "footclient",
    "org.kde.konsole.desktop": "konsole",
    "com.mitchellh.ghostty.desktop": "ghostty",
    "ghostty.desktop": "ghostty",
    "kitty.desktop": "kitty",
    "alacritty.desktop": "alacritty",
    "Alacritty.desktop": "alacritty",
    "wezterm.desktop": "wezterm",
    "org.wezfurlong.wezterm.desktop": "wezterm",
    "brave-browser.desktop": "brave",
    "firefox.desktop": "firefox",
    "chromium.desktop": "chromium",
    "google-chrome.desktop": "google-chrome-stable",
    "zen.desktop": "zen-browser",
}


def resolve_cmd(desktop_id):
    if desktop_id in WELL_KNOWN:
        return WELL_KNOWN[desktop_id]

    # Inspect desktop file Exec=
    for app_dir in [
        Path.home() / ".local" / "share" / "applications",
        Path("/usr/local/share/applications"),
        Path("/usr/share/applications"),
    ]:
        f = app_dir / desktop_id
        if f.is_file():
            try:
                for line in f.read_text().splitlines():
                    if line.startswith("Exec="):
                        val = line[5:].strip().split()[0]
                        return Path(val).name
            except OSError:
                pass

    return desktop_id.replace(".desktop", "")


def update_vars_file(var_name, cmd, path=None):
    if path is None:
        cfg_dir = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config")))
        path = cfg_dir / "xiu" / "vars.lua"
    path.parent.mkdir(parents=True, exist_ok=True)

    if not path.is_file():
        path.write_text(
            f'-- xiu personalization overrides\nreturn {{\n    {var_name} = "{cmd}",\n}}\n'
        )
        return

    text = path.read_text()
    pattern = rf'({var_name}\s*=\s*)"[^"]*"'
    if re.search(pattern, text):
        text = re.sub(pattern, rf'\1"{cmd}"', text)
    else:
        # insert before closing brace
        text = re.sub(r'(\}\s*)$', f'    {var_name} = "{cmd}",\n\\1', text)
    path.write_text(text)


def main():
    if len(sys.argv) < 3:
        sys.exit(1)

    cat_key = sys.argv[1]
    desktop_id = sys.argv[2]

    var_name = VAR_MAP.get(cat_key)
    if not var_name:
        sys.exit(0)

    cmd = resolve_cmd(desktop_id)
    update_vars_file(var_name, cmd)

    try:
        subprocess.run(["hyprctl", "reload"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except OSError:
        pass


if __name__ == "__main__":
    main()
