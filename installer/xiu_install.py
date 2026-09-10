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


def detect():
    """Read the whole machine state the flow branches on into one dict."""
    family = distro.detect_family()
    return {
        "family": family,
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
    return {
        "profile": profile,
        "aur_choice": info["aur_helper"] or "yay",
        "optional_ids": set(full_ids) if profile == "full" else set(),
        "file_manager": "dolphin",
        "greeter": "sddm" if args.sddm else "none",
        "sddm_theme": "torii",
        "browser_theme": True,
        "fresh_configs": False,
        "legacy_swap": "fallback",
        "grub": False,
        "fish": True,
        "brave": args.brave,
    }


def _wizard(args, info, manifest):
    """
    Walk the few questions that shape the install. Raises RuntimeError up from the
    UI when there is no controlling terminal, so the caller can drop to defaults.
    """
    family = info["family"]

    pidx = tui.select_one("Install profile", [
        ("Quick", "Core rice, sensible defaults, no questions", True),
        ("Full", "Everything, plus the daily apps", False),
        ("Custom", "Walk every choice yourself", False),
    ], default=1 if args.full else 0)
    profile = ("quick", "full", "custom")[pidx]

    aur_choice = "yay"
    if family == "arch":
        aidx = tui.select_one("AUR helper", [
            ("yay", "Build AUR packages with yay", True),
            ("paru", "Build AUR packages with paru", False),
            ("None", "Skip the AUR, use fallbacks instead", False),
        ], default=0)
        aur_choice = ("yay", "paru", "none")[aidx]

    fidx = tui.select_one("File manager", [
        ("dolphin", "KDE file manager, native dialogs for the rice", True),
        ("yazi", "TUI file manager, keyboard-driven with image previews", False),
        ("thunar", "Xfce file manager, themed through the palette", False),
        ("none", "Keep whatever I use today", False),
    ], default=0)
    file_manager = ("dolphin", "yazi", "thunar", "none")[fidx]

    gidx = tui.select_one("Login screen", [
        ("TTY", "No greeter; start Hyprland from a terminal login", True),
        ("SDDM", "Graphical login with a xiu theme", False),
        ("greetd", "Minimal greeter with tuigreet", False),
    ], default=0)
    greeter = ("none", "sddm", "greetd")[gidx]

    sddm_theme = "torii"
    if greeter == "sddm":
        tidx = tui.select_one("SDDM theme", [
            ("washi", "Mirrors the lock screen: frosted wallpaper, the Zen Kaku clock, palette-following", True),
            ("torii", "The cinematic torii video wallpaper", False),
        ], default=0)
        sddm_theme = ("washi", "torii")[tidx]

    lidx = tui.select_one("Legacy tools", [
        ("Fallback", "ghostty and cliphist stay available as the optional terminal and clipboard backends", True),
        ("Clean swap", "remove ghostty and cliphist once foot and clipvault are in", False),
    ], default=0)
    legacy_swap = ("fallback", "clean")[lidx]

    keep = tui.confirm("Keep your existing configs", [
        "Your fish config, keybinds and Settings changes are carried across",
        "and three-way merged on updates. (Recommended)",
        "Answer No to start from the repo's defaults instead.",
    ])
    fresh_configs = not keep

    full_pkgs = [p for p in manifest["packages"] if p.get("group") == "full"]
    optional_ids = set()
    if profile in ("full", "custom"):
        options = [(p["id"], p["desc"], False) for p in full_pkgs]
        preselect = range(len(full_pkgs)) if profile == "full" else ()
        chosen = tui.select_many("Optional apps", options, preselect=preselect)
        optional_ids = {full_pkgs[i]["id"] for i in chosen}

    browser_theme = tui.confirm("Browser live theme", [
        "Register the xiu native theme host and copy the Firefox/Zen",
        "userChrome into your profiles, so browsers follow the palette.",
    ])

    grub = False
    if info["bootloader"] == "grub":
        grub = tui.confirm("GRUB theme", [
            "Install the xiu GRUB theme.",
            "Theme only, it does not touch your boot entries.",
        ])

    brave = True if args.brave else False
    if not args.brave:
        bidx = tui.select_one("Brave browser", [
            ("Install Brave", "Brave browser with the matching xiu theme", True),
            ("Skip", "Leave Brave out for now", False),
        ], default=1)
        brave = bidx == 0

    fish = tui.confirm("Login shell", ["Set fish as your login shell. (Recommended)"])

    return {
        "profile": profile, "aur_choice": aur_choice, "optional_ids": optional_ids,
        "file_manager": file_manager, "greeter": greeter,
        "sddm_theme": sddm_theme,
        "browser_theme": browser_theme, "fresh_configs": fresh_configs,
        "legacy_swap": legacy_swap, "grub": grub, "fish": fish, "brave": brave,
    }


def _choice_ids(choices):
    """The full-group package ids an explicit wizard/quickstart choice pulls in,
    so a Quick install still gets the chosen file manager or greeter."""
    ids = set()
    fm = choices.get("file_manager")
    if fm and fm != "none":
        ids.add(fm)
    if choices.get("greeter") == "sddm":
        ids.add("sddm")
    elif choices.get("greeter") == "greetd":
        ids.update(("greetd", "greetd-tuigreet"))
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
        rows = [r for r in rows if r["id"] not in ("ghostty", "cliphist")]

    repos, native, aur, fb, skipped = [], [], [], [], []
    optional_native = set()
    for r in rows:
        if r["action"] == "skip":
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
        if not r["aur"] and not r["required"]:
            optional_native.add(r["target"])
    return {"repos": repos, "native": native, "aur": aur, "fallbacks": fb,
            "skipped": skipped, "optional_native": optional_native}


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
    cmds = {
        "openrc": ["sudo rc-update add NetworkManager default && sudo rc-service NetworkManager start",
                   "sudo rc-update add bluetoothd default && sudo rc-service bluetoothd start"],
        "runit": ["sudo ln -s /etc/sv/NetworkManager /var/service",
                  "sudo ln -s /etc/sv/bluetoothd /var/service"],
        "dinit": ["sudo dinitctl enable NetworkManager", "sudo dinitctl enable bluetoothd"],
        "s6": ["s6-rc -u change NetworkManager", "s6-rc -u change bluetoothd"],
    }
    lines = ["Non-systemd init detected, enable the services yourself:"]
    lines += cmds.get(init, ["Enable NetworkManager and bluetooth with your init's tools."])
    return lines


def sudo_keepalive():
    """
    Ask for the password once, then keep the sudo timestamp warm in the
    background so no later step prompts again. Returns a stop callback the runner
    calls when the install is done.
    """
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
    if choices["file_manager"] != "none":
        lines.append(f"Install the {choices['file_manager']} file manager.")
    if cli_local(args.source):
        lines.append("Build the xiu CLI from this checkout.")
    if choices["greeter"] == "sddm":
        lines.append(f"Install the {choices.get('sddm_theme', 'torii')} SDDM login theme.")
    elif choices["greeter"] == "greetd":
        lines.append("Set up greetd with the tuigreet login.")
    if choices["browser_theme"]:
        lines.append("Wire the browsers into the palette (userChrome + live theme host).")
    if choices["legacy_swap"] == "clean":
        lines.append("Remove ghostty and cliphist once foot and clipvault are in.")
    if choices["fresh_configs"]:
        lines.append("Start from the repo's config defaults (your old files are still backed up).")
    if choices["grub"]:
        lines.append("Install the GRUB theme.")
    if choices["brave"]:
        lines.append("Install Brave with the matching xiu theme.")
    if choices["fish"]:
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
        if has_image:
            print(f"  wallpapers already present -> {wp}")
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
        print("  would set xdg defaults (file manager, imv, zathura) -> %s" % mimeapps)
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
    Land the wizard's file-manager choice where Super+E reads it:
    ~/.config/xiu/vars.lua, the override file vars.lua loads on every start.
    Written only when the file doesn't exist — a box that already
    personalized it keeps every line as-is. yazi is a TUI, so it rides in a
    foot window the way the editor default does. Fail-soft like every step.
    """
    dest = deploy.CONFIG_ROOT / "xiu" / "vars.lua"
    if dry:
        print("  would seed vars.lua (file manager: %s)" % choices["file_manager"])
        return True, ""
    try:
        if dest.is_file():
            print(f"  vars.lua already present -> {dest} (left untouched)")
            return True, ""
        fm = choices["file_manager"]
        body = {
            "dolphin": '    fileManager = "dolphin",  -- chosen in the installer',
            "thunar": '    fileManager = "thunar",  -- chosen in the installer',
            "yazi": '    fileManager = "foot -e yazi",  -- chosen in the installer; a TUI needs the terminal',
            "none": '    -- fileManager = "dolphin",  -- uncomment to set yours',
        }.get(fm, '    -- fileManager = "dolphin",')
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(VARS_TEMPLATE % {"body": body})
        print(f"  seeded vars.lua -> {dest}")
        return True, ""
    except OSError as exc:
        return False, f"{exc}: seed vars.lua"


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
            return True, ""
        tail = out.strip().splitlines()
        return False, "ya pkg add: " + (tail[-1] if tail else "failed")
    print("  yazi plugins installed (%d)" % len(YAZI_PLUGINS))
    return True, ""


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
    engine = Path(config_root) / "hypr" / "scripts" / "xiu-update.py"
    if not head or not engine.exists():
        return True, ""
    try:
        result = subprocess.run(
            [sys.executable, str(engine), "baseline", "--sha", head,
             "--config-root", str(config_root)],
            capture_output=True, text=True)
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
    steps.append(("log back in", "fish and the input group need a fresh session"))
    if info["init"] != "systemd" and do_pkgs:
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
                   "(arch, debian, fedora or suse), so no packages will be installed.")
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

    plan = _build_plan(manifest, info, choices, cli_local(args.source))

    summary = _summary_lines(info, choices, plan, args, do_pkgs)
    if args.quickstart:
        tui.info(summary)
    else:
        try:
            if not tui.confirm("Ready", summary):
                tui.outro("Cancelled")
                return 0
        except RuntimeError:
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
                        ["sudo", "systemctl", "enable", "--now", "NetworkManager.service"], dry)
                    record(ok, detail, "Enable NetworkManager",
                           "Enable NetworkManager.service yourself.")
                ok, detail = _run(
                    ["sudo", "systemctl", "enable", "--now", "bluetooth.service"], dry)
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

        # j. fish as the login shell, kept even with --no-deps. Never chsh onto a
        #    binary that is not there: root's chsh skips the shell validation, so
        #    a missing fish would land in /etc/passwd and break every login.
        if choices["fish"]:
            fishbin = shutil.which("fish")
            if fishbin:
                # chsh prompts for the login password through PAM, which a piped
                # `curl | bash` run has no terminal for, so it always failed. Set
                # it as root instead; the sudo timestamp is already warm.
                ok, detail = _run(["sudo", "chsh", "-s", fishbin, getpass.getuser()], dry)
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

        # l3. yazi plugins, through yazi's own package manager (no-op when
        #     yazi isn't installed — the group is optional).
        ok, detail = install_yazi_plugins(dry)
        record(ok, detail, "Install yazi plugins",
               "Run: ya pkg add <plugins> (see installer/xiu_install.py YAZI_PLUGINS).")

        ok, detail = seed_wallpapers(dry)
        record(ok, detail, "Seed wallpapers",
               "Copy any image into ~/Pictures/xiu/wallpapers yourself.")

        # m. login screen.
        if choices["greeter"] == "sddm":
            theme = choices.get("sddm_theme", "torii")
            if theme not in ("torii", "washi"):
                theme = "torii"
            sddm_installer = os.path.join(args.source, "sddm", "themes", theme, "install.sh")
            if os.path.isfile(sddm_installer):
                ok, detail = _run(["sh", sddm_installer], dry)
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
        #     spicetify gets pointed at the xiu theme and applied once, and
        #     vesktop's themes dir is created so the palette pipeline has
        #     somewhere to drop the xiu CSS (vesktop normally creates it on
        #     first run, which may be after the first wallpaper change).
        if not dry:
            if "spicetify-cli" in choices["optional_ids"] and shutil.which("spicetify"):
                ok, detail = _run(["spicetify", "config", "current_theme", "xiu",
                                   "color_scheme", "xiu"], dry)
                record(ok, detail, "Select spicetify theme",
                       "Run: spicetify config current_theme xiu color_scheme xiu")
                ok, detail = _run(["spicetify", "backup", "apply"], dry)
                record(ok, detail, "Apply spicetify theme",
                       "Run: spicetify backup apply")
            if "vesktop" in choices["optional_ids"]:
                try:
                    (Path.home() / ".config" / "vesktop" / "themes").mkdir(
                        parents=True, exist_ok=True)
                except OSError:
                    pass

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
            for old_id, new_id in (("ghostty", "foot"), ("cliphist", "clipvault")):
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

    link = Path.home() / ".local" / "bin" / "ricelin"
    if link.is_symlink():
        try:
            link.unlink()
            print(f"  removed: {link}")
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
