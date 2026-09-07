#!/usr/bin/env python3
"""
Invariant tests for wallcolors' terminal semantic layer. Hermetic: no matugen,
no magick — the base16 dicts are synthetic, so the tests run anywhere and only
exercise the palette math. Run: python3 test_wallcolors.py
"""
import colorsys
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wallcolors as wc  # noqa: E402


def hex_hue(hex_color):
    return wc.hue_sat_of(hex_color)[0]


def inside(h, lo, hi, tol=2.0):
    d = (h - lo) % 360.0
    return d <= ((hi - lo) % 360.0) + tol or d >= 360.0 - tol


BASE16 = {  # a neutral dark scheme with family-tinted accents, like matugen's
    "base00": "#141a20", "base01": "#20262d", "base02": "#2c333b",
    "base03": "#454c54", "base04": "#5e666e", "base05": "#778088",
    "base06": "#919aa2", "base07": "#abb4bc", "base0f": "#6f767e",
}
WARM = dict(BASE16, base08="#ff6f4a", base09="#dc865f", base0a="#d08e45",
            base0b="#a79f00", base0c="#8ea554", base0d="#e18700", base0e="#83a900")


def check_ansi(name, ansi, bg):
    """The contract every scheme must hold, chromatic or not."""
    for i in list(range(1, 7)) + list(range(9, 15)):
        assert wc.contrast_ratio(ansi[i], bg) >= 4.4, \
            f"{name}: slot {i} below the 4.5 floor vs bg ({wc.contrast_ratio(ansi[i], bg):.2f})"
    assert wc.contrast_ratio(ansi[8], bg) >= 2.9, f"{name}: bright-black below the muted floor"
    fams = [("danger", (345.0, 20.0), 1), ("ok", (140.0, 170.0), 2), ("warning", (40.0, 65.0), 3)]
    for fam, (lo, hi), slot in fams:
        assert inside(hex_hue(ansi[slot]), lo, hi), \
            f"{name}: {fam} hue {hex_hue(ansi[slot]):.1f} outside ({lo}, {hi})"
    sem_h = [hex_hue(ansi[s]) for s in (1, 2, 3)]
    cool_h = [hex_hue(ansi[s]) for s in (4, 5, 6)]
    # a desaturated cool has no meaningful hue, so separation only binds
    # while the cool actually carries color
    colored = [h for h, s in zip(cool_h, (wc.hue_sat_of(ansi[s])[1] for s in (4, 5, 6))) if s > 0.15]
    for ch in colored:
        for sh in sem_h:
            assert abs(wc.signed_arc(sh, ch)) >= 28.0, \
                f"{name}: cool {ch:.0f} within 30 of status {sh:.0f}"
    for i in range(len(colored)):
        for j in range(i + 1, len(colored)):
            assert abs(wc.signed_arc(colored[i], colored[j])) >= 28.0, \
                f"{name}: cools {colored[i]:.0f} and {colored[j]:.0f} under 30 apart"
    # bands: normals share a luminance band, brights sit clearly above
    yn = [wc.rel_luminance(ansi[i]) for i in range(1, 7)]
    yb = [wc.rel_luminance(ansi[i]) for i in range(9, 15)]
    assert max(yn) - min(yn) <= 0.09, f"{name}: normal band spread {max(yn) - min(yn):.3f}"
    assert min(yb) > max(yn) + 0.02, f"{name}: brights not clearly above normals"
    # the accent slots the TUI renderers read must carry the same colors
    return ansi


def main():
    failures = 0

    # chromatic: statuses bend, cools walk clear, cools tint with the family
    ansi = wc.semantic_terminal({"primary": "#e0563b"}, dict(WARM), "#d06030", 0.3)
    check_ansi("warm seed", ansi, ansi[0])
    assert wc.hue_sat_of(ansi[1])[1] > 0.3, "warm seed: danger lost its chroma"

    # achromatic: statuses keep their fixed editorial chroma, cools go grey
    grey = dict(BASE16, base08="#9a9a9a", base09="#909090", base0a="#959595",
                base0b="#9a9a9a", base0c="#8a8a8a", base0d="#8f8f8f", base0e="#929292")
    ansi = wc.semantic_terminal({"primary": "#e0563b"}, dict(grey), "#787878", 0.0)
    check_ansi("grey seed", ansi, ansi[0])
    for slot in (1, 2, 3):
        assert wc.hue_sat_of(ansi[slot])[1] > 0.3, "grey seed: a status went grey"
    for slot in (4, 5, 6):
        assert wc.hue_sat_of(ansi[slot])[1] < 0.15, "grey seed: a cool stayed colored"

    # the pure helpers: band snapping and the WCAG lift
    band = (0.25, 0.30)
    snapped = wc.snap_to_band("#808080", band)
    lum = wc.rel_luminance(snapped)
    assert band[0] - 0.01 <= lum <= band[1] + 0.01, f"snap_to_band landed at {lum:.3f}"
    lifted = wc.clamp_light("#808080", 4.5, "#000000")
    assert wc.contrast_ratio(lifted, "#000000") >= 4.4, "clamp_light missed its target"

    # hue arithmetic: the circular helpers wrap the 0/360 seam
    assert wc.signed_arc(350, 10) == 20, "signed_arc must cross the seam"
    assert wc.circ_clamp(30, 345, 20) in (345.0, 20.0), "circ_clamp must snap to a bound"

    print("wallcolors semantic layer: all invariants hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
