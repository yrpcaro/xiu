//! Color space mathematics, WCAG contrast calculation, and terminal semantic palette generation.

use std::collections::HashMap;

pub const VOICE_CONTRAST: f64 = 4.5;
pub const VOICE_WIDTH: f64 = 0.05;
pub const LIGHT_CONTRAST: f64 = 6.0;
pub const LIGHT_WIDTH: f64 = 0.06;
pub const GREEN_ZONE: (f64, f64) = (90.0, 200.0);
pub const GREEN_ZONE_PENALTY: f64 = 0.15;

pub const ACC_SAT_CAP: f64 = 0.65;
pub const RAMP_LO: f64 = 0.08;
pub const RAMP_HI: f64 = 0.20;

pub const SEMANTIC_BEND: f64 = 15.0;
pub const SEMANTIC_SAT: f64 = 0.55;

pub const ANSI_FLOOR: f64 = 4.5;
pub const ANSI_FLOOR_MUTED: f64 = 3.0;
pub const COOL_MIN_SEP: f64 = 30.0;
pub const COOL_SPREAD: f64 = 40.0;

pub fn hex_to_rgb(hex: &str) -> (f64, f64, f64) {
    let s = hex.trim().trim_start_matches('#');
    if s.len() < 6 {
        return (0.0, 0.0, 0.0);
    }
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0) as f64 / 255.0;
    (r, g, b)
}

pub fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    let ir = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let ig = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let ib = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{ir:02x}{ig:02x}{ib:02x}")
}

pub fn rgb_to_hls(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let maxc = r.max(g).max(b);
    let minc = r.min(g).min(b);
    let sumc = maxc + minc;
    let rangec = maxc - minc;
    let l = sumc / 2.0;
    if rangec == 0.0 {
        return (0.0, l, 0.0);
    }
    let s = if l <= 0.5 {
        rangec / sumc
    } else {
        rangec / (2.0 - sumc)
    };
    let rc = (maxc - r) / rangec;
    let gc = (maxc - g) / rangec;
    let bc = (maxc - b) / rangec;
    let h = if r == maxc {
        bc - gc
    } else if g == maxc {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    let mut h = (h / 6.0) % 1.0;
    if h < 0.0 {
        h += 1.0;
    }
    (h, l, s)
}

fn v_helper(m1: f64, m2: f64, mut hue: f64) -> f64 {
    hue = hue % 1.0;
    if hue < 0.0 {
        hue += 1.0;
    }
    if hue < 1.0 / 6.0 {
        m1 + (m2 - m1) * hue * 6.0
    } else if hue < 0.5 {
        m2
    } else if hue < 2.0 / 3.0 {
        m1 + (m2 - m1) * (2.0 / 3.0 - hue) * 6.0
    } else {
        m1
    }
}

pub fn hls_to_rgb(mut h: f64, l: f64, s: f64) -> (f64, f64, f64) {
    h = h % 1.0;
    if h < 0.0 {
        h += 1.0;
    }
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    if s == 0.0 {
        return (l, l, l);
    }
    let m2 = if l <= 0.5 {
        l * (1.0 + s)
    } else {
        l + s - (l * s)
    };
    let m1 = 2.0 * l - m2;
    (
        v_helper(m1, m2, h + 1.0 / 3.0),
        v_helper(m1, m2, h),
        v_helper(m1, m2, h - 1.0 / 3.0),
    )
}

pub fn tint(hue: f64, sat: f64, light: f64) -> String {
    let (r, g, b) = hls_to_rgb(hue, light, sat);
    rgb_to_hex(r, g, b)
}

pub fn tint_deg(h_deg: f64, s: f64, l: f64) -> String {
    let mut h = (h_deg % 360.0) / 360.0;
    if h < 0.0 {
        h += 1.0;
    }
    tint(h, s, l)
}

pub fn lerp(x: f64, x0: f64, x1: f64, y0: f64, y1: f64) -> f64 {
    let t = ((x - x0) / (x1 - x0)).clamp(0.0, 1.0);
    y0 + t * (y1 - y0)
}

pub fn hue_sat_of(hex_color: &str) -> (f64, f64) {
    let (r, g, b) = hex_to_rgb(hex_color);
    let (h, _l, s) = rgb_to_hls(r, g, b);
    (h * 360.0, s)
}

fn linearize(c8: f64) -> f64 {
    let c = c8 / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn rel_luminance(hex_color: &str) -> f64 {
    let s = hex_color.trim().trim_start_matches('#');
    if s.len() < 6 {
        return 0.0;
    }
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0) as f64;
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0) as f64;
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0) as f64;
    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

pub fn contrast_ratio(hex_a: &str, hex_b: &str) -> f64 {
    let la = rel_luminance(hex_a);
    let lb = rel_luminance(hex_b);
    let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

pub fn signed_arc(a_deg: f64, b_deg: f64) -> f64 {
    let mut d = (b_deg - a_deg) % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

pub fn circ_clamp(h_deg: f64, lo_deg: f64, hi_deg: f64) -> f64 {
    let mut d1 = (h_deg - lo_deg) % 360.0;
    if d1 < 0.0 {
        d1 += 360.0;
    }
    let mut d2 = (hi_deg - lo_deg) % 360.0;
    if d2 < 0.0 {
        d2 += 360.0;
    }
    if d1 <= d2 {
        let mut h = h_deg % 360.0;
        if h < 0.0 {
            h += 360.0;
        }
        return h;
    }
    if signed_arc(h_deg, lo_deg).abs() <= signed_arc(h_deg, hi_deg).abs() {
        let mut l = lo_deg % 360.0;
        if l < 0.0 {
            l += 360.0;
        }
        l
    } else {
        let mut h = hi_deg % 360.0;
        if h < 0.0 {
            h += 360.0;
        }
        h
    }
}

pub fn band(target_contrast: f64, base_hex: &str, width: f64) -> (f64, f64) {
    let y = target_contrast * (rel_luminance(base_hex) + 0.05) - 0.05;
    (y, y + width)
}

pub fn snap_to_band(hex_color: &str, band_tuple: (f64, f64)) -> String {
    let (lo, hi) = band_tuple;
    let lum = rel_luminance(hex_color);
    if lum >= lo && lum <= hi {
        return hex_color.to_string();
    }
    let (r, g, b) = hex_to_rgb(hex_color);
    let (h, _l, s) = rgb_to_hls(r, g, b);
    let target = (lo + hi) / 2.0;
    let mut lo_l = 0.0;
    let mut hi_l = 1.0;
    for _ in 0..40 {
        let mid = (lo_l + hi_l) / 2.0;
        if rel_luminance(&tint(h, s, mid)) < target {
            lo_l = mid;
        } else {
            hi_l = mid;
        }
    }
    tint(h, s, hi_l)
}

pub fn sat_cap(h_deg: f64, base_cap: f64) -> f64 {
    let mut h = h_deg % 360.0;
    if h < 0.0 {
        h += 360.0;
    }
    if h >= GREEN_ZONE.0 && h <= GREEN_ZONE.1 {
        (base_cap - GREEN_ZONE_PENALTY).max(0.05)
    } else {
        base_cap
    }
}

pub fn chroma_ramp(share: f64) -> f64 {
    if share <= RAMP_LO {
        0.0
    } else if share >= RAMP_HI {
        1.0
    } else {
        (share - RAMP_LO) / (RAMP_HI - RAMP_LO)
    }
}

pub fn clamp_light(hex_color: &str, target: f64, bg_hex: &str) -> String {
    if contrast_ratio(hex_color, bg_hex) >= target {
        return hex_color.to_string();
    }
    let (r, g, b) = hex_to_rgb(hex_color);
    let (h, l, s) = rgb_to_hls(r, g, b);
    if contrast_ratio(&tint(h, s, 1.0), bg_hex) < target {
        return tint(h, s, 1.0);
    }
    let mut lo = l;
    let mut hi = 1.0;
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if contrast_ratio(&tint(h, s, mid), bg_hex) >= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    tint(h, s, hi)
}

pub fn bend_semantic(base_hue_deg: f64, dominant_deg: f64, bounds: (f64, f64)) -> f64 {
    let d = signed_arc(base_hue_deg, dominant_deg);
    let bent = base_hue_deg + d.clamp(-SEMANTIC_BEND, SEMANTIC_BEND);
    circ_clamp(bent, bounds.0, bounds.1)
}

pub fn semantic_terminal(
    _pill: &HashMap<String, String>,
    b: &mut HashMap<String, String>,
    seed: &str,
    share: Option<f64>,
) -> Vec<String> {
    let bg = b.get("base00").cloned().unwrap_or_else(|| "#141a20".to_string());
    let fg = b.get("base07").cloned().unwrap_or_else(|| "#abb4bc".to_string());
    let (dom, dom_sat) = hue_sat_of(seed);
    let chromatic = dom_sat > 0.02;
    let eff_share = share.unwrap_or(if chromatic { 1.0 } else { 0.0 });
    let ramp = if chromatic { chroma_ramp(eff_share) } else { 0.0 };

    let danger_bounds = (345.0, 20.0);
    let ok_bounds = (140.0, 170.0);
    let warning_bounds = (40.0, 65.0);

    let sem_danger = if chromatic { bend_semantic(0.0, dom, danger_bounds) } else { 0.0 };
    let sem_ok = if chromatic { bend_semantic(160.0, dom, ok_bounds) } else { 160.0 };
    let sem_warning = if chromatic { bend_semantic(55.0, dom, warning_bounds) } else { 55.0 };

    let clear_walk = |start_h: f64, avoid: &[f64]| -> f64 {
        let mut h = start_h % 360.0;
        if h < 0.0 {
            h += 360.0;
        }
        for _ in 0..360 {
            if avoid.iter().all(|&a| signed_arc(a, h).abs() >= COOL_MIN_SEP) {
                return h;
            }
            h = (h + 1.0) % 360.0;
        }
        h
    };

    let sems = [sem_danger, sem_ok, sem_warning];
    let blue = if chromatic { clear_walk(dom, &sems) } else { dom };
    let magenta = if chromatic { clear_walk((dom - COOL_SPREAD + 360.0) % 360.0, &[sem_danger, sem_ok, sem_warning, blue]) } else { dom };
    let cyan = if chromatic { clear_walk((dom + COOL_SPREAD) % 360.0, &[sem_danger, sem_ok, sem_warning, blue, magenta]) } else { dom };

    let hues = [sem_danger, sem_ok, sem_warning, blue, magenta, cyan];
    let voice = band(VOICE_CONTRAST, &bg, VOICE_WIDTH);
    let light = band(LIGHT_CONTRAST, &bg, LIGHT_WIDTH);

    let slot = |h_deg: f64, band_t: (f64, f64), is_semantic: bool| -> String {
        let s = if is_semantic {
            sat_cap(h_deg, SEMANTIC_SAT)
        } else if chromatic {
            sat_cap(h_deg, ACC_SAT_CAP) * ramp
        } else {
            0.05
        };
        let c = snap_to_band(&tint_deg(h_deg, s, 0.55), band_t);
        clamp_light(&c, ANSI_FLOOR, &bg)
    };

    let normals: Vec<String> = hues.iter().enumerate().map(|(i, &h)| slot(h, voice, i < 3)).collect();
    let brights: Vec<String> = hues.iter().enumerate().map(|(i, &h)| slot(h, light, i < 3)).collect();

    let mut ansi = Vec::with_capacity(16);
    ansi.push(bg.clone());
    ansi.extend(normals);
    let base03 = b.get("base03").cloned().unwrap_or_else(|| "#454c54".to_string());
    ansi.push(clamp_light(&fg, ANSI_FLOOR, &bg)); // 7: white
    ansi.push(clamp_light(&base03, ANSI_FLOOR_MUTED, &bg)); // 8: bright black
    ansi.extend(brights); // 9-14
    ansi.push(clamp_light(&fg, LIGHT_CONTRAST, &bg)); // 15: bright white

    let base09 = b.get("base09").cloned().unwrap_or_else(|| "#dc865f".to_string());
    let (o_hue, _o_s) = hue_sat_of(&base09);
    let o_sat = if chromatic { sat_cap(o_hue, ACC_SAT_CAP) * ramp } else { 0.05 };

    b.insert("base08".to_string(), ansi[1].clone());
    b.insert("base0a".to_string(), ansi[3].clone());
    b.insert("base0b".to_string(), ansi[2].clone());
    b.insert("base0c".to_string(), ansi[6].clone());
    b.insert("base0d".to_string(), ansi[4].clone());
    b.insert("base0e".to_string(), ansi[5].clone());
    let o_clamped = clamp_light(&snap_to_band(&tint_deg(o_hue, o_sat, 0.55), voice), ANSI_FLOOR, &bg);
    b.insert("base09".to_string(), o_clamped);

    ansi
}

pub fn gnome_accent_color(hex_color: &str) -> &'static str {
    let (h, s) = hue_sat_of(hex_color);
    if s < 0.15 {
        return "slate";
    }
    if h >= 345.0 || h < 15.0 {
        "red"
    } else if h < 45.0 {
        "orange"
    } else if h < 70.0 {
        "yellow"
    } else if h < 150.0 {
        "green"
    } else if h < 190.0 {
        "teal"
    } else if h < 255.0 {
        "blue"
    } else if h < 290.0 {
        "purple"
    } else {
        "pink"
    }
}

pub const SURF_NAMES: &[&str] = &[
    "surface",
    "surface_container_low",
    "surface_container",
    "surface_container_high",
    "surface_container_highest",
    "outline_variant",
];
pub const DARK_STEPS: &[f64] = &[0.0, 0.022, 0.038, 0.065, 0.100, 0.225];
pub const LIGHT_STEPS: &[f64] = &[0.0, -0.045, -0.075, -0.115, -0.160, -0.340];
pub const TEXT_KEYS: &[&str] = &[
    "cream", "bright", "subtle", "dim", "faint", "icon_dim", "tick_rest",
];
pub const DARK_TEXT: &[(f64, f64)] = &[
    (0.90, 0.05),
    (0.97, 0.03),
    (0.73, 0.07),
    (0.54, 0.06),
    (0.44, 0.05),
    (0.81, 0.07),
    (0.75, 0.08),
];
pub const LIGHT_TEXT: &[(f64, f64)] = &[
    (0.20, 0.18),
    (0.10, 0.20),
    (0.36, 0.14),
    (0.48, 0.10),
    (0.56, 0.08),
    (0.28, 0.12),
    (0.34, 0.12),
];
pub const VARIANTS: &[&str] = &[
    "auto",
    "content",
    "expressive",
    "fidelity",
    "fruit-salad",
    "monochrome",
    "neutral",
    "rainbow",
    "tonal-spot",
    "vibrant",
];

pub fn accent_mult(variant: &str) -> f64 {
    match variant {
        "monochrome" => 0.45,
        "neutral" => 0.55,
        "content" => 0.85,
        "fidelity" => 0.90,
        "tonal-spot" => 1.0,
        "vibrant" => 1.25,
        "expressive" => 1.30,
        "rainbow" => 1.35,
        "fruit-salad" => 1.30,
        _ => 1.0,
    }
}

pub fn generate_manual(
    mut hue: f64,
    mode: &str,
    sat_arg: f64,
    variant_arg: &str,
) -> (HashMap<String, String>, String, String) {
    let mut sat = sat_arg.clamp(0.0, 1.0);
    let mean_l = if mode == "light" { 0.85 } else { 0.12 };
    let chromatic = sat > 0.02;
    if !chromatic {
        hue = 0.0;
        sat = 0.0;
    }
    let mut variant = variant_arg.to_string();
    if variant == "auto" {
        variant = if !chromatic {
            "neutral".to_string()
        } else {
            "tonal-spot".to_string()
        };
    }

    let light = mean_l >= 0.40;
    let surf_sat = if light {
        sat.min(0.26)
    } else {
        (if chromatic { sat.max(0.30) } else { 0.0 }).min(0.45)
    };
    let mut acc_sat = if chromatic {
        if light {
            (sat + 0.18).min(0.85)
        } else {
            (sat.max(0.30) + 0.12).min(0.82)
        }
    } else {
        0.0
    };
    acc_sat = (acc_sat * accent_mult(&variant)).min(0.95);

    let (base, steps, text, acc_l, deep_l, glow_l) = if light {
        (
            lerp(mean_l, 0.40, 0.66, 0.80, 0.93),
            LIGHT_STEPS,
            LIGHT_TEXT,
            0.42,
            0.30,
            0.55,
        )
    } else {
        (
            lerp(mean_l, 0.0, 0.40, 0.045, 0.20),
            DARK_STEPS,
            DARK_TEXT,
            0.70,
            0.34,
            0.86,
        )
    };

    let mut pill = HashMap::new();
    for (name, step) in SURF_NAMES.iter().zip(steps.iter()) {
        pill.insert(name.to_string(), tint(hue, surf_sat, base + step));
    }
    pill.insert("primary".to_string(), tint(hue, acc_sat, acc_l));
    pill.insert(
        "primary_container".to_string(),
        tint(hue, if chromatic { (acc_sat + 0.08).min(0.9) } else { 0.0 }, deep_l),
    );
    pill.insert(
        "on_primary_container".to_string(),
        tint(hue, if chromatic { acc_sat.min(0.45) } else { 0.0 }, glow_l),
    );
    pill.insert(
        "outline".to_string(),
        tint(hue, surf_sat, base + if light { -0.35 } else { 0.35 }),
    );
    for (key, (lit, st)) in TEXT_KEYS.iter().zip(text.iter()) {
        pill.insert(key.to_string(), tint(hue, if chromatic { *st } else { 0.0 }, *lit));
    }

    let seed = if chromatic {
        tint(hue, sat, 0.45)
    } else {
        "#787878".to_string()
    };
    (pill, seed, variant)
}

pub fn generate_dynamic_palette(
    hue_opt: Option<f64>,
    mut sat: f64,
    mean_l: f64,
    variant_arg: &str,
    mode: &str,
) -> (HashMap<String, String>, String, String) {
    let chromatic = hue_opt.is_some();
    let mut hue = hue_opt.unwrap_or(0.0);
    if !chromatic {
        hue = 0.0;
        sat = 0.0;
    }
    let mut variant = variant_arg.to_string();
    if variant == "auto" {
        variant = if !chromatic {
            "neutral".to_string()
        } else {
            "tonal-spot".to_string()
        };
    }

    let light = if mode == "light" {
        true
    } else if mode == "dark" {
        false
    } else {
        mean_l >= 0.40
    };

    let surf_sat = if light {
        sat.min(0.26)
    } else {
        (if chromatic { sat.max(0.30) } else { 0.0 }).min(0.45)
    };
    let mut acc_sat = if chromatic {
        if light {
            (sat + 0.18).min(0.85)
        } else {
            (sat.max(0.30) + 0.12).min(0.82)
        }
    } else {
        0.0
    };
    acc_sat = (acc_sat * accent_mult(&variant)).min(0.95);

    let (base, steps, text, acc_l, deep_l, glow_l) = if light {
        (
            lerp(mean_l.max(0.40), 0.40, 0.66, 0.80, 0.93),
            LIGHT_STEPS,
            LIGHT_TEXT,
            0.42,
            0.30,
            0.55,
        )
    } else {
        (
            lerp(mean_l.min(0.40), 0.0, 0.40, 0.045, 0.20),
            DARK_STEPS,
            DARK_TEXT,
            0.70,
            0.34,
            0.86,
        )
    };

    let mut pill = HashMap::new();
    for (name, step) in SURF_NAMES.iter().zip(steps.iter()) {
        pill.insert(name.to_string(), tint(hue, surf_sat, base + step));
    }
    pill.insert("primary".to_string(), tint(hue, acc_sat, acc_l));
    pill.insert(
        "primary_container".to_string(),
        tint(hue, if chromatic { (acc_sat + 0.08).min(0.9) } else { 0.0 }, deep_l),
    );
    pill.insert(
        "on_primary_container".to_string(),
        tint(hue, if chromatic { acc_sat.min(0.45) } else { 0.0 }, glow_l),
    );
    pill.insert(
        "outline".to_string(),
        tint(hue, surf_sat, base + if light { -0.35 } else { 0.35 }),
    );
    for (key, (lit, st)) in TEXT_KEYS.iter().zip(text.iter()) {
        pill.insert(key.to_string(), tint(hue, if chromatic { *st } else { 0.0 }, *lit));
    }

    let seed = if chromatic {
        tint(hue, sat, 0.45)
    } else {
        "#787878".to_string()
    };
    (pill, seed, variant)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_hue(hex: &str) -> f64 {
        hue_sat_of(hex).0
    }

    fn inside(h: f64, lo: f64, hi: f64, tol: f64) -> bool {
        let d = (h - lo).rem_euclid(360.0);
        d <= (hi - lo).rem_euclid(360.0) + tol || d >= 360.0 - tol
    }

    fn base16_map() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("base00".into(), "#141a20".into());
        m.insert("base01".into(), "#20262d".into());
        m.insert("base02".into(), "#2c333b".into());
        m.insert("base03".into(), "#454c54".into());
        m.insert("base04".into(), "#5e666e".into());
        m.insert("base05".into(), "#778088".into());
        m.insert("base06".into(), "#919aa2".into());
        m.insert("base07".into(), "#abb4bc".into());
        m.insert("base0f".into(), "#6f767e".into());
        m
    }

    fn warm_base16() -> HashMap<String, String> {
        let mut m = base16_map();
        m.insert("base08".into(), "#ff6f4a".into());
        m.insert("base09".into(), "#dc865f".into());
        m.insert("base0a".into(), "#d08e45".into());
        m.insert("base0b".into(), "#a79f00".into());
        m.insert("base0c".into(), "#8ea554".into());
        m.insert("base0d".into(), "#e18700".into());
        m.insert("base0e".into(), "#83a900".into());
        m
    }

    fn grey_base16() -> HashMap<String, String> {
        let mut m = base16_map();
        m.insert("base08".into(), "#9a9a9a".into());
        m.insert("base09".into(), "#909090".into());
        m.insert("base0a".into(), "#959595".into());
        m.insert("base0b".into(), "#9a9a9a".into());
        m.insert("base0c".into(), "#8a8a8a".into());
        m.insert("base0d".into(), "#8f8f8f".into());
        m.insert("base0e".into(), "#929292".into());
        m
    }

    fn check_ansi(name: &str, ansi: &[String], bg: &str) {
        for i in (1..7).chain(9..15) {
            let cr = contrast_ratio(&ansi[i], bg);
            assert!(
                cr >= 4.4,
                "{name}: slot {i} below 4.5 floor vs bg ({cr:.2})"
            );
        }
        let cr8 = contrast_ratio(&ansi[8], bg);
        assert!(cr8 >= 2.9, "{name}: bright-black below muted floor ({cr8:.2})");

        let fams = [
            ("danger", (345.0, 20.0), 1),
            ("ok", (140.0, 170.0), 2),
            ("warning", (40.0, 65.0), 3),
        ];
        for (fam, (lo, hi), slot) in fams {
            let h = hex_hue(&ansi[slot]);
            assert!(
                inside(h, lo, hi, 2.0),
                "{name}: {fam} hue {h:.1} outside ({lo}, {hi})"
            );
        }

        let sem_h: Vec<f64> = (1..=3).map(|s| hex_hue(&ansi[s])).collect();
        let cool_h: Vec<f64> = (4..=6).map(|s| hex_hue(&ansi[s])).collect();
        let colored: Vec<f64> = cool_h
            .into_iter()
            .zip((4..=6).map(|s| hue_sat_of(&ansi[s]).1))
            .filter(|&(_, s)| s > 0.15)
            .map(|(h, _)| h)
            .collect();

        for ch in &colored {
            for sh in &sem_h {
                assert!(
                    signed_arc(*sh, *ch).abs() >= 28.0,
                    "{name}: cool {ch:.0} within 30 of status {sh:.0}"
                );
            }
        }
        for i in 0..colored.len() {
            for j in (i + 1)..colored.len() {
                assert!(
                    signed_arc(colored[i], colored[j]).abs() >= 28.0,
                    "{name}: cools under 30 apart"
                );
            }
        }

        let yn: Vec<f64> = (1..7).map(|i| rel_luminance(&ansi[i])).collect();
        let yb: Vec<f64> = (9..15).map(|i| rel_luminance(&ansi[i])).collect();
        let yn_max = yn.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let yn_min = yn.iter().cloned().fold(f64::INFINITY, f64::min);
        let yb_min = yb.iter().cloned().fold(f64::INFINITY, f64::min);

        assert!(
            yn_max - yn_min <= 0.091,
            "{name}: normal band spread {:.3}",
            yn_max - yn_min
        );
        assert!(yb_min > yn_max + 0.02, "{name}: brights not clearly above normals");
    }

    #[test]
    fn test_warm_seed_semantic() {
        let mut pill = HashMap::new();
        pill.insert("primary".into(), "#e0563b".into());
        let mut b = warm_base16();
        let ansi = semantic_terminal(&pill, &mut b, "#d06030", Some(0.3));
        check_ansi("warm seed", &ansi, &ansi[0]);
        assert!(hue_sat_of(&ansi[1]).1 > 0.3, "warm seed: danger lost chroma");
    }

    #[test]
    fn test_grey_seed_semantic() {
        let mut pill = HashMap::new();
        pill.insert("primary".into(), "#e0563b".into());
        let mut b = grey_base16();
        let ansi = semantic_terminal(&pill, &mut b, "#787878", Some(0.0));
        check_ansi("grey seed", &ansi, &ansi[0]);
        for slot in 1..=3 {
            assert!(
                hue_sat_of(&ansi[slot]).1 > 0.3,
                "grey seed: status went grey"
            );
        }
        for slot in 4..=6 {
            assert!(
                hue_sat_of(&ansi[slot]).1 < 0.15,
                "grey seed: cool stayed colored"
            );
        }
    }

    #[test]
    fn test_pure_helpers() {
        let band = (0.25, 0.30);
        let snapped = snap_to_band("#808080", band);
        let lum = rel_luminance(&snapped);
        assert!(
            lum >= band.0 - 0.01 && lum <= band.1 + 0.01,
            "snap_to_band landed at {lum:.3}"
        );

        let lifted = clamp_light("#808080", 4.5, "#000000");
        assert!(
            contrast_ratio(&lifted, "#000000") >= 4.4,
            "clamp_light missed target"
        );

        assert_eq!(signed_arc(350.0, 10.0), 20.0);
        let c = circ_clamp(30.0, 345.0, 20.0);
        assert!(c == 345.0 || c == 20.0);
    }

    #[test]
    fn test_achromatic_generation() {
        let (achro_pill, achro_seed, achro_var) = generate_manual(0.09, "dark", 0.0, "auto");
        assert_eq!(achro_var, "neutral");
        assert_eq!(achro_seed, "#787878");
        for k in &["surface", "primary", "cream", "bright", "dim"] {
            let (_, s) = hue_sat_of(achro_pill.get(*k).unwrap());
            assert!(s < 0.01, "slot {k} has unexpected saturation {s}");
        }
    }

    #[test]
    fn test_gnome_accent_colors() {
        assert_eq!(gnome_accent_color("#ff0000"), "red");
        assert_eq!(gnome_accent_color("#ff8800"), "orange");
        assert_eq!(gnome_accent_color("#ffff00"), "yellow");
        assert_eq!(gnome_accent_color("#00ff00"), "green");
        assert_eq!(gnome_accent_color("#00ffff"), "teal");
        assert_eq!(gnome_accent_color("#0000ff"), "blue");
        assert_eq!(gnome_accent_color("#8800ff"), "purple");
        assert_eq!(gnome_accent_color("#ff00aa"), "pink");
        assert_eq!(gnome_accent_color("#888888"), "slate");
    }
}
