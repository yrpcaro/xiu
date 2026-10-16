#!/usr/bin/env python3
"""
Test default application query and selection through the xiu CLI.
"""
import os
import subprocess
import tempfile
from pathlib import Path

# 1. Test probe through xiu default-app --probe
res = subprocess.run(["xiu", "default-app", "--probe"], capture_output=True, text=True)
assert res.returncode == 0, f"probe failed with code {res.returncode}: {res.stderr}"
assert len(res.stdout.strip()) > 0, "probe produced empty output"

# 2. Test vars.lua update in an isolated XDG_CONFIG_HOME
with tempfile.TemporaryDirectory() as td:
    env = dict(os.environ)
    env["XDG_CONFIG_HOME"] = td
    env["HOME"] = td

    vars_file = Path(td) / "xiu" / "vars.lua"

    # Set browser to firefox
    res1 = subprocess.run(
        ["xiu", "default-app", "x-scheme-handler/http", "firefox.desktop"],
        capture_output=True, text=True, env=env
    )
    assert res1.returncode == 0, f"setting browser failed: {res1.stderr}"
    assert vars_file.is_file(), "vars.lua was not created"
    content = vars_file.read_text()
    assert 'browser = "firefox"' in content, f"firefox not in vars.lua: {content}"

    # Update browser to brave
    res2 = subprocess.run(
        ["xiu", "default-app", "x-scheme-handler/http", "brave-browser.desktop"],
        capture_output=True, text=True, env=env
    )
    assert res2.returncode == 0, f"updating browser failed: {res2.stderr}"
    content = vars_file.read_text()
    assert 'browser = "brave"' in content, f"brave not in vars.lua: {content}"
    assert 'browser = "firefox"' not in content, f"stale firefox still in vars.lua: {content}"

    # Add terminal key
    res3 = subprocess.run(
        ["xiu", "default-app", "x-scheme-handler/terminal", "ghostty.desktop"],
        capture_output=True, text=True, env=env
    )
    assert res3.returncode == 0, f"setting terminal failed: {res3.stderr}"
    content = vars_file.read_text()
    assert 'browser = "brave"' in content, f"brave missing after terminal addition: {content}"
    assert 'terminal = "ghostty"' in content, f"ghostty missing from vars.lua: {content}"

    # Add editor key
    res4 = subprocess.run(
        ["xiu", "default-app", "text/plain", "dev.zed.Zed.desktop"],
        capture_output=True, text=True, env=env
    )
    assert res4.returncode == 0, f"setting editor failed: {res4.stderr}"
    content = vars_file.read_text()
    assert 'browser = "brave"' in content, f"brave missing after editor addition: {content}"
    assert 'terminal = "ghostty"' in content, f"ghostty missing after editor addition: {content}"
    assert 'editor = "zeditor"' in content, f"editor missing from vars.lua: {content}"

    # Verify mimeapps.list
    mimeapps_file = Path(td) / "mimeapps.list"
    assert mimeapps_file.is_file(), "mimeapps.list was not created"
    mime_content = mimeapps_file.read_text()
    assert "x-scheme-handler/http=brave-browser.desktop" in mime_content
    assert "x-scheme-handler/terminal=ghostty.desktop" in mime_content
    assert "text/plain=dev.zed.Zed.desktop" in mime_content

    # Verify uwsm/env
    uwsm_file = Path(td) / "uwsm" / "env"
    assert uwsm_file.is_file(), "uwsm/env was not created"
    uwsm_content = uwsm_file.read_text()
    assert "BROWSER=brave" in uwsm_content
    assert "TERMINAL=ghostty" in uwsm_content
    assert "EDITOR=zeditor" in uwsm_content
    assert "VISUAL=zeditor" in uwsm_content

    # Verify environment.d
    env_editor = Path(td) / "environment.d" / "10-editor.conf"
    assert env_editor.is_file()
    assert "EDITOR=zeditor" in env_editor.read_text()
    env_apps = Path(td) / "environment.d" / "10-default-apps.conf"
    assert env_apps.is_file()
    assert "TERMINAL=ghostty" in env_apps.read_text()

    # Verify fish editor config
    fish_editor = Path(td) / "fish" / "conf.d" / "00-editor.fish"
    assert fish_editor.is_file()
    assert "set -gx EDITOR zeditor" in fish_editor.read_text()

    # Verify KDE kdeglobals
    kde_file = Path(td) / "kdeglobals"
    assert kde_file.is_file()
    kde_content = kde_file.read_text()
    assert "BrowserApplication=brave-browser.desktop" in kde_content
    assert "TerminalApplication=ghostty" in kde_content
    assert "Editor=zeditor" in kde_content

    # 3. Test File Manager and Portal Chooser propagation
    # Set file manager to yazi
    res_yazi = subprocess.run(
        ["xiu", "default-app", "inode/directory", "xiu-yazi.desktop"],
        capture_output=True, text=True, env=env
    )
    assert res_yazi.returncode == 0, f"setting fileManager yazi failed: {res_yazi.stderr}"
    content = vars_file.read_text()
    assert 'fileManager = "yazi"' in content

    portals_file = Path(td) / "xdg-desktop-portal" / "portals.conf"
    hypr_portals_file = Path(td) / "xdg-desktop-portal" / "hyprland-portals.conf"
    assert portals_file.is_file(), "portals.conf missing"
    assert hypr_portals_file.is_file(), "hyprland-portals.conf missing"
    assert "org.freedesktop.impl.portal.FileChooser=termfilechooser;gtk" in portals_file.read_text()
    assert "org.freedesktop.impl.portal.FileChooser=termfilechooser;gtk" in hypr_portals_file.read_text()

    termfilechooser_cfg = Path(td) / "xdg-desktop-portal-termfilechooser" / "config.toml"
    assert termfilechooser_cfg.is_file(), "termfilechooser config.toml missing"
    tf_content = termfilechooser_cfg.read_text()
    assert "cmd=xiu yazi-chooser" in tf_content
    assert "env=TERMCMD=ghostty" in tf_content

    # Set file manager to dolphin
    res_dolphin = subprocess.run(
        ["xiu", "default-app", "inode/directory", "org.kde.dolphin.desktop"],
        capture_output=True, text=True, env=env
    )
    assert res_dolphin.returncode == 0
    assert "org.freedesktop.impl.portal.FileChooser=kde;gtk" in portals_file.read_text()
    assert "FileManager=dolphin" in kde_file.read_text()

    # Set file manager to thunar
    res_thunar = subprocess.run(
        ["xiu", "default-app", "inode/directory", "thunar.desktop"],
        capture_output=True, text=True, env=env
    )
    assert res_thunar.returncode == 0
    assert "org.freedesktop.impl.portal.FileChooser=gtk" in portals_file.read_text()

print("test_default_apps: all tests passed")
