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
    "audio/mpeg": "musicPlayer",
    "video/mp4": "videoPlayer",
    "image/png": "imageViewer",
    "application/pdf": "documentViewer",
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
    "spotify.desktop": "spotify",
    "spotify-launcher.desktop": "spotify-launcher",
    "amberol.desktop": "amberol",
    "io.bassi.Amberol.desktop": "amberol",
    "org.gnome.Lollypop.desktop": "lollypop",
    "rhythmbox.desktop": "rhythmbox",
    "clementine.desktop": "clementine",
    "imv.desktop": "imv",
    "imv-dir.desktop": "imv-dir",
    "feh.desktop": "feh",
    "org.gnome.Loupe.desktop": "loupe",
    "org.kde.gwenview.desktop": "gwenview",
    "viewnior.desktop": "viewnior",
    "mpv.desktop": "mpv",
    "vlc.desktop": "vlc",
    "org.gnome.Totem.desktop": "totem",
    "io.github.celluloid_player.Celluloid.desktop": "celluloid",
    "org.pwmt.zathura.desktop": "zathura",
    "org.pwmt.zathura-pdf-mupdf.desktop": "zathura",
    "org.gnome.Papers.desktop": "papers",
    "org.kde.okular.desktop": "okular",
    "evince.desktop": "evince",
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


def probe_desktop_entries(dirs=None):
    if dirs is None:
        dirs = [
            Path("/usr/share/applications"),
            Path("/usr/local/share/applications"),
            Path.home() / ".local" / "share" / "applications",
        ]
    seen = set()
    entries = []
    for d in dirs:
        p = Path(d)
        if not p.is_dir():
            continue
        try:
            items = sorted(p.glob("*.desktop"))
        except OSError:
            continue
        for f in items:
            desktop_id = f.name
            if desktop_id in seen or desktop_id == "foot-server.desktop":
                continue
            seen.add(desktop_id)
            try:
                content = f.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue

            mimes = ""
            is_term = False
            no_disp = False
            for line in content.splitlines():
                line = line.strip()
                if line.startswith("MimeType="):
                    mimes = line[9:]
                elif line.startswith("NoDisplay=") and line[10:].lower() == "true":
                    no_disp = True
                elif ("TerminalEmulator" in line or "x-scheme-handler/terminal" in line) and "Terminal=true" not in line:
                    is_term = True

            # imv.desktop, imv-dir.desktop, and other media/viewer tools often have NoDisplay=true.
            # Only ignore NoDisplay=true if it has no mimes, is not a terminal, and is not in WELL_KNOWN.
            if no_disp and not mimes and not is_term and desktop_id not in WELL_KNOWN:
                continue

            term = " terminal" if is_term else ""
            entries.append(f"{desktop_id}: {mimes}{term}")
    return entries


def main():
    if len(sys.argv) > 1 and sys.argv[1] in ("--probe", "-p", "probe"):
        for line in probe_desktop_entries():
            print(line)
        sys.exit(0)

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
