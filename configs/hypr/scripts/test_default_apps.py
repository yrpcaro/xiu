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

print("test_default_apps: all tests passed")
