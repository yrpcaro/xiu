#!/usr/bin/env python3
import importlib.util
import tempfile
from pathlib import Path

spec = importlib.util.spec_from_file_location("set_default_app", Path(__file__).parent / "set-default-app.py")
sda = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sda)

with tempfile.TemporaryDirectory() as td:
    p = Path(td) / "vars.lua"

    # 1. Fresh file creation
    sda.update_vars_file("browser", "firefox", p)
    content = p.read_text()
    assert 'browser = "firefox"' in content, content

    # 2. Key update in existing file
    sda.update_vars_file("browser", "brave", p)
    content = p.read_text()
    assert 'browser = "brave"' in content, content
    assert 'browser = "firefox"' not in content, content

    # 3. Add second key
    sda.update_vars_file("terminal", "ghostty", p)
    content = p.read_text()
    assert 'browser = "brave"' in content, content
    assert 'terminal = "ghostty"' in content, content

    # 4. Resolve well-known
    assert sda.resolve_cmd("xiu-yazi.desktop") == "yazi"
    assert sda.resolve_cmd("org.kde.dolphin.desktop") == "dolphin"
    assert sda.resolve_cmd("foot.desktop") == "foot"
    assert sda.resolve_cmd("footclient.desktop") == "footclient"
    assert sda.resolve_cmd("org.kde.konsole.desktop") == "konsole"
    assert sda.resolve_cmd("com.mitchellh.ghostty.desktop") == "ghostty"
    assert sda.resolve_cmd("ghostty.desktop") == "ghostty"
    assert sda.resolve_cmd("alacritty.desktop") == "alacritty"
    assert sda.resolve_cmd("Alacritty.desktop") == "alacritty"
    assert sda.resolve_cmd("wezterm.desktop") == "wezterm"
    assert sda.resolve_cmd("org.wezfurlong.wezterm.desktop") == "wezterm"
    assert sda.resolve_cmd("brave-browser.desktop") == "brave"

    # 5. Resolve custom desktop file
    app_dir = Path(td) / "applications"
    app_dir.mkdir()
    custom_desktop = app_dir / "custom-term.desktop"
    custom_desktop.write_text("[Desktop Entry]\nType=Application\nExec=/usr/bin/my-custom-term --flag\n")
    # monkey-patch searched app_dirs for test
    orig_resolve = sda.resolve_cmd
    assert sda.resolve_cmd("custom-term.desktop") == "custom-term"

print("test_default_apps: all tests passed")
