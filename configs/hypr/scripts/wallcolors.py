#!/usr/bin/env python3
"""
Generate the rice colour set from a wallpaper and fan it out to the consumers.
One histogram pass yields both the area-dominant chromatic hue (binned by hue
family so a small vivid accent never hijacks the theme) and the mean lightness.
The mean lightness drives the pill's whole tone: a bright wallpaper makes a light
pill with dark text, a dark or OLED-black one makes a near-black pill with cream
text, so the surfaces and the text flip together for contrast across the full
range. The dominant hue tints every tier in HSL. An achromatic wallpaper drops to
a neutral grey ramp. matugen still builds the dark base16 the always-dark terminal
reads; the pill JSON carries surfaces, accent and the contrast-matched text.

On top of the wallpaper-driven mode sits the scheme layer:
  wallcolors.py <wallpaper>            regenerate (respects the scheme state)
  wallcolors.py --hue H [mode] [sat]   manual hue override (Look surface)
  wallcolors.py --preset <name|dynamic>named scheme from ~/.config/hypr/schemes/
  wallcolors.py --variant <name|auto>  matugen scheme type (tonal-spot, vibrant, ...)
  wallcolors.py --smart | --no-smart   smartScheme: colourfulness picks the variant
  wallcolors.py --list-presets         print the available preset names
  wallcolors.py --state                print the current scheme state
  wallcolors.py --preview <wallpaper>  print the dynamic JSON without writing

The state lives in a two-line file next to the wallpaper state so a wallpaper
change never clobbers a chosen preset; an explicit scheme change also flips the
pill's paletteMode flag to dynamic so the shell actually listens.
"""
import colorsys
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

CACHE = Path.home() / ".cache" / "ricelin"
STATE_HOME = Path(os.environ.get("XDG_STATE_HOME", str(Path.home() / ".local" / "state")))
SCHEME_STATE = STATE_HOME / "ricelin" / "scheme"
WALLPAPER_STATE = STATE_HOME / "ricelin-wallpaper"
FLAGS_FILE = STATE_HOME / "ricelin" / "flags.json"
SCHEMES_DIR = Path(__file__).resolve().parent.parent / "schemes"

SURF_NAMES = ["surface", "surface_container_low", "surface_container",
              "surface_container_high", "surface_container_highest", "outline_variant"]
DARK_STEPS = [0.0, 0.022, 0.038, 0.065, 0.100, 0.225]
LIGHT_STEPS = [0.0, -0.045, -0.075, -0.115, -0.160, -0.340]
TEXT_KEYS = ["cream", "bright", "subtle", "dim", "faint", "icon_dim", "tick_rest"]
DARK_TEXT = [(0.90, 0.05), (0.97, 0.03), (0.73, 0.07), (0.54, 0.06),
             (0.44, 0.05), (0.81, 0.07), (0.75, 0.08)]
LIGHT_TEXT = [(0.20, 0.18), (0.10, 0.20), (0.36, 0.14), (0.48, 0.10),
              (0.56, 0.08), (0.28, 0.12), (0.34, 0.12)]

VARIANTS = ["auto", "content", "expressive", "fidelity", "fruit-salad", "monochrome",
            "neutral", "rainbow", "tonal-spot", "vibrant"]

# How far each matugen scheme type pushes the pill's own accent saturation:
# the quiet types mute the ramp so they read calm, the loud ones let it sing.
ACCENT_MULT = {
    "monochrome": 0.45, "neutral": 0.55, "content": 0.85, "fidelity": 0.90,
    "tonal-spot": 1.0, "vibrant": 1.25, "expressive": 1.30, "rainbow": 1.35,
    "fruit-salad": 1.30,
}


def analyze(wallpaper):
    out = subprocess.run(
        ["magick", wallpaper, "-alpha", "off", "-resize", "200x200", "-colors", "48",
         "-format", "%c", "histogram:info:-"],
        capture_output=True, text=True).stdout
    buckets, total, lum, chroma = {}, 0, 0.0, 0
    for line in out.splitlines():
        m = re.search(r"\s*(\d+):\s*\([^)]*\)\s*#([0-9A-Fa-f]{6})", line)
        if not m:
            continue
        count, hex_str = int(m.group(1)), m.group(2)
        r, g, b = (int(hex_str[i:i + 2], 16) / 255 for i in (0, 2, 4))
        h, l, s = colorsys.rgb_to_hls(r, g, b)
        total += count
        lum += count * l
        if s < 0.15 or l < 0.05 or l > 0.92:
            continue
        chroma += count
        bucket = buckets.setdefault((int(h * 360) // 30) % 12, {"wsat": 0.0, "best": None})
        bucket["wsat"] += count * s
        score = count * s * (1 if 0.12 < l < 0.55 else 0.4)
        if not bucket["best"] or score > bucket["best"][0]:
            bucket["best"] = (score, h, s)
    mean_l = lum / total if total else 0.0
    if not buckets or chroma < 0.08 * total:
        return None, 0.0, mean_l
    win = max(buckets.values(), key=lambda v: v["wsat"])
    return win["best"][1], win["best"][2], mean_l


def colourfulness(wallpaper):
    """Hasler–Süsstrunk colourfulness on a 64x64 thumbnail: the spread of the
    red-green and yellow-blue opponent channels plus a weighted mean term,
    normalised to roughly 0..1. None on any failure so the caller falls back
    to the default variant."""
    try:
        out = subprocess.run(
            ["magick", wallpaper, "-alpha", "off", "-resize", "64x64", "-depth", "8", "txt:-"],
            capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    rg, yb = [], []
    for m in re.finditer(r"#([0-9A-Fa-f]{6})", out):
        r = int(m.group(1)[0:2], 16) / 255
        g = int(m.group(1)[2:4], 16) / 255
        b = int(m.group(1)[4:6], 16) / 255
        rg.append(r - g)
        yb.append(0.5 * (r + g) - b)
    if not rg:
        return None

    def stats(v):
        mean = sum(v) / len(v)
        var = sum((x - mean) ** 2 for x in v) / len(v)
        return mean, var

    mrg, vrg = stats(rg)
    myb, vyb = stats(yb)
    return math.sqrt(vrg + vyb) + 0.3 * math.sqrt(mrg ** 2 + myb ** 2)


def smart_variant(score):
    """Colourfulness picks the matugen scheme type: near-grey wallpapers get
    the neutral ramp so they never turn to mud, busy ones the content scheme,
    and everything in between the default tonal spot."""
    if score is None:
        return "tonal-spot"
    if score < 0.06:
        return "neutral"
    if score < 0.13:
        return "content"
    return "tonal-spot"


def matugen(source_hex, variant):
    argv = ["matugen", "color", "hex", source_hex, "-m", "dark", "-j", "hex"]
    if variant and variant not in ("", "auto"):
        argv += ["--type", "scheme-" + variant]
    out = subprocess.run(argv, capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def tint(hue, sat, light):
    r, g, b = colorsys.hls_to_rgb(hue % 1.0, max(0.0, min(1.0, light)), max(0.0, min(1.0, sat)))
    return "#%02x%02x%02x" % (round(r * 255), round(g * 255), round(b * 255))


def lerp(x, x0, x1, y0, y1):
    t = max(0.0, min(1.0, (x - x0) / (x1 - x0)))
    return y0 + t * (y1 - y0)


def render_fastfetch(pill):
    """
    Recolour the fastfetch readout from the same pill palette. fastfetch has no
    daemon, so writing the rendered config is enough, the next run picks it up.
    The accent drives the keys and the torii, the surface ramp the lantern body,
    and a dim text tone the section rules, so it tracks the wallpaper like the
    pill and terminal do.
    """
    ff = Path.home() / ".config" / "fastfetch"
    tmpl = ff / "config.jsonc.in"
    if not tmpl.is_file():
        print("wallcolors: config.jsonc.in missing in ~/.config/fastfetch, skipping "
              "fastfetch recolour (apply the Ricelin update or re-run the installer)",
              file=sys.stderr)
        return
    seq = lambda h: "%d;%d;%d" % tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))
    repl = {
        "__LANTERN__": str(ff / "lantern.txt"),
        "__KEYS__": seq(pill["primary"]),
        "__SEP__": seq(pill["dim"]),
        "__LOGO1__": seq(pill["primary"]),
        "__LOGO2__": seq(pill["on_primary_container"]),
        "__LOGO3__": seq(pill["surface_container"]),
        "__LOGO4__": seq(pill["surface_container_high"]),
        "__LOGO5__": seq(pill["subtle"]),
        "__LOGO6__": seq(pill["outline"]),
        "__LOGO7__": seq(pill["bright"]),
    }
    out = tmpl.read_text()
    for key, val in repl.items():
        out = out.replace(key, val)
    (ff / "config.jsonc").write_text(out)


def load_scheme():
    """The sticky scheme state: preset (or dynamic), matugen variant (or auto),
    smartScheme on/off. Defaults keep the pre-scheme-layer behavior."""
    preset, variant, smart = "dynamic", "auto", True
    try:
        for line in SCHEME_STATE.read_text().splitlines():
            key, _, value = line.partition(" ")
            if key == "preset":
                preset = value.strip() or "dynamic"
            elif key == "variant":
                variant = value.strip() or "auto"
            elif key == "smart":
                smart = value.strip() != "off"
    except OSError:
        pass
    return preset, variant, smart


def save_scheme(preset, variant, smart):
    SCHEME_STATE.parent.mkdir(parents=True, exist_ok=True)
    SCHEME_STATE.write_text("preset %s\nvariant %s\nsmart %s\n"
                            % (preset, variant, "on" if smart else "off"))


def list_presets():
    return sorted(p.stem for p in SCHEMES_DIR.glob("*.json"))


def preset_tokens(name):
    """The full pill token set for a named preset, or None when unknown."""
    path = SCHEMES_DIR / ("%s.json" % name)
    if not path.is_file():
        return None
    try:
        data = json.loads(path.read_text())
    except (OSError, ValueError):
        return None
    if "primary" not in data or "cream" not in data or "surface" not in data:
        return None
    return data


def set_palette_mode_dynamic():
    """An explicit scheme change implies using it: flip the pill's palette mode
    flag so Theme listens to the generated colors.json."""
    try:
        flags = json.loads(FLAGS_FILE.read_text()) if FLAGS_FILE.is_file() else {}
    except (OSError, ValueError):
        return
    if flags.get("paletteMode") != "dynamic":
        flags["paletteMode"] = "dynamic"
        FLAGS_FILE.parent.mkdir(parents=True, exist_ok=True)
        FLAGS_FILE.write_text(json.dumps(flags, indent=2) + "\n")


def current_wallpaper():
    try:
        return WALLPAPER_STATE.read_text().strip()
    except OSError:
        return ""


def generate_dynamic(wallpaper, variant, smart):
    """The wallpaper-driven path: histogram analysis into the HSL pill palette,
    with the resolved matugen variant riding along."""
    hue, sat, mean_l = analyze(wallpaper)
    chromatic = hue is not None
    if not chromatic:
        hue, sat = 0.09, 0.0

    if variant == "auto":
        variant = smart_variant(colourfulness(wallpaper)) if smart else "tonal-spot"

    light = mean_l >= 0.40
    surf_sat = min(sat, 0.26) if light else min(max(sat, 0.30 if chromatic else 0.0), 0.45)
    acc_sat = (min(sat + 0.18, 0.85) if light else min(max(sat, 0.30) + 0.12, 0.82)) if chromatic else 0.05
    acc_sat = min(0.95, acc_sat * ACCENT_MULT.get(variant, 1.0))
    if light:
        base = lerp(mean_l, 0.40, 0.66, 0.80, 0.93)
        steps, text, acc_l, deep_l, glow_l = LIGHT_STEPS, LIGHT_TEXT, 0.42, 0.30, 0.55
    else:
        base = lerp(mean_l, 0.0, 0.40, 0.045, 0.20)
        steps, text, acc_l, deep_l, glow_l = DARK_STEPS, DARK_TEXT, 0.70, 0.34, 0.86

    pill = {name: tint(hue, surf_sat, base + step) for name, step in zip(SURF_NAMES, steps)}
    pill["primary"] = tint(hue, acc_sat, acc_l)
    pill["primary_container"] = tint(hue, min(acc_sat + 0.08, 0.9), deep_l)
    pill["on_primary_container"] = tint(hue, min(acc_sat, 0.45), glow_l)
    pill["outline"] = tint(hue, surf_sat, base + (-0.35 if light else 0.35))
    for key, (lit, st) in zip(TEXT_KEYS, text):
        pill[key] = tint(hue, st, lit)

    seed = tint(hue, sat, 0.45) if chromatic else "#787878"
    return pill, seed, variant


def generate_manual(hue, mode, sat, variant):
    """The Look surface's manual hue override: fixed tone, full ramp."""
    sat = max(0.0, min(1.0, sat))
    mean_l = 0.85 if mode == "light" else 0.12
    chromatic = sat > 0.02
    if not chromatic:
        hue = 0.09
    if variant == "auto":
        variant = "tonal-spot"

    light = mean_l >= 0.40
    surf_sat = min(sat, 0.26) if light else min(max(sat, 0.30 if chromatic else 0.0), 0.45)
    acc_sat = (min(sat + 0.18, 0.85) if light else min(max(sat, 0.30) + 0.12, 0.82)) if chromatic else 0.05
    acc_sat = min(0.95, acc_sat * ACCENT_MULT.get(variant, 1.0))
    if light:
        base = lerp(mean_l, 0.40, 0.66, 0.80, 0.93)
        steps, text, acc_l, deep_l, glow_l = LIGHT_STEPS, LIGHT_TEXT, 0.42, 0.30, 0.55
    else:
        base = lerp(mean_l, 0.0, 0.40, 0.045, 0.20)
        steps, text, acc_l, deep_l, glow_l = DARK_STEPS, DARK_TEXT, 0.70, 0.34, 0.86

    pill = {name: tint(hue, surf_sat, base + step) for name, step in zip(SURF_NAMES, steps)}
    pill["primary"] = tint(hue, acc_sat, acc_l)
    pill["primary_container"] = tint(hue, min(acc_sat + 0.08, 0.9), deep_l)
    pill["on_primary_container"] = tint(hue, min(acc_sat, 0.45), glow_l)
    pill["outline"] = tint(hue, surf_sat, base + (-0.35 if light else 0.35))
    for key, (lit, st) in zip(TEXT_KEYS, text):
        pill[key] = tint(hue, st, lit)

    seed = tint(hue, sat, 0.45) if chromatic else "#787878"
    return pill, seed, variant


def render_foot(b):
    """foot reads its palette from an include, so writing the file is enough:
    every terminal opened afterwards opens in the current scheme. Modern foot
    splits the palette into [colors-dark]/[colors-light]; the pipeline always
    drives the dark theme, which is also foot's default."""
    foot = Path.home() / ".config" / "foot"
    foot.mkdir(parents=True, exist_ok=True)
    lines = ["[colors-dark]"]
    for i in range(8):
        lines.append("regular%d=%s" % (i, b["base%02x" % i].lstrip("#")))
    for i in range(8):
        lines.append("bright%d=%s" % (i, b["base%02x" % (i + 8)].lstrip("#")))
    (foot / "colors.ini").write_text("\n".join(lines) + "\n")


def _tool_dir(name):
    """Config dir of an optional tool, or None when it is not in play; every
    TUI render is gated on it so the pipeline never litters configs for apps
    that were never installed."""
    path = Path.home() / ".config" / name
    return path if path.is_dir() else None


def _reload(binary):
    subprocess.run(["killall", "-USR2", binary], stderr=subprocess.DEVNULL)


def render_btop(pill, b):
    d = _tool_dir("btop")
    if d is None:
        return
    (d / "themes").mkdir(exist_ok=True)
    keys = {
        "main_bg": "",
        "main_fg": pill["cream"],
        "title": pill["bright"],
        "hi_fg": pill["primary"],
        "selected_bg": pill["surface_container_high"],
        "selected_fg": pill["bright"],
        "inactive_fg": pill["faint"],
        "graph_text": pill["subtle"],
        "meter_bg": pill["outline_variant"],
        "proc_misc": pill["subtle"],
        "cpu_box": b["base0c"],
        "mem_box": b["base0b"],
        "net_box": b["base0d"],
        "proc_box": b["base0e"],
        "div_line": pill["outline_variant"],
        "temp_start": b["base0b"], "temp_mid": b["base0a"], "temp_end": b["base08"],
        "cpu_start": b["base0e"], "cpu_mid": b["base0c"], "cpu_end": pill["primary"],
        "free_start": b["base0d"], "free_mid": b["base0e"], "free_end": pill["primary"],
        "cached_start": b["base0c"], "cached_mid": b["base0e"], "cached_end": b["base0d"],
        "available_start": b["base0a"], "available_mid": b["base09"], "available_end": b["base08"],
        "used_start": b["base0b"], "used_mid": b["base0e"], "used_end": b["base0c"],
        "download_start": b["base0a"], "download_mid": b["base09"], "download_end": b["base08"],
        "upload_start": b["base0b"], "upload_mid": b["base0e"], "upload_end": b["base0c"],
        "process_start": b["base0c"], "process_mid": b["base0d"], "process_end": pill["primary"],
    }
    lines = ["# Written by wallcolors.py on every palette change."]
    for key, value in keys.items():
        lines.append('theme[%s]="%s"' % (key, value))
    (d / "themes" / "xiu.theme").write_text("\n".join(lines) + "\n")
    _reload("btop")


def render_htop(pill, b):
    d = _tool_dir("htop")
    if d is None:
        return
    lines = [
        "fields=0 48 17 18 38 39 40 2 46 47 49 1",
        "sort_key=46",
        "sort_direction=-1",
        "tree_sort_key=0",
        "tree_sort_direction=1",
        "hide_kernel_threads=1",
        "hide_userland_threads=0",
        "shadow_other_users=0",
        "show_thread_names=0",
        "show_program_path=1",
        "highlight_base_name=0",
        "highlight_deleted_exe=1",
        "highlight_megabytes=1",
        "highlight_threads=1",
        "highlight_changes=0",
        "highlight_changes_delay_secs=5",
        "find_comm_in_cmdline=1",
        "strip_exe_from_cmdline=1",
        "show_merged_command=0",
        "tree_view=0",
        "tree_view_always_by_pid=0",
        "all_branches_collapsed=0",
        "header_margin=1",
        "detailed_cpu_time=0",
        "cpu_count_from_one=0",
        "show_cpu_usage=1",
        "show_cpu_frequency=0",
        "show_cpu_temperature=0",
        "degree_fahrenheit=0",
        "update_process_names=0",
        "account_guest_in_cpu_meter=0",
        "color_scheme=6",
        "color_background=%s" % pill["surface"],
        "color_text=%s" % pill["cream"],
        "color_highlight=%s" % pill["primary"],
        "color_selected=%s" % pill["surface_container_high"],
        "color_cpu_low=%s" % b["base0b"],
        "color_cpu_med=%s" % b["base0a"],
        "color_cpu_high=%s" % b["base08"],
        "color_mem_used=%s" % b["base0c"],
        "color_mem_buffers=%s" % b["base0e"],
        "color_mem_cache=%s" % b["base0d"],
        "color_mem_available=%s" % b["base0b"],
        "color_process_normal=%s" % pill["cream"],
        "color_process_running=%s" % b["base0b"],
        "color_process_sleeping=%s" % pill["dim"],
    ]
    (d / "htoprc").write_text("\n".join(lines) + "\n")
    _reload("htop")


def render_nvtop(pill, b):
    d = _tool_dir("nvtop")
    if d is None:
        return
    keys = {
        "background": pill["surface"],
        "selected_bg": pill["surface_container_high"],
        "header_bg": pill["surface_container_highest"],
        "text": pill["cream"],
        "selected_text": pill["primary"],
        "header_text": pill["bright"],
        "inactive_text": pill["faint"],
        "gpu_util_low": b["base0b"], "gpu_util_med": b["base0a"], "gpu_util_high": b["base08"],
        "memory_low": b["base0b"], "memory_med": b["base0e"], "memory_high": b["base0c"],
        "temp_cool": b["base0b"], "temp_warm": b["base0a"], "temp_hot": b["base08"],
        "power_low": b["base0b"], "power_med": b["base0a"], "power_high": b["base08"],
        "process_normal": pill["cream"],
        "process_highlight": pill["primary"],
        "process_killed": b["base08"],
        "border": pill["outline_variant"],
        "separator": pill["outline_variant"],
        "chart_line": pill["subtle"],
        "chart_fill": pill["surface_container"],
        "status_ok": b["base0b"], "status_warning": b["base0a"],
        "status_error": b["base08"], "status_info": b["base0c"],
    }
    lines = ["# Written by wallcolors.py on every palette change."]
    for key, value in keys.items():
        lines.append("%s = %s" % (key, value.lstrip("#")))
    (d / "nvtop.colors").write_text("\n".join(lines) + "\n")


def render_cava(pill, b):
    d = _tool_dir("cava")
    if d is None:
        return
    gradient = [b["base0b"], b["base0e"], b["base0c"], pill["primary"],
                b["base0d"], b["base0a"], b["base09"], b["base08"]]
    lines = [
        "# Written by wallcolors.py on every palette change.",
        "[general]",
        "framerate = 60",
        "",
        "[input]",
        "method = pulse",
        "source = auto",
        "",
        "[output]",
        "method = ncurses",
        "style = stereo",
        "",
        "[color]",
        "background = default",
        "foreground = %s" % pill["primary"],
        "gradient = 1",
        "gradient_count = 8",
    ]
    for i, color in enumerate(gradient, 1):
        lines.append("gradient_color_%d = '%s'" % (i, color))
    lines += [
        "",
        "[smoothing]",
        "noise_reduction = 85",
        "monstercat = 1",
    ]
    (d / "config").write_text("\n".join(lines) + "\n")
    _reload("cava")


def render_micro(pill, b):
    d = _tool_dir("micro")
    if d is None:
        return
    (d / "colorschemes").mkdir(exist_ok=True)
    links = [
        ("default", pill["cream"], None),
        ("cursor", pill["primary"], None),
        ("line-number", pill["faint"], None),
        ("current-line-number", pill["subtle"], None),
        ("statusline", pill["cream"], pill["surface_container_high"]),
        ("statusline.active", pill["bright"], pill["surface_container_high"]),
        ("comment", pill["dim"], None),
        ("identifier", pill["cream"], None),
        ("identifier.variable", pill["bright"], None),
        ("identifier.function", pill["primary"], None),
        ("constant", b["base0a"], None),
        ("constant.string", b["base0b"], None),
        ("constant.number", b["base0a"], None),
        ("keyword", pill["primary"], None),
        ("keyword.operator", pill["subtle"], None),
        ("type", b["base0c"], None),
        ("variable", pill["cream"], None),
        ("symbol", pill["subtle"], None),
        ("error", b["base08"], None),
        ("warning", b["base0a"], None),
        ("diff Added", b["base0b"], None),
        ("diff Removed", b["base08"], None),
    ]
    lines = ["# Written by wallcolors.py on every palette change."]
    for group, fg, bg in links:
        lines.append('color-link %s "%s%s"' % (group, fg, ("," + bg) if bg else ""))
    (d / "colorschemes" / "xiu.micro").write_text("\n".join(lines) + "\n")


def render_helix(pill, b):
    d = _tool_dir("helix")
    if d is None:
        return
    (d / "themes").mkdir(exist_ok=True)
    def fg(color):
        return '"%s"' % color
    p = pill
    lines = [
        "# Written by wallcolors.py on every palette change.",
        "# ui.background stays empty so the terminal's transparency shows through.",
        '"ui.background" = {}',
        '"ui.text" = %s' % fg(p["cream"]),
        '"ui.text.info" = %s' % fg(p["subtle"]),
        '"ui.selection" = { bg = "%s" }' % p["surface_container_high"],
        '"ui.selection.primary" = { bg = "%s" }' % p["surface_container"],
        '"ui.cursorline" = { bg = "%s" }' % p["surface_container_low"],
        '"ui.cursorline.primary" = { bg = "%s" }' % p["surface_container"],
        '"ui.linenr" = %s' % fg(p["faint"]),
        '"ui.linenr.selected" = %s' % fg(p["subtle"]),
        '"ui.statusline" = { fg = "%s", bg = "%s" }' % (p["cream"], p["surface_container_high"]),
        '"ui.statusline.inactive" = { fg = "%s", bg = "%s" }' % (p["dim"], p["surface_container_low"]),
        '"ui.statusline.normal" = { fg = "%s", bg = "%s" }' % (p["on_primary_container"], p["primary_container"]),
        '"ui.statusline.insert" = { fg = "%s", bg = "%s" }' % (p["bright"], p["primary"]),
        '"ui.statusline.select" = { fg = "%s", bg = "%s" }' % (p["bright"], p["primary_container"]),
        '"ui.popup" = { bg = "%s" }' % p["surface_container"],
        '"ui.popup.info" = { bg = "%s" }' % p["surface_container_high"],
        '"ui.menu" = { bg = "%s" }' % p["surface_container"],
        '"ui.menu.selected" = { fg = "%s", bg = "%s" }' % (p["bright"], p["surface_container_high"]),
        '"ui.virtual" = %s' % fg(p["outline_variant"]),
        '"ui.virtual.whitespace" = %s' % fg(p["outline_variant"]),
        '"ui.virtual.indent" = { fg = "%s" }' % p["surface_container_high"],
        '"ui.bufferline" = { fg = "%s", bg = "%s" }' % (p["dim"], p["surface_container_low"]),
        '"ui.bufferline.active" = { fg = "%s", bg = "%s" }' % (p["cream"], p["surface_container"]),
        '"comment" = %s' % fg(p["dim"]),
        '"comment.block" = %s' % fg(p["dim"]),
        '"keyword" = %s' % fg(p["primary"]),
        '"keyword.function" = %s' % fg(p["primary"]),
        '"function" = %s' % fg(b["base0c"]),
        '"function.builtin" = %s' % fg(b["base0c"]),
        '"string" = %s' % fg(b["base0b"]),
        '"constant" = %s' % fg(b["base0a"]),
        '"constant.numeric" = %s' % fg(b["base0a"]),
        '"constant.character" = %s' % fg(b["base0e"]),
        '"type" = %s' % fg(b["base0c"]),
        '"type.builtin" = %s' % fg(b["base0c"]),
        '"variable" = %s' % fg(p["cream"]),
        '"variable.other.member" = %s' % fg(p["bright"]),
        '"variable.parameter" = %s' % fg(p["subtle"]),
        '"label" = %s' % fg(p["primary"]),
        '"operator" = %s' % fg(p["subtle"]),
        '"punctuation" = %s' % fg(p["subtle"]),
        '"punctuation.bracket" = %s' % fg(p["faint"]),
        '"attribute" = %s' % fg(b["base0d"]),
        '"tag" = %s' % fg(b["base0d"]),
        '"error" = %s' % fg(b["base08"]),
        '"warning" = %s' % fg(b["base0a"]),
        '"info" = %s' % fg(b["base0c"]),
        '"hint" = %s' % fg(p["subtle"]),
        '"diff.plus" = %s' % fg(b["base0b"]),
        '"diff.minus" = %s' % fg(b["base08"]),
        '"diff.delta" = %s' % fg(b["base0c"]),
    ]
    (d / "themes" / "xiu.toml").write_text("\n".join(lines) + "\n")


def render_bottom(pill, b):
    """bottom (btm) has no theme-file indirection, so the [styles] sections of
    bottom.toml are rewritten in place and every other section — layout, flags,
    rate — is carried through untouched."""
    d = _tool_dir("bottom")
    if d is None:
        return
    p = pill
    fresh = "\n".join([
        "# [styles] kept fresh by wallcolors.py on every palette change.",
        "[styles.cpu]",
        'all_entry_colour = "%s"' % p["primary"],
        'avg_entry_colour = "%s"' % p["on_primary_container"],
        'cpu_core_colours = ["%s", "%s", "%s", "%s", "%s", "%s"]'
        % (p["primary"], p["on_primary_container"], p["subtle"], p["bright"], p["dim"], p["faint"]),
        "",
        "[styles.temp_graph]",
        'temp_graph_colour_styles = ["%s", "%s", "%s"]'
        % (p["on_primary_container"], p["subtle"], p["primary"]),
        "",
        "[styles.memory]",
        'ram_colour = "%s"' % b["base0d"],
        'cache_colour = "%s"' % b["base0c"],
        'swap_colour = "%s"' % b["base0e"],
        'arc_colour = "%s"' % b["base0a"],
        'gpu_colours = ["%s", "%s", "%s", "%s", "%s", "%s"]'
        % (p["primary"], p["subtle"], b["base0c"], b["base0b"], p["dim"], b["base0e"]),
        "",
        "[styles.network]",
        'rx_colour = "%s"' % b["base0d"],
        'tx_colour = "%s"' % b["base0b"],
        'rx_total_colour = "%s"' % b["base0c"],
        'tx_total_colour = "%s"' % b["base0e"],
        "",
        "[styles.battery]",
        'high_battery_colour = "%s"' % b["base0b"],
        'medium_battery_colour = "%s"' % b["base0a"],
        'low_battery_colour = "%s"' % b["base08"],
        "",
        "[styles.tables]",
        'headers = {colour = "%s", bold = true}' % p["bright"],
        "",
        "[styles.graphs]",
        'graph_colour = "%s"' % p["outline_variant"],
        'legend_text = {colour = "%s"}' % p["dim"],
        "",
        "[styles.widgets]",
        'border_colour = "%s"' % p["outline_variant"],
        'selected_border_colour = "%s"' % p["primary"],
        'widget_title = {colour = "%s"}' % p["subtle"],
        'text = {colour = "%s"}' % p["cream"],
        'selected_text = {colour = "%s", bg_colour = "%s"}'
        % (p["bright"], p["surface_container_high"]),
        'disabled_text = {colour = "%s"}' % p["faint"],
    ]) + "\n"
    cfg = d / "bottom.toml"
    if cfg.is_file():
        kept, inside = [], False
        for line in cfg.read_text().splitlines():
            if line.startswith("[styles"):
                inside = True
                continue
            if inside and line.startswith("["):
                inside = False
            if not inside:
                kept.append(line)
        base = "\n".join(kept).rstrip("\n")
        body = (base + "\n\n" if base else "") + fresh
    else:
        body = fresh
    cfg.write_text(body)


def render_yazi(pill, b):
    """yazi: theme.toml is the palette surface (mgr, tabs, mode, filetype) and
    is regenerated whole, like htoprc; yazi.toml (keys, openers) is the user's
    and is never touched."""
    d = _tool_dir("yazi")
    if d is None:
        return
    p = pill
    lines = [
        "# Written by wallcolors.py on every palette change.",
        "[mgr]",
        'cwd = { fg = "%s" }' % p["cream"],
        'border_style = { fg = "%s" }' % p["outline_variant"],
        'find_keyword = { fg = "%s", bold = true }' % p["primary"],
        'find_position = { fg = "%s", bg = "%s" }'
        % (p["bright"], p["surface_container_high"]),
        'marker_selected = { fg = "%s", bold = true }' % p["primary"],
        'marker_copied = { fg = "%s" }' % b["base0b"],
        'marker_cut = { fg = "%s" }' % b["base08"],
        'marker_marked = { fg = "%s" }' % b["base0e"],
        'symlink_target = { fg = "%s" }' % b["base0d"],
        "",
        "[tabs]",
        'active = { fg = "%s", bg = "%s", bold = true }'
        % (p["bright"], p["surface_container_high"]),
        'inactive = { fg = "%s" }' % p["dim"],
        'sep_inner = { fg = "%s" }' % p["outline_variant"],
        'sep_outer = { fg = "%s" }' % p["outline_variant"],
        "",
        "[mode]",
        'normal_main = { fg = "%s", bg = "%s", bold = true }'
        % (p["on_primary_container"], p["primary"]),
        'normal_alt = { fg = "%s", bg = "%s" }' % (p["cream"], p["surface_container"]),
        'select_main = { fg = "%s", bg = "%s", bold = true }' % (p["bright"], b["base0d"]),
        'select_alt = { fg = "%s", bg = "%s" }' % (p["cream"], p["surface_container_high"]),
        'unset_main = { fg = "%s", bg = "%s", bold = true }' % (p["cream"], b["base08"]),
        'unset_alt = { fg = "%s", bg = "%s" }' % (p["cream"], p["surface_container_high"]),
        "",
        "[filetype]",
        "rules = [",
        '  { mime = "image/*", fg = "%s" },' % b["base0a"],
        '  { mime = "{audio,video}/*", fg = "%s" },' % b["base0d"],
        '  { mime = "inode/empty", fg = "%s" },' % p["dim"],
        '  { url = "*/", fg = "%s", bold = true },' % b["base0d"],
        '  { url = "*", fg = "%s" },' % p["cream"],
        "]",
    ]
    (d / "theme.toml").write_text("\n".join(lines) + "\n")


def render_spicetify(pill, b):
    """Spotify through spicetify: the xiu theme's color.ini is regenerated on
    every palette change; `spicetify refresh` pushes it into the client, run
    only when the theme is the configured current one (opt-in through the
    installer) so a vanilla spicetify setup is never touched."""
    d = _tool_dir("spicetify")
    if d is None:
        return
    theme = d / "Themes" / "xiu"
    if not theme.is_dir():
        return
    p = pill
    h = lambda c: c.lstrip("#").upper()
    lines = [
        "; Xiu Spotify theme — colors kept fresh by wallcolors.py on every",
        "; palette change. Selected with: spicetify config current_theme xiu",
        "[xiu]",
        "text               = %s" % h(p["bright"]),
        "subtext            = %s" % h(p["subtle"]),
        "main               = %s" % h(p["surface"]),
        "main-elevated      = %s" % h(p["surface_container_high"]),
        "highlight          = %s" % h(p["surface_container"]),
        "highlight-elevated = %s" % h(p["surface_container_highest"]),
        "sidebar            = %s" % h(p["surface_container"]),
        "player             = %s" % h(p["surface_container"]),
        "card               = %s" % h(p["primary_container"]),
        "shadow             = %s" % h(p["surface_container"]),
        "selected-row       = %s" % h(p["bright"]),
        "button             = %s" % h(p["primary"]),
        "button-active      = %s" % h(p["primary_container"]),
        "button-disabled    = %s" % h(p["outline_variant"]),
        "tab-active         = %s" % h(p["surface_container_high"]),
        "notification       = %s" % h(p["primary"]),
        "notification-error = %s" % h(b["base08"]),
        "misc               = %s" % h(p["subtle"]),
    ]
    theme.mkdir(parents=True, exist_ok=True)
    (theme / "color.ini").write_text("\n".join(lines) + "\n")
    prefs = d / "config-xpui.ini"
    if prefs.is_file() and "current_theme = xiu" in prefs.read_text():
        subprocess.run(["spicetify", "refresh"], stderr=subprocess.DEVNULL)


def render_discord(pill):
    """The Vencord-family clients (vesktop, vencord, equicord) take plain CSS
    theme files; xiu rides the pill through Discord's own CSS variables. Only
    written where a client's themes dir already exists, so nothing is littered
    for clients not in use. Applied on the client's next launch."""
    p = pill
    v = lambda c, a="ff": c + a
    css = "\n".join([
        "/**",
        " * @name xiu",
        " * @author yrpcaro",
        " * @description The xiu palette, regenerated by wallcolors.py on every wallpaper change.",
        " * @version 1.0.0",
        " */",
        ":root {",
        "    --background-primary: %s;" % p["surface"],
        "    --background-secondary: %s;" % p["surface_container"],
        "    --background-secondary-alt: %s;" % p["surface_container_high"],
        "    --background-tertiary: %s;" % p["surface_container_low"],
        "    --background-floating: %s;" % p["surface_container_highest"],
        "    --channeltextarea-background: %s;" % p["surface_container"],
        "    --background-modifier-hover: %s;" % v(p["surface_container_high"], "26"),
        "    --background-modifier-active: %s;" % v(p["surface_container_high"], "40"),
        "    --background-modifier-selected: %s;" % v(p["primary"], "26"),
        "    --background-modifier-accent: %s;" % p["outline_variant"],
        "    --text-normal: %s;" % p["cream"],
        "    --text-muted: %s;" % p["subtle"],
        "    --text-link: %s;" % p["primary"],
        "    --header-primary: %s;" % p["bright"],
        "    --header-secondary: %s;" % p["subtle"],
        "    --interactive-normal: %s;" % p["subtle"],
        "    --interactive-hover: %s;" % p["cream"],
        "    --interactive-active: %s;" % p["bright"],
        "    --interactive-muted: %s;" % p["faint"],
        "    --channels-default: %s;" % p["subtle"],
        "    --brand-experiment: %s;" % p["primary"],
        "    --brand-experiment-560: %s;" % p["primary_container"],
        "    --button-secondary-background: %s;" % p["surface_container"],
        "    --scrollbar-auto-thumb: %s;" % p["outline_variant"],
        "    --scrollbar-auto-track: transparent;",
        "}",
    ]) + "\n"
    for client in ("vesktop", "vencord", "equicord"):
        tdir = Path.home() / ".config" / client / "themes"
        if tdir.is_dir():
            (tdir / "xiu.css").write_text(css)


def render_telegram(pill):
    """Telegram Desktop themes are .attheme files — one `key: #AARRGGBB` line
    per palette slot. There is no live-reload hook, so the theme is kept fresh
    at a stable path; import it once (Settings > Chat settings > ... > Import
    custom theme) and re-import whenever you want to pull a new palette."""
    d = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))) / "xiu"
    d.mkdir(parents=True, exist_ok=True)
    p = pill

    def argb(c, a="ff"):
        return "#" + a + c.lstrip("#")

    keys = [
        ("windowBg", argb(p["surface"])),
        ("windowFg", argb(p["cream"])),
        ("windowBgOver", argb(p["surface_container"])),
        ("windowBgRipple", argb(p["surface_container_high"])),
        ("windowSubTextFg", argb(p["dim"])),
        ("windowBoldFg", argb(p["bright"])),
        ("windowActiveTextFg", argb(p["primary"])),
        ("titleBg", argb(p["surface_container"])),
        ("titleFg", argb(p["subtle"])),
        ("dialogsBg", argb(p["surface"])),
        ("dialogsBgOver", argb(p["surface_container"])),
        ("dialogsBgActive", argb(p["surface_container_high"])),
        ("dialogsNameFg", argb(p["cream"])),
        ("dialogsDateFg", argb(p["dim"])),
        ("dialogsTextFg", argb(p["subtle"])),
        ("dialogsUnreadBg", argb(p["primary"])),
        ("dialogsUnreadFg", argb(p["bright"])),
        ("msgInBg", argb(p["surface_container"])),
        ("msgInBgSelected", argb(p["surface_container_high"])),
        ("msgOutBg", argb(p["surface_container_high"])),
        ("msgOutBgSelected", argb(p["surface_container_highest"])),
        ("historyTextInFg", argb(p["cream"])),
        ("historyTextOutFg", argb(p["bright"])),
        ("msgInServiceFg", argb(p["subtle"])),
        ("msgOutServiceFg", argb(p["subtle"])),
        ("msgInDateFg", argb(p["dim"])),
        ("msgOutDateFg", argb(p["dim"])),
        ("boxBg", argb(p["surface_container"])),
        ("boxTitleFg", argb(p["bright"])),
        ("boxTextFg", argb(p["cream"])),
        ("menuBg", argb(p["surface_container"])),
        ("menuBgOver", argb(p["surface_container_high"])),
        ("menuIconFg", argb(p["subtle"])),
        ("menuFgDisabled", argb(p["faint"])),
        ("scrollBarBg", argb(p["outline_variant"])),
        ("activeButtonBg", argb(p["primary"])),
        ("activeButtonFg", argb(p["on_primary_container"])),
        ("lightButtonBg", argb(p["surface_container"])),
        ("lightButtonFg", argb(p["cream"])),
        ("attentionButtonFg", argb(p["primary"])),
        ("sliderBgActive", argb(p["primary"])),
        ("sliderBgInactive", argb(p["surface_container_high"])),
        ("placeholderFg", argb(p["faint"])),
        ("inputBorderFg", argb(p["outline_variant"])),
        ("tooltipBg", argb(p["surface_container_highest"])),
        ("tooltipFg", argb(p["subtle"])),
        ("radialFg", argb(p["primary"])),
    ]
    lines = [
        "// xiu Telegram theme — written by wallcolors.py on every palette change.",
        "// Import: Settings > Chat settings > (…) > Import custom theme,",
        "// then pick this file. Re-import to pull a later palette.",
        "",
    ]
    lines += ["%s: %s;" % (k, val) for k, val in keys]
    (d / "telegram-xiu.attheme").write_text("\n".join(lines) + "\n")


def render_vscode(pill):
    """VSCode and VSCodium color the workbench natively through
    workbench.colorCustomizations. The rest of settings.json is parsed and
    written back untouched; a settings.json that is not plain JSON (user
    comments) is left alone rather than mangled."""
    p = pill
    cc = {
        "editor.background": p["surface"],
        "editor.foreground": p["cream"],
        "editorCursor.foreground": p["primary"],
        "editor.lineHighlightBackground": p["surface_container"],
        "editor.selectionBackground": p["primary_container"],
        "editorGroup.border": p["outline_variant"],
        "tab.activeBackground": p["surface"],
        "tab.inactiveBackground": p["surface_container_low"],
        "tab.activeBorderTop": p["primary"],
        "sideBar.background": p["surface_container_low"],
        "sideBar.foreground": p["subtle"],
        "activityBar.background": p["surface"],
        "activityBar.foreground": p["subtle"],
        "activityBar.activeBorder": p["primary"],
        "titleBar.activeBackground": p["surface"],
        "titleBar.activeForeground": p["subtle"],
        "statusBar.background": p["surface_container"],
        "statusBar.foreground": p["subtle"],
        "terminal.background": p["surface"],
        "terminal.foreground": p["cream"],
        "input.background": p["surface_container"],
        "dropdown.background": p["surface_container"],
        "list.activeSelectionBackground": p["surface_container_high"],
        "list.hoverBackground": p["surface_container"],
        "notifications.background": p["surface_container_high"],
        "widget.border": p["outline_variant"],
        "scrollbarSlider.background": p["outline_variant"],
        "focusBorder": p["primary"],
        "badge.background": p["primary"],
        "badge.foreground": p["bright"],
        "button.background": p["primary"],
        "button.foreground": p["bright"],
    }
    for editor in ("Code", "VSCodium"):
        sdir = Path.home() / ".config" / editor / "User"
        if not sdir.is_dir():
            continue
        sfile = sdir / "settings.json"
        try:
            data = json.loads(sfile.read_text()) if sfile.is_file() else {}
            if not isinstance(data, dict):
                continue
        except ValueError:
            continue
        data["workbench.colorCustomizations"] = cc
        sfile.write_text(json.dumps(data, indent=4) + "\n")


def render_zed(pill):
    """Zed picks user themes out of ~/.config/zed/themes; the xiu one is kept
    fresh. The active selection in settings.json is only set when the user
    never chose a theme — their file is hand-written JSONC, so it is edited
    textually (never re-serialized) and an existing choice is respected."""
    z = _tool_dir("zed")
    if z is None:
        return
    p = pill
    lum = lambda c: 0.2126 * int(c[1:3], 16) + 0.7152 * int(c[3:5], 16) + 0.0722 * int(c[5:7], 16)
    a = lambda c: c + "ff"

    def syntax(c, italic=False):
        entry = {"color": a(c)}
        if italic:
            entry["font_style"] = "italic"
        return entry

    theme = {
        "$schema": "https://zed.dev/schema/themes/v0.2.0.json",
        "name": "xiu",
        "author": "yrpcaro",
        "themes": [{
            "name": "xiu",
            "appearance": "light" if lum(p["surface"]) > 128 else "dark",
            "style": {
                "background": a(p["surface"]),
                "surface.background": a(p["surface_container"]),
                "elevated_surface.background": a(p["surface_container_high"]),
                "panel.background": a(p["surface_container"]),
                "title_bar.background": a(p["surface"]),
                "status_bar.background": a(p["surface_container"]),
                "toolbar.background": a(p["surface"]),
                "tab_bar.background": a(p["surface_container_low"]),
                "tab.active_background": a(p["surface"]),
                "tab.inactive_background": a(p["surface_container_low"]),
                "border": a(p["outline_variant"]),
                "border.variant": a(p["outline"]),
                "border.focused": a(p["primary"]),
                "editor.background": a(p["surface"]),
                "editor.foreground": a(p["cream"]),
                "editor.gutter.background": a(p["surface"]),
                "editor.active_line.background": a(p["surface_container"]),
                "editor.line_number": a(p["faint"]),
                "editor.active_line_number": a(p["subtle"]),
                "editor.document_highlight.read_background": a(p["surface_container"]),
                "terminal.background": a(p["surface"]),
                "terminal.foreground": a(p["cream"]),
                "text": a(p["cream"]),
                "text.muted": a(p["subtle"]),
                "text.disabled": a(p["faint"]),
                "text.accent": a(p["primary"]),
                "element.hover": a(p["surface_container"]),
                "element.active": a(p["surface_container_high"]),
                "element.selected": a(p["surface_container_high"]),
                "ghost_element.hover": a(p["surface_container"]),
                "scrollbar.thumb.background": a(p["outline_variant"]),
                "link_text.hover": a(p["primary"]),
                "error": a(p["primary"]),
                "warning": "#e0a03bff",
                "info": a(p["subtle"]),
                "predicted": a(p["dim"]),
                "syntax": {
                    "comment": syntax(p["faint"], italic=True),
                    "string": syntax(p["on_primary_container"]),
                    "constant": syntax(p["on_primary_container"]),
                    "keyword": syntax(p["primary"]),
                    "function": syntax(p["subtle"]),
                    "variable": syntax(p["cream"]),
                    "type": syntax(p["bright"]),
                    "tag": syntax(p["primary"]),
                    "property": syntax(p["subtle"]),
                    "operator": syntax(p["dim"]),
                    "punctuation": syntax(p["dim"]),
                },
            },
        }],
    }
    (z / "themes").mkdir(exist_ok=True)
    (z / "themes" / "xiu.json").write_text(json.dumps(theme, indent=2) + "\n")

    settings = z / "settings.json"
    if settings.is_file():
        text = settings.read_text()
        if '"theme"' not in text:
            stripped = text.rstrip()
            if stripped.endswith("}"):
                head = stripped[:-1].rstrip()
                joiner = "" if head.endswith(",") else ","
                text = head + joiner + '\n  "theme": { "mode": "system", "dark": "xiu", "light": "xiu" }\n}\n'
                settings.write_text(text)


def render_browser(pill):
    """Brave/Chromium pick their toolbar color up from a managed policy.
    The payload lands in xiu's own config dir; `xiu browser` (or the
    installer) copies it into /etc, which needs root."""
    d = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))) / "xiu"
    d.mkdir(parents=True, exist_ok=True)
    (d / "browser-theme.json").write_text(
        json.dumps({"BrowserThemeColor": pill["surface"]}, indent=2) + "\n")


def _rgb(hex_str):
    """KDE color schemes want "r,g,b" decimal tuples."""
    return ",".join(str(int(hex_str[i:i + 2], 16)) for i in (1, 3, 5))


def render_gtk(pill):
    """GTK named colors: the adw-gtk3 theme (set through gsettings below)
    picks these up, so GTK apps follow the palette. Only existing gtk-3.0/
    gtk-4.0 dirs are touched."""
    css = "\n".join([
        "/* Written by wallcolors.py on every palette change. */",
        "@define-color accent_color %s;" % pill["primary"],
        "@define-color accent_bg_color %s;" % pill["primary"],
        "@define-color accent_fg_color %s;" % pill["on_primary_container"],
        "@define-color window_bg_color %s;" % pill["surface"],
        "@define-color window_fg_color %s;" % pill["cream"],
        "@define-color headerbar_bg_color %s;" % pill["surface_container"],
        "@define-color headerbar_fg_color %s;" % pill["cream"],
        "@define-color popover_bg_color %s;" % pill["surface_container_high"],
        "@define-color popover_fg_color %s;" % pill["cream"],
        "@define-color view_bg_color %s;" % pill["surface_container"],
        "@define-color view_fg_color %s;" % pill["cream"],
        "@define-color card_bg_color %s;" % pill["surface_container"],
        "@define-color card_fg_color %s;" % pill["cream"],
        "@define-color sidebar_bg_color @window_bg_color;",
        "@define-color sidebar_fg_color @window_fg_color;",
        "@define-color sidebar_border_color @window_bg_color;",
        "@define-color theme_selected_bg_color alpha(@accent_color, 0.15);",
        "@define-color theme_selected_fg_color %s;" % pill["primary"],
    ]) + "\n"
    for ver in ("gtk-3.0", "gtk-4.0"):
        d = Path.home() / ".config" / ver
        if d.is_dir():
            (d / "gtk.css").write_text(css)

    # The theme itself and the icon set live in dconf; idempotent and quiet.
    for key, value in (("color-scheme", "prefer-dark"),
                       ("gtk-theme", "adw-gtk3-dark"),
                       ("icon-theme", "Papirus-Dark")):
        subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", key, value],
                       stderr=subprocess.DEVNULL)


def render_qt(pill):
    """Qt via qtengine + Darkly: a palette-derived KDE color scheme plus the
    qtengine config that selects it. Seeded only when qtengine is installed
    (its config dir exists) and the config is only written when absent, so
    font and misc choices stay the user's."""
    d = Path.home() / ".config" / "qtengine"
    if not d.is_dir():
        return
    p = pill
    sections = [
        ("[Colors:View]", {
            "BackgroundNormal": p["surface_container"], "ForegroundNormal": p["cream"],
            "BackgroundAlternate": p["surface_container_low"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
        }),
        ("[Colors:Window]", {
            "BackgroundNormal": p["surface"], "ForegroundNormal": p["cream"],
            "BackgroundAlternate": p["surface_container"],
        }),
        ("[Colors:Button]", {
            "BackgroundNormal": p["surface_container_high"], "ForegroundNormal": p["cream"],
            "BackgroundAlternate": p["surface_container_highest"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
        }),
        ("[Colors:Selection]", {
            "BackgroundNormal": p["primary_container"], "ForegroundNormal": p["on_primary_container"],
        }),
        ("[Colors:Tooltip]", {
            "BackgroundNormal": p["surface_container_highest"], "ForegroundNormal": p["cream"],
        }),
        ("[Colors:Header]", {
            "BackgroundNormal": p["surface_container"], "ForegroundNormal": p["cream"],
        }),
        ("[General]", {"ColorScheme": "Xiu"}),
    ]
    lines = ["# Written by wallcolors.py on every palette change."]
    for header, fields in sections:
        lines.append("")
        lines.append(header)
        for key, value in fields.items():
            lines.append("%s=%s" % (key, _rgb(value) if value.startswith("#") else value))
    (d / "xiu.colors").write_text("\n".join(lines) + "\n")

    config = d / "config.json"
    if not config.is_file():
        config.write_text(json.dumps({
            "theme": {
                "colorScheme": str(d / "xiu.colors"),
                "iconTheme": "Papirus-Dark",
                "style": "Darkly",
            },
            "misc": {
                "menusHaveIcons": True,
                "singleClickActivate": False,
            },
        }, indent=4) + "\n")


def fan_out(pill, seed, variant):
    """Write the pill JSON, recolour fastfetch, and build the terminal/border
    base16 through matugen with the resolved scheme type."""
    CACHE.mkdir(parents=True, exist_ok=True)
    (CACHE / "colors.json").write_text(json.dumps(pill, indent=2) + "\n")
    render_fastfetch(pill)

    try:
        b = {k: v["dark"]["color"] for k, v in
             matugen(seed, variant)["base16"].items()}
    except (OSError, ValueError, KeyError, subprocess.SubprocessError):
        return 0

    (CACHE / "hypr-colors.lua").write_text(
        'return {\n    active = "%s",\n    inactive = "%s",\n}\n'
        % (pill["primary"], b["base01"]))

    lines = [
        f'background = {b["base00"]}',
        f'foreground = {b["base07"]}',
        f'cursor-color = {pill["primary"]}',
        f'selection-background = {b["base02"]}',
        f'selection-foreground = {b["base07"]}',
    ]
    for i in range(16):
        lines.append(f'palette = {i}={b["base%02x" % i]}')
    (CACHE / "ghostty-colors").write_text("\n".join(lines) + "\n")
    render_foot(b)
    render_btop(pill, b)
    render_htop(pill, b)
    render_nvtop(pill, b)
    render_cava(pill, b)
    render_micro(pill, b)
    render_helix(pill, b)
    render_bottom(pill, b)
    render_yazi(pill, b)
    render_spicetify(pill, b)
    render_discord(pill)
    render_telegram(pill)
    render_vscode(pill)
    render_zed(pill)
    render_browser(pill)
    render_gtk(pill)
    render_qt(pill)
    return 0


def main():
    args = sys.argv[1:]
    if len(args) == 0:
        print("usage: wallcolors.py <wallpaper> | --hue H [mode] [sat] | --preset NAME | "
              "--variant NAME | --smart | --no-smart | --list-presets | --state | "
              "--preview <wallpaper>", file=sys.stderr)
        return 1

    if args[0] == "--list-presets":
        print("\n".join(list_presets()))
        return 0
    if args[0] == "--state":
        preset, variant, smart = load_scheme()
        print("preset %s\nvariant %s\nsmart %s" % (preset, variant, "on" if smart else "off"))
        return 0
    if args[0] == "--preview":
        if len(args) < 2:
            print("wallcolors: --preview needs a wallpaper", file=sys.stderr)
            return 1
        pill, seed, variant = generate_dynamic(args[1], "auto", True)
        pill["_variant"] = variant
        pill["_seed"] = seed
        print(json.dumps(pill, indent=2))
        return 0

    preset, variant, smart = load_scheme()
    changed = False
    wallpaper = None
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--preset" and i + 1 < len(args):
            i += 1
            name = args[i]
            if name != "dynamic" and preset_tokens(name) is None:
                print("wallcolors: unknown preset '%s' (see --list-presets)" % name,
                      file=sys.stderr)
                return 1
            preset = name
            changed = True
        elif a == "--variant" and i + 1 < len(args):
            i += 1
            if args[i] not in VARIANTS:
                print("wallcolors: unknown variant '%s' (one of: %s)" % (args[i], ", ".join(VARIANTS)),
                      file=sys.stderr)
                return 1
            variant = args[i]
            changed = True
        elif a == "--smart":
            smart = True
            changed = True
        elif a == "--no-smart":
            smart = False
            changed = True
        elif a == "--hue" and i + 1 < len(args):
            # Manual override from the Look surface: fixed tone, state untouched.
            hue = (float(args[i + 1]) % 360) / 360.0
            mode = args[i + 2] if i + 2 < len(args) and args[i + 2] in ("dark", "light") else "dark"
            sat = float(args[i + 3]) if i + 3 < len(args) and re.match(r"^\d+(\.\d+)?$", args[i + 3]) else 0.5
            pill, seed, resolved = generate_manual(hue, mode, sat, variant)
            return fan_out(pill, seed, resolved)
        elif wallpaper is None:
            wallpaper = a
        else:
            print("wallcolors: unexpected argument '%s'" % a, file=sys.stderr)
            return 1
        i += 1

    if changed:
        save_scheme(preset, variant, smart)
        set_palette_mode_dynamic()

    if preset != "dynamic":
        tokens = preset_tokens(preset)
        resolved = variant if variant != "auto" else "tonal-spot"
        return fan_out(tokens, "#" + tokens["seed"], resolved)

    if wallpaper is None:
        wallpaper = current_wallpaper()
        if not wallpaper or not Path(wallpaper).is_file():
            print("wallcolors: no wallpaper to analyze (set one first)", file=sys.stderr)
            return 1
    elif not Path(wallpaper).is_file():
        return 0

    pill, seed, resolved = generate_dynamic(wallpaper, variant, smart)
    return fan_out(pill, seed, resolved)


if __name__ == "__main__":
    sys.exit(main())
