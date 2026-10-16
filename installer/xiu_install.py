#!/usr/bin/env python3
"""
The xiu installer orchestrator: the thin top layer that ties distro
detection, the package planner, the fallback handlers, the config deploy and the
terminal UI into one real install flow.

It only sequences and runs; every decision lives in the modules it imports.
distro.py says what to do with each package, pkg.py builds the argv that does it,
fallbacks.py describes the from-source work, deploy.py moves the configs, tui.py
draws the prompts. This file just walks them in order, asks the user the few
questions that matter, runs each step fail-soft (one bad package never aborts the
rest), and prints a report at the end.

--dry-run walks the whole flow and changes nothing: every command prints as
`would run: ...` and the deploy runs with apply=False. That is the primary test
path and is meant to work headless. --quickstart skips the wizard and takes the
Quick-profile defaults, so it pairs with --dry-run for a non-interactive check.
"""
import argparse
import getpass
import os
import re
import shlex
import shutil
import subprocess
import sys
import threading
from pathlib import Path

import deploy
import distro
import fallbacks
import grub_theme
import pkg
import tui


def _run(argv, dry, env=None):
    """
    Run one command, or print it as `would run:` under a dry run. Returns
    (ok, detail); detail carries the failure text the report turns into a hint.
    A missing binary or a non-zero exit is a soft failure, never a raise, so the
    install keeps going past a single bad step.
    """
    printable = " ".join(shlex.quote(a) for a in argv)
    if dry:
        print(f"  would run: {printable}")
        return True, ""
    runenv = None
    if env:
        runenv = dict(os.environ)
        runenv.update(env)
    try:
        result = subprocess.run(argv, env=runenv)
    except OSError as exc:
        return False, f"{exc}: {printable}"
    if result.returncode != 0:
        return False, f"exit {result.returncode}: {printable}"
    return True, ""


def _shell(cmd, dry):
    """Run a shell step (a pipe or a redirect), or print it under a dry run."""
    if dry:
        print(f"  would run: {cmd}")
        return True, ""
    try:
        result = subprocess.run(["sh", "-c", cmd])
    except OSError as exc:
        return False, f"{exc}: {cmd}"
    if result.returncode != 0:
        return False, f"exit {result.returncode}: {cmd}"
    return True, ""


def _compositor():
    """The running Wayland session, read off the environment the rice sets."""
    if os.environ.get("HYPRLAND_INSTANCE_SIGNATURE"):
        return "Hyprland"
    if os.environ.get("NIRI_SOCKET"):
        return "Niri"
    return os.environ.get("XDG_CURRENT_DESKTOP") or "Unknown"


def _bootloader():
    """
    The boot loader in use, so the GRUB theme prompt only shows on a GRUB box.
    Spotted by the config the loader leaves on disk, with its tool on PATH as the
    backup signal.
    """
    if os.path.isfile("/boot/grub/grub.cfg") or shutil.which("grub-mkconfig"):
        return "grub"
    if os.path.isdir("/boot/loader/entries") or shutil.which("bootctl"):
        return "systemd-boot"
    if (os.path.isfile("/boot/limine.conf")
            or os.path.isfile("/boot/limine/limine.conf") or shutil.which("limine")):
        return "limine"
    return "other"


def _has_display_manager():
    """True when a login manager is set up, so the SDDM theme prompt makes sense."""
    if os.path.exists("/etc/systemd/system/display-manager.service"):
        return True
    return any(shutil.which(dm) for dm in ("sddm", "gdm", "lightdm", "ly", "greetd"))


def _active(unit):
    """Read-only check whether a systemd unit is active right now."""
    try:
        return subprocess.run(["systemctl", "is-active", "--quiet", unit]).returncode == 0
    except OSError:
        return False


def _root_cmd():
    """sudo when it is there, else doas (common on Gentoo); the wizard can override."""
    return "doas" if not shutil.which("sudo") and shutil.which("doas") else "sudo"


def detect():
    """Read the whole machine state the flow branches on into one dict."""
    family = distro.detect_family()
    return {
        "family": family,
        "root_cmd": _root_cmd(),
        "pretty": distro.detect_pretty(),
        "compositor": _compositor(),
        "pm": distro.PM.get(family, "Unknown"),
        "aur_helper": pkg.aur_helper(),
        "existing": deploy.detect_existing(),
        "bootloader": _bootloader(),
        "init": distro.detect_init(),
        "immutable": distro.is_immutable(),
    }


def _default_choices(args, info, manifest):
    """The non-interactive choices for --quickstart and the no-terminal fallback."""
    full_ids = {p["id"] for p in manifest["packages"] if p.get("group") == "full"}
    profile = "full" if args.full else "quick"
    quick_opt_ids = {
        "yazi",
        "ripdrag",
        "surge",
        "uwsm",
        "zathura",
        "zathura-pdf-mupdf",
        "imv",
        "bottom",
        "zed",
        "spotify",
        "spicetify-cli",
        "telegram-desktop",
        "micro",
        "dolphin",
    }
    aur_choice = "paru" if (info.get("aur_helper") == "paru" or shutil.which("paru")) else (info.get("aur_helper") or "paru")
    return {
        "profile": profile,
        "terminal": "foot",
        "extra_terminals": [],
        "aur_choice": aur_choice,
        "optional_ids": set(full_ids) if profile == "full" else quick_opt_ids,
        "file_manager": "dolphin",
        "editor": "helix",
        "browser": "brave",
        "media_player": "mpv",
        "greeter": "sddm" if args.sddm else "none",
        "sddm_theme": "washi",
        "browser_theme": False,
        "fresh_configs": True,
        "legacy_swap": "clean",
        "grub": False,
        "fish": True,
        "brave": True,
        "root_cmd": info.get("root_cmd") or _root_cmd(),
        "overlays": True,
        "services": True,
        "chat_theme": True,
        "spicetify": True,
        "kb_layout": "us,ir",
        "natural_scroll": True,
        "tap_to_click": True,
        "idle_timeout": "5m",
    }


def _wizard(args, info, manifest):
    """
    Walk the structured installer wizard:
      Stage 1: Setup Strategy & Core Environment (Profile, AUR helper, Shell, Strategy, Legacy cleanup)
      Stage 2: Default Applications (Terminal, Additional terminals, Editor, File Manager, Browser, Media Player)
      Stage 3: Desktop & Session (Greeter / Login Screen, SDDM Theme, GRUB Bootloader)
      Stage 4: Input & Hardware (Keyboard Layout, Touchpad Scrolling, Tap-to-Click, Idle Timeout)
      Stage 5: Theming & Integrations (Spotify / Spicetify, Browser Live Theming, Chat Themes)
      Stage 6: Applications & Utilities (Utilities & Tools, Applications & Extras)
      Stage 7: Review & Confirmation
    Raises RuntimeError when there is no controlling terminal, falling back to defaults.
    """
    family = info["family"]

    # --- Setup Strategy & Profile ---
    tui.section("Setup Strategy", "Choose your installation mode")
    pidx = tui.select_one("Install profile", [
        ("Quick", "Recommended defaults: foot, dolphin, helix, brave, mpv, TTY, clean swap, fresh start", True),
        ("Custom", "Interactive setup: customize applications, desktop, input, and theming", False),
        ("Full", "Complete installation: everything in the repository with all daily applications", False),
    ], default=1 if args.full else 0)
    profile = ("quick", "custom", "full")[pidx]

    aur_choice = "paru"
    if family == "arch":
        default_aur = 0 if info.get("aur_helper") == "paru" or not info.get("aur_helper") else (1 if info.get("aur_helper") == "yay" else 0)
        aidx = tui.select_one("AUR helper", [
            ("paru", "Build AUR packages with paru (Recommended)", True),
            ("yay", "Build AUR packages with yay", False),
            ("None", "Skip the AUR, use fallbacks instead", False),
        ], default=default_aur)
        aur_choice = ("paru", "yay", "none")[aidx]

    if profile == "quick":
        choices = _default_choices(args, info, manifest)
        choices["profile"] = "quick"
        choices["aur_choice"] = aur_choice
        tui.section("Quick Installation", "Deploying recommended xiu rice configuration")
        summary_lines = [
            "Profile: Quick (fast install, core essentials)",
            f"AUR helper: {aur_choice}" if family == "arch" else None,
            "Login shell: fish ($EDITOR=helix)",
            "Config strategy: Fresh start (clean repository defaults, backed up to .bak)",
            "Legacy cleanup: Clean swap (retires ghostty and cliphist)",
            "Default terminal: foot",
            "Default editor: helix",
            "Default file manager: dolphin",
            "Default web browser: brave",
            "Default media player: mpv",
            "Login screen: none (direct TTY login)",
            "Keyboard layout: us,ir (dual English + Persian)",
            "Touchpad: Natural scrolling and tap-to-click enabled",
            "Idle timeout: 5m lock screen",
            "Theming: GTK 3/4, Qt Darkly, Quickshell YAMIS monochrome icons",
            "Spotify: Spicetify and Marketplace enabled for Spotify launcher",
            "Chat themes: Telegram xiu palette enabled",
        ]
        summary_lines = [l for l in summary_lines if l is not None]
        ready = tui.confirm("Ready to install with these settings?", summary_lines)
        if not ready:
            raise KeyboardInterrupt
        return choices

    # --- Stage 1: System & Packages ---
    tui.section("System & Packages", "Login shell, setup strategy, and tool migrations")

    sidx = tui.select_one("Login shell", [
        ("fish", "Set fish as default login shell with xiu environment & $EDITOR (Recommended)", True),
        ("keep", "Keep your current login shell", False),
    ], default=0)
    fish = (sidx == 0)

    csidx = tui.select_one("Configuration strategy", [
        ("Fresh start", "Deploy clean repository defaults (backs up existing to .bak) (Recommended)", True),
        ("Preserve existing", "Preserve existing user configs and merge on updates", False),
    ], default=0)
    fresh_configs = (csidx == 0)

    lidx = tui.select_one("Legacy tools", [
        ("Clean swap", "Retire ghostty and cliphist once foot and clipvault are in (Recommended)", True),
        ("Keep both", "Leave ghostty and cliphist installed alongside replacements", False),
    ], default=0)
    legacy_swap = ("clean", "fallback")[lidx]

    root_cmd, overlays = info.get("root_cmd", "sudo"), True
    if family == "gentoo":
        ridx = tui.select_one("Root command", [
            ("sudo", "Run the privileged steps through sudo", True),
            ("doas", "Run the privileged steps through doas", False),
        ], default=0 if root_cmd == "sudo" else 1)
        root_cmd = ("sudo", "doas")[ridx]
        overlays = tui.confirm("Gentoo overlays", [
            "Enable the GURU and hyproverlay overlays and accept their ~amd64 packages.",
        ])

    services = True
    if info.get("init") == "openrc":
        services = tui.confirm("OpenRC services", [
            "Add NetworkManager and bluetooth to default runlevel and start them now.",
        ])

    # --- Stage 2: Default Applications ---
    tui.section("Default Applications", "Core applications for your desktop workflow")

    tidx = tui.select_one("Default terminal", [
        ("foot", "Fast, lightweight Wayland-native terminal with xiu theme (Recommended)", True),
        ("ghostty", "Modern GPU-accelerated terminal with native styling", False),
        ("alacritty", "Fast minimalist GPU-accelerated terminal", False),
    ], default=0)
    terminal = ("foot", "ghostty", "alacritty")[tidx]

    other_terms = [t for t in ("foot", "ghostty", "alacritty") if t != terminal]
    term_opts = [(t, f"Install {t} as an alternative terminal", False) for t in other_terms]
    chosen_terms = tui.select_many("Additional terminals", term_opts, preselect=())
    extra_terminals = [other_terms[i] for i in chosen_terms]

    eidx = tui.select_one("Default text editor", [
        ("helix", "Modern modal terminal editor with LSP, exposed via $EDITOR (Recommended)", True),
        ("nvim", "Neovim modal editor with xiu palette", False),
        ("micro", "Intuitive terminal text editor", False),
    ], default=0)
    editor = ("helix", "nvim", "micro")[eidx]

    fidx = tui.select_one("Default file manager", [
        ("dolphin", "KDE file manager with native dialogs for the rice (Recommended)", True),
        ("yazi", "TUI file manager with termfilechooser & full preview suite", False),
        ("thunar", "Xfce file manager themed through palette", False),
        ("none", "Keep whatever is currently installed", False),
    ], default=0)
    file_manager = ("dolphin", "yazi", "thunar", "none")[fidx]

    bidx = tui.select_one("Default web browser", [
        ("brave", "Brave browser with matching xiu appearance theme (Recommended)", True),
        ("zen-browser-bin", "Zen browser with userChrome integration", False),
        ("firefox", "Firefox browser with userChrome integration", False),
        ("none", "Keep system default browser", False),
    ], default=0 if not args.brave else 0)
    browser = ("brave", "zen-browser-bin", "firefox", "none")[bidx]
    brave = (browser == "brave")

    midx = tui.select_one("Default media player", [
        ("mpv", "mpv media player with uosc UI and xiu palette (Recommended)", True),
        ("none", "Keep system default media player", False),
    ], default=0)
    media_player = ("mpv", "none")[midx]

    # --- Stage 3: Desktop & Session ---
    tui.section("Desktop & Session", "Display manager, login themes, and bootloader")

    gidx = tui.select_one("Login screen", [
        ("TTY", "No greeter; start Hyprland from terminal login (Recommended)", True),
        ("SDDM", "Graphical login with a xiu theme", False),
        ("greetd", "Minimal greeter with tuigreet", False),
    ], default=0 if not args.sddm else 1)
    greeter = ("none", "sddm", "greetd")[gidx]

    sddm_theme = "washi"
    if greeter == "sddm":
        tidx = tui.select_one("SDDM theme", [
            ("washi", "Clean frosted wallpaper and Zen Kaku clock (Recommended)", True),
            ("torii", "The cinematic torii video wallpaper", False),
        ], default=0)
        sddm_theme = ("washi", "torii")[tidx]

    grub = False
    if info.get("bootloader") == "grub":
        grub_idx = tui.select_one("GRUB bootloader theme", [
            ("Skip", "Keep system bootloader untouched (Recommended)", True),
            ("Install theme", "Install matching xiu GRUB theme", False),
        ], default=0)
        grub = (grub_idx == 1)

    # --- Stage 4: Input & Hardware ---
    tui.section("Input & Hardware", "Keyboard layout, touchpad gestures, and idle lock behavior")

    kidx = tui.select_one("Keyboard layout", [
        ("us,ir", "US English + Persian (winkeys) dual layout (Recommended)", True),
        ("us", "US English layout only", False),
        ("keep", "Keep existing Hyprland input layout untouched", False),
    ], default=0)
    kb_layout = ("us,ir", "us", "keep")[kidx]

    nidx = tui.select_one("Touchpad scrolling", [
        ("Natural", "Content follows finger direction: natural_scroll = true (Recommended)", True),
        ("Traditional", "Scroll bar direction: natural_scroll = false", False),
    ], default=0)
    natural_scroll = (nidx == 0)

    tcidx = tui.select_one("Touchpad tap-to-click", [
        ("Enabled", "Tap touchpad to click: tap_to_click = true (Recommended)", True),
        ("Disabled", "Physical click only: tap_to_click = false", False),
    ], default=0)
    tap_to_click = (tcidx == 0)

    itidx = tui.select_one("Idle lock timeout", [
        ("5m", "Lock screen after 5m, turn off displays after 6m (Recommended)", True),
        ("10m", "Lock screen after 10m, turn off displays after 11m", False),
        ("15m", "Lock screen after 15m, turn off displays after 16m", False),
        ("Disabled", "Do not automatically lock screen on idle", False),
    ], default=0)
    idle_timeout = ("5m", "10m", "15m", "disabled")[itidx]

    # --- Stage 5: Theming & Integrations ---
    tui.section("Theming & Integrations", "Application customization, Spicetify, and chat styling")

    spidx = tui.select_one("Spotify skinning (Spicetify)", [
        ("Enable", "Install Spicetify CLI and Marketplace for Spotify launcher (Recommended)", True),
        ("Disable", "Leave Spotify unskinned without Spicetify", False),
    ], default=0)
    spicetify = (spidx == 0)

    btidx = tui.select_one("Browser live theming", [
        ("Disable", "Leave browsers unskinned (Recommended)", True),
        ("Enable", "Register live theme host and userChrome for Firefox/Zen", False),
    ], default=0)
    browser_theme = (btidx == 1)

    ctidx = tui.select_one("Chat client themes", [
        ("Enable", "Deploy xiu themes for Vesktop Discord and Telegram (Recommended)", True),
        ("Disable", "Leave chat apps unskinned", False),
    ], default=0)
    chat_theme = (ctidx == 0)

    # --- Stage 6: Applications & Utilities ---
    tui.section("Applications & Utilities", "Select optional utilities and daily applications")

    util_defs = [
        ("yazi", "TUI file manager with image previews", True),
        ("termfilechooser", "Portal FileChooser backend opening yazi", True if file_manager == "yazi" else False),
        ("ripdrag", "Drag and drop utility for file managers", True),
        ("zathura", "Keyboard-driven document viewer", True),
        ("zathura-pdf-mupdf", "PDF backend for zathura", True),
        ("imv", "Lightweight image viewer", True),
        ("simple-mtpfs", "Android phone MTP filesystem mount", True if file_manager == "yazi" else False),
        ("bottom", "Graphical terminal system monitor (btm)", True),
        ("surge", "TUI download manager", True),
        ("mangofetch", "Terminal media and stream downloader", False),
        ("uwsm", "Universal Wayland Session Manager wrapper", True),
    ]
    util_preselect = range(len(util_defs)) if profile == "full" else [i for i, (_, _, rec) in enumerate(util_defs) if rec]
    chosen_utils = tui.select_many("Utilities & Tools", [(i, d, r) for i, d, r in util_defs], preselect=util_preselect)

    app_defs = [
        ("zed", "High-performance modern GUI code editor", True),
        ("spotify", "Spotify client via spotify-launcher", True),
        ("spicetify-cli", "Spicetify CLI and Marketplace", True if spicetify else False),
        ("telegram-desktop", "Telegram messaging client", True if chat_theme else False),
        ("vesktop", "Vencord Discord client", False),
        ("micro", "Intuitive terminal text editor", True if editor == "micro" else False),
        ("neovim", "Neovim modal terminal editor", True if editor == "nvim" else False),
        ("htop", "Interactive process viewer", False),
        ("nvtop", "GPU process monitor", False),
        ("trash-cli", "Command-line trash management", True if file_manager == "yazi" else False),
        ("keepassxc", "Offline password manager", False),
        ("rnote", "Handwritten notes and sketch app", False),
        ("mpvpaper", "Animated video wallpaper utility", False),
        ("nvibrant", "Screen color vibrance control", False),
    ]
    app_preselect = range(len(app_defs)) if profile == "full" else [i for i, (_, _, rec) in enumerate(app_defs) if rec]
    chosen_apps = tui.select_many("Applications & Extras", [(i, d, r) for i, d, r in app_defs], preselect=app_preselect)

    optional_ids = {util_defs[i][0] for i in chosen_utils} | {app_defs[i][0] for i in chosen_apps}
    if file_manager == "dolphin":
        optional_ids.add("dolphin")
    elif file_manager == "thunar":
        optional_ids.add("thunar")
    elif file_manager == "yazi":
        optional_ids.update(YAZI_SUITE_IDS)
    if browser in ("zen-browser-bin", "firefox"):
        optional_ids.add(browser)

    # --- Stage 7: Review & Confirmation ---
    tui.section("Installation Review", "Review your configuration before installing")

    summary_lines = [
        f"Profile: {profile.capitalize()}",
        f"AUR helper: {aur_choice}" if family == "arch" else None,
        f"Login shell: {'fish ($EDITOR=' + editor + ')' if fish else 'Current shell'}",
        f"Config strategy: {'Fresh start (clean defaults)' if fresh_configs else 'Preserve existing'}",
        f"Legacy cleanup: {'Clean swap (retire ghostty/cliphist)' if legacy_swap == 'clean' else 'Keep both'}",
        f"Default terminal: {terminal}" + (f" (+ {', '.join(extra_terminals)})" if extra_terminals else ""),
        f"Default editor: {editor}",
        f"Default file manager: {file_manager}" + (" (with termfilechooser, previews & tools)" if file_manager == "yazi" else ""),
        f"Default web browser: {browser}",
        f"Default media player: {media_player}",
        f"Login screen: {greeter.upper() if greeter != 'none' else 'TTY (no greeter)'}" + (f" ({sddm_theme})" if greeter == "sddm" else ""),
        f"Keyboard layout: {kb_layout}",
        f"Touchpad: {'Natural' if natural_scroll else 'Traditional'} scrolling, tap-to-click {'enabled' if tap_to_click else 'disabled'}",
        f"Idle timeout: {idle_timeout}",
        f"Theming: Spicetify ({'enabled' if spicetify else 'disabled'}), Browser live ({'enabled' if browser_theme else 'disabled'}), Chat ({'enabled' if chat_theme else 'disabled'})",
        f"Optional packages: {len(optional_ids)} selected",
    ]
    summary_lines = [line for line in summary_lines if line is not None]
    ready = tui.confirm("Ready to install with these settings?", summary_lines)
    if not ready:
        raise KeyboardInterrupt

    return {
        "profile": profile, "aur_choice": aur_choice, "optional_ids": optional_ids,
        "terminal": terminal, "extra_terminals": extra_terminals,
        "file_manager": file_manager, "editor": editor, "browser": browser,
        "media_player": media_player, "greeter": greeter,
        "sddm_theme": sddm_theme, "browser_theme": browser_theme,
        "fresh_configs": fresh_configs, "legacy_swap": legacy_swap,
        "grub": grub, "fish": fish, "brave": brave,
        "root_cmd": root_cmd, "overlays": overlays, "services": services,
        "spicetify": spicetify, "chat_theme": chat_theme,
        "kb_layout": kb_layout, "natural_scroll": natural_scroll,
        "tap_to_click": tap_to_click, "idle_timeout": idle_timeout,
    }


# The full suite of companion tools, portals, previewers and thumbnailers
# required for yazi to operate as a full-featured file manager.
YAZI_SUITE_IDS = {
    "yazi",
    "termfilechooser",
    "ripdrag",
    "simple-mtpfs",
    "trash-cli",
    "lazygit",
    "7zip",
    "ffmpegthumbnailer",
    "poppler",
    "resvg",
    "chafa",
    "imv",
    "zathura",
    "zathura-pdf-mupdf",
}


def _choice_ids(choices):
    """The full-group package ids an explicit wizard/quickstart choice pulls in,
    so a Quick install still gets the chosen file manager or greeter."""
    ids = set()
    term = choices.get("terminal", "foot")
    if term in ("ghostty", "alacritty"):
        ids.add(term)
    for extra in choices.get("extra_terminals", []):
        ids.add(extra)
    fm = choices.get("file_manager")
    if fm and fm != "none":
        ids.add(fm)
        if fm == "yazi":
            ids.update(YAZI_SUITE_IDS)
    if choices.get("greeter") == "sddm":
        ids.add("sddm")
    elif choices.get("greeter") == "greetd":
        ids.update(("greetd", "greetd-tuigreet"))
    if choices.get("editor") == "nvim":
        ids.add("neovim")
    elif choices.get("editor") == "micro":
        ids.add("micro")
    if choices.get("browser") in ("zen-browser-bin", "firefox"):
        ids.add(choices["browser"])
    if choices.get("media_player") == "mpv":
        ids.add("mpv")
    if choices.get("spicetify"):
        ids.add("spicetify-cli")
    return ids


def _build_plan(manifest, info, choices, cli_local):
    """
    Resolve the chosen groups into concrete batches. Native packages already on
    the box are dropped (idempotent); the AUR ones, the repos to enable and the
    fallbacks are kept. The xiu CLI is dropped from every list when
    cli_local — the dedicated build step compiles it from the running checkout
    instead of the generic cargo fallback's second clone. Returns the split
    lists the runner walks.
    """
    family = info["family"]
    by_id = {p["id"]: p for p in manifest["packages"]}
    profile = choices["profile"]
    rows = distro.plan(manifest, family, ("core", "full"), choices["aur_choice"])
    keep = set(choices["optional_ids"]) | _choice_ids(choices)
    rows = [r for r in rows if r["group"] == "core" or r["id"] in keep]
    # The clean swap never installs what it is about to retire: ghostty and
    # cliphist stay in the package list as the fallback choice's alternatives,
    # but installing them just to pacman -Rns them a minute later wastes a
    # build (ghostty drags its own shell-integration/terminfo/gtk4-layer-shell
    # deps in with it). The removal pass stays for boxes that already had
    # them installed.
    if choices.get("legacy_swap") == "clean":
        keep_ids = set()
        if choices.get("terminal") == "ghostty" or "ghostty" in choices.get("extra_terminals", []):
            keep_ids.add("ghostty")
        rows = [r for r in rows if r["id"] not in ({"ghostty", "cliphist"} - keep_ids)]

    repos, native, aur, fb, skipped, manual = [], [], [], [], [], []
    optional_native = set()
    atom_repos = {}
    for r in rows:
        if r["action"] == "skip":
            continue
        if (r["repo"] or "").startswith("overlay:") and not choices.get("overlays", True):
            manual.append(r["target"])
            continue
        if r["id"] == "xiu" and cli_local:
            continue
        if r["action"] == "fallback":
            if fallbacks.present(r["target"], by_id[r["id"]]):
                skipped.append(r["id"])
            else:
                fb.append((r["id"], r["target"], by_id[r["id"]]))
            continue
        if pkg.is_installed(r["target"], family):
            skipped.append(r["id"])
            continue
        if r["repo"] and r["repo"] not in repos:
            repos.append(r["repo"])
        (aur if r["aur"] else native).append(r["target"])
        atom_repos[r["target"]] = r["repo"]
        if not r["aur"] and not r["required"]:
            optional_native.add(r["target"])
    return {"repos": repos, "native": native, "aur": aur, "fallbacks": fb,
            "skipped": skipped, "optional_native": optional_native,
            "manual": manual, "atom_repos": atom_repos}


def cli_local(source):
    """
    True when the installer is running out of a repo checkout that carries the
    cli/ crate, so the xiu CLI can be built in place instead of cloned again.
    """
    return (Path(source).resolve().parent / "cli" / "Cargo.toml").is_file()


def build_xiu_cli(source, dry):
    """
    Build and install the shell's own Rust CLI from the checkout the installer
    is running out of — the exact commit being deployed, not whatever main a
    second clone would fetch (the generic cargo fallback, kept for source dirs
    without a checkout, clones the repo from github again). Runs on every
    install path, --no-deps included: the keybinds call `xiu`, so a box without
    the binary has dead binds. Always rebuilds, so a re-run also updates the
    binary the first deploy left behind (the update engine never does).
    Compiles into the shared build cache so the checkout stays clean, and
    installs into ~/.local/bin, which needs no root. Returns (ok, detail,
    built) with built=False when this source has no cli/ crate and the caller
    should fall through to the generic fallback.
    """
    if not cli_local(source):
        return True, "", False
    crate = Path(source).resolve().parent / "cli" / "Cargo.toml"
    target = os.path.join(fallbacks.BUILD_DIR, "xiu-target")
    bin_ = os.path.join(fallbacks.BIN_DIR, "xiu")

    # Fast path: if the local checkout already has an up-to-date release binary,
    # install it directly without paying for a redundant cargo rebuild.
    local_release = crate.parent / "target" / "release" / "xiu"
    src_dir = crate.parent / "src"
    up_to_date = False
    if local_release.is_file():
        try:
            bin_mtime = local_release.stat().st_mtime
            src_mtimes = [p.stat().st_mtime for p in src_dir.rglob("*.rs")]
            if src_mtimes and bin_mtime >= max([crate.stat().st_mtime] + src_mtimes):
                up_to_date = True
        except OSError:
            pass

    if up_to_date:
        shell_install = "mkdir -p %s && install -m755 %s %s" % (
            shlex.quote(fallbacks.BIN_DIR),
            shlex.quote(str(local_release)),
            shlex.quote(bin_),
        )
        if dry:
            print("  would install prebuilt xiu cli -> %s" % bin_)
            return True, "", True
        ok, detail = _shell(shell_install, dry)
        if ok:
            print("  installed prebuilt: xiu cli -> %s" % bin_)
            return True, "", True

    shell_build = (
        fallbacks._CARGO_PREP
        + "; CARGO_TARGET_DIR=%s cargo build --release --manifest-path %s"
        % (shlex.quote(target), shlex.quote(str(crate)))
    )
    shell_install = "mkdir -p %s && install -m755 %s %s" % (
        shlex.quote(fallbacks.BIN_DIR),
        shlex.quote(os.path.join(target, "release", "xiu")),
        shlex.quote(bin_),
    )
    if dry:
        print("  would run: %s" % shell_build)
        print("  would run: %s" % shell_install)
        return True, "", True
    ok, detail = _shell(shell_build, dry)
    if not ok:
        return ok, detail, True
    ok, detail = _shell(shell_install, dry)
    if not ok:
        return ok, detail, True
    print("  built: xiu cli -> %s" % bin_)
    return True, "", True


def _aur_install_argv(names, family, aur_choice):
    """
    The unwrapped AUR-helper install command (the helper self-escalates, so sudo
    would break makepkg). Falls back to the chosen helper name when none is on
    PATH yet, so a dry run can print the line before the helper is bootstrapped.
    """
    if pkg.aur_helper():
        return pkg.install_argv(names, family, aur=True)
    helper = aur_choice if aur_choice in ("yay", "paru") else "yay"
    return [helper, "-S", "--needed", "--noconfirm", *names]


def _service_note(init):
    """The manual service commands for a non-systemd init, printed not run."""
    su = distro.ROOT
    cmds = {
        "openrc": [f"{su} rc-update add NetworkManager default && {su} rc-service NetworkManager start",
                   f"{su} rc-update add bluetooth default && {su} rc-service bluetooth start"],
        "runit": [f"{su} ln -s /etc/sv/NetworkManager /var/service",
                  f"{su} ln -s /etc/sv/bluetoothd /var/service"],
        "dinit": [f"{su} dinitctl enable NetworkManager", f"{su} dinitctl enable bluetoothd"],
        "s6": ["s6-rc -u change NetworkManager", "s6-rc -u change bluetoothd"],
    }
    lines = ["Non-systemd init detected, enable the services yourself:"]
    lines += cmds.get(init, ["Enable NetworkManager and bluetooth with your init's tools."])
    return lines


def _openrc_service_shell(name):
    """
    Enable and start one OpenRC service. bluez names its script bluetooth on
    Gentoo and bluetoothd on Artix, so the daemon spelling is tried second.
    """
    su = distro.ROOT
    return (f's={name}; [ -e /etc/init.d/$s ] || s={name}d; '
            f'{su} rc-update add $s default && {su} rc-service $s start')


def sudo_keepalive():
    """
    Ask for the password once, then keep the sudo timestamp warm in the
    background so no later step prompts again. Returns a stop callback the runner
    calls when the install is done. doas has no timestamp to refresh, so there it
    is a no-op and each privileged step prompts on its own unless persist is set.
    """
    if distro.ROOT != "sudo":
        return lambda: None
    subprocess.run(["sudo", "-v"])
    stop = threading.Event()

    def _loop():
        while not stop.wait(60):
            subprocess.run(["sudo", "-n", "-v"],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    threading.Thread(target=_loop, daemon=True).start()
    return stop.set


def _summary_lines(info, choices, plan, args, do_pkgs):
    """The go/no-go summary the Ready confirm shows."""
    lines = []
    if not do_pkgs:
        lines.append("Skipping packages, deploying configs only.")
    else:
        count = len(plan["native"]) + len(plan["aur"]) + len(plan["fallbacks"])
        lines.append(f"Install {count} packages ({len(plan['skipped'])} already present).")
        if plan["repos"]:
            lines.append("Enable repos: " + ", ".join(plan["repos"]) + ".")
        if plan["fallbacks"]:
            names = ", ".join(sorted({h for _, h, _ in plan["fallbacks"]}))
            lines.append("Build via fallback: " + names + ".")
        if info["family"] == "gentoo":
            if pkg.portage_config_argv(plan.get("atom_repos", {})):
                lines.append("Write package.accept_keywords/ricelin and package.use/ricelin "
                             "under /etc/portage.")
            if plan.get("manual"):
                lines.append("Overlays skipped, install yourself: emerge -av "
                             + " ".join(plan["manual"]) + ".")
            lines.append("emerge compiles from source; this is the long part.")
    if choices.get("file_manager") and choices["file_manager"] != "none":
        fm_label = f"Install the {choices['file_manager']} file manager"
        if choices["file_manager"] == "yazi":
            fm_label += " (with termfilechooser, previews & tools)"
        lines.append(fm_label + ".")
    if cli_local(args.source):
        lines.append("Build the xiu CLI from this checkout.")
    if choices.get("editor"):
        lines.append(f"Default text editor: {choices['editor']}.")
    if choices.get("browser") and choices["browser"] != "none":
        lines.append(f"Default web browser: {choices['browser']}.")
    if choices.get("media_player") and choices["media_player"] != "none":
        lines.append(f"Default media player: {choices['media_player']}.")
    if choices.get("greeter") == "sddm":
        lines.append(f"Install the {choices.get('sddm_theme', 'washi')} SDDM login theme.")
    elif choices.get("greeter") == "greetd":
        lines.append("Set up greetd with the tuigreet login.")
    if choices.get("kb_layout"):
        lines.append(f"Keyboard layout: {choices['kb_layout']}.")
    if choices.get("idle_timeout"):
        lines.append(f"Idle lock screen timeout: {choices['idle_timeout']}.")
    if choices.get("browser_theme"):
        lines.append("Wire the browsers into the palette (userChrome + live theme host).")
    if choices.get("spicetify"):
        lines.append("Spotify launcher themed via Spicetify and Marketplace.")
    if choices.get("chat_theme"):
        lines.append("Chat clients (Telegram/Vesktop) themed with xiu palette.")
    if choices.get("legacy_swap") == "clean":
        lines.append("Remove ghostty and cliphist once foot and clipvault are in.")
    if choices.get("fresh_configs"):
        lines.append("Start from the repo's config defaults (your old files are still backed up).")
    if choices.get("grub"):
        lines.append("Install the GRUB theme.")
    if choices.get("brave") and choices.get("browser") != "brave":
        lines.append("Install Brave with the matching xiu theme.")
    if choices.get("fish"):
        lines.append("Set fish as your login shell.")
    if _is_update(info):
        lines.append("Update the xiu config; your Settings are kept.")
    else:
        lines.append("Back up and deploy the xiu config.")
    return lines


def _is_update(info):
    """
    True when this run lands on top of an earlier xiu deploy, spotted by the
    managed marker on the two dirs that always deploy. That flips the messaging
    from "back up and deploy" to "update, your files are kept", since a managed
    replace makes no backup and carries the protected user files across.
    """
    existing = info["existing"]
    return any(existing.get(name, {}).get("managed") for name in ("hypr", "quickshell"))


def seed_wallpapers(dry):
    """
    Give a fresh box a wallpaper to show. Every wallpaper consumer reads
    ~/Pictures/xiu/wallpapers (wallpaper.sh, the picker, the search, the
    palette), but that dir is gitignored and untracked, so a clone ships none:
    no background, an empty picker, the palette never fires. A box carrying
    the pre-xiu collection at ~/Ricelin/wallpapers gets it moved over first
    (only when the new home doesn't exist yet, so the move settles after one
    run). Then create the dir plus the downloads subfolder and the ricelin
    cache, and when it holds no images yet, copy the tracked starter set in
    so swww, the picker and the palette all light up. Fail-soft like every
    other step: an OSError comes back as (ok, detail) for the report instead
    of aborting the run.
    """
    home = Path.home()
    wp = home / "Pictures" / "xiu" / "wallpapers"
    old = home / "Ricelin" / "wallpapers"
    starters = Path(__file__).resolve().parent / "starter-wallpapers"
    if dry:
        print("  would seed wallpapers -> ~/Pictures/xiu/wallpapers "
              "(moving ~/Ricelin/wallpapers first when present)")
        return True, ""
    try:
        if old.is_dir() and not wp.exists():
            (home / "Pictures" / "xiu").mkdir(parents=True, exist_ok=True)
            shutil.move(str(old), str(wp))
            try:
                (home / "Ricelin").rmdir()
            except OSError:
                pass
        (wp / "downloads").mkdir(parents=True, exist_ok=True)
        (home / ".cache" / "ricelin").mkdir(parents=True, exist_ok=True)
        exts = (".jpg", ".jpeg", ".png")
        has_image = any(p.is_file() and p.suffix.lower() in exts for p in wp.iterdir())
        def _ensure_wallpaper_state():
            imgs = sorted([p for p in wp.iterdir() if p.is_file() and p.suffix.lower() in exts])
            if imgs:
                for s_sub in ("xiu/wallpaper", "ricelin-wallpaper"):
                    s_file = Path.home() / ".local" / "state" / s_sub
                    if not s_file.is_file():
                        s_file.parent.mkdir(parents=True, exist_ok=True)
                        s_file.write_text(str(imgs[0]) + "\n")

        if has_image:
            print(f"  wallpapers already present -> {wp}")
            _ensure_wallpaper_state()
            return True, ""
        if not starters.is_dir():
            print(f"  no starter wallpapers to seed at {starters}")
            return True, ""
        seeded = 0
        for src in sorted(starters.iterdir()):
            if src.is_file() and src.suffix.lower() in exts:
                shutil.copy2(src, wp / src.name)
                seeded += 1
        print(f"  seeded {seeded} starter wallpaper(s) -> {wp}")
        _ensure_wallpaper_state()
        return True, ""
    except OSError as exc:
        return False, f"{exc}: seed wallpapers"


VARS_TEMPLATE = """\
-- xiu personalization. Any key from modules/vars.lua can be overridden here:
-- return { terminal = "ghostty", fileManager = "yazi" }
-- This file is yours; updates and re-installs never touch it again.
-- See ~/.config/hypr/modules/vars.lua for every key and its default.
return {
%(body)s
}
"""


# Desktop ids for the xdg defaults, keyed the way the wizard names the
# file managers. yazi is a TUI with no runnable graphical entry of its own —
# the installer drops a foot-wrapping xiu-yazi.desktop into the user's local
# applications dir (see set_xdg_defaults) and routes folder opens to it, so
# xdg-open and the file dialogs that delegate reach yazi in the terminal
# instead of whatever GUI manager happens to be installed.
DESKTOP_IDS = {
    "dolphin": "org.kde.dolphin.desktop",
    "thunar": "thunar.desktop",
    "yazi": "xiu-yazi.desktop",
}

# The xdg defaults xiu claims. inode/directory routes every folder open
# (xdg-open, file dialogs' "open containing folder", the portal's fallback)
# through the file manager; the rest pin the viewers this rice ships.
# Both lists merge under [Default Applications], never overwriting a key
# the user already set somewhere else.
DEFAULT_MIMES = {
    "imv.desktop": [
        "image/png", "image/jpeg", "image/gif", "image/bmp", "image/webp",
        "image/avif", "image/heif", "image/tiff", "image/svg+xml",
    ],
    "org.pwmt.zathura.desktop": [
        "application/pdf", "application/epub+zip", "application/oxps",
        "application/x-fictionbook", "application/x-mobipocket-ebook",
    ],
    "mpv.desktop": [
        "video/mp4", "video/mkv", "video/webm", "video/x-matroska", "video/quicktime",
        "audio/mpeg", "audio/flac", "audio/ogg", "audio/wav",
    ],
}


def set_xdg_defaults(choices, dry):
    """
    Make the shipped apps the xdg defaults: the wizard's file manager owns
    inode/directory (folder opens — and the file dialogs that delegate to the
    default handler — land in it), imv owns the images and zathura the
    documents. Merges into ~/.config/mimeapps.list under
    [Default Applications] without touching any other key, so a browser or
    user-set default survives; re-runs only rewrite our own keys. A yazi
    choice first installs xiu-yazi.desktop (the foot wrapper from the repo's
    configs/yazi/applications) into the user's local applications dir, since
    a TUI cannot be launched without a terminal; imv/zathura skip their keys
    when their desktop files are absent (not installed yet — the defaults
    then wait for the next re-run).
    """
    home = Path.home()
    mimeapps = home / ".config" / "mimeapps.list"
    if dry:
        print("  would set xdg defaults (file manager, imv, zathura, helix) -> %s" % mimeapps)
        return True, ""
    # The yazi wrapper lands first so the folder default points at a desktop
    # file that exists; the source is the repo checkout the installer runs
    # from, and a missing source (source tree without it) just skips the
    # yazi default rather than failing the step.
    if choices.get("file_manager") == "yazi":
        # The wrapper ships in the repo at configs/yazi/applications; this
        # module lives in installer/, so walk up to the checkout root first
        # and fall back to a sibling-of-installer layout for safety.
        root = Path(__file__).resolve().parent.parent
        src = next((
            p for p in (
                root / "configs" / "yazi" / "applications" / "xiu-yazi.desktop",
                Path(__file__).resolve().parent / "yazi" / "applications" / "xiu-yazi.desktop",
            ) if p.is_file()
        ), None)
        if src is not None:
            try:
                local_apps = home / ".local" / "share" / "applications"
                local_apps.mkdir(parents=True, exist_ok=True)
                dest = local_apps / "xiu-yazi.desktop"
                dest.write_text(src.read_text())
                print(f"  installed yazi desktop wrapper -> {dest}")
            except OSError:
                pass

    def desktop_exists(desktop_id):
        # The yazi wrapper lives in the user's local dir, not /usr/share.
        return (
            (Path("/usr/share/applications") / desktop_id).is_file()
            or (home / ".local" / "share" / "applications" / desktop_id).is_file()
        )

    want = {}
    fm = DESKTOP_IDS.get(choices.get("file_manager"))
    if fm and desktop_exists(fm):
        want[fm] = ["inode/directory"]
    for desktop, mimes in DEFAULT_MIMES.items():
        if desktop_exists(desktop):
            want.setdefault(desktop, []).extend(mimes)
    editor_desktop = next((d for d in ("Helix.desktop", "helix.desktop") if desktop_exists(d)), None)
    if editor_desktop:
        want.setdefault(editor_desktop, []).extend([
            "text/plain", "text/markdown", "text/x-c", "text/x-c++",
            "text/x-python", "text/x-rust", "application/json",
        ])
    browser_desktop = next((d for d in ("brave-browser.desktop", "brave.desktop") if desktop_exists(d)), None)
    if choices.get("browser") == "brave" and browser_desktop:
        want.setdefault(browser_desktop, []).extend([
            "text/html", "application/xhtml+xml", "x-scheme-handler/http",
            "x-scheme-handler/https", "x-scheme-handler/about", "x-scheme-handler/unknown",
        ])
    if not want:
        print("  no xdg defaults to set (none of the desktop files present)")
        return True, ""
    try:
        # Parse the existing file into sections, preserving everything we
        # do not own. configparser would mangle ordering and case; a hand
        # walk keeps the file byte-stable apart from our keys.
        sections = {}
        order = []
        if mimeapps.is_file():
            current = None
            for raw in mimeapps.read_text().splitlines():
                if raw.startswith("[") and raw.endswith("]"):
                    current = raw[1:-1]
                    if current not in sections:
                        sections[current] = []
                        order.append(current)
                elif current is not None:
                    sections[current].append(raw)
        if "Default Applications" not in sections:
            sections["Default Applications"] = []
            order.append("Default Applications")
        ours = {m for mimes in want.values() for m in mimes}
        sections["Default Applications"] = [
            l for l in sections["Default Applications"]
            if l.split("=")[0].strip() not in ours
        ]
        for desktop, mimes in want.items():
            for m in mimes:
                sections["Default Applications"].append("%s=%s" % (m, desktop))
        lines = []
        for name in order:
            lines.append("[%s]" % name)
            lines.extend(sections[name])
        mimeapps.parent.mkdir(parents=True, exist_ok=True)
        tmp = mimeapps.with_suffix(".list.new")
        tmp.write_text("\n".join(lines) + "\n")
        tmp.replace(mimeapps)
        print("  set xdg defaults -> %s (%d keys)" % (mimeapps, sum(len(v) for v in want.values())))
        return True, ""
    except OSError as exc:
        return False, f"{exc}: set xdg defaults"


def seed_vars(choices, dry):
    """
    Land the wizard's terminal, file-manager, editor and browser choice where Hyprland reads it:
    ~/.config/xiu/vars.lua, the override file vars.lua loads on every start.
    Written only when the file doesn't exist — a box that already
    personalized it keeps every line as-is. Fail-soft like every step.
    """
    dest = deploy.CONFIG_ROOT / "xiu" / "vars.lua"
    terminal = choices.get("terminal", "foot")
    fm = choices.get("file_manager", "dolphin")
    ed = choices.get("editor", "helix")
    br = choices.get("browser", "brave")
    if dry:
        print("  would seed vars.lua (terminal: %s, file manager: %s, editor: %s, browser: %s)" % (terminal, fm, ed, br))
        return True, ""
    try:
        if dest.is_file():
            print(f"  vars.lua already present -> {dest} (left untouched)")
            return True, ""
        term_cmd = f"{terminal} -e yazi" if terminal != "foot" else "foot -e yazi"
        body_fm = {
            "dolphin": '    fileManager = "dolphin",  -- chosen in the installer',
            "thunar": '    fileManager = "thunar",  -- chosen in the installer',
            "yazi": f'    fileManager = "{term_cmd}",  -- chosen in the installer; a TUI needs the terminal',
            "none": '    -- fileManager = "dolphin",  -- uncomment to set yours',
        }.get(fm, '    -- fileManager = "dolphin",')
        term_line = f'    terminal = "{terminal}",  -- chosen in the installer'
        ed_cmd = f"{terminal} -e {ed}" if ed in ("helix", "nvim", "micro") else ed
        ed_line = f'    editor = "{ed_cmd}",  -- chosen in the installer'
        br_line = f'    browser = "{br}",  -- chosen in the installer'
        body = f"{term_line}\n{body_fm}\n{ed_line}\n{br_line}"
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(VARS_TEMPLATE % {"body": body})
        print(f"  seeded vars.lua -> {dest}")
        return True, ""
    except OSError as exc:
        return False, f"{exc}: seed vars.lua"


def apply_input_preferences(choices, dry):
    """
    Apply keyboard layout and touchpad preferences to ~/.config/hypr/modules/input.lua.
    """
    kb = choices.get("kb_layout", "us,ir")
    ns = choices.get("natural_scroll", True)
    tc = choices.get("tap_to_click", True)
    dest = deploy.CONFIG_ROOT / "hypr" / "modules" / "input.lua"
    if dry:
        print("  would apply input preferences (layout: %s, natural scroll: %s, tap to click: %s) -> %s"
              % (kb, ns, tc, dest))
        return True, ""
    try:
        if not dest.is_file():
            return True, ""
        content = dest.read_text()
        if kb != "keep":
            content = re.sub(r'kb_layout\s*=\s*"[^"]*"', f'kb_layout          = "{kb}"', content)
            if kb == "us":
                content = re.sub(r'kb_variant\s*=\s*"[^"]*",?\n?', '', content)
            elif kb == "us,ir":
                if 'kb_variant' not in content:
                    content = content.replace(f'kb_layout          = "{kb}",', f'kb_layout          = "{kb}",\n        kb_variant         = ",winkeys",')
        ns_str = "true" if ns else "false"
        content = re.sub(r'natural_scroll\s*=\s*(?:true|false)', f'natural_scroll = {ns_str}', content)
        tc_str = "true" if tc else "false"
        if "tap_to_click" in content:
            content = re.sub(r'tap_to_click\s*=\s*(?:true|false)', f'tap_to_click   = {tc_str}', content)
        elif "touchpad = {" in content:
            content = content.replace("touchpad = {", f"touchpad = {{\n            tap_to_click   = {tc_str},")
        dest.write_text(content)
        print(f"  applied input preferences -> {dest}")
        return True, ""
    except OSError as exc:
        return False, f"{exc}: apply input preferences"


def apply_idle_preferences(choices, dry):
    """
    Apply lockscreen idle timeout preferences to ~/.config/hypr/hypridle.conf.
    """
    timeout_choice = choices.get("idle_timeout", "5m")
    dest = deploy.CONFIG_ROOT / "hypr" / "hypridle.conf"
    if dry:
        print("  would apply idle timeout preferences (%s) -> %s" % (timeout_choice, dest))
        return True, ""
    try:
        timeouts = {
            "5m": (300, 360),
            "10m": (600, 660),
            "15m": (900, 960),
        }
        if timeout_choice in timeouts:
            lock_s, dpms_s = timeouts[timeout_choice]
            idle_cfg = f"""general {{
    lock_cmd = xiu lock
    before_sleep_cmd = loginctl lock-session
    after_sleep_cmd = hyprctl dispatch dpms on
}}

listener {{
    timeout = {lock_s}
    on-timeout = xiu lock --idle
}}

listener {{
    timeout = {dpms_s}
    on-timeout = hyprctl dispatch dpms off
    on-resume = hyprctl dispatch dpms on
}}
"""
            dest.write_text(idle_cfg)
            print(f"  applied idle timeout ({timeout_choice}) -> {dest}")
        return True, ""
    except OSError as exc:
        return False, f"{exc}: apply idle preferences"


# The yazi plugins the shipped yazi.toml keymap binds. ya pkg add is
# idempotent — an already-added package is a no-op — so this list is also the
# upgrade path: `ya pkg upgrade` refreshes them all.
YAZI_PLUGINS = [
    "yazi-rs/plugins:smart-enter",
    "yazi-rs/plugins:smart-paste",
    "yazi-rs/plugins:smart-filter",
    "yazi-rs/plugins:full-border",
    "yazi-rs/plugins:mime-ext",
    "yazi-rs/plugins:git",
    "yazi-rs/plugins:vcs-files",
    "yazi-rs/plugins:chmod",
    "yazi-rs/plugins:diff",
    "yazi-rs/plugins:mount",
    "yazi-rs/plugins:toggle-pane",
    "yazi-rs/plugins:zoom",
    "Rolv-Apneseth/bypass",
    "KKV9/compress",
    "dedukun/bookmarks",
    "ourongxing/fast-enter",
    "imsi32/yatline",
    "boydaihungst/restore",
    "Lil-Dank/lazygit",
    "TD-Sky/sudo",
    "h-hg/yamb",
]


def wire_portal_chooser(dry, choices=None):
    """
    Point the termfilechooser portal at the rice's yazi wrapper, so every
    app's file dialog opens yazi in foot. The backend reads
    ~/.config/xdg-desktop-portal-termfilechooser/config.toml for its cmd; our
    wrapper ships in configs/hypr/scripts and the deployed portals.conf
    routes FileChooser to the backend.

    When the backend is not installed (e.g. on a box without the AUR package)
    or yazi is not the active default file manager, the FileChooser routing is
    removed from the deployed portals.conf so that file dialogs fall back cleanly
    to GTK/KDE. If termfilechooser is present and yazi is default, we wire
    config.toml, ensure portals.conf routes FileChooser to termfilechooser,
    and restart xdg-desktop-portal.
    """
    home = Path.home()
    cfg_dir = home / ".config" / "xdg-desktop-portal-termfilechooser"
    cfg = cfg_dir / "config"
    cfg_toml = cfg_dir / "config.toml"
    portal_confs = [
        home / ".config" / "xdg-desktop-portal" / "hyprland-portals.conf",
        home / ".config" / "xdg-desktop-portal" / "portals.conf",
    ]

    is_yazi = (choices.get("file_manager") == "yazi") if choices else True
    backend_present = (
        Path("/usr/share/xdg-desktop-portal/portals/termfilechooser.portal").is_file()
        or shutil.which("xdg-desktop-portal-termfilechooser") is not None
        or Path("/usr/lib/xdg-desktop-portal-termfilechooser").is_file()
        or Path("/usr/libexec/xdg-desktop-portal-termfilechooser").is_file()
        or (dry and is_yazi)
    )

    if not backend_present or not is_yazi:
        if not dry:
            for pconf in portal_confs:
                if pconf.is_file():
                    try:
                        text = pconf.read_text()
                        lines = [l for l in text.splitlines(keepends=True)
                                 if not l.strip().startswith("org.freedesktop.impl.portal.FileChooser=termfilechooser")]
                        if len(lines) != len(text.splitlines(keepends=True)):
                            pconf.write_text("".join(lines))
                    except OSError:
                        pass
        if not backend_present:
            print("  xdg-desktop-portal-termfilechooser not installed; file chooser remains GTK")
        else:
            print("  yazi is not default file manager; file chooser remains GTK/KDE")
        return True, ""

    if dry:
        print(f"  would wire the portal chooser -> {cfg} and route FileChooser in portals.conf")
        return True, ""

    xiu_bin = shutil.which("xiu") or (home / ".local" / "bin" / "xiu")
    if isinstance(xiu_bin, Path) and not xiu_bin.is_file() and shutil.which("xiu") is None:
        print("  xiu binary not installed yet; portal chooser waits for the next re-run")
        return True, ""

    try:
        cfg_dir.mkdir(parents=True, exist_ok=True)
        content = (
            "[filechooser]\n"
            "cmd=xiu yazi-chooser\n"
            f"default_dir={home}\n"
            "env=TERMCMD=foot\n"
        )
        cfg.write_text(content)
        cfg_toml.write_text(content)

        for pconf in portal_confs:
            pconf.parent.mkdir(parents=True, exist_ok=True)
            if pconf.is_file():
                text = pconf.read_text()
                if "org.freedesktop.impl.portal.FileChooser=termfilechooser" not in text:
                    if "[preferred]" in text:
                        text = text.replace(
                            "[preferred]",
                            "[preferred]\norg.freedesktop.impl.portal.FileChooser=termfilechooser;gtk\n"
                        )
                    else:
                        text += "\n[preferred]\norg.freedesktop.impl.portal.FileChooser=termfilechooser;gtk\n"
                    pconf.write_text(text)
            else:
                pconf.write_text(
                    "[preferred]\ndefault=gtk\norg.freedesktop.impl.portal.FileChooser=termfilechooser;gtk\n"
                )

        try:
            subprocess.run(["systemctl", "--user", "restart", "xdg-desktop-portal"],
                           capture_output=True, timeout=5)
        except Exception:
            pass

        print(f"  wired portal FileChooser -> yazi ({cfg})")
        return True, ""
    except OSError as exc:
        return False, f"{exc}: wire portal chooser"


def wire_gnome_keyring(dry):
    """
    Configure the system to use gnome-keyring for Secret Service storage
    (GitHub CLI, Git credential helper, browsers, and desktop apps).

    1. Enables systemd user socket/service for gnome-keyring-daemon if present.
    2. Configures Git credential helper to use git-credential-libsecret.
    3. Runs gh auth setup-git if gh is available.
    """
    if shutil.which("systemctl"):
        if dry:
            print("  would enable systemd user socket: gnome-keyring-daemon.socket")
        else:
            try:
                subprocess.run(
                    ["systemctl", "--user", "enable", "--now", "gnome-keyring-daemon.socket"],
                    capture_output=True, timeout=5
                )
            except Exception:
                pass

    git_bin = shutil.which("git")
    libsecret_helper = None
    for cand in [
        "/usr/lib/git-core/git-credential-libsecret",
        "/usr/libexec/git-core/git-credential-libsecret",
        "/usr/local/libexec/git-core/git-credential-libsecret",
    ]:
        if Path(cand).is_file():
            libsecret_helper = cand
            break

    if git_bin and libsecret_helper:
        if dry:
            print(f"  would configure git credential helper -> {libsecret_helper}")
        else:
            try:
                subprocess.run(
                    ["git", "config", "--global", "credential.helper", libsecret_helper],
                    capture_output=True, timeout=5
                )
            except Exception:
                pass

    if shutil.which("gh"):
        if dry:
            print("  would run: gh auth setup-git")
        else:
            try:
                subprocess.run(["gh", "auth", "setup-git"], capture_output=True, timeout=5)
            except Exception:
                pass

    print("  configured gnome-keyring and git credential helper")
    return True, ""


def install_yazi_plugins(dry):
    """
    Add the plugins the shipped yazi.toml keymap binds, through yazi's own
    package manager. Idempotent: `ya pkg add` is a no-op for an already-added
    package, so re-runs only fetch what's missing. Skipped entirely when yazi
    isn't installed on this box (the group is optional) or has no `ya` CLI —
    the keymap degrades to yazi's own keys and the next re-run installs the
    plugins once yazi is present.
    """
    if shutil.which("yazi") is None or shutil.which("ya") is None:
        print("  yazi not installed here — plugins wait for the next re-run")
        return True, ""
    if dry:
        print("  would ya pkg add %d plugins" % len(YAZI_PLUGINS))
        return True, ""
    argv = ["ya", "pkg", "add"] + YAZI_PLUGINS
    try:
        r = subprocess.run(argv, capture_output=True, text=True)
    except OSError as exc:
        return False, f"{exc}: ya pkg add"
    if r.returncode != 0:
        out = (r.stderr or "") + (r.stdout or "")
        # "already exists" is the re-run answer: everything we bind is
        # present, which is the success condition, not a failure.
        if "already exists" in out:
            print("  yazi plugins already installed")
            _patch_yatline_deprecation()
            return True, ""
        tail = out.strip().splitlines()
        return False, "ya pkg add: " + (tail[-1] if tail else "failed")
    _patch_yatline_deprecation()
    print("  yazi plugins installed (%d)" % len(YAZI_PLUGINS))
    return True, ""


def _patch_yatline_deprecation():
    yatline_main = Path.home() / ".config" / "yazi" / "plugins" / "yatline.yazi" / "main.lua"
    if yatline_main.is_file():
        try:
            content = yatline_main.read_text(encoding="utf-8")
            if "hovered:icon()" in content:
                content = content.replace(
                    "local icon = hovered:icon().text",
                    "local icon = nil\n\t\t\tif th and th.icon then\n\t\t\t\tlocal ok, res = pcall(function() return th.icon:match(hovered) end)\n\t\t\t\tif ok and res then icon = res end\n\t\t\tend\n\t\t\tlocal icon_str = (icon and icon.text) or (cha.is_dir and \"\\uf115\" or \"\\uf15b\")\n\t\t\treturn icon_str .. \" \" .. name"
                )
                yatline_main.chmod(0o644)
                yatline_main.write_text(content, encoding="utf-8")
        except Exception:
            pass


def bridge_wallpaper_binary(dry):
    """
    Point the rice's awww binary at swww. The wallpaper scripts call awww and
    awww-daemon (the CachyOS names); the manifest installs the `swww` package,
    which is real awww on CachyOS but plain swww/swww-daemon everywhere else
    (vanilla Arch, Fedora, openSUSE, a Debian source build). On those boxes no
    awww binary exists, so the wallpaper never sets. When awww is missing but
    swww is present, symlink the awww names onto swww in ~/.local/bin. A no-op
    where awww is the real binary. Returns (ok, detail, bridged) so the caller
    folds it into record() and flags the PATH note only when a link was made.
    """
    if dry:
        print("  would bridge: awww -> swww")
        return True, "", False
    if shutil.which("awww"):
        return True, "", False
    swww = shutil.which("swww")
    if not swww:
        return True, "", False
    pairs = [("awww", swww)]
    swww_daemon = shutil.which("swww-daemon")
    if swww_daemon:
        pairs.append(("awww-daemon", swww_daemon))
    bindir = Path.home() / ".local" / "bin"
    try:
        bindir.mkdir(parents=True, exist_ok=True)
        for name, target in pairs:
            link = bindir / name
            if link.is_symlink() or link.exists():
                link.unlink()
            link.symlink_to(target)
    except OSError as exc:
        return False, f"{exc}: bridge awww -> swww", False
    print(f"  bridged: awww -> {swww} (in {bindir})")
    return True, "", True


def bridge_rishot_binary(dry):
    """
    Point ~/.local/bin/rishot at the xiu rishot wrapper script in
    ~/.config/hypr/scripts/rishot so screenshot keybinds, launcher, and desktop app
    always follow the xiu theme.
    """
    if dry:
        print("  would bridge: rishot -> ~/.config/hypr/scripts/rishot")
        return True, "", False
    script = Path.home() / ".config" / "hypr" / "scripts" / "rishot"
    bindir = Path.home() / ".local" / "bin"
    try:
        bindir.mkdir(parents=True, exist_ok=True)
        link = bindir / "rishot"
        if script.exists():
            try:
                script.chmod(0o755)
            except OSError:
                pass
            if link.is_symlink() or link.exists():
                link.unlink()
            link.symlink_to(script)
            print(f"  bridged: rishot -> {script} (in {bindir})")
            return True, "", True
        return True, "", False
    except OSError as exc:
        return False, f"{exc}: bridge rishot", False


def deploy_brave_theme(source, dry):
    """
    Copy the bundled Brave theme into ~/.config/xiu so the user can point Brave
    at it. Chromium signs its own preferences, so the theme can never be applied
    reliably from outside; it just has to sit on disk, ready to load from
    brave://settings. A pre-rename copy at ~/.config/ricelin/brave-theme moves
    over first. Returns (ok, detail) so the caller folds it into record().
    """
    dest_show = "~/.config/xiu/brave-theme"
    if dry:
        print(f"  would deploy: brave-theme -> {dest_show}")
        return True, ""
    src = os.path.join(source, "brave-theme")
    if not os.path.isdir(src):
        return False, f"brave theme not found at {src}"
    dest = os.path.expanduser(dest_show)
    try:
        old = os.path.expanduser("~/.config/ricelin/brave-theme")
        if os.path.isdir(old) and not os.path.isdir(dest):
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            shutil.move(old, dest)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.copytree(src, dest, dirs_exist_ok=True)
    except OSError as exc:
        return False, f"{exc}: copy brave-theme"
    print(f"  deployed: brave-theme -> {dest_show}")
    return True, ""


def deploy_browser_theme(source, dry):
    """
    Register the xiu native theme host with Firefox and Zen and drop the
    userChrome plus its enabling prefs into every existing profile, so the
    browsers follow the palette.

    The host script is copied to a stable path (~/.config/xiu/xiufox-host.py,
    the path baked into the manifest template) so registration never breaks
    when the deploy set is reorganized. Profiles are only touched where a
    browser root already exists — nothing is created for browsers not in use,
    and an existing foreign userChrome.css is parked as .pre-xiu first. The
    prefs land as a marked block in user.js so hand-written prefs around it
    survive re-runs. The live-theme extension itself is unsigned (MV2): it
    loads as a temporary add-on via about:debugging, or self-signed for
    permanent use — the report says so.

    Returns (ok, detail, count) with the number of profiles wired, for notes.
    """
    if dry:
        print("  would deploy: browser theme host + userChrome into profiles")
        return True, "", 0
    src = Path(source) / "browser-integration"
    home = Path.home()
    try:
        xiu = home / ".config" / "xiu"
        xiu.mkdir(parents=True, exist_ok=True)
        host = xiu / "xiufox-host.py"
        shutil.copy2(src / "xiufox-host.py", host)
        host.chmod(0o755)

        manifest = (src / "native-host-manifest.json").read_text().replace(
            "__HOME__", str(home))

        wired = 0
        for root, browser in ((home / ".mozilla", "firefox"), (home / ".zen", "zen")):
            if not root.is_dir():
                continue
            nm = root / "native-messaging-hosts"
            nm.mkdir(parents=True, exist_ok=True)
            (nm / "io.github.yrpcaro.xiu.json").write_text(manifest)

            userchrome = (src.parent / browser / "userChrome.css").read_text()
            userjs = (src.parent / browser / "user.js").read_text() \
                if (src.parent / browser / "user.js").is_file() else ""
            block = "// >>> xiu (managed by the installer)\n" + userjs + "// <<< xiu\n"
            for prof in sorted(root.rglob("prefs.js")):
                prof_dir = prof.parent
                chrome = prof_dir / "chrome"
                chrome.mkdir(exist_ok=True)
                uc = chrome / "userChrome.css"
                if uc.is_file() and uc.read_text() != userchrome:
                    keep = uc.with_suffix(".css.pre-xiu")
                    if not keep.exists():
                        shutil.copy2(uc, keep)
                uc.write_text(userchrome)
                ujs = prof_dir / "user.js"
                body = ""
                if ujs.is_file():
                    body = ujs.read_text()
                    if ">>> xiu" in body:
                        head = body.split("// >>> xiu")[0].rstrip("\n")
                        tail = body.split("// <<< xiu", 1)[-1].lstrip("\n")
                        body = (head + "\n\n" if head.strip() else "") + \
                            (tail if tail.strip() else "")
                ujs.write_text(((body.rstrip("\n") + "\n\n") if body.strip() else "")
                               + block)
                wired += 1
    except OSError as exc:
        return False, f"{exc}: browser theme", wired
    print(f"  wired: browser live theme into {wired} profile(s)")
    return True, "", wired


GREETD_CONF = """\
# xiu: greetd + tuigreet. Pick your session once (F2); --remember-session
# brings you straight back to it. Managed by the xiu installer: a config you
# hand-edited is left alone, remove the file to let xiu take it back.
[terminal]
vt = 1

[default_session]
command = "tuigreet --time --asterisks --remember --remember-session"
user = "greeter"
"""


def install_greetd(dry):
    """
    Write /etc/greetd/config.toml (a hand-edited config is left alone, with a
    note) and point display-manager.service at greetd, stepping any other
    enabled greeter aside first. Returns (results, notes): results are
    (ok, detail, step, hint) tuples the caller folds into record(), notes are
    plain strings for the report.
    """
    results, notes = [], []
    cfg = Path("/etc/greetd/config.toml")
    if cfg.is_file():
        try:
            existing = cfg.read_text()
        except OSError as exc:
            results.append((False, str(exc), "Read greetd config",
                            "Check /etc/greetd permissions."))
            return results, notes
        if existing.strip() != GREETD_CONF.strip():
            notes.append("Left your existing /etc/greetd/config.toml alone; "
                         "delete it and re-run to let xiu manage it.")
    elif dry:
        print("  would run: write /etc/greetd/config.toml (tuigreet)")
        print("  would run: sudo systemctl enable greetd.service")
        return results, notes
    else:
        import tempfile
        with tempfile.NamedTemporaryFile("w", delete=False) as tmp:
            tmp.write(GREETD_CONF)
            tmpname = tmp.name
        try:
            ok = subprocess.run(["sudo", "install", "-m", "644", "-D",
                                 tmpname, str(cfg)]).returncode == 0
        finally:
            os.unlink(tmpname)
        results.append((ok, "" if ok else "sudo install failed",
                        "Write greetd config",
                        "Write /etc/greetd/config.toml yourself (see the repo's GREETD_CONF)."))

    dm = "/etc/systemd/system/display-manager.service"
    if os.path.islink(dm):
        current = os.path.basename(os.path.realpath(dm))
        if current not in ("greetd.service", "") and not dry:
            subprocess.run(["sudo", "systemctl", "disable", current],
                           stderr=subprocess.DEVNULL)
            notes.append(f"Stepped {current} aside as the display manager.")
    if not dry:
        ok = subprocess.run(["sudo", "systemctl", "enable", "greetd.service"],
                            stderr=subprocess.DEVNULL).returncode == 0
        results.append((ok, "" if ok else "systemctl enable failed",
                        "Enable greetd", "Run: sudo systemctl enable greetd"))
    return results, notes


def _seed_update_baseline(source, config_root, dry):
    """
    Hand the in-app updater the commit just installed, so its first check counts new
    commits from here instead of treating a fresh box as already up to date. Without
    it the updater has no synced sha to count from and reports a box that is really
    several commits back as current, with no way to ever reach an apply.

    Best effort: only the git-clone install path (the real curl-bash flow) has a sha
    to record, so a tarball or dev run with no checkout is simply skipped. The engine
    itself ignores a box that already carries a manifest or that updates through
    plain git, so calling it here is always safe. Returns (ok, detail); a failed
    baseline means the updater would report a stale box as current forever, so the
    caller surfaces it instead of letting it vanish.
    """
    if dry:
        return True, ""
    repo = Path(source).resolve().parent
    try:
        head = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return True, ""
    if not head:
        return True, ""
    engine = Path(config_root) / "hypr" / "scripts" / "xiu-update.py"
    if shutil.which("xiu"):
        cmd = ["xiu", "update", "baseline", "--sha", head, "--config-root", str(config_root)]
    elif engine.exists():
        cmd = [sys.executable, str(engine), "baseline", "--sha", head, "--config-root", str(config_root)]
    else:
        return True, ""
    try:
        result = subprocess.run(cmd, capture_output=True, text=True)
    except OSError as exc:
        return False, str(exc)
    if result.returncode != 0:
        return False, (result.stderr or result.stdout or "").strip() or "baseline failed"
    return True, ""


def _report(plan, failures, notes, info, choices, args, do_pkgs, dry):
    """The closing report: a package tally, the next steps, and anything still owed."""
    tally = None
    if do_pkgs:
        parts = []
        landed = plan["native"] + plan["aur"]
        if landed:
            parts.append(f"{len(landed)} new")
        if plan["fallbacks"]:
            parts.append(f"{len(plan['fallbacks'])} built")
        if plan["skipped"]:
            parts.append(f"{len(plan['skipped'])} already here")
        if parts:
            tally = ("would add " if dry else "") + " · ".join(parts)

    steps = []
    if dry:
        steps.append(("dry run", "nothing changed, a real run ends like this"))
    openrc_done = info["init"] == "openrc" and choices.get("services")
    if info["init"] != "systemd" and not openrc_done and do_pkgs:
        steps.append(("enable services", "NetworkManager and bluetooth via your init"))
    if do_pkgs or shutil.which("Hyprland"):
        steps.append(("start Hyprland", "from a TTY"))
    else:
        steps.append(("install Hyprland", "packages were skipped, the rice needs it"))
    steps.append(("open the launcher", "Super+Space; keybinds live in Settings"))
    steps.append(("pick a wallpaper", "Super+C to swap or grab more"))
    if choices["brave"]:
        steps.append(("brave theme",
                      "brave://settings/appearance, load ~/.config/xiu/brave-theme"))
    if choices.get("browser_theme"):
        steps.append(("browser theme",
                      "Firefox/Zen: about:debugging, Load Temporary Add-on, "
                      "~/.config/xiu/browser-integration/manifest.json; Brave: run xiu browser"))

    attention = []
    for step, _detail, hint in failures:
        cmd = hint[len("Run: "):] if hint.startswith("Run: ") else hint
        attention.append((step, cmd))

    title = "Dry run complete" if dry else "xiu is in"
    tui.closing(title, tally, steps, attention, notes or None)


def run(args):
    """Walk the whole install flow and return an exit code."""
    dry = args.dry_run
    manifest = distro.load_manifest()
    info = detect()
    family_ok = info["family"] in distro.FAMILIES
    do_pkgs = not args.no_deps and family_ok and not info["immutable"]

    tui.banner()
    helper = info["aur_helper"]
    if helper:
        helper_label = helper
    elif info["family"] == "arch":
        helper_label = "Will install yay"
    else:
        helper_label = "Not needed"
    has_config = any(v["exists"] for v in info["existing"].values())
    if _is_update(info):
        config_label = "xiu (this run updates it, your Settings are kept)"
    elif has_config:
        config_label = "Found (backed up before anything is replaced)"
    else:
        config_label = "Fresh machine"
    tui.detected([
        ("OS", info["pretty"], True),
        ("Session", info["compositor"], True),
        ("Packages", info["pm"], True),
        ("AUR helper", helper_label, True),
        ("Configs", config_label, True),
    ])

    # An unsupported family (Gentoo, Void, ...) gets a loud gate up front, not a
    # quiet mid-run info line: nothing installs there, so the rice will miss its
    # dependencies unless the user provides them. Same gate for a read-only-root
    # box like SteamOS, where the package manager cannot write the system at all.
    if not args.no_deps and not do_pkgs:
        if info["immutable"]:
            why = "This system has a read-only root, so no packages can be installed."
        else:
            why = (f"{info['pretty']} is not a supported distro family "
                   "(arch, debian, fedora, suse or gentoo), so no packages will be installed.")
        warn = [why,
                "Only the configs will be deployed. The rice needs Hyprland, "
                "quickshell and its other dependencies installed by hand, "
                "at your own risk."]
        if args.quickstart:
            tui.info(warn)
        else:
            try:
                if not tui.confirm("Unsupported system", warn + ["Continue anyway?"]):
                    tui.outro("Cancelled")
                    return 0
            except RuntimeError:
                tui.info(warn)

    if args.quickstart:
        choices = _default_choices(args, info, manifest)
    else:
        try:
            choices = _wizard(args, info, manifest)
        except RuntimeError:
            tui.info(["No controlling terminal, taking the Quick defaults."])
            choices = _default_choices(args, info, manifest)

    distro.ROOT = choices.get("root_cmd", "sudo")
    plan = _build_plan(manifest, info, choices, cli_local(args.source))

    summary = _summary_lines(info, choices, plan, args, do_pkgs)
    tui.info(summary)

    failures, notes = [], []

    def record(ok, detail, step, hint):
        if not ok:
            failures.append((step, detail, hint))

    needs_sudo = (do_pkgs or choices["greeter"] != "none" or choices.get("grub")
                  or choices["fish"] or choices["brave"]
                  or (choices["legacy_swap"] == "clean" and do_pkgs)) and not dry
    keepalive_stop = sudo_keepalive() if needs_sudo else None
    try:
        if do_pkgs:
            family = info["family"]

            # a. refresh the package index first so a stale list never sinks the run.
            refresh_env = pkg.INSTALL_ENV if family == "debian" else None
            ok, detail = _run(pkg.refresh_argv(family), dry, env=refresh_env)
            record(ok, detail, "Refresh package index",
                   "Update the package list yourself, then re-run.")

            # b. bootstrap an AUR helper on Arch when one is wanted but missing.
            if family == "arch" and choices["aur_choice"] != "none" and pkg.aur_helper() is None:
                for step_argv in pkg.ensure_aur_helper_steps():
                    ok, detail = _run(step_argv, dry)
                    record(ok, detail, "Bootstrap yay",
                           "Install yay or paru by hand, then re-run.")

            # c. enable any extra repos the native packages need.
            for repo in plan["repos"]:
                for argv in pkg.enable_repo_argv(repo, family):
                    ok, detail = _run(pkg.privileged(argv, family), dry)
                    record(ok, detail, f"Enable repo {repo}",
                           "Enable the repo by hand, then re-run.")

            # c2. gentoo: the per-atom keyword and USE lines, in files of our own.
            if family == "gentoo":
                for argv in pkg.portage_config_argv(plan.get("atom_repos", {})):
                    ok, detail = _run(pkg.privileged(argv, family), dry)
                    record(ok, detail, "Write portage overrides",
                           "Make sure /etc/portage/package.accept_keywords and package.use "
                           "are directories, then re-run.")
            if plan.get("manual"):
                notes.append("Overlays skipped, these need one: emerge -av "
                             + " ".join(plan["manual"]))

            # d. the native batch, one install, sudo-wrapped. If the whole
            #    transaction aborts on a single bad name, retry each package
            #    alone so one failure never loses the family's core set.
            if plan["native"]:
                batch = pkg.install_argv(plan["native"], family)
                if family == "fedora":
                    batch = [*batch, "--skip-broken"]
                ok, detail = _run(pkg.privileged(batch, family), dry, env=pkg.INSTALL_ENV)
                if ok or dry:
                    record(ok, detail, "Install packages",
                           "Re-run; a single failed package will not block the rest.")
                else:
                    for name in plan["native"]:
                        argv = pkg.privileged(pkg.install_argv([name], family), family)
                        ok, detail = _run(argv, dry, env=pkg.INSTALL_ENV)
                        if not ok and name in plan["optional_native"]:
                            notes.append(f"Optional package {name} did not install, skipped.")
                            continue
                        record(ok, detail, f"Install {name}",
                               "Install this one package by hand, then re-run.")

            # e. the AUR batch, unwrapped, the helper escalates itself. Same
            #    per-package retry on a batch abort.
            if plan["aur"]:
                argv = _aur_install_argv(plan["aur"], family, choices["aur_choice"])
                ok, detail = _run(argv, dry)
                if ok or dry:
                    record(ok, detail, "Install AUR packages",
                           "Build the AUR packages with your helper, then re-run.")
                else:
                    for name in plan["aur"]:
                        one = _aur_install_argv([name], family, choices["aur_choice"])
                        ok, detail = _run(one, dry)
                        record(ok, detail, f"Install AUR {name}",
                               "Build this AUR package by hand, then re-run.")

            # f. the fallbacks, each handler's steps in order. A failed step ends
            #    that fallback: later steps build on it, and one attention entry
            #    per package beats a doubled failure label.
            for fid, handler, pkgdict in plan["fallbacks"]:
                for step in fallbacks.steps_for(handler, pkgdict, family):
                    if "run" in step:
                        ok, detail = _run(step["run"], dry)
                    else:
                        ok, detail = _shell(step["shell"], dry)
                    if not ok:
                        record(ok, detail, f"Fallback {fid} ({handler})",
                               "Follow the project's own install steps for this one.")
                        break

            # g. wire up uinput on every family, the dotool handler's last steps.
            #    Skipped only when the dotool fallback already ran them (off Arch),
            #    so uinput ends up set up everywhere with no double work.
            dotool_fb = any(handler == "dotool" for _, handler, _ in plan["fallbacks"])
            if not dotool_fb:
                for step in fallbacks.steps_for("dotool", {"id": "dotool"}, family)[4:]:
                    if "run" in step:
                        ok, detail = _run(step["run"], dry)
                    else:
                        ok, detail = _shell(step["shell"], dry)
                    record(ok, detail, "Set up uinput",
                           "Add yourself to the input group and reload udev by hand.")

            # g2. ddcutil reaches the monitors' DDC/CI through /dev/i2c-*; the
            #     package ships the udev rule granting the i2c group, so all
            #     that is left is the membership. Without it the mixer's
            #     per-monitor brightness faders detect nothing.
            def _in_group(name):
                try:
                    out = subprocess.run(["id", "-nG"], capture_output=True, text=True)
                    return (" " + name + " ") in (" " + out.stdout.replace(",", " ") + " ")
                except OSError:
                    return True

            if shutil.which("ddcutil") and not _in_group("i2c"):
                ok, detail = _shell(
                    'sudo groupadd -f i2c && sudo usermod -aG i2c "$(id -un)" '
                    '&& sudo udevadm control --reload && sudo udevadm trigger', dry)
                record(ok, detail, "Set up i2c for DDC",
                       "Run: sudo usermod -aG i2c $(whoami), then log out and back in.")
                notes.append("Added you to the i2c group so the mixer can see every "
                             "monitor's brightness — one re-login for it to land.")

            # h. services: enable on systemd, print the manual commands otherwise.
            if info["init"] == "systemd":
                if _active("systemd-networkd") or _active("iwd"):
                    notes.append("Another network manager is active, left NetworkManager "
                                 "alone. The Link surface wants NetworkManager.")
                else:
                    ok, detail = _run(
                        [distro.ROOT, "systemctl", "enable", "--now", "NetworkManager.service"], dry)
                    record(ok, detail, "Enable NetworkManager",
                           "Enable NetworkManager.service yourself.")
                ok, detail = _run(
                    [distro.ROOT, "systemctl", "enable", "--now", "bluetooth.service"], dry)
                record(ok, detail, "Enable bluetooth", "Enable bluetooth.service yourself.")
                if shutil.which("hyprsunset"):
                    # A --user systemctl needs the user's session bus; from a
                    # bare TTY (the usual curl|bash fresh install) there is
                    # none — DBUS_SESSION_BUS_ADDRESS/XDG_RUNTIME_DIR unset —
                    # and systemctl refuses with the bus-connect error. Only
                    # try when the bus is actually there, and defer to the
                    # first login otherwise.
                    user_bus = (
                        os.environ.get("DBUS_SESSION_BUS_ADDRESS")
                        or (Path(os.environ.get("XDG_RUNTIME_DIR", "")) / "bus").exists()
                    ) if os.environ.get("XDG_RUNTIME_DIR") else False
                    if user_bus:
                        ok, detail = _run(
                            ["systemctl", "--user", "enable", "--now", "hyprsunset.service"], dry)
                        record(ok, detail, "Enable night light",
                               "Run: systemctl --user enable --now hyprsunset.service")
                    else:
                        notes.append("No user session bus yet (installed from a console?) — "
                                     "night light starts on first login: "
                                     "systemctl --user enable --now hyprsunset.service")
                else:
                    notes.append("hyprsunset is not installed, night light left off. "
                                 "Install it and run: systemctl --user enable --now hyprsunset.service")
            elif info["init"] == "openrc" and choices.get("services"):
                for name in ("NetworkManager", "bluetooth"):
                    ok, detail = _shell(_openrc_service_shell(name), dry)
                    record(ok, detail, f"Enable {name}", f"Enable {name} with rc-update yourself.")
            else:
                notes.extend(_service_note(info["init"]))

        # i. bridge the wallpaper binary onto swww when the rice's awww name is
        #    missing, so the background sets on every family, not just CachyOS.
        ok, detail, bridged = bridge_wallpaper_binary(dry)
        record(ok, detail, "Bridge wallpaper binary",
               "Symlink ~/.local/bin/awww to $(command -v swww) yourself.")
        if bridged:
            notes.append("Linked awww to swww in ~/.local/bin. Make sure "
                         "~/.local/bin is on PATH so the wallpaper script finds it.")

        # i2. bridge rishot onto the xiu wrapper so keybinds and launcher follow the theme
        ok, detail, _ = bridge_rishot_binary(dry)
        record(ok, detail, "Bridge rishot binary",
               "Symlink ~/.local/bin/rishot to ~/.config/hypr/scripts/rishot yourself.")

        # j. fish as the login shell, kept even with --no-deps. Never chsh onto a
        #    binary that is not there: root's chsh skips the shell validation, so
        #    a missing fish would land in /etc/passwd and break every login.
        if choices["fish"]:
            fishbin = shutil.which("fish")
            if fishbin:
                # chsh prompts for the login password through PAM, which a piped
                # `curl | bash` run has no terminal for, so it always failed. Set
                # it as root instead; the sudo timestamp is already warm.
                ok, detail = _run([distro.ROOT, "chsh", "-s", fishbin, getpass.getuser()], dry)
                record(ok, detail, "Set fish as login shell",
                       "Run: chsh -s $(command -v fish)")
            elif dry:
                print("  would set fish as login shell (once fish is installed)")
            else:
                record(False, "fish is not installed, login shell left unchanged",
                       "Set fish as login shell",
                       "Install fish, then run: chsh -s $(command -v fish)")

        # k. deploy the configs and make them portable. A copytree or write
        #    that hits an OSError mid-iteration is recorded and stepped past,
        #    so a real run still finishes with a report instead of a traceback.
        #    A "start fresh" choice skips carrying the protected user files
        #    across a managed replace (their pristine .bak backups still land).
        try:
            for action in deploy.deploy(src=args.source, config_root=deploy.CONFIG_ROOT,
                                        apply=not dry,
                                        keep_preserved=not choices["fresh_configs"]):
                if action["action"] == "skip":
                    print(f"  deploy skip: {action['item']} ({action.get('reason', '')})")
                    continue
                head = "would deploy" if dry else "deployed"
                extra = f" (backup {action['backup']})" if action.get("backup") else ""
                if action.get("preserved"):
                    extra += f" (kept {len(action['preserved'])} user files)"
                print(f"  {head}: {action['item']} -> {action['dest']}{extra}")
        except OSError as exc:
            record(False, str(exc), "Deploy configs",
                   "Check ~/.config permissions and re-run the installer.")
        try:
            for action in deploy.neutralize(config_root=deploy.CONFIG_ROOT, apply=not dry,
                                            src=args.source):
                head = "would neutralize" if dry else "neutralized"
                print(f"  {head}: {action['step']}")
        except OSError as exc:
            record(False, str(exc), "Neutralize configs",
                   "Check ~/.config permissions and re-run the installer.")

        # k2. build the xiu CLI from this very checkout, after the deploy so
        #     the binaries and the configs it drives land together. Runs on
        #     every path (the keybinds call xiu), rebuilds so a re-run also
        #     refreshes the binary, and never needs root.
        ok, detail, built = build_xiu_cli(args.source, dry)
        record(ok, detail, "Build the xiu CLI",
               "Run: cd <xiu repo>/cli && cargo build --release "
               "&& install -m755 target/release/xiu ~/.local/bin/xiu")
        if ok and not built:
            notes.append("No cli/ crate next to the configs source; the xiu "
                         "CLI will come through the cargo fallback instead.")

        # l. seed a starter wallpaper so the first boot has a background, a
        #    populated picker and a palette to render.
        ok, detail = seed_vars(choices, dry)
        record(ok, detail, "Seed vars.lua",
               "Create ~/.config/xiu/vars.lua yourself "
               "(see ~/.config/hypr/modules/vars.lua for the keys).")

        # l2. xdg defaults: the file manager owns folders, imv the images,
        #     zathura the documents — merged, never clobbering a user's own.
        ok, detail = set_xdg_defaults(choices, dry)
        record(ok, detail, "Set xdg default apps",
               "Edit ~/.config/mimeapps.list [Default Applications] yourself.")

        ok, detail = apply_input_preferences(choices, dry)
        record(ok, detail, "Apply input preferences",
               "Edit ~/.config/hypr/modules/input.lua yourself.")

        ok, detail = apply_idle_preferences(choices, dry)
        record(ok, detail, "Apply idle timeout",
               "Edit ~/.config/hypr/hypridle.conf yourself.")

        # l3. yazi plugins, through yazi's own package manager (no-op when
        #     yazi isn't installed — the group is optional).
        ok, detail = install_yazi_plugins(dry)
        record(ok, detail, "Install yazi plugins",
               "Run: ya pkg add <plugins> (see installer/xiu_install.py YAZI_PLUGINS).")

        # l4. the portal FileChooser backend, pointed at the yazi wrapper.
        ok, detail = wire_portal_chooser(dry, choices)
        record(ok, detail, "Wire portal file chooser",
               "Write ~/.config/xdg-desktop-portal-termfilechooser/config.toml "
               "pointing cmd= at xiu yazi-chooser.")

        # l5. gnome-keyring and secret service setup.
        ok, detail = wire_gnome_keyring(dry)
        record(ok, detail, "Configure gnome-keyring",
               "Enable gnome-keyring-daemon and configure git credential helper.")

        ok, detail = seed_wallpapers(dry)
        record(ok, detail, "Seed wallpapers",
               "Copy any image into ~/Pictures/xiu/wallpapers yourself.")

        # Apply wallpaper theme so yazi, Qt, GTK and terminals follow theme immediately
        xiu_bin = Path.home() / ".local" / "bin" / "xiu"
        if not dry:
            cmd = [str(xiu_bin), "wallcolors", "--apply"] if xiu_bin.is_file() else ["xiu", "wallcolors", "--apply"] if shutil.which("xiu") else None
            if cmd:
                ok_wc, detail_wc = _run(cmd, dry=False)
                record(ok_wc, detail_wc, "Apply initial wallpaper theme",
                       "Run: xiu wallcolors --apply yourself.")
            else:
                yz = deploy.CONFIG_ROOT / "yazi"
                if yz.is_dir():
                    pill, b_pal, _ = deploy._yazi_palette()
                    deploy._render_yazi(yz, pill, b_pal, apply=True)

            # Rebuild icon cache for yet-another-monochrome-icon-set if present
            yamis_dir = Path.home() / ".local" / "share" / "icons" / "yet-another-monochrome-icon-set"
            if yamis_dir.is_dir() and shutil.which("gtk-update-icon-cache"):
                _run(["gtk-update-icon-cache", "-q", "-f", "-t", str(yamis_dir)], False)

            # Apply gsettings for dark theme and icons
            if shutil.which("gsettings"):
                for k, v in [
                    ("color-scheme", "prefer-dark"),
                    ("gtk-theme", "adw-gtk3-dark"),
                    ("icon-theme", "yet-another-monochrome-icon-set"),
                ]:
                    _run(["gsettings", "set", "org.gnome.desktop.interface", k, v], False)

            # Apply KDE window decoration and plasma style
            if shutil.which("kwriteconfig6"):
                _run(["kwriteconfig6", "--file", "kwinrc", "--group", "org.kde.kdecoration2", "--key", "theme", "Darkly"], False)
                _run(["kwriteconfig6", "--file", "kwinrc", "--group", "org.kde.kdecoration2", "--key", "plugin", "org.kde.darkly"], False)
                _run(["kwriteconfig6", "--file", "kwinrc", "--group", "org.kde.kdecoration2", "--key", "library", "org.kde.darkly"], False)
                _run(["kwriteconfig6", "--file", "plasmarc", "--group", "Theme", "--key", "name", "Darkly"], False)
                _run(["kwriteconfig6", "--file", "kdeglobals", "--group", "KDE", "--key", "widgetStyle", "Darkly"], False)
            if shutil.which("plasma-apply-desktoptheme"):
                _run(["plasma-apply-desktoptheme", "Darkly"], False)
        else:
            print("  would run: xiu wallcolors --apply")

        # m. login screen.
        if choices["greeter"] == "sddm":
            theme = choices.get("sddm_theme", "torii")
            if theme not in ("torii", "washi"):
                theme = "torii"
            sddm_installer = os.path.join(args.source, "sddm", "themes", theme, "install.sh")
            if os.path.isfile(sddm_installer):
                ok, detail = _run(["sh", sddm_installer], dry, env={"SUDO": distro.ROOT})
                record(ok, detail, f"Install SDDM theme ({theme})",
                       "Run the SDDM theme installer by hand.")
            else:
                notes.append(f"SDDM installer not found at {sddm_installer}, skipped.")
        elif choices["greeter"] == "greetd":
            results, gnotes = install_greetd(dry)
            for gok, gdetail, gstep, ghint in results:
                record(gok, gdetail, gstep, ghint)
            notes.extend(gnotes)
        if choices["grub"] and info["bootloader"] == "grub":
            for action in grub_theme.apply(args.source, dry):
                if dry:
                    printable = " ".join(shlex.quote(a) for a in action["cmd"])
                    print(f"  would run: {printable}")
                else:
                    record(action["ok"], action["detail"], "Install GRUB theme",
                           "Run the GRUB theme steps by hand.")

        # n. browsers into the palette: native host registration plus the
        #     userChrome and its prefs into every existing profile. The
        #     extension itself is unsigned and loads as a temporary add-on;
        #     the report says how.
        if choices["browser_theme"]:
            ok, detail, wired = deploy_browser_theme(args.source, dry)
            record(ok, detail, "Wire browser theme",
                   "Copy configs/browser-integration by hand and load the userChrome.")
            if wired == 0 and not dry:
                notes.append("No Firefox/Zen profiles found yet; re-run the "
                             "installer after their first launch to wire the theme in.")

        # n2. chosen extras that need one activation step past the package:
        #     spicetify is pointed at spotify-launcher's install layout
        #     (~/.local/share/spotify-launcher/install/usr/share/spotify —
        #     spicetify's documented path for the launcher) and Spicetify
        #     Marketplace installed; spotify-launcher downloads the Spotify
        #     binary on its FIRST RUN, so on a fresh box the apply is deferred
        #     with a note, and re-running the installer after that first launch
        #     lands it.
        #     vesktop's themes dir is created so the palette pipeline has
        #     somewhere to drop the xiu CSS (vesktop normally creates it on
        #     first run, which may be after the first wallpaper change).
        spotify_dir = (Path.home() / ".local" / "share" / "spotify-launcher"
                       / "install" / "usr" / "share" / "spotify")
        spicetify_cmd = "spicetify" if shutil.which("spicetify") else str(Path.home() / ".spicetify" / "spicetify") if (Path.home() / ".spicetify" / "spicetify").is_file() else None
        want_spicetify = "spicetify-cli" in choices.get("optional_ids", set()) or choices.get("profile") == "quick" or choices.get("spicetify")
        if not dry:
            if want_spicetify and spicetify_cmd:
                ok, detail = _run([
                    spicetify_cmd, "config",
                    "spotify_path", str(spotify_dir),
                    "prefs_path", str(Path.home() / ".config" / "spotify" / "prefs"),
                ], False)
                record(ok, detail, "Configure Spicetify for Spotify launcher",
                       "Run: %s config spotify_path '%s' prefs_path '%s'"
                       % (spicetify_cmd, spotify_dir, Path.home() / ".config" / "spotify" / "prefs"))
                market_cmd = "curl -fsSL https://raw.githubusercontent.com/spicetify/spicetify-marketplace/main/resources/install.sh | sh"
                ok, detail = _shell(market_cmd, False)
                record(ok, detail, "Install Spicetify Marketplace",
                       "Run: " + market_cmd)
                if spotify_dir.is_dir():
                    ok, detail = _run([spicetify_cmd, "backup", "apply"], False)
                    record(ok, detail, "Apply Spicetify",
                           f"Run: {spicetify_cmd} backup apply")
                else:
                    notes.append("spotify-launcher hasn't downloaded Spotify yet — "
                                 f"launch it once, then: {spicetify_cmd} backup apply")
            if "vesktop" in choices.get("optional_ids", set()):
                try:
                    (Path.home() / ".config" / "vesktop" / "themes").mkdir(
                        parents=True, exist_ok=True)
                except OSError:
                    pass
        elif dry and want_spicetify:
            print(f"  would run: {spicetify_cmd or 'spicetify'} config spotify_path '{spotify_dir}' prefs_path '{Path.home() / '.config' / 'spotify' / 'prefs'}'")

        # n3. clean swap: retire the legacy alternatives once their
        #    replacements are confirmed in. Fail-soft by design — the package
        #    manager refuses a removal something else needs, we just surface
        #    it. The fallback choice leaves both installed instead.
        if do_pkgs and choices["legacy_swap"] == "clean":
            fam = info["family"]
            remove_argv = {
                "arch": ["pacman", "-Rns", "--noconfirm"],
                "debian": ["apt-get", "remove", "-y"],
                "fedora": ["dnf", "remove", "-y"],
                "suse": ["zypper", "rm", "-y"],
            }.get(fam)
            keep_ghostty = choices.get("terminal") == "ghostty" or "ghostty" in choices.get("extra_terminals", [])
            swap_pairs = [("cliphist", "clipvault")] if keep_ghostty else [("ghostty", "foot"), ("cliphist", "clipvault")]
            for old_id, new_id in swap_pairs:
                if remove_argv is None:
                    break
                old_pkg = next((p for p in manifest["packages"] if p["id"] == old_id), None)
                new_pkg = next((p for p in manifest["packages"] if p["id"] == new_id), None)
                old = (old_pkg or {}).get("names", {}).get(fam)
                new = (new_pkg or {}).get("names", {}).get(fam)
                if not (old and new and pkg.is_installed(new, fam) and shutil.which(old)):
                    continue
                ok, detail = _run(pkg.privileged(remove_argv + [old], fam), dry)
                if ok and not dry:
                    notes.append(f"Removed {old} ({new} takes over).")
                else:
                    record(ok, detail, f"Remove {old}",
                           f"Run it yourself once you are sure: {' '.join(remove_argv)} {old}")

        # o. optional Brave: install it through the same resolve/fallback path the
        #    core packages use (arch -> AUR brave-bin, off arch -> Flathub), then
        #    drop the theme files in place. The theme is never auto-applied, since
        #    Chromium signs its prefs; the user loads it from brave://settings.
        if choices["brave"]:
            if do_pkgs:
                family = info["family"]
                brave_pkg = next(
                    (p for p in manifest["packages"] if p["id"] == "brave"), None)
                if brave_pkg is None:
                    action, target = "skip", None
                else:
                    action, target = distro.resolve(brave_pkg, family, choices["aur_choice"])
                if action == "skip":
                    notes.append("No Brave package for this distro, skipped the install.")
                elif action == "fallback":
                    for step in fallbacks.steps_for(target, brave_pkg, family):
                        if "run" in step:
                            ok, detail = _run(step["run"], dry)
                        else:
                            ok, detail = _shell(step["shell"], dry)
                        record(ok, detail, "Install Brave",
                               "Install Brave by hand, then load its theme.")
                elif pkg.is_installed(target, family):
                    notes.append("Brave is already installed.")
                elif distro.is_aur(brave_pkg, family):
                    ok, detail = _run(
                        _aur_install_argv([target], family, choices["aur_choice"]), dry)
                    record(ok, detail, "Install Brave",
                           "Install Brave by hand, then load its theme.")
                else:
                    argv = pkg.privileged(pkg.install_argv([target], family), family)
                    ok, detail = _run(argv, dry, env=pkg.INSTALL_ENV)
                    record(ok, detail, "Install Brave",
                           "Install Brave by hand, then load its theme.")
            else:
                notes.append("Skipped the Brave install, only deployed its theme.")
            ok, detail = deploy_brave_theme(args.source, dry)
            record(ok, detail, "Deploy Brave theme",
                   "Copy configs/brave-theme to ~/.config/xiu/brave-theme yourself.")
    finally:
        if keepalive_stop:
            keepalive_stop()

    ok, detail = _seed_update_baseline(args.source, deploy.CONFIG_ROOT, dry)
    if not ok:
        failures.append(("Seed update baseline", detail,
                         "Open Settings > Updates once; the first apply sets it up."))
    _report(plan, failures, notes, info, choices, args, do_pkgs, dry)
    return 0


def run_uninstall(args):
    """
    Remove every xiu-managed config and put the pre-install backups back.
    Packages stay; only the deployed files go. Confirms interactively before
    touching anything, and refuses to run headless, since a piped one-liner
    should never be able to wipe a config unattended.
    """
    dry = args.dry_run
    tui.banner()
    plan = deploy.uninstall(config_root=deploy.CONFIG_ROOT, apply=False)
    removals = [a for a in plan if a["action"] == "remove"]
    if not removals:
        tui.info(["Nothing xiu-managed found in ~/.config, nothing to remove."])
        tui.outro("Done")
        return 0

    lines = []
    for a in removals:
        line = f"Remove {a['dest']}"
        if a["restored"]:
            line += f", restore your backup from {a['restored']}"
        lines.append(line)
    portage_files = [f for f in pkg.portage_config_files() if os.path.exists(f)]
    for f in portage_files:
        lines.append(f"Remove {f} (needs root)")
    lines.append("Installed packages are not touched.")

    if dry:
        tui.info(lines)
        tui.outro("Dry run complete")
        return 0
    try:
        if not tui.confirm("Remove xiu", lines):
            tui.outro("Cancelled")
            return 0
    except RuntimeError:
        tui.info(["No controlling terminal; run the uninstall from a real "
                  "terminal so it can confirm first."])
        return 1

    for a in deploy.uninstall(config_root=deploy.CONFIG_ROOT, apply=True):
        if a["action"] == "remove":
            tail = f" (restored {a['restored']})" if a["restored"] else ""
            print(f"  removed: {a['dest']}{tail}")

    for f in portage_files:
        ok, _ = _run([_root_cmd(), "rm", "-f", f], dry)
        print(f"  removed: {f}" if ok else f"  could not remove {f}, delete it yourself")

    for name in ("xiu", "ricelin"):
        cli_bin = Path.home() / ".local" / "bin" / name
        if cli_bin.is_file() or cli_bin.is_symlink():
            try:
                cli_bin.unlink()
                print(f"  removed: {cli_bin}")
            except OSError:
                pass
    tui.info(["The repo clone in ~/.local/share/xiu and your wallpapers in "
              "~/Pictures/xiu/wallpapers are left for you to delete."])
    tui.outro("xiu removed")
    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Install the xiu Hyprland rice across distro families.")
    parser.add_argument("--dry-run", action="store_true",
                        help="Walk the whole flow and change nothing")
    parser.add_argument("--quickstart", action="store_true",
                        help="Skip the wizard, take the Quick-profile defaults")
    parser.add_argument("--source", default=str(deploy.CONFIGS),
                        help="The repo configs directory to deploy from")
    parser.add_argument("--full", action="store_true",
                        help="Preselect the Full profile")
    parser.add_argument("--sddm", action="store_true",
                        help="Preselect the torii SDDM login theme")
    parser.add_argument("--brave", action="store_true",
                        help="Preselect Brave plus its xiu theme")
    parser.add_argument("--no-deps", action="store_true",
                        help="Skip the package step, only deploy the configs")
    parser.add_argument("--reinstall", action="store_true",
                        help="Run the full install over an existing xiu install")
    parser.add_argument("--uninstall", action="store_true",
                        help="Remove the deployed configs and restore the backups")
    args = parser.parse_args()
    try:
        if args.uninstall:
            return run_uninstall(args)
        return run(args)
    except KeyboardInterrupt:
        tui.outro("Cancelled")
        return 130


if __name__ == "__main__":
    raise SystemExit(main())
