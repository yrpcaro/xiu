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
import configparser
import json
import math
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

CACHE = Path.home() / ".cache" / "ricelin"
CACHE_XIU = Path.home() / ".cache" / "xiu"
STATE_HOME = Path(os.environ.get("XDG_STATE_HOME", str(Path.home() / ".local" / "state")))
SCHEME_STATE = STATE_HOME / "ricelin" / "scheme"
WALLPAPER_STATE = STATE_HOME / "ricelin-wallpaper"
WALLPAPER_STATE_XIU = STATE_HOME / "xiu" / "wallpaper"
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
        m = re.search(r"\s*(\d+):\s*\([^)]*\)\s*#([0-9A-Fa-f]{6,16})", line)
        if not m:
            continue
        count, hex_str = int(m.group(1)), m.group(2)
        if len(hex_str) > 6:
            # Q16/HDRI builds print 16-bit (and alpha) components; keep the
            # high byte of each of the first three so #RRRRGGGGBBBB still
            # reads as #RRGGBB instead of a garbage hue.
            step = len(hex_str) // (4 if len(hex_str) % 3 else 3)
            hex_str = "".join(hex_str[i:i + 2] for i in range(0, 3 * step, step))
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
    share = chroma / total if total else 0.0
    if not buckets or chroma < 0.08 * total:
        return None, 0.0, mean_l, share
    win = max(buckets.values(), key=lambda v: v["wsat"])
    return win["best"][1], win["best"][2], mean_l, share


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


def tint_deg(h_deg, s, l):
    return tint((h_deg % 360.0) / 360.0, s, l)


def hue_sat_of(hex_color):
    """(hue in degrees, saturation) of a #rrggbb, for reading matugen's slots."""
    r, g, b = (int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5))
    h, _l, s = colorsys.rgb_to_hls(r, g, b)
    return h * 360.0, s


# ---------------------------------------------------------------------------
# The terminal's semantic layer, ported from CapsuleOS's Night Glass engine
# (a sibling Ricelin fork): fixed perceived-luminance bands so the tonal
# composition never changes with the wallpaper, per-zone chroma ceilings so
# nothing goes neon, WCAG contrast floors against the terminal's own
# background, and hue-bent statuses — red, green and yellow stay recognizable
# but lean toward the wallpaper's hue inside their family bounds, and hold a
# fixed editorial chroma so they stay colored even on grey wallpapers. It runs
# as a post-process over matugen's base16: matugen still owns the surface
# ramp, the seed hue and the scheme presets; this layer owns what the 16 ANSI
# slots actually look like, which a raw base16 dump leaves half grey (slots
# 1-6 land on matugen's ramp) and half neon (the accents at full material
# saturation).

# Value bands: every chromatic slot snaps its perceived luminance into the
# voice band (normals) or the light band (brights), both stated as contrast
# against the terminal background. Green-zone hues get an extra chroma cut —
# mid greens look brighter than they read, so they need holding down.
VOICE_CONTRAST, VOICE_WIDTH = 4.5, 0.05
LIGHT_CONTRAST, LIGHT_WIDTH = 6.0, 0.06
GREEN_ZONE, GREEN_ZONE_PENALTY = (90.0, 200.0), 0.15

# Chroma ceilings and the ramp: cool slots scale their saturation with how
# chromatic the wallpaper actually was (share of chromatic pixels), so a
# near-grey wallpaper keeps near-grey cools while the statuses stay colored.
ACC_SAT_CAP = 0.65
RAMP_LO, RAMP_HI = 0.08, 0.20

# Status families: canonical hue, the circular bounds the bent hue may live
# in, how far it may lean toward the wallpaper hue, and the fixed chroma that
# survives achromatic wallpapers. The ok family is teal/mint rather than leaf
# green (CapsuleOS's user-validated pick); it still reads as "green" next to
# the bent red and yellow.
TERMINAL_SEMANTIC = {"danger": (0.0, (345.0, 20.0)),
                     "ok": (160.0, (140.0, 170.0)),
                     "warning": (55.0, (40.0, 65.0))}
SEMANTIC_BEND = 15.0
SEMANTIC_SAT = 0.55

# WCAG floors: normals and brights against the background, bright-black gets
# the lower muted floor. There is deliberately no floor against the selection
# background: both terminals draw selection-foreground over ANSI colors, and
# matugen's light base02 would lift every slot out of its band.
ANSI_FLOOR, ANSI_FLOOR_MUTED = 4.5, 3.0
COOL_MIN_SEP, COOL_SPREAD = 30.0, 40.0


def _linearize(c8):
    c = c8 / 255.0
    return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4


def rel_luminance(hex_color):
    r, g, b = (int(hex_color[i:i + 2], 16) for i in (1, 3, 5))
    return 0.2126 * _linearize(r) + 0.7152 * _linearize(g) + 0.0722 * _linearize(b)


def contrast_ratio(hex_a, hex_b):
    la, lb = rel_luminance(hex_a), rel_luminance(hex_b)
    lighter, darker = max(la, lb), min(la, lb)
    return (lighter + 0.05) / (darker + 0.05)


def signed_arc(a_deg, b_deg):
    """Signed shortest arc a->b in (-180, 180]."""
    d = (b_deg - a_deg) % 360.0
    return d - 360.0 if d > 180.0 else d


def circ_clamp(h_deg, lo_deg, hi_deg):
    """Clamp onto the circular interval lo->hi (walking clockwise lo..hi)."""
    if (h_deg - lo_deg) % 360.0 <= (hi_deg - lo_deg) % 360.0:
        return h_deg % 360.0
    # outside: snap to the nearer endpoint by circular distance
    if abs(signed_arc(h_deg, lo_deg)) <= abs(signed_arc(h_deg, hi_deg)):
        return lo_deg % 360.0
    return hi_deg % 360.0


def band(target_contrast, base_hex, width):
    y = target_contrast * (rel_luminance(base_hex) + 0.05) - 0.05
    return (y, y + width)


def snap_to_band(hex_color, band_tuple):
    """Lift/drop the color's lightness (hue and sat held) until its perceived
    luminance lands inside the band; Y is monotone in HSL lightness, so a
    binary search finds it. Outside colors always come back inside."""
    lo, hi = band_tuple
    if lo <= rel_luminance(hex_color) <= hi:
        return hex_color
    _h, l, s = colorsys.rgb_to_hls(*(int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5)))
    target = (lo + hi) / 2
    lo_l, hi_l = 0.0, 1.0
    for _ in range(40):
        mid = (lo_l + hi_l) / 2
        if rel_luminance(tint(_h, s, mid)) < target:
            lo_l = mid
        else:
            hi_l = mid
    return tint(_h, s, hi_l)


def sat_cap(h_deg, base_cap):
    h = h_deg % 360.0
    if GREEN_ZONE[0] <= h <= GREEN_ZONE[1]:
        return max(0.05, base_cap - GREEN_ZONE_PENALTY)
    return base_cap


def chroma_ramp(share):
    if share <= RAMP_LO:
        return 0.0
    if share >= RAMP_HI:
        return 1.0
    return (share - RAMP_LO) / (RAMP_HI - RAMP_LO)


def clamp_light(hex_color, target, bg_hex):
    """Smallest lightness >= the input's whose tint meets `target` WCAG
    contrast against `bg_hex`; hue/sat are held fixed so only lightness moves,
    and the result is never darker than the input (a lift, never a floor) —
    the right direction whenever the color sits above its background, which
    in an always-dark terminal is always. Best effort: if even white falls
    short of `target`, white ships."""
    if contrast_ratio(hex_color, bg_hex) >= target:
        return hex_color
    _h, l, s = colorsys.rgb_to_hls(*(int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5)))
    if contrast_ratio(tint(_h, s, 1.0), bg_hex) < target:
        return tint(_h, s, 1.0)
    lo, hi = l, 1.0
    for _ in range(40):
        mid = (lo + hi) / 2
        if contrast_ratio(tint(_h, s, mid), bg_hex) >= target:
            hi = mid
        else:
            lo = mid
    return tint(_h, s, hi)


def bend_semantic(base_hue_deg, dominant_deg, bounds):
    d = signed_arc(base_hue_deg, dominant_deg)
    bent = base_hue_deg + max(-SEMANTIC_BEND, min(SEMANTIC_BEND, d))
    return circ_clamp(bent, *bounds)


def semantic_terminal(pill, b, seed, share=None):
    """matugen's base16 through the semantic layer: returns the 16-slot ANSI
    list and rewrites the six accent slots in `b` in place, so every TUI
    renderer (btop, yazi, helix, …) that reads them as red/yellow/green/
    cyan/blue/magenta shows exactly what the terminal shows. The grey ramp
    (base00-07) stays matugen's: it is the variant- and preset-tuned surface
    the borders and backgrounds already use.

    Statuses (ANSI 1-3) bend their canonical family hue toward the seed's hue
    inside their bounds. Cools (4-6) sit in the seed's family too — dominant
    carries the blue slot, magenta and cyan spread to its sides — walked
    clear of the statuses and of each other, with saturation capped per hue
    zone and scaled by the wallpaper's chroma share (1.0 assumed when the
    caller doesn't know — presets and manual hues are deliberate choices).
    Orange (base09) keeps matugen's hue but joins the bands and floors, since
    the gradients that use it want an orange, not a bent status."""
    bg, fg = b["base00"], b["base07"]
    dom, dom_sat = hue_sat_of(seed)
    chromatic = dom_sat > 0.02
    if share is None:
        share = 1.0 if chromatic else 0.0
    ramp = chroma_ramp(share) if chromatic else 0.0

    sem = {name: (bend_semantic(h, dom, bounds) if chromatic else h)
           for name, (h, bounds) in TERMINAL_SEMANTIC.items()}

    # Cool slots: the seed's own family, spread around it like CapsuleOS's
    # trio (dominant carries the cool slots — matugen's base16 accent slots
    # follow the material scheme's roles, which for a warm seed puts a gold
    # in the blue slot), then walked clockwise until each clears the semantic
    # hues and the cools already placed: a gold-family seed bends warning
    # onto the dominant hue, and un-walked cools would render yellow and
    # "blue" as the same color.
    def clear_walk(h, avoid):
        for _ in range(360):
            if all(abs(signed_arc(a, h)) >= COOL_MIN_SEP for a in avoid):
                return h % 360
            h = (h + 1) % 360
        return h % 360

    sems = [sem["danger"], sem["ok"], sem["warning"]]
    blue = clear_walk(dom, sems) if chromatic else dom
    magenta = clear_walk((dom - COOL_SPREAD) % 360, sems + [blue]) if chromatic else dom
    cyan = clear_walk((dom + COOL_SPREAD) % 360, sems + [blue, magenta]) if chromatic else dom

    hues = [sem["danger"], sem["ok"], sem["warning"], blue, magenta, cyan]
    voice, light = band(VOICE_CONTRAST, bg, VOICE_WIDTH), band(LIGHT_CONTRAST, bg, LIGHT_WIDTH)

    def slot(h_deg, band_t, semantic):
        if semantic:
            s = sat_cap(h_deg, SEMANTIC_SAT)      # statuses stay colored on grey walls
        else:
            s = sat_cap(h_deg, ACC_SAT_CAP) * ramp if chromatic else 0.05
        c = snap_to_band(tint_deg(h_deg, s, 0.55), band_t)
        return clamp_light(c, ANSI_FLOOR, bg)

    normals = [slot(h, voice, i < 3) for i, h in enumerate(hues)]
    brights = [slot(h, light, i < 3) for i, h in enumerate(hues)]

    # No selection-safety pass: foot and ghostty both draw
    # selection-foreground over whatever was selected, so an ANSI color never
    # renders on the selection background — and matugen's base02 runs light
    # enough that a 3.0 floor against it would lift every slot out of its
    # band and flatten normals and brights together.
    ansi = [bg] + normals
    ansi.append(clamp_light(fg, ANSI_FLOOR, bg))                # 7: white
    ansi.append(clamp_light(b["base03"], ANSI_FLOOR_MUTED, bg))  # 8: bright black
    ansi += brights                                             # 9-14
    ansi.append(clamp_light(fg, LIGHT_CONTRAST, bg))             # 15: bright white

    # orange keeps its own hue but joins the bands and floors
    o_hue = hue_sat_of(b["base09"])[0]
    o_sat = sat_cap(o_hue, ACC_SAT_CAP) * ramp if chromatic else 0.05

    b["base08"], b["base0a"], b["base0b"] = ansi[1], ansi[3], ansi[2]
    b["base0c"], b["base0d"], b["base0e"] = ansi[6], ansi[4], ansi[5]
    b["base09"] = clamp_light(snap_to_band(tint_deg(o_hue, o_sat, 0.55), voice),
                              ANSI_FLOOR, bg)
    return ansi


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
              "fastfetch recolour (apply the xiu update or re-run the installer)",
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
    for p in (WALLPAPER_STATE_XIU, WALLPAPER_STATE):
        try:
            if p.is_file():
                w = p.read_text().strip()
                if w:
                    return w
        except OSError:
            pass
    return ""


def generate_dynamic(wallpaper, variant, smart):
    """The wallpaper-driven path: histogram analysis into the HSL pill palette,
    with the resolved matugen variant and the wallpaper's chroma share (the
    fraction of pixels that are chromatic at all — the terminal's cool slots
    scale their saturation with it) riding along."""
    hue, sat, mean_l, share = analyze(wallpaper)
    chromatic = hue is not None
    if not chromatic:
        hue, sat = 0.0, 0.0

    if variant == "auto":
        if not chromatic:
            variant = "neutral"
        else:
            variant = smart_variant(colourfulness(wallpaper)) if smart else "tonal-spot"

    light = mean_l >= 0.40
    surf_sat = min(sat, 0.26) if light else min(max(sat, 0.30 if chromatic else 0.0), 0.45)
    acc_sat = (min(sat + 0.18, 0.85) if light else min(max(sat, 0.30) + 0.12, 0.82)) if chromatic else 0.0
    acc_sat = min(0.95, acc_sat * ACCENT_MULT.get(variant, 1.0))
    if light:
        base = lerp(mean_l, 0.40, 0.66, 0.80, 0.93)
        steps, text, acc_l, deep_l, glow_l = LIGHT_STEPS, LIGHT_TEXT, 0.42, 0.30, 0.55
    else:
        base = lerp(mean_l, 0.0, 0.40, 0.045, 0.20)
        steps, text, acc_l, deep_l, glow_l = DARK_STEPS, DARK_TEXT, 0.70, 0.34, 0.86

    pill = {name: tint(hue, surf_sat, base + step) for name, step in zip(SURF_NAMES, steps)}
    pill["primary"] = tint(hue, acc_sat, acc_l)
    pill["primary_container"] = tint(hue, min(acc_sat + 0.08, 0.9) if chromatic else 0.0, deep_l)
    pill["on_primary_container"] = tint(hue, min(acc_sat, 0.45) if chromatic else 0.0, glow_l)
    pill["outline"] = tint(hue, surf_sat, base + (-0.35 if light else 0.35))
    for key, (lit, st) in zip(TEXT_KEYS, text):
        pill[key] = tint(hue, st if chromatic else 0.0, lit)

    seed = tint(hue, sat, 0.45) if chromatic else "#787878"
    return pill, seed, variant, share


def generate_manual(hue, mode, sat, variant):
    """The Look surface's manual hue override: fixed tone, full ramp."""
    sat = max(0.0, min(1.0, sat))
    mean_l = 0.85 if mode == "light" else 0.12
    chromatic = sat > 0.02
    if not chromatic:
        hue, sat = 0.0, 0.0
    if variant == "auto":
        variant = "neutral" if not chromatic else "tonal-spot"

    light = mean_l >= 0.40
    surf_sat = min(sat, 0.26) if light else min(max(sat, 0.30 if chromatic else 0.0), 0.45)
    acc_sat = (min(sat + 0.18, 0.85) if light else min(max(sat, 0.30) + 0.12, 0.82)) if chromatic else 0.0
    acc_sat = min(0.95, acc_sat * ACCENT_MULT.get(variant, 1.0))
    if light:
        base = lerp(mean_l, 0.40, 0.66, 0.80, 0.93)
        steps, text, acc_l, deep_l, glow_l = LIGHT_STEPS, LIGHT_TEXT, 0.42, 0.30, 0.55
    else:
        base = lerp(mean_l, 0.0, 0.40, 0.045, 0.20)
        steps, text, acc_l, deep_l, glow_l = DARK_STEPS, DARK_TEXT, 0.70, 0.34, 0.86

    pill = {name: tint(hue, surf_sat, base + step) for name, step in zip(SURF_NAMES, steps)}
    pill["primary"] = tint(hue, acc_sat, acc_l)
    pill["primary_container"] = tint(hue, min(acc_sat + 0.08, 0.9) if chromatic else 0.0, deep_l)
    pill["on_primary_container"] = tint(hue, min(acc_sat, 0.45) if chromatic else 0.0, glow_l)
    pill["outline"] = tint(hue, surf_sat, base + (-0.35 if light else 0.35))
    for key, (lit, st) in zip(TEXT_KEYS, text):
        pill[key] = tint(hue, st if chromatic else 0.0, lit)

    seed = tint(hue, sat, 0.45) if chromatic else "#787878"
    return pill, seed, variant


def render_foot(pill, b, ansi):
    """foot's entire color section lives in the include (foot.ini never
    reopens [colors-dark]), so writing the file is enough: every terminal
    opened afterwards opens in the current scheme. Modern foot splits the
    palette into [colors-dark]/[colors-light]; the pipeline always drives the
    dark theme, which is also foot's default. The 16 slots are the semantic
    ANSI list — matugen's raw base16 would leave the regular colors on its
    grey ramp."""
    lines = [
        "[colors-dark]",
        "blur=yes",
        "alpha=0.85",
        "background=%s" % b["base00"].lstrip("#"),
        "foreground=%s" % b["base07"].lstrip("#"),
        "cursor=%s %s" % (pill["primary"].lstrip("#"), b["base07"].lstrip("#")),
        "selection-background=%s" % b["base02"].lstrip("#"),
        "selection-foreground=%s" % b["base07"].lstrip("#"),
    ]
    for i in range(8):
        lines.append("regular%d=%s" % (i, ansi[i].lstrip("#")))
    for i in range(8):
        lines.append("bright%d=%s" % (i, ansi[i + 8].lstrip("#")))
    body = "\n".join(lines) + "\n"

    foot = _tool_dir("foot")
    if foot is None:
        foot = Path.home() / ".config" / "foot"
        foot.mkdir(parents=True, exist_ok=True)
    (foot / "colors.ini").write_text(body)

    CACHE_XIU.mkdir(parents=True, exist_ok=True)
    (CACHE_XIU / "foot-colors.ini").write_text(body)


def osc_sequence(code, hex_color):
    """One OSC color sequence with the ST terminator, e.g. \\e]11;rgb:1a/2b/3c\\e\\\\"""
    c = hex_color.lstrip("#")
    return "\x1b]%s;rgb:%s/%s/%s\x1b\\" % (code, c[0:2], c[2:4], c[4:6])


def broadcast_terminal(pill, b, ansi):
    """caelestia's live retheme: push the palette into every open pty as OSC
    color sequences (10 foreground, 11 background, 12 cursor, 17 selection
    background, 4 the sixteen palette slots), so running terminals and the
    TUIs inside them recolor the moment the wallpaper changes — no restart,
    no reload, and it reaches terminals that never read a config file. The
    same bytes are persisted to sequences.txt and replayed by config.fish at
    shell start, which themes those terminals' fresh sessions too. New
    foot/ghostty windows are covered by colors.ini/ghostty-colors as before.
    A pty that is busy, closed or not ours is skipped, never fatal."""
    seq = (osc_sequence(10, b["base07"]) + osc_sequence(11, b["base00"])
           + osc_sequence(12, pill["primary"]) + osc_sequence(17, b["base02"]))
    for i, hex_color in enumerate(ansi):
        c = hex_color.lstrip("#")
        seq += "\x1b]4;%d;rgb:%s/%s/%s\x1b\\" % (i, c[0:2], c[2:4], c[4:6])
    CACHE.mkdir(parents=True, exist_ok=True)
    (CACHE / "sequences.txt").write_text(seq)
    CACHE_XIU.mkdir(parents=True, exist_ok=True)
    (CACHE_XIU / "sequences.txt").write_text(seq)
    data = seq.encode()
    try:
        entries = list(Path("/dev/pts").iterdir())
    except OSError:
        return
    for pt in entries:
        if not pt.name.isdigit():
            continue
        try:
            fd = os.open(str(pt), os.O_WRONLY | os.O_NONBLOCK | os.O_NOCTTY)
            try:
                os.write(fd, data)
            finally:
                os.close(fd)
        except OSError:
            continue


def render_starship(pill):
    """The prompt follows the pill: the shipped starship.toml.in carries
    tokens, this fills them with the current palette and writes the real
    config next to it. config.fish points STARSHIP_CONFIG at the rendered
    file, so every new shell opens in the current scheme."""
    fish = _tool_dir("fish")
    if fish is None:
        return
    tmpl = fish / "starship.toml.in"
    if not tmpl.is_file():
        print("wallcolors: starship.toml.in missing in ~/.config/fish, skipping "
              "starship recolour (apply the xiu update or re-run the installer)",
              file=sys.stderr)
        return
    repl = {
        "__CHAR_OK__": pill["primary"],
        "__CHAR_ERR__": pill["bright"],
        "__DIR__": pill["cream"],
        "__BRANCH__": pill["subtle"],
        "__STATUS__": pill["dim"],
        "__DURATION__": pill["faint"],
    }
    out = tmpl.read_text()
    for key, val in repl.items():
        out = out.replace(key, val)
    (fish / "starship.toml").write_text(out)


def render_fish(pill, ansi):
    """fish's syntax colors, regenerated on every palette change and sourced
    by config.fish before the user's own file, so a hand override still wins.
    Commands take the accent and the quote family the warm text ramp; errors
    and escapes reuse the terminal's own bent red and yellow, so the shell
    and the terminal agree on what a mistake looks like."""
    fish = _tool_dir("fish")
    if fish is None:
        return
    # NB: every value is quoted — a bare #hex reads as a comment in fish
    q = lambda hex_color: '"%s"' % hex_color
    lines = [
        "# Written by wallcolors.py on every palette change; sourced by config.fish.",
        "set -g fish_color_command %s" % q(pill["primary"]),
        "set -g fish_color_param %s" % q(pill["cream"]),
        "set -g fish_color_option %s" % q(pill["subtle"]),
        "set -g fish_color_quote %s" % q(pill["on_primary_container"]),
        "set -g fish_color_escape %s" % q(ansi[3]),
        "set -g fish_color_redirection %s" % q(pill["subtle"]),
        "set -g fish_color_comment %s" % q(pill["faint"]),
        "set -g fish_color_error %s" % q(ansi[1]),
        "set -g fish_color_operator %s" % q(pill["on_primary_container"]),
        "set -g fish_color_autosuggestion %s" % q(pill["faint"]),
        "set -g fish_color_cancel %s" % q(pill["dim"]),
        "set -g fish_color_search_match --background=%s" % q(pill["primary_container"]),
        "set -g fish_color_selection --background=%s" % q(pill["surface_container_high"]),
        "set -g fish_pager_color_prefix %s" % q(pill["primary"]),
        "set -g fish_pager_color_completion %s" % q(pill["cream"]),
        "set -g fish_pager_color_description %s" % q(pill["subtle"]),
        "set -g fish_pager_color_progress %s" % q(pill["dim"]),
        "set -g fish_pager_color_selected_background --background=%s"
            % q(pill["surface_container_high"]),
    ]
    (fish / "syntax.fish").write_text("\n".join(lines) + "\n")


def _tool_dir(name):
    """Config dir of an optional tool, or None when it is not in play; every
    TUI render is gated on it so the pipeline never litters configs for apps
    that were never installed."""
    path = Path.home() / ".config" / name
    return path if path.is_dir() else None


def _reload(binary):
    """The USR2 hot-reload poke — btop only (verified in its source:
    btop.cpp registers SIGUSR2 for the same reload CTRL+R does). The other
    TUIs either document a different signal (cava: USR1, poked inline in
    its renderer) or read their config once and have no signal path at all
    (htop, nvtop, yazi, helix, micro, bottom — those pick the new palette
    up on next launch; the terminal's own colors follow live through the
    OSC broadcast)."""
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
    # No poke: htop reads its config once at start and watches nothing —
    # running instances keep the old colors and the next launch picks the
    # new palette up. (killall -USR2 was a no-op here: htop has no signal
    # handler for it.)


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
    # cava's documented live-reload signal is SIGUSR1 (README: "Sending cava
    # a SIGUSR1 signal will force cava to reload its configuration file") —
    # USR2 did nothing, so a running visualizer kept its old bars.
    subprocess.run(["killall", "-USR1", "cava"], stderr=subprocess.DEVNULL)


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
        ("gutter", pill["faint"], None),
        ("cursor-line", None, pill["surface_container_low"]),
        ("color-column", None, pill["surface_container_low"]),
        ("statusline", pill["cream"], pill["surface_container"]),
        ("statusline.active", pill["bright"], pill["surface_container_high"]),
        ("tabbar", pill["dim"], None),
        ("divider", pill["surface_container_high"], None),
        ("indent-char", pill["outline_variant"], None),
        ("comment", "italic " + pill["dim"], None),
        ("identifier", pill["cream"], None),
        ("identifier.class", b["base0a"], None),
        ("identifier.var", pill["cream"], None),
        ("identifier.macro", b["base0e"], None),
        ("identifier.function", b["base0d"], None),
        ("constant", b["base0a"], None),
        ("constant.bool", b["base09"], None),
        ("constant.number", b["base09"], None),
        ("constant.string", b["base0b"], None),
        ("constant.string.escape", b["base0e"], None),
        ("constant.specialChar", b["base0e"], None),
        ("statement", pill["primary"], None),
        ("keyword", pill["primary"], None),
        ("keyword.operator", pill["subtle"], None),
        ("symbol", pill["subtle"], None),
        ("symbol.brackets", pill["faint"], None),
        ("symbol.tag", b["base0d"], None),
        ("preproc", b["base0e"], None),
        ("type", b["base0c"], None),
        ("type.keyword", pill["primary"], None),
        ("special", b["base0e"], None),
        ("underlined", pill["primary"], None),
        ("error", b["base08"], None),
        ("warning", b["base0a"], None),
        ("todo", "bold " + pill["primary"], None),
        ("diff-added", b["base0b"], None),
        ("diff-modified", b["base0a"], None),
        ("diff-deleted", b["base08"], None),
    ]
    lines = ["# Written by wallcolors.py on every palette change."]
    for group, fg, bg in links:
        if fg is None:
            lines.append('color-link %s ",%s"' % (group, bg))
        else:
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
        '"ui.background.separator" = { fg = "%s" }' % p["outline_variant"],
        '"ui.gutter" = {}',
        '"ui.gutter.selected" = {}',
        '"ui.text" = %s' % fg(p["cream"]),
        '"ui.text.focus" = %s' % fg(p["bright"]),
        '"ui.text.info" = %s' % fg(p["subtle"]),
        '"ui.selection" = { bg = "%s" }' % p["surface_container_high"],
        '"ui.selection.primary" = { bg = "%s" }' % p["surface_container"],
        '"ui.cursorline" = {}',
        '"ui.cursorline.primary" = { underline = { color = "%s", style = "line" } }' % p["outline_variant"],
        '"ui.cursorline.secondary" = { underline = { color = "%s", style = "line" } }' % p["outline_variant"],
        '"ui.linenr" = { fg = "%s" }' % p["faint"],
        '"ui.linenr.selected" = { fg = "%s" }' % p["subtle"],
        '"ui.statusline" = { fg = "%s" }' % p["cream"],
        '"ui.statusline.inactive" = { fg = "%s" }' % p["dim"],
        '"ui.statusline.separator" = { fg = "%s" }' % p["outline_variant"],
        '"ui.statusline.normal" = { fg = "%s", bg = "%s", modifiers = ["bold"] }' % (p["on_primary_container"], p["primary_container"]),
        '"ui.statusline.insert" = { fg = "%s", bg = "%s", modifiers = ["bold"] }' % (p["bright"], p["primary"]),
        '"ui.statusline.select" = { fg = "%s", bg = "%s", modifiers = ["bold"] }' % (p["bright"], p["primary_container"]),
        '"ui.bufferline" = { fg = "%s" }' % p["dim"],
        '"ui.bufferline.active" = { fg = "%s", underline = { color = "%s", style = "line" }, modifiers = ["bold"] }' % (p["bright"], p["primary"]),
        '"ui.bufferline.background" = {}',
        '"ui.popup" = { bg = "%s" }' % p["surface_container"],
        '"ui.popup.info" = { bg = "%s" }' % p["surface_container_high"],
        '"ui.window" = { fg = "%s" }' % p["outline_variant"],
        '"ui.help" = { fg = "%s", bg = "%s" }' % (p["cream"], p["surface_container"]),
        '"ui.menu" = { bg = "%s" }' % p["surface_container"],
        '"ui.menu.selected" = { fg = "%s", bg = "%s" }' % (p["bright"], p["surface_container_high"]),
        '"ui.virtual" = %s' % fg(p["outline_variant"]),
        '"ui.virtual.whitespace" = %s' % fg(p["outline_variant"]),
        '"ui.virtual.indent-guide" = { fg = "%s" }' % p["outline_variant"],
        '"ui.virtual.ruler" = {}',
        # Syntax scopes
        '"attribute" = %s' % fg(b["base0d"]),
        '"type" = %s' % fg(b["base0a"]),
        '"type.builtin" = %s' % fg(b["base0c"]),
        '"type.enum" = %s' % fg(b["base0a"]),
        '"type.enum.variant" = %s' % fg(b["base0c"]),
        '"constructor" = %s' % fg(b["base0c"]),
        '"constant" = %s' % fg(b["base0a"]),
        '"constant.builtin" = %s' % fg(b["base0a"]),
        '"constant.builtin.boolean" = %s' % fg(b["base09"]),
        '"constant.character" = %s' % fg(b["base0e"]),
        '"constant.character.escape" = %s' % fg(b["base0e"]),
        '"constant.numeric" = %s' % fg(b["base09"]),
        '"constant.numeric.integer" = %s' % fg(b["base09"]),
        '"constant.numeric.float" = %s' % fg(b["base09"]),
        '"string" = %s' % fg(b["base0b"]),
        '"string.regexp" = %s' % fg(b["base0c"]),
        '"string.special" = %s' % fg(b["base0e"]),
        '"string.special.symbol" = %s' % fg(b["base0b"]),
        '"comment" = { fg = "%s", modifiers = ["italic"] }' % p["dim"],
        '"comment.line" = { fg = "%s", modifiers = ["italic"] }' % p["dim"],
        '"comment.block" = { fg = "%s", modifiers = ["italic"] }' % p["dim"],
        '"comment.block.documentation" = { fg = "%s", modifiers = ["italic"] }' % p["subtle"],
        '"variable" = %s' % fg(p["cream"]),
        '"variable.builtin" = %s' % fg(p["primary"]),
        '"variable.parameter" = %s' % fg(p["subtle"]),
        '"variable.other.member" = %s' % fg(p["bright"]),
        '"label" = %s' % fg(p["primary"]),
        '"punctuation" = %s' % fg(p["dim"]),
        '"punctuation.bracket" = %s' % fg(p["faint"]),
        '"punctuation.delimiter" = %s' % fg(p["dim"]),
        '"punctuation.special" = %s' % fg(p["primary"]),
        '"keyword" = %s' % fg(p["primary"]),
        '"keyword.control" = %s' % fg(p["primary"]),
        '"keyword.control.conditional" = %s' % fg(p["primary"]),
        '"keyword.control.repeat" = %s' % fg(p["primary"]),
        '"keyword.control.import" = %s' % fg(p["primary"]),
        '"keyword.control.return" = %s' % fg(p["primary"]),
        '"keyword.control.exception" = %s' % fg(p["primary"]),
        '"keyword.operator" = %s' % fg(p["primary"]),
        '"keyword.directive" = %s' % fg(b["base0e"]),
        '"keyword.function" = %s' % fg(p["primary"]),
        '"keyword.storage" = %s' % fg(p["primary"]),
        '"keyword.storage.type" = %s' % fg(p["primary"]),
        '"operator" = %s' % fg(p["subtle"]),
        '"function" = %s' % fg(b["base0d"]),
        '"function.builtin" = %s' % fg(b["base0c"]),
        '"function.method" = %s' % fg(b["base0d"]),
        '"function.macro" = %s' % fg(b["base0e"]),
        '"tag" = %s' % fg(p["primary"]),
        '"special" = %s' % fg(b["base0e"]),
        '"markup.heading" = { fg = "%s", modifiers = ["bold"] }' % p["bright"],
        '"markup.bold" = { modifiers = ["bold"] }',
        '"markup.italic" = { modifiers = ["italic"] }',
        '"markup.strikethrough" = { modifiers = ["crossed_out"] }',
        '"markup.link.url" = { fg = "%s", modifiers = ["underlined"] }' % p["dim"],
        '"markup.link.text" = %s' % fg(p["primary"]),
        '"markup.raw" = %s' % fg(b["base0b"]),
        '"markup.list" = %s' % fg(p["primary"]),
        '"diff.plus" = %s' % fg(b["base0b"]),
        '"diff.minus" = %s' % fg(b["base08"]),
        '"diff.delta" = %s' % fg(b["base0c"]),
        '"error" = %s' % fg(b["base08"]),
        '"warning" = %s' % fg(b["base0a"]),
        '"info" = %s' % fg(b["base0c"]),
        '"hint" = %s' % fg(p["subtle"]),
        '"diagnostic.error" = { underline = { color = "%s", style = "curl" } }' % b["base08"],
        '"diagnostic.warning" = { underline = { color = "%s", style = "curl" } }' % b["base0a"],
        '"diagnostic.info" = { underline = { color = "%s", style = "curl" } }' % b["base0c"],
        '"diagnostic.hint" = { underline = { color = "%s", style = "curl" } }' % p["subtle"],
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
    """yazi: theme.toml is the palette surface (mgr, tabs, mode, filetype, status, etc.)
    and is regenerated whole, like htoprc; yazi.toml (keys, openers) is the user's
    and is never touched."""
    d = _tool_dir("yazi")
    if d is None:
        return
    p = pill
    lines = [
        "# Written by wallcolors.py on every palette change.",
        "[manager]",
        'cwd = { fg = "%s", bold = true }' % p["cream"],
        'hovered = { fg = "%s", bg = "%s", bold = true }'
        % (p["bright"], p["surface_container_high"]),
        'preview_hovered = { underline = true }',
        'border_style = { fg = "%s" }' % p["outline_variant"],
        'find_keyword = { fg = "%s", bold = true }' % p["primary"],
        'find_position = { fg = "%s", bg = "%s" }'
        % (p["bright"], p["surface_container_high"]),
        'marker_selected = { fg = "%s", bold = true }' % p["primary"],
        'marker_copied = { fg = "%s" }' % b["base0b"],
        'marker_cut = { fg = "%s" }' % b["base08"],
        'marker_marked = { fg = "%s" }' % b["base0e"],
        'tab_active = { fg = "%s", bg = "%s", bold = true }'
        % (p["bright"], p["surface_container_high"]),
        'tab_inactive = { fg = "%s" }' % p["dim"],
        'count_copied = { fg = "%s", bg = "%s" }' % (p["bright"], b["base0b"]),
        'count_cut = { fg = "%s", bg = "%s" }' % (p["bright"], b["base08"]),
        'count_selected = { fg = "%s", bg = "%s" }' % (p["bright"], p["primary"]),
        'border_symbol = "│"',
        'syntect_theme = ""',
        "",
        "[tabs]",
        'active = { fg = "%s", bg = "%s", bold = true }'
        % (p["bright"], p["surface_container_high"]),
        'inactive = { fg = "%s" }' % p["dim"],
        'sep = { fg = "%s" }' % p["outline_variant"],
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
        "[status]",
        'separator_open = ""',
        'separator_close = ""',
        'separator_style = { fg = "%s" }' % p["outline_variant"],
        "",
        "[select]",
        'border = { fg = "%s" }' % p["primary"],
        'active = { fg = "%s", bg = "%s" }' % (p["bright"], p["surface_container_high"]),
        'inactive = { fg = "%s" }' % p["cream"],
        "",
        "[input]",
        'border = { fg = "%s" }' % p["primary"],
        'title = { fg = "%s", bold = true }' % p["cream"],
        'value = { fg = "%s" }' % p["bright"],
        'selected = { bg = "%s" }' % p["surface_container_high"],
        "",
        "[which]",
        'mask = { bg = "%s" }' % p["surface_container"],
        'cand = { fg = "%s" }' % b["base0c"],
        'rest = { fg = "%s" }' % p["subtle"],
        'desc = { fg = "%s" }' % p["cream"],
        'separator = "  "',
        'separator_style = { fg = "%s" }' % p["outline_variant"],
        "",
        "[filetype]",
        "rules = [",
        '  { mime = "image/*", fg = "%s" },' % b["base0a"],
        '  { mime = "{audio,video}/*", fg = "%s" },' % b["base0d"],
        '  { mime = "application/{zip,rar,7z*,tar*,gzip,xz}", fg = "%s" },' % b["base0e"],
        '  { mime = "application/{pdf,doc*,epub*}", fg = "%s" },' % b["base0b"],
        '  { mime = "inode/empty", fg = "%s" },' % p["dim"],
        '  { url = "*/", fg = "%s", bold = true },' % b["base0d"],
        '  { url = "*", fg = "%s" },' % p["cream"],
        "]",
    ]
    (d / "theme.toml").write_text("\n".join(lines) + "\n")


def render_userchrome(pill):
    """Recolor the xiu palette block inside every deployed userChrome.css
    (Firefox and Zen profiles the installer wired), so the browser chrome
    follows the wallpaper even before the live-theme extension loads. Only
    files carrying the xiu variable block are touched; a user's own
    userChrome is never rewritten."""
    subs = {
        "--xiu-surface": pill["surface"],
        "--xiu-surface-high": pill["surface_container_high"],
        "--xiu-cream": pill["cream"],
        "--xiu-dim": pill["dim"],
        "--xiu-outline": pill["outline_variant"],
        "--xiu-primary": pill["primary"],
    }
    for root in (Path.home() / ".mozilla" / "firefox", Path.home() / ".zen"):
        if not root.is_dir():
            continue
        for uc in root.glob("*/chrome/userChrome.css"):
            try:
                text = uc.read_text()
            except OSError:
                continue
            if "--xiu-surface:" not in text:
                continue
            for key, value in subs.items():
                text = re.sub(r"%s:\s*#[0-9a-fA-F]{3,8};" % re.escape(key),
                              "%s: %s;" % (key, value), text)
            uc.write_text(text)


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
        "titleBar.inactiveBackground": p["surface"],
        "titleBar.inactiveForeground": p["faint"],
        "editorGroup.border": "#00000000",
        "sideBar.border": "#00000000",
        "tab.border": "#00000000",
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


def render_zed(pill, b=None):
    """Zed picks user themes out of ~/.config/zed/themes; the xiu themes
    ("xiu" flat and "xiu blur") are kept fresh, based on the Vesper and
    Vesper Blur themes dynamically mapped across the wallpaper palette.
    The active selection in settings.json is only set when the user never
    chose a theme — their file is hand-written JSONC, so it is edited
    textually (never re-serialized) and an existing choice is respected."""
    z = _tool_dir("zed")
    if z is None:
        return
    p = pill
    if b is None:
        b = {
            "base00": p["surface"],
            "base01": p["surface_container_low"],
            "base02": p["surface_container"],
            "base03": p["surface_container_high"],
            "base04": p["outline_variant"],
            "base05": p["dim"],
            "base06": p["subtle"],
            "base07": p["cream"],
            "base08": p["primary"],
            "base09": p.get("on_primary_container", p["primary"]),
            "base0a": p.get("tick_rest", p["primary"]),
            "base0b": p.get("tick_rest", p["cream"]),
            "base0c": p.get("subtle", p["cream"]),
            "base0d": p.get("primary", p["bright"]),
            "base0e": p.get("primary_container", p["primary"]),
            "base0f": p.get("outline", p["dim"]),
        }
    lum = lambda c: 0.2126 * int(c[1:3], 16) + 0.7152 * int(c[3:5], 16) + 0.0722 * int(c[5:7], 16)
    hex_a = lambda c, al: "#" + c.lstrip("#")[:6] + al
    a = lambda c: c if c is None else ("#" + c.lstrip("#")[:6] + "ff" if len(c.lstrip("#")) <= 6 else ("#" + c.lstrip("#") if not c.startswith("#") else c))

    def syntax(c, italic=False, bold=False):
        entry = {"color": a(c)}
        if italic:
            entry["font_style"] = "italic"
        if bold:
            entry["font_weight"] = 700
        return entry

    syntax_tree = {
        "attribute": syntax(p["subtle"]),
        "boolean": syntax(p["primary"]),
        "comment": syntax(p["dim"], italic=True),
        "comment.doc": syntax(p["dim"], italic=True),
        "constant": syntax(p["primary"]),
        "constructor": syntax(p["primary"]),
        "emphasis": syntax(p["cream"], italic=True),
        "emphasis.strong": syntax(p["bright"], bold=True),
        "function": syntax(p["primary"]),
        "function.builtin": syntax(p["primary"]),
        "function.method": syntax(p["primary"]),
        "function.macro": syntax(p["primary"]),
        "keyword": syntax(p["subtle"]),
        "keyword.control": syntax(p["subtle"]),
        "keyword.operator": syntax(p["subtle"]),
        "label": syntax(p["primary"]),
        "link_text": syntax(p["cream"]),
        "link_uri": syntax(p["primary"]),
        "number": syntax(p["primary"]),
        "operator": syntax(p["subtle"]),
        "punctuation": syntax(p["subtle"]),
        "punctuation.bracket": syntax(p["subtle"]),
        "punctuation.delimiter": syntax(p["subtle"]),
        "punctuation.list_marker": syntax(p["subtle"]),
        "punctuation.special": syntax(p["subtle"]),
        "string": syntax(b["base0b"]),
        "string.escape": syntax(p["subtle"]),
        "string.regex": syntax(b.get("base0c", p["subtle"])),
        "string.special": syntax(b["base0b"]),
        "string.special.symbol": syntax(b["base0b"]),
        "tag": syntax(p["primary"]),
        "text.literal": syntax(b["base0b"]),
        "title": syntax(p["primary"], bold=True),
        "type": syntax(p["primary"]),
        "type.builtin": syntax(p["primary"]),
        "variable": syntax(p["cream"]),
        "variable.special": syntax(p["subtle"]),
    }

    players = [
        {
            "cursor": a(p["primary"]),
            "selection": hex_a(p["bright"], "25"),
            "background": a(p["primary"]),
        },
        {
            "cursor": a(b["base0b"]),
            "selection": hex_a(b["base0b"], "25"),
            "background": a(b["base0b"]),
        },
        {
            "cursor": a(b["base08"]),
            "selection": hex_a(b["base08"], "25"),
            "background": a(b["base08"]),
        },
        {
            "cursor": a(p["subtle"]),
            "selection": hex_a(p["subtle"], "25"),
            "background": a(p["subtle"]),
        },
    ]

    base_elements = {
        # Borders - flat UI by default
        "border": "#00000000",
        "border.variant": "#00000000",
        "border.focused": a(p["primary"]),
        "border.selected": a(p["primary"]),
        "border.transparent": "#00000000",
        "border.disabled": "#00000000",
        "panel.focused_border": "#00000000",
        "pane.focused_border": "#00000000",
        "pane_group.border": "#00000000",

        # Elevated surface & container overlays
        "elevated_surface.background": a(p["surface_container"]),
        "panel.overlay_background": a(p["surface_container"]),

        # Elements & interactive items
        "element.background": a(p["surface_container_low"]),
        "element.hover": a(p["surface_container"]),
        "element.active": a(p["surface_container_high"]),
        "element.selected": a(p["surface_container_high"]),
        "element.disabled": a(p["surface_container_low"]),
        "ghost_element.background": "#00000000",
        "ghost_element.hover": a(p["surface_container"]),
        "ghost_element.active": a(p["surface_container_high"]),
        "ghost_element.selected": a(p["surface_container_high"]),
        "ghost_element.disabled": "#00000000",
        "drop_target.background": hex_a(p["primary"], "50"),

        # Typography & Icons
        "text": a(p["cream"]),
        "text.muted": a(p["subtle"]),
        "text.placeholder": a(p["dim"]),
        "text.disabled": a(p["faint"]),
        "text.accent": a(p["primary"]),
        "icon": a(p["cream"]),
        "icon.muted": a(p["subtle"]),
        "icon.disabled": a(p["faint"]),
        "icon.placeholder": a(p["dim"]),
        "icon.accent": a(p["primary"]),

        # Guides & Indents
        "panel.indent_guide": a(p["outline_variant"]),
        "panel.indent_guide_active": a(p["outline"]),
        "panel.indent_guide_hover": a(p["primary"]),
        "editor.indent_guide": a(p["outline_variant"]),
        "editor.indent_guide_active": a(p["outline"]),
        "editor.wrap_guide": a(p["outline_variant"]),
        "editor.active_wrap_guide": a(p["outline_variant"]),
        "editor.invisible": a(p["faint"]),

        # Scrollbars
        "scrollbar.thumb.background": hex_a(p["outline_variant"], "80"),
        "scrollbar.thumb.hover_background": a(p["outline"]),
        "scrollbar.thumb.active_background": a(p["outline"]),
        "scrollbar.thumb.border": "#00000000",
        "scrollbar.track.background": "#00000000",
        "scrollbar.track.border": "#00000000",

        # Search
        "search.match_background": hex_a(p["bright"], "25"),
        "search.active_match_background": hex_a(p["primary"], "50"),

        # Editor foreground, numbers, highlights
        "editor.foreground": a(p["cream"]),
        "editor.line_number": a(p["faint"]),
        "editor.active_line_number": a(p["bright"]),
        "editor.highlighted_line.background": hex_a(p["bright"], "10"),
        "editor.document_highlight.read_background": hex_a(p["bright"], "15"),
        "editor.document_highlight.write_background": hex_a(p["bright"], "15"),
        "editor.subheader.background": a(p["surface_container_low"]),

        # Link
        "link_text.hover": a(p.get("primary_container", p["primary"])),

        # Git & Diagnostics
        "conflict": a(b["base0a"]),
        "conflict.background": hex_a(b["base0a"], "15"),
        "conflict.border": a(b["base0a"]),
        "created": a(b["base0b"]),
        "created.background": hex_a(b["base0b"], "15"),
        "created.border": a(b["base0b"]),
        "deleted": a(b["base08"]),
        "deleted.background": hex_a(b["base08"], "15"),
        "deleted.border": a(b["base08"]),
        "error": a(b["base08"]),
        "error.background": hex_a(b["base08"], "25"),
        "error.border": a(b["base08"]),
        "hidden": a(p["dim"]),
        "hidden.background": a(p["surface"]),
        "hidden.border": a(p["surface"]),
        "hint": a(p["subtle"]),
        "hint.background": a(p["surface_container_low"]),
        "hint.border": a(p["surface_container"]),
        "ignored": a(p["dim"]),
        "ignored.background": a(p["surface"]),
        "ignored.border": a(p["surface"]),
        "info": a(b["base0c"]),
        "info.background": a(p["surface_container_low"]),
        "info.border": a(p["surface_container"]),
        "modified": a(b["base0a"]),
        "modified.background": hex_a(b["base0a"], "15"),
        "modified.border": a(b["base0a"]),
        "predictive": a(p["dim"]),
        "predictive.background": a(p["surface"]),
        "predictive.border": a(p["surface"]),
        "renamed": a(b["base0a"]),
        "renamed.background": hex_a(b["base0a"], "15"),
        "renamed.border": a(b["base0a"]),
        "success": a(b["base0b"]),
        "success.background": hex_a(b["base0b"], "15"),
        "success.border": a(b["base0b"]),
        "unreachable": a(b["base08"]),
        "unreachable.background": hex_a(b["base08"], "15"),
        "unreachable.border": a(b["base08"]),
        "warning": a(b["base0a"]),
        "warning.background": hex_a(b["base0a"], "25"),
        "warning.border": a(b["base0a"]),

        # Terminal ANSI
        "terminal.foreground": a(p["cream"]),
        "terminal.bright_foreground": a(p["bright"]),
        "terminal.dim_foreground": a(p["dim"]),
        "terminal.ansi.background": a(p["surface"]),
        "terminal.ansi.black": a(b["base00"]),
        "terminal.ansi.bright_black": a(b["base03"]),
        "terminal.ansi.red": a(b["base08"]),
        "terminal.ansi.bright_red": a(b["base08"]),
        "terminal.ansi.green": a(b["base0b"]),
        "terminal.ansi.bright_green": a(b["base0b"]),
        "terminal.ansi.yellow": a(b["base0a"]),
        "terminal.ansi.bright_yellow": a(b["base0a"]),
        "terminal.ansi.blue": a(b["base0d"]),
        "terminal.ansi.bright_blue": a(b["base0d"]),
        "terminal.ansi.magenta": a(b["base0e"]),
        "terminal.ansi.bright_magenta": a(b["base0e"]),
        "terminal.ansi.cyan": a(b["base0c"]),
        "terminal.ansi.bright_cyan": a(b["base0c"]),
        "terminal.ansi.white": a(b["base07"]),
        "terminal.ansi.bright_white": a(p["bright"]),

        "players": players,
        "syntax": syntax_tree,
    }

    # Flat theme: seamless flat UI, borderless panels, identical active/inactive header
    flat_style = dict(base_elements)
    flat_style.update({
        "background": a(p["surface"]),
        "surface.background": a(p["surface"]),
        "panel.background": a(p["surface"]),
        "title_bar.background": a(p["surface"]),
        "title_bar.inactive_background": a(p["surface"]),
        "status_bar.background": a(p["surface"]),
        "toolbar.background": a(p["surface"]),
        "tab_bar.background": a(p["surface"]),
        "tab.active_background": a(p["surface_container_low"]),
        "tab.inactive_background": a(p["surface"]),
        "editor.background": a(p["surface"]),
        "editor.gutter.background": a(p["surface"]),
        "editor.active_line.background": a(p["surface_container_low"]),
        "terminal.background": a(p["surface"]),
    })

    # Xiu Blur theme: translucent alpha backgrounds supporting Hyprland blur
    blur_style = dict(base_elements)
    blur_style.update({
        "background.appearance": "blurred",
        "background": hex_a(p["surface"], "b8"),
        "surface.background": hex_a(p["surface"], "b8"),
        "panel.background": "#00000000",
        "title_bar.background": hex_a(p["surface"], "b8"),
        "title_bar.inactive_background": hex_a(p["surface"], "b8"),
        "status_bar.background": hex_a(p["surface"], "b8"),
        "toolbar.background": "#00000000",
        "tab_bar.background": "#00000000",
        "tab.active_background": a(p["surface_container_low"]),
        "tab.inactive_background": "#00000000",
        "editor.background": "#00000000",
        "editor.gutter.background": "#00000000",
        "editor.active_line.background": "#00000000",
        "terminal.background": "#00000000",
    })

    appearance = "light" if lum(p["surface"]) > 128 else "dark"
    theme = {
        "$schema": "https://zed.dev/schema/themes/v0.2.0.json",
        "name": "xiu",
        "author": "yrpcaro",
        "themes": [
            {
                "name": "xiu",
                "appearance": appearance,
                "style": flat_style,
            },
            {
                "name": "xiu blur",
                "appearance": appearance,
                "style": blur_style,
            },
        ],
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
    """Brave/Chromium pick their toolbar color up from a managed policy,
    caelestia's trick: BrowserThemeColor tints the chrome, BrowserColorScheme
    makes dark-mode follow the device rather than a manual flag, and the
    --refresh-platform-policy poke makes a RUNNING browser re-read the policy
    with no restart. The payload lands in xiu's own config dir; `xiu browser`
    (or the installer) copies it into /etc, which needs root."""
    d = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))) / "xiu"
    d.mkdir(parents=True, exist_ok=True)
    (d / "browser-theme.json").write_text(
        json.dumps({
            "BrowserThemeColor": pill["surface"],
            "BrowserColorScheme": "device",
        }, indent=2) + "\n")
    # Best-effort poke: it only matters when the browser runs, and a fresh
    # install applies the policy at first launch anyway. Detached with a
    # short timeout — Brave's --no-startup-window still lingers as a live
    # process, and wallcolors runs at boot where a hang would be fatal.
    for cmd in ("brave", "chromium"):
        if shutil.which(cmd):
            subprocess.Popen(
                ["timeout", "10", cmd, "--refresh-platform-policy", "--no-startup-window"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                start_new_session=True)


def _rgb(hex_str):
    """KDE color schemes want "r,g,b" decimal tuples."""
    return ",".join(str(int(hex_str[i:i + 2], 16)) for i in (1, 3, 5))


def gnome_accent_color(hex_color):
    """Map accent color to one of GNOME 47+'s supported desktop accent colors."""
    h, s = hue_sat_of(hex_color)
    if s < 0.15:
        return "slate"
    if h >= 345 or h < 15:
        return "red"
    elif 15 <= h < 45:
        return "orange"
    elif 45 <= h < 70:
        return "yellow"
    elif 70 <= h < 150:
        return "green"
    elif 150 <= h < 190:
        return "teal"
    elif 190 <= h < 255:
        return "blue"
    elif 255 <= h < 290:
        return "purple"
    else:
        return "pink"


def get_active_icon_theme(is_dark=True):
    """Resolve the active icon theme, defaulting to yet-another-monochrome-icon-set."""
    theme = ""
    if shutil.which("gsettings"):
        try:
            res = subprocess.run(["gsettings", "get", "org.gnome.desktop.interface", "icon-theme"],
                                 capture_output=True, text=True, timeout=2)
            theme = res.stdout.strip().strip("'\"")
        except Exception:
            pass
    if not theme:
        kde_cfg = Path.home() / ".config" / "kdeglobals"
        if kde_cfg.is_file():
            try:
                for line in kde_cfg.read_text().splitlines():
                    if line.startswith("Theme="):
                        theme = line.split("=", 1)[1].strip()
                        break
            except Exception:
                pass
    if not theme:
        for ver in ("gtk-3.0", "gtk-4.0"):
            ini = Path.home() / ".config" / ver / "settings.ini"
            if ini.is_file():
                try:
                    for line in ini.read_text().splitlines():
                        if line.startswith("gtk-icon-theme-name="):
                            theme = line.split("=", 1)[1].strip()
                            break
                except Exception:
                    pass
            if theme:
                break

    yamis_name = "yet-another-monochrome-icon-set"
    local_icons = Path.home() / ".local" / "share" / "icons"
    usr_icons = Path("/usr/share/icons")
    has_yamis = ((local_icons / yamis_name).is_dir() or (usr_icons / yamis_name).is_dir())

    if has_yamis or theme == yamis_name:
        return yamis_name

    is_generic = (not theme) or theme.lower() in (
        "breeze", "breeze-dark", "breeze-light", "breeze_light", "adwaita", "adwaitalegacy", "hicolor",
        "breeze-round-chameleon dark icons", "breeze-round-chameleon light icons", "papirus", "papirus-dark"
    )
    if is_generic or not theme:
        return yamis_name

    return theme


def _update_gtk_settings(settings_file, theme_name, icon_theme, is_dark):
    """Ensure gtk-3.0/gtk-4.0 settings.ini selects the theme and icon theme."""
    cp = configparser.RawConfigParser(strict=False)
    cp.optionxform = str
    if settings_file.is_file():
        try:
            cp.read(str(settings_file))
        except Exception:
            pass
    if not cp.has_section("Settings"):
        cp.add_section("Settings")
    cp.set("Settings", "gtk-theme-name", theme_name)
    cp.set("Settings", "gtk-icon-theme-name", icon_theme)
    cp.set("Settings", "gtk-application-prefer-dark-theme", "true" if is_dark else "false")
    if cp.has_option("Settings", "gtk-modules"):
        mods = [m for m in cp.get("Settings", "gtk-modules").split(":") if m and m != "colorreload-gtk-module"]
        if mods:
            cp.set("Settings", "gtk-modules", ":".join(mods))
        else:
            cp.remove_option("Settings", "gtk-modules")
    settings_file.parent.mkdir(parents=True, exist_ok=True)
    with open(settings_file, "w") as f:
        cp.write(f)


def _update_xsettingsd(conf_path, theme_name, icon_theme):
    """Ensure ~/.config/xsettingsd/xsettingsd.conf specifies the active GTK and icon theme."""
    lines = []
    if conf_path.is_file():
        try:
            lines = conf_path.read_text().splitlines()
        except OSError:
            lines = []
    new_lines = []
    has_theme = False
    has_icon = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("Net/ThemeName"):
            new_lines.append('Net/ThemeName "%s"' % theme_name)
            has_theme = True
        elif stripped.startswith("Net/IconThemeName"):
            new_lines.append('Net/IconThemeName "%s"' % icon_theme)
            has_icon = True
        else:
            new_lines.append(line)
    if not has_theme:
        new_lines.append('Net/ThemeName "%s"' % theme_name)
    if not has_icon:
        new_lines.append('Net/IconThemeName "%s"' % icon_theme)
    conf_path.parent.mkdir(parents=True, exist_ok=True)
    conf_path.write_text("\n".join(new_lines) + "\n")


def _update_kdeglobals(kdeglobals, sections, icon_theme, primary_hex):
    """Update KDE color schemes and icon settings in ~/.config/kdeglobals preserving custom user options."""
    cp = configparser.RawConfigParser(strict=False)
    cp.optionxform = str
    if kdeglobals.is_file():
        try:
            cp.read(str(kdeglobals))
        except Exception:
            pass
    for header, fields in sections:
        sec = header.strip("[]")
        if not cp.has_section(sec):
            cp.add_section(sec)
        for k, v in fields.items():
            val = _rgb(v) if v.startswith("#") else v
            cp.set(sec, k, val)
    if not cp.has_section("General"):
        cp.add_section("General")
    cp.set("General", "ColorScheme", "Xiu")
    cp.set("General", "AccentColor", _rgb(primary_hex))
    if not cp.has_section("Icons"):
        cp.add_section("Icons")
    cp.set("Icons", "Theme", icon_theme)
    kdeglobals.parent.mkdir(parents=True, exist_ok=True)
    with open(kdeglobals, "w") as f:
        cp.write(f)


def render_gtk(pill):
    """GTK named colors: the adw-gtk3 theme (set through gsettings, settings.ini,
    and xsettingsd) picks these up, so GTK apps follow the palette."""
    is_dark = rel_luminance(pill["surface"]) < 0.40
    theme_name = "adw-gtk3-dark" if is_dark else "adw-gtk3"
    color_scheme = "prefer-dark" if is_dark else "prefer-light"
    icon_theme = get_active_icon_theme(is_dark)
    gnome_accent = gnome_accent_color(pill["primary"])
    accent_fg = "#000000" if rel_luminance(pill["primary"]) > 0.45 else "#ffffff"

    css_lines = [
        "/* Written by wallcolors.py on every palette change. */",
        "@define-color accent_color %s;" % pill["primary"],
        "@define-color accent_bg_color %s;" % pill["primary"],
        "@define-color accent_fg_color %s;" % accent_fg,
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
        "@define-color theme_selected_bg_color alpha(@accent_color, 0.25);",
        "@define-color theme_selected_fg_color %s;" % pill["primary"],
        "@define-color theme_bg_color @window_bg_color;",
        "@define-color theme_fg_color @window_fg_color;",
        "@define-color theme_base_color @view_bg_color;",
        "@define-color theme_text_color @view_fg_color;",
        "",
        "/* Concrete widget selectors to guarantee recoloring across GTK engines */",
        "window, .background {",
        "    background-color: @window_bg_color;",
        "    color: @window_fg_color;",
        "}",
        "headerbar, .titlebar {",
        "    background-color: @headerbar_bg_color;",
        "    color: @headerbar_fg_color;",
        "}",
        "view, .view, textview text {",
        "    background-color: @view_bg_color;",
        "    color: @view_fg_color;",
        "}",
        "popover, .popover, menu, .menu {",
        "    background-color: @popover_bg_color;",
        "    color: @popover_fg_color;",
        "}",
        "card, .card {",
        "    background-color: @card_bg_color;",
        "    color: @card_fg_color;",
        "}",
        "button.suggested-action {",
        "    background-color: @accent_bg_color;",
        "    color: @accent_fg_color;",
        "}",
        "button.suggested-action:hover {",
        "    background-color: alpha(@accent_bg_color, 0.85);",
        "}",
        "switch:checked {",
        "    background-color: @accent_bg_color;",
        "    color: @accent_fg_color;",
        "}",
        "selection, *:selected {",
        "    background-color: @theme_selected_bg_color;",
        "    color: @window_fg_color;",
        "}",
    ]
    css = "\n".join(css_lines) + "\n"

    for ver in ("gtk-3.0", "gtk-4.0"):
        d = Path.home() / ".config" / ver
        d.mkdir(parents=True, exist_ok=True)
        (d / "gtk.css").write_text(css)
        _update_gtk_settings(d / "settings.ini", theme_name, icon_theme, is_dark)

    xsettings_conf = Path.home() / ".config" / "xsettingsd" / "xsettingsd.conf"
    _update_xsettingsd(xsettings_conf, theme_name, icon_theme)
    subprocess.run(["killall", "-HUP", "xsettingsd"], stderr=subprocess.DEVNULL)

    if shutil.which("gsettings"):
        for key, value in (("color-scheme", color_scheme),
                           ("gtk-theme", theme_name),
                           ("icon-theme", icon_theme),
                           ("accent-color", gnome_accent)):
            subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", key, value],
                           stderr=subprocess.DEVNULL)


def render_qt(pill):
    """Qt via qtengine + Darkly / KDE Plasma theme: a palette-derived KDE color scheme
    plus qtengine and kdeglobals configs."""
    is_dark = rel_luminance(pill["surface"]) < 0.40
    icon_theme = get_active_icon_theme(is_dark)
    p = pill
    sections = [
        ("[Colors:View]", {
            "BackgroundNormal": p["surface_container"], "ForegroundNormal": p["cream"],
            "BackgroundAlternate": p["surface_container_low"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
            "ForegroundActive": p["primary"],
        }),
        ("[Colors:Window]", {
            "BackgroundNormal": p["surface"], "ForegroundNormal": p["cream"],
            "BackgroundAlternate": p["surface_container"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
            "ForegroundActive": p["primary"],
        }),
        ("[Colors:Button]", {
            "BackgroundNormal": p["surface_container_high"], "ForegroundNormal": p["cream"],
            "BackgroundAlternate": p["surface_container_highest"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
            "ForegroundActive": p["primary"],
        }),
        ("[Colors:Selection]", {
            "BackgroundNormal": p["primary_container"], "ForegroundNormal": p["on_primary_container"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
            "ForegroundActive": p["cream"],
        }),
        ("[Colors:Tooltip]", {
            "BackgroundNormal": p["surface_container_highest"], "ForegroundNormal": p["cream"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
        }),
        ("[Colors:Header]", {
            "BackgroundNormal": p["surface_container"], "ForegroundNormal": p["cream"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
        }),
        ("[Colors:Complementary]", {
            "BackgroundNormal": p["surface_container_low"], "ForegroundNormal": p["cream"],
            "DecorationFocus": p["primary"], "DecorationHover": p["primary"],
        }),
        ("[WM]", {
            "activeBackground": p["surface"], "activeForeground": p["cream"],
            "inactiveBackground": p["surface_container_low"], "inactiveForeground": p["dim"],
        }),
        ("[General]", {
            "ColorScheme": "Xiu",
            "AccentColor": p["primary"],
        }),
        ("[Icons]", {
            "Theme": icon_theme,
        }),
    ]
    lines = ["# Written by wallcolors.py on every palette change."]
    for header, fields in sections:
        lines.append("")
        lines.append(header)
        for key, value in fields.items():
            lines.append("%s=%s" % (key, _rgb(value) if value.startswith("#") else value))
    colors_content = "\n".join(lines) + "\n"

    # 1. QtEngine config and colors
    d_qtengine = Path.home() / ".config" / "qtengine"
    plugin6 = Path("/usr/lib/qt6/plugins/platformthemes/libqt6engine-plugin.so")
    plugin5 = Path("/usr/lib/qt5/plugins/platformthemes/libqt5engine-plugin.so")
    if d_qtengine.is_dir() or plugin6.is_file() or plugin5.is_file():
        d_qtengine.mkdir(parents=True, exist_ok=True)
        (d_qtengine / "xiu.colors").write_text(colors_content)
        config = d_qtengine / "config.json"
        cfg_data = {}
        if config.is_file():
            try:
                cfg_data = json.loads(config.read_text())
            except Exception:
                cfg_data = {}
        theme_cfg = cfg_data.setdefault("theme", {})
        theme_cfg["colorScheme"] = str(d_qtengine / "xiu.colors")
        theme_cfg["iconTheme"] = icon_theme
        theme_cfg.setdefault("style", "Darkly")
        misc_cfg = cfg_data.setdefault("misc", {})
        misc_cfg.setdefault("menusHaveIcons", True)
        misc_cfg.setdefault("singleClickActivate", False)
        config.write_text(json.dumps(cfg_data, indent=4) + "\n")
        # Notify running qtengine apps via DBus
        subprocess.run(["dbus-send", "--session", "--type=signal", "/",
                        "org.qtengine.ConfigWatcher.configChanged"],
                       stderr=subprocess.DEVNULL)

    # 2. KDE color scheme & kdeglobals
    d_kde_schemes = Path.home() / ".local" / "share" / "color-schemes"
    d_kde_schemes.mkdir(parents=True, exist_ok=True)
    (d_kde_schemes / "Xiu.colors").write_text(colors_content)

    kdeglobals = Path.home() / ".config" / "kdeglobals"
    _update_kdeglobals(kdeglobals, sections, icon_theme, p["primary"])

    if shutil.which("kwriteconfig6"):
        subprocess.run(["kwriteconfig6", "--file", "kdeglobals", "--group", "General",
                        "--key", "ColorScheme", "Xiu", "--notify"],
                       stderr=subprocess.DEVNULL)
        subprocess.run(["kwriteconfig6", "--file", "kdeglobals", "--group", "Icons",
                        "--key", "Theme", icon_theme, "--notify"],
                       stderr=subprocess.DEVNULL)

    if shutil.which("plasma-apply-colorscheme"):
        # If Xiu is already active, plasma-apply-colorscheme skips reloading Xiu.colors.
        # Toggle briefly to Breeze to force a full scheme reload of Xiu.
        fallback_theme = "BreezeDark" if is_dark else "BreezeLight"
        subprocess.run(["plasma-apply-colorscheme", fallback_theme],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(["plasma-apply-colorscheme", "Xiu"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(["plasma-apply-colorscheme", "-a", p["primary"]],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    # Broadcast KGlobalSettings PaletteChanged notification (type 0, arg 0) and IconChanged (type 1)
    subprocess.run(["dbus-send", "--session", "--type=signal", "/KGlobalSettings",
                    "org.kde.KGlobalSettings.notifyChange", "int32:0", "int32:0"],
                   stderr=subprocess.DEVNULL)
    subprocess.run(["dbus-send", "--session", "--type=signal", "/KGlobalSettings",
                    "org.kde.KGlobalSettings.notifyChange", "int32:1", "int32:0"],
                   stderr=subprocess.DEVNULL)


def fan_out(pill, seed, variant, share=None):
    """Write the pill JSON, recolour fastfetch, and build the terminal/border
    base16 through matugen with the resolved scheme type — then run the
    semantic layer over it, so the terminal's 16 slots and every TUI renderer
    read a scheme with fixed luminance bands, chroma ceilings, WCAG floors
    and hue-bent statuses instead of matugen's raw dump."""
    CACHE.mkdir(parents=True, exist_ok=True)
    CACHE_XIU.mkdir(parents=True, exist_ok=True)
    c_json = json.dumps(pill, indent=2) + "\n"
    (CACHE / "colors.json").write_text(c_json)
    (CACHE_XIU / "colors.json").write_text(c_json)
    render_fastfetch(pill)
    render_starship(pill)

    try:
        b = {k: v["dark"]["color"] for k, v in
             matugen(seed, variant)["base16"].items()}
    except (OSError, ValueError, KeyError, subprocess.SubprocessError):
        return 0

    b_json = json.dumps(b, indent=2) + "\n"
    (CACHE / "base16.json").write_text(b_json)
    (CACHE_XIU / "base16.json").write_text(b_json)

    ansi = semantic_terminal(pill, b, seed, share)
    render_fish(pill, ansi)
    broadcast_terminal(pill, b, ansi)

    hypr_colors = (
        'return {\n'
        '    active = "%s",\n'
        '    inactive = "%s",\n'
        '    locked_active = "%s",\n'
        '    locked_inactive = "%s",\n'
        '    group_active = "%s",\n'
        '    group_inactive = "%s",\n'
        '    group_locked_active = "%s",\n'
        '    group_locked_inactive = "%s",\n'
        '    text_color = "%s",\n'
        '}\n'
        % (
            pill["primary"],
            b["base01"],
            pill.get("on_primary_container", b.get("base09", "#f0b85e")),
            b["base02"],
            pill["primary"],
            pill.get("surface_container", b["base01"]),
            pill.get("on_primary_container", b.get("base09", "#f0b85e")),
            b["base02"],
            pill.get("cream", b["base07"]),
        )
    )
    (CACHE / "hypr-colors.lua").write_text(hypr_colors)
    (CACHE_XIU / "hypr-colors.lua").write_text(hypr_colors)

    lines = [
        f'background = {b["base00"]}',
        f'foreground = {b["base07"]}',
        f'cursor-color = {pill["primary"]}',
        f'selection-background = {b["base02"]}',
        f'selection-foreground = {b["base07"]}',
    ]
    for i, hex_color in enumerate(ansi):
        lines.append(f'palette = {i}={hex_color}')
    (CACHE / "ghostty-colors").write_text("\n".join(lines) + "\n")
    render_foot(pill, b, ansi)
    render_btop(pill, b)
    render_htop(pill, b)
    render_nvtop(pill, b)
    render_cava(pill, b)
    render_micro(pill, b)
    render_helix(pill, b)
    render_bottom(pill, b)
    render_yazi(pill, b)
    render_discord(pill)
    render_userchrome(pill)
    render_vscode(pill)
    render_zed(pill, b)
    render_browser(pill)
    render_qt(pill)
    render_gtk(pill)
    render_user_templates(pill, b)
    return 0


def render_user_templates(pill, b):
    """caelestia's escape hatch: every file in ~/.config/xiu/templates/ gets
    `{{ $token }}` filled from the pill (and `{{ $baseNN }}` from the
    terminal's base16) and lands beside the template with its extension kept
    minus `.in`. Whatever app config the user points here follows the palette
    with zero code — the last mile for apps this pipeline does not know. The
    directory is absent by default and never created by the pipeline: it is
    purely the user's own.
    """
    src = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))) / "xiu" / "templates"
    if not src.is_dir():
        return
    tokens = dict(pill)
    for k, v in b.items():
        tokens[k] = v
    for f in src.iterdir():
        if not f.is_file() or not f.name.endswith(".in"):
            continue
        try:
            text = f.read_text()
            for k, v in tokens.items():
                text = text.replace("{{ $%s }}" % k, v.lstrip("#"))
            out = f.with_name(f.name[:-3])
            out.write_text(text)
        except OSError:
            continue


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
        pill, seed, variant, _share = generate_dynamic(args[1], "auto", True)
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

    pill, seed, resolved, share = generate_dynamic(wallpaper, variant, smart)
    return fan_out(pill, seed, resolved, share)


if __name__ == "__main__":
    sys.exit(main())
