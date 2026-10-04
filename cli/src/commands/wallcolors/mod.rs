//! The wallcolors engine: wallpaper histogram and colourfulness analysis,
//! matugen scheme derivation, semantic terminal palette layer, and consumer fan-out.

pub mod color;
pub mod render;

use crate::helpers::{cache_file, config_file, home_path, state_file};
use crate::json::{self, Json};
use color::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const O_NONBLOCK_NOCTTY: i32 = 0x800 | 0x100;

pub fn schemes_dir() -> PathBuf {
    let user_dir = config_file(&["hypr", "schemes"]);
    if user_dir.is_dir() {
        return user_dir;
    }
    let local_dir = home_path(&[".config", "hypr", "schemes"]);
    if local_dir.is_dir() {
        return local_dir;
    }
    // Check repository schemes directory
    if let Ok(cwd) = std::env::current_dir() {
        let repo_dir = cwd.join("configs/hypr/schemes");
        if repo_dir.is_dir() {
            return repo_dir;
        }
    }
    user_dir
}

pub fn list_presets() -> Vec<String> {
    let dir = schemes_dir();
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

pub fn preset_tokens(name: &str) -> Option<HashMap<String, String>> {
    let path = schemes_dir().join(format!("{name}.json"));
    if !path.is_file() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let parsed = json::parse(&content).ok()?;
    let mut tokens = HashMap::new();
    if let Json::Obj(entries) = parsed {
        for (k, v) in entries {
            if let Some(s) = v.as_str() {
                tokens.insert(k, s.to_string());
            }
        }
    }
    if !tokens.contains_key("primary")
        || !tokens.contains_key("cream")
        || !tokens.contains_key("surface")
    {
        return None;
    }
    Some(tokens)
}

pub fn load_scheme() -> (String, String, bool, String) {
    let mut preset = "dynamic".to_string();
    let mut variant = "auto".to_string();
    let mut smart = true;
    let mut mode = "dark".to_string();

    let json_paths = [
        cache_file("xiu/scheme.json"),
        cache_file("ricelin/scheme.json"),
        state_file("xiu/scheme.json"),
        state_file("ricelin/scheme.json"),
    ];

    for p in json_paths {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(Json::Obj(entries)) = json::parse(&content) {
                for (k, v) in entries {
                    match k.as_str() {
                        "preset" => {
                            if let Json::Str(s) = v {
                                if !s.is_empty() { preset = s; }
                            }
                        }
                        "variant" => {
                            if let Json::Str(s) = v {
                                if !s.is_empty() { variant = s; }
                            }
                        }
                        "smart" => {
                            if let Json::Bool(b) = v {
                                smart = b;
                            } else if let Json::Str(s) = v {
                                smart = s != "off";
                            }
                        }
                        "mode" => {
                            if let Json::Str(s) = v {
                                if !s.is_empty() { mode = s; }
                            }
                        }
                        _ => {}
                    }
                }
                return (preset, variant, smart, mode);
            }
        }
    }

    let paths = [
        state_file("xiu/scheme"),
        state_file("ricelin/scheme"),
    ];

    for p in paths {
        if let Ok(content) = fs::read_to_string(&p) {
            for line in content.lines() {
                let mut parts = line.splitn(2, ' ');
                let key = parts.next().unwrap_or("").trim();
                let val = parts.next().unwrap_or("").trim();
                match key {
                    "preset" => {
                        if !val.is_empty() {
                            preset = val.to_string();
                        }
                    }
                    "variant" => {
                        if !val.is_empty() {
                            variant = val.to_string();
                        }
                    }
                    "smart" => {
                        smart = val != "off";
                    }
                    "mode" => {
                        if !val.is_empty() {
                            mode = val.to_string();
                        }
                    }
                    _ => {}
                }
            }
            break;
        }
    }

    (preset, variant, smart, mode)
}

pub fn save_scheme(preset: &str, variant: &str, smart: bool, mode: &str) {
    let content = format!(
        "preset {preset}\nvariant {variant}\nsmart {}\nmode {mode}\n",
        if smart { "on" } else { "off" }
    );
    for sub in &["xiu/scheme", "ricelin/scheme"] {
        let f = state_file(sub);
        if let Some(parent) = f.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(f, &content);
    }

    let scheme_json = json::stringify_pretty(
        &Json::Obj(vec![
            ("preset".to_string(), Json::Str(preset.to_string())),
            ("variant".to_string(), Json::Str(variant.to_string())),
            ("smart".to_string(), Json::Bool(smart)),
            ("mode".to_string(), Json::Str(mode.to_string())),
        ]),
        2,
    ) + "\n";
    for sub in &["xiu", "ricelin"] {
        let cd = cache_file(sub);
        let _ = fs::create_dir_all(&cd);
        let _ = fs::write(cd.join("scheme.json"), &scheme_json);

        let sd = state_file(sub);
        let _ = fs::create_dir_all(&sd);
        let _ = fs::write(sd.join("scheme.json"), &scheme_json);
    }
}

pub fn set_palette_mode_dynamic() {
    let f = state_file("ricelin/flags.json");
    let mut flags = if f.is_file() {
        fs::read_to_string(&f)
            .ok()
            .and_then(|c| json::parse(&c).ok())
            .unwrap_or(Json::Obj(Vec::new()))
    } else {
        Json::Obj(Vec::new())
    };

    if let Json::Obj(ref mut entries) = flags {
        if let Some(pos) = entries.iter().position(|(k, _)| k == "paletteMode") {
            entries[pos] = ("paletteMode".to_string(), Json::Str("dynamic".to_string()));
        } else {
            entries.push(("paletteMode".to_string(), Json::Str("dynamic".to_string())));
        }
        if let Some(parent) = f.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&f, json::stringify_pretty(&flags, 2) + "\n");
    }
}

pub fn current_wallpaper() -> String {
    for p in &[state_file("xiu/wallpaper"), state_file("ricelin-wallpaper")] {
        if p.is_file() {
            if let Ok(c) = fs::read_to_string(p) {
                let trimmed = c.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    String::new()
}

pub fn analyze(wallpaper: &str) -> (Option<f64>, f64, f64, f64) {
    let out = Command::new("magick")
        .args([
            wallpaper,
            "-alpha",
            "off",
            "-resize",
            "200x200",
            "-colors",
            "48",
            "-format",
            "%c",
            "histogram:info:-",
        ])
        .output();

    let text = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return (None, 0.0, 0.0, 0.0),
    };

    let re = match Regex::new(r"\s*(\d+):\s*\([^)]*\)\s*#([0-9A-Fa-f]{6,16})") {
        Ok(r) => r,
        Err(_) => return (None, 0.0, 0.0, 0.0),
    };

    struct Bucket {
        wsat: f64,
        best: Option<(f64, f64, f64)>, // score, h, s
    }

    let mut buckets: HashMap<usize, Bucket> = HashMap::new();
    let mut total: u64 = 0;
    let mut lum: f64 = 0.0;
    let mut chroma: u64 = 0;

    for line in text.lines() {
        let Some(caps) = re.captures(line) else {
            continue;
        };
        let count: u64 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        let mut hex_str = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        if hex_str.len() > 6 {
            let step = hex_str.len() / if hex_str.len() % 3 == 0 { 3 } else { 4 };
            let mut short_hex = String::new();
            for i in (0..3 * step).step_by(step) {
                if i + 2 <= hex_str.len() {
                    short_hex.push_str(&hex_str[i..i + 2]);
                }
            }
            hex_str = short_hex;
        }
        let (r, g, b) = hex_to_rgb(&hex_str);
        let (h, l, s) = rgb_to_hls(r, g, b);
        total += count;
        lum += count as f64 * l;
        if s < 0.15 || l < 0.05 || l > 0.92 {
            continue;
        }
        chroma += count;
        let bucket_idx = ((h * 360.0).round() as usize / 30) % 12;
        let bucket = buckets.entry(bucket_idx).or_insert(Bucket {
            wsat: 0.0,
            best: None,
        });
        bucket.wsat += count as f64 * s;
        let factor = if l > 0.12 && l < 0.55 { 1.0 } else { 0.4 };
        let score = count as f64 * s * factor;
        if bucket.best.is_none() || score > bucket.best.unwrap().0 {
            bucket.best = Some((score, h, s));
        }
    }

    let mean_l = if total > 0 { lum / total as f64 } else { 0.0 };
    let share = if total > 0 { chroma as f64 / total as f64 } else { 0.0 };

    if buckets.is_empty() || (chroma as f64) < 0.08 * total as f64 {
        return (None, 0.0, mean_l, share);
    }

    if let Some(win) = buckets.values().max_by(|a, b| a.wsat.total_cmp(&b.wsat)) {
        if let Some((_score, h, s)) = win.best {
            return (Some(h), s, mean_l, share);
        }
    }

    (None, 0.0, mean_l, share)
}

pub fn colourfulness(wallpaper: &str) -> Option<f64> {
    let out = Command::new("magick")
        .args([
            wallpaper,
            "-alpha",
            "off",
            "-resize",
            "64x64",
            "-depth",
            "8",
            "txt:-",
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let re = Regex::new(r"#([0-9A-Fa-f]{6})").ok()?;
    let mut rg = Vec::new();
    let mut yb = Vec::new();

    for caps in re.captures_iter(&text) {
        if let Some(m) = caps.get(1) {
            let (r, g, b) = hex_to_rgb(m.as_str());
            rg.push(r - g);
            yb.push(0.5 * (r + g) - b);
        }
    }

    if rg.is_empty() {
        return None;
    }

    let stats = |v: &[f64]| -> (f64, f64) {
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64;
        (mean, var)
    };

    let (mrg, vrg) = stats(&rg);
    let (myb, vyb) = stats(&yb);

    Some((vrg + vyb).sqrt() + 0.3 * (mrg.powi(2) + myb.powi(2)).sqrt())
}

pub fn smart_variant(score: Option<f64>) -> &'static str {
    match score {
        None => "tonal-spot",
        Some(s) if s < 0.06 => "neutral",
        Some(s) if s < 0.13 => "content",
        _ => "tonal-spot",
    }
}

pub fn matugen(source_hex: &str, variant: &str, mode: &str) -> Result<Json, String> {
    let m = if mode == "light" { "light" } else { "dark" };
    let mut cmd = Command::new("matugen");
    cmd.args(["color", "hex", source_hex, "-m", m, "-j", "hex"]);
    if !variant.is_empty() && variant != "auto" {
        cmd.args(["--type", &format!("scheme-{variant}")]);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("matugen exited with status {:?}", out.status));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    json::parse(&text).map_err(|e| e.to_string())
}

pub fn generate_dynamic(
    wallpaper: &str,
    variant_arg: &str,
    smart: bool,
    mode: &str,
) -> (HashMap<String, String>, String, String, f64) {
    let (hue_opt, sat, mean_l, share) = analyze(wallpaper);
    let chromatic = hue_opt.is_some();

    let mut resolved_variant = variant_arg.to_string();
    if resolved_variant == "auto" {
        if !chromatic {
            resolved_variant = "neutral".to_string();
        } else if smart {
            resolved_variant = smart_variant(colourfulness(wallpaper)).to_string();
        } else {
            resolved_variant = "tonal-spot".to_string();
        }
    }

    let (pill, seed, resolved) =
        generate_dynamic_palette(hue_opt, sat, mean_l, &resolved_variant, mode);
    (pill, seed, resolved, share)
}

pub fn osc_sequence(code: &str, hex_color: &str) -> String {
    let s = hex_color.trim().trim_start_matches('#');
    let (r, g, b) = if s.len() >= 6 {
        (&s[0..2], &s[2..4], &s[4..6])
    } else {
        ("00", "00", "00")
    };
    format!("\x1b]{code};rgb:{r}/{g}/{b}\x1b\\")
}

pub fn broadcast_terminal(
    pill: &HashMap<String, String>,
    b: &HashMap<String, String>,
    ansi: &[String],
) {
    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let mut seq = format!(
        "{}{}{}{}",
        osc_sequence("10", &g(b, "base07")),
        osc_sequence("11", &g(b, "base00")),
        osc_sequence("12", &g(pill, "primary")),
        osc_sequence("17", &g(b, "base02")),
    );
    for (i, hex_color) in ansi.iter().enumerate() {
        let s = hex_color.trim().trim_start_matches('#');
        let (r, g, b_part) = if s.len() >= 6 {
            (&s[0..2], &s[2..4], &s[4..6])
        } else {
            ("00", "00", "00")
        };
        seq.push_str(&format!("\x1b]4;{i};rgb:{r}/{g}/{b_part}\x1b\\"));
    }

    for dir_sub in &["ricelin", "xiu"] {
        let dir = cache_file(dir_sub);
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("sequences.txt"), &seq);
    }

    let data = seq.as_bytes();
    if let Ok(entries) = fs::read_dir("/dev/pts") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.chars().all(|c| c.is_ascii_digit()) {
                let path = entry.path();
                if let Ok(mut file) = fs::OpenOptions::new()
                    .write(true)
                    .custom_flags(O_NONBLOCK_NOCTTY)
                    .open(&path)
                {
                    use std::io::Write;
                    let _ = file.write_all(data);
                }
            }
        }
    }
}

pub fn fan_out(
    pill: &HashMap<String, String>,
    seed: &str,
    variant: &str,
    share: Option<f64>,
    mode: &str,
) -> i32 {
    let mut pill_json_obj = Vec::new();
    let mut pill_keys: Vec<String> = pill.keys().cloned().collect();
    pill_keys.sort();
    for k in pill_keys {
        pill_json_obj.push((k.clone(), Json::Str(pill.get(&k).unwrap().clone())));
    }
    let c_json = json::stringify_pretty(&Json::Obj(pill_json_obj), 2) + "\n";

    for sub in &["ricelin", "xiu"] {
        let d = cache_file(sub);
        let _ = fs::create_dir_all(&d);
        let _ = fs::write(d.join("colors.json"), &c_json);
    }

    render::render_fastfetch(pill);
    render::render_starship(pill);

    let m_mode = if mode == "light" || rel_luminance(pill.get("surface").map(String::as_str).unwrap_or("#000000")) >= 0.40 {
        "light"
    } else {
        "dark"
    };

    let mut b: HashMap<String, String> = HashMap::new();
    if let Ok(mat_res) = matugen(seed, variant, m_mode) {
        if let Some(b16) = mat_res.get("base16") {
            if let Json::Obj(entries) = b16 {
                for (k, val) in entries {
                    let mut col = "#000000".to_string();
                    if let Some(mode_obj) = val.get(m_mode).or_else(|| val.get("dark")) {
                        if let Some(c) = mode_obj.get("color").and_then(Json::as_str) {
                            col = c.to_string();
                        }
                    }
                    b.insert(k.to_string(), col);
                }
            }
        }
    }

    if b.is_empty() {
        // Fallback default neutral base16
        b.insert("base00".into(), pill.get("surface").cloned().unwrap_or_else(|| "#141a20".into()));
        b.insert("base01".into(), pill.get("surface_container_low").cloned().unwrap_or_else(|| "#20262d".into()));
        b.insert("base02".into(), pill.get("surface_container").cloned().unwrap_or_else(|| "#2c333b".into()));
        b.insert("base03".into(), pill.get("surface_container_high").cloned().unwrap_or_else(|| "#454c54".into()));
        b.insert("base04".into(), pill.get("outline_variant").cloned().unwrap_or_else(|| "#5e666e".into()));
        b.insert("base05".into(), pill.get("dim").cloned().unwrap_or_else(|| "#778088".into()));
        b.insert("base06".into(), pill.get("subtle").cloned().unwrap_or_else(|| "#919aa2".into()));
        b.insert("base07".into(), pill.get("cream").cloned().unwrap_or_else(|| "#abb4bc".into()));
        b.insert("base08".into(), "#ff6f4a".into());
        b.insert("base09".into(), "#dc865f".into());
        b.insert("base0a".into(), "#d08e45".into());
        b.insert("base0b".into(), "#a79f00".into());
        b.insert("base0c".into(), "#8ea554".into());
        b.insert("base0d".into(), "#e18700".into());
        b.insert("base0e".into(), "#83a900".into());
        b.insert("base0f".into(), "#6f767e".into());
    }

    let mut b_json_obj = Vec::new();
    let mut b_keys: Vec<String> = b.keys().cloned().collect();
    b_keys.sort();
    for k in b_keys {
        b_json_obj.push((k.clone(), Json::Str(b.get(&k).unwrap().clone())));
    }
    let b_json = json::stringify_pretty(&Json::Obj(b_json_obj), 2) + "\n";

    for sub in &["ricelin", "xiu"] {
        let d = cache_file(sub);
        let _ = fs::create_dir_all(&d);
        let _ = fs::write(d.join("base16.json"), &b_json);
    }

    let ansi = semantic_terminal(pill, &mut b, seed, share);
    render::render_fish(pill, &ansi);
    broadcast_terminal(pill, &b, &ansi);

    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let hypr_colors = format!(
        r#"return {{
    active = "{active}",
    inactive = "{inactive}",
    locked_active = "{locked_active}",
    locked_inactive = "{locked_inactive}",
    group_active = "{group_active}",
    group_inactive = "{group_inactive}",
    group_locked_active = "{group_locked_active}",
    group_locked_inactive = "{group_locked_inactive}",
    text_color = "{text_color}",
}}
"#,
        active = g(pill, "primary"),
        inactive = g(&b, "base01"),
        locked_active = pill.get("on_primary_container").cloned().unwrap_or_else(|| g(&b, "base09")),
        locked_inactive = g(&b, "base02"),
        group_active = g(pill, "primary"),
        group_inactive = pill.get("surface_container").cloned().unwrap_or_else(|| g(&b, "base01")),
        group_locked_active = pill.get("on_primary_container").cloned().unwrap_or_else(|| g(&b, "base09")),
        group_locked_inactive = g(&b, "base02"),
        text_color = pill.get("cream").cloned().unwrap_or_else(|| g(&b, "base07")),
    );

    for sub in &["ricelin", "xiu"] {
        let d = cache_file(sub);
        let _ = fs::create_dir_all(&d);
        let _ = fs::write(d.join("hypr-colors.lua"), &hypr_colors);
    }

    let mut ghostty_lines = vec![
        format!("background = {}", g(&b, "base00")),
        format!("foreground = {}", g(&b, "base07")),
        format!("cursor-color = {}", g(pill, "primary")),
        format!("selection-background = {}", g(&b, "base02")),
        format!("selection-foreground = {}", g(&b, "base07")),
    ];
    for (i, hex_color) in ansi.iter().enumerate() {
        ghostty_lines.push(format!("palette = {i}={hex_color}"));
    }
    let ghostty_content = ghostty_lines.join("\n") + "\n";
    for sub in &["ricelin", "xiu"] {
        let d = cache_file(sub);
        let _ = fs::create_dir_all(&d);
        let _ = fs::write(d.join("ghostty-colors"), &ghostty_content);
    }

    render::render_foot(pill, &b, &ansi);
    render::render_btop(pill, &b);
    render::render_htop(pill, &b);
    render::render_nvtop(pill, &b);
    render::render_cava(pill, &b);
    render::render_micro(pill, &b, None);
    render::render_helix(pill, &b, None);
    render::render_bottom(pill, &b);
    render::render_yazi(pill, &b);
    render::render_spicetify(pill, &b);
    render::render_discord(pill);
    render::render_userchrome(pill);
    render::render_vscode(pill);
    render::render_zed(pill, Some(&b), None);
    render::render_browser(pill);
    render::render_qt(pill);
    render::render_gtk(pill);
    render::render_user_templates(pill, &b);

    0
}

pub fn wallcolors(args: &[String]) -> i32 {
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help") {
        eprintln!(
            "usage: xiu wallcolors [wallpaper] | --mode <dark|light> | --toggle | --hue H [mode] [sat] | --preset NAME | \
             --variant NAME | --smart | --no-smart | --list-presets | --state | \
             --preview <wallpaper> | --apply"
        );
        return 0;
    }

    // Test evaluation dispatcher for hermetic testing by test_wallcolors.py
    if args.len() > 1 && args[0] == "--test-eval" {
        return test_eval(&args[1..]);
    }

    if !args.is_empty() && args[0] == "--list-presets" {
        println!("{}", list_presets().join("\n"));
        return 0;
    }

    if !args.is_empty() && args[0] == "--state" {
        let (preset, variant, smart, mode) = load_scheme();
        println!("preset {preset}\nvariant {variant}\nsmart {}\nmode {mode}", if smart { "on" } else { "off" });
        return 0;
    }

    if !args.is_empty() && args[0] == "--preview" {
        if args.len() < 2 {
            eprintln!("wallcolors: --preview needs a wallpaper");
            return 1;
        }
        let (mut pill, seed, variant, _share) = generate_dynamic(&args[1], "auto", true, "dark");
        pill.insert("_variant".to_string(), variant);
        pill.insert("_seed".to_string(), seed);
        let mut obj = Vec::new();
        let mut keys: Vec<String> = pill.keys().cloned().collect();
        keys.sort();
        for k in keys {
            obj.push((k.clone(), Json::Str(pill.get(&k).unwrap().clone())));
        }
        println!("{}", json::stringify_pretty(&Json::Obj(obj), 2));
        return 0;
    }

    let (mut preset, mut variant, mut smart, mut mode) = load_scheme();
    let mut changed = false;
    let mut wallpaper: Option<String> = None;
    let mut i = 0;

    while i < args.len() {
        let a = &args[i];
        if a == "--preset" && i + 1 < args.len() {
            i += 1;
            let name = &args[i];
            if name != "dynamic" && preset_tokens(name).is_none() {
                eprintln!("wallcolors: unknown preset '{name}' (see --list-presets)");
                return 1;
            }
            preset = name.clone();
            changed = true;
        } else if (a == "--mode" || a == "-m") && i + 1 < args.len() {
            i += 1;
            let m = &args[i];
            if m != "dark" && m != "light" {
                eprintln!("wallcolors: unknown mode '{m}' (one of: dark, light)");
                return 1;
            }
            mode = m.clone();
            changed = true;
        } else if a == "--dark" {
            mode = "dark".to_string();
            changed = true;
        } else if a == "--light" {
            mode = "light".to_string();
            changed = true;
        } else if a == "--toggle" {
            mode = if mode == "dark" { "light".to_string() } else { "dark".to_string() };
            changed = true;
        } else if a == "--apply" {
            changed = true;
        } else if a == "--variant" && i + 1 < args.len() {
            i += 1;
            let v = &args[i];
            if v == "dark" || v == "light" {
                mode = v.clone();
                changed = true;
            } else if !VARIANTS.contains(&v.as_str()) {
                eprintln!("wallcolors: unknown variant '{v}' (one of: {})", VARIANTS.join(", "));
                return 1;
            } else {
                variant = v.clone();
                changed = true;
            }
        } else if a == "--smart" {
            smart = true;
            changed = true;
        } else if a == "--no-smart" {
            smart = false;
            changed = true;
        } else if a == "--hue" && i + 1 < args.len() {
            let h_val: f64 = args[i + 1].parse().unwrap_or(30.0);
            let hue = (h_val.rem_euclid(360.0)) / 360.0;
            let mode_arg = if i + 2 < args.len() && (args[i + 2] == "dark" || args[i + 2] == "light") {
                &args[i + 2]
            } else {
                &mode
            };
            let sat: f64 = if i + 3 < args.len() {
                args[i + 3].parse().unwrap_or(0.5)
            } else {
                0.5
            };
            let (pill, seed, resolved) = generate_manual(hue, mode_arg, sat, &variant);
            return fan_out(&pill, &seed, &resolved, None, mode_arg);
        } else if wallpaper.is_none() {
            wallpaper = Some(a.clone());
        } else {
            eprintln!("wallcolors: unexpected argument '{a}'");
            return 1;
        }
        i += 1;
    }

    if changed {
        save_scheme(&preset, &variant, smart, &mode);
        set_palette_mode_dynamic();
    }

    if preset != "dynamic" {
        if let Some(tokens) = preset_tokens(&preset) {
            let resolved = if variant != "auto" { &variant } else { "tonal-spot" };
            let seed = format!("#{}", tokens.get("seed").cloned().unwrap_or_else(|| "787878".to_string()));
            return fan_out(&tokens, &seed, resolved, None, &mode);
        }
    }

    let wp = match wallpaper {
        Some(w) => w,
        None => {
            let cur = current_wallpaper();
            if cur.is_empty() || !Path::new(&cur).is_file() {
                eprintln!("wallcolors: no wallpaper to analyze (set one first)");
                return 1;
            }
            cur
        }
    };

    if !Path::new(&wp).is_file() {
        return 0;
    }

    let (pill, seed, resolved, share) = generate_dynamic(&wp, &variant, smart, &mode);
    fan_out(&pill, &seed, &resolved, Some(share), &mode)
}

fn test_eval(args: &[String]) -> i32 {
    if args.is_empty() {
        return 1;
    }
    let func = &args[0];
    match func.as_str() {
        "semantic_terminal" => {
            // Args: <pill_json> <b_json> <seed> [share]
            let pill_json = args.get(1).map(String::as_str).unwrap_or("{}");
            let b_json = args.get(2).map(String::as_str).unwrap_or("{}");
            let seed = args.get(3).map(String::as_str).unwrap_or("#d06030");
            let share: Option<f64> = args.get(4).and_then(|s| s.parse().ok());

            let mut pill = HashMap::new();
            if let Ok(Json::Obj(entries)) = json::parse(pill_json) {
                for (k, v) in entries {
                    if let Some(s) = v.as_str() {
                        pill.insert(k, s.to_string());
                    }
                }
            }
            let mut b = HashMap::new();
            if let Ok(Json::Obj(entries)) = json::parse(b_json) {
                for (k, v) in entries {
                    if let Some(s) = v.as_str() {
                        b.insert(k, s.to_string());
                    }
                }
            }
            let ansi = semantic_terminal(&pill, &mut b, seed, share);
            let mut b_obj = Vec::new();
            for (k, v) in b {
                b_obj.push((k, Json::Str(v)));
            }
            let ansi_arr: Vec<Json> = ansi.into_iter().map(Json::Str).collect();
            let mut res = Vec::new();
            res.push(("ansi".to_string(), Json::Arr(ansi_arr)));
            res.push(("b".to_string(), Json::Obj(b_obj)));
            println!("{}", json::stringify(&Json::Obj(res)));
            0
        }
        "contrast_ratio" => {
            let a = args.get(1).map(String::as_str).unwrap_or("#000000");
            let b = args.get(2).map(String::as_str).unwrap_or("#ffffff");
            println!("{}", contrast_ratio(a, b));
            0
        }
        "rel_luminance" => {
            let a = args.get(1).map(String::as_str).unwrap_or("#000000");
            println!("{}", rel_luminance(a));
            0
        }
        "hue_sat_of" => {
            let a = args.get(1).map(String::as_str).unwrap_or("#000000");
            let (h, s) = hue_sat_of(a);
            println!("[{h}, {s}]");
            0
        }
        "signed_arc" => {
            let a: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let b: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            println!("{}", signed_arc(a, b));
            0
        }
        "circ_clamp" => {
            let h: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let lo: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let hi: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            println!("{}", circ_clamp(h, lo, hi));
            0
        }
        "snap_to_band" => {
            let hex = args.get(1).map(String::as_str).unwrap_or("#808080");
            let lo: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.25);
            let hi: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.30);
            println!("{}", snap_to_band(hex, (lo, hi)));
            0
        }
        "clamp_light" => {
            let hex = args.get(1).map(String::as_str).unwrap_or("#808080");
            let target: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4.5);
            let bg = args.get(3).map(String::as_str).unwrap_or("#000000");
            println!("{}", clamp_light(hex, target, bg));
            0
        }
        "generate_manual" => {
            let hue: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let mode = args.get(2).map(String::as_str).unwrap_or("dark");
            let sat: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let variant = args.get(4).map(String::as_str).unwrap_or("auto");
            let (pill, seed, var) = generate_manual(hue, mode, sat, variant);
            let mut p_obj = Vec::new();
            for (k, v) in pill {
                p_obj.push((k, Json::Str(v)));
            }
            let res = vec![
                Json::Obj(p_obj),
                Json::Str(seed),
                Json::Str(var),
            ];
            println!("{}", json::stringify(&Json::Arr(res)));
            0
        }
        "gnome_accent_color" => {
            let hex = args.get(1).map(String::as_str).unwrap_or("#ff0000");
            println!("{}", gnome_accent_color(hex));
            0
        }
        "update_gtk_settings" => {
            let file = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("settings.ini"));
            let theme = args.get(2).map(String::as_str).unwrap_or("adw-gtk3-dark");
            let icon = args.get(3).map(String::as_str).unwrap_or("yet-another-monochrome-icon-set");
            let is_dark = args.get(4).map(|s| s == "true" || s == "1").unwrap_or(true);
            render::update_gtk_settings(&file, theme, icon, is_dark);
            0
        }
        "update_xsettingsd" => {
            let file = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("xsettingsd.conf"));
            let theme = args.get(2).map(String::as_str).unwrap_or("adw-gtk3-dark");
            let icon = args.get(3).map(String::as_str).unwrap_or("yet-another-monochrome-icon-set");
            render::update_xsettingsd(&file, theme, icon);
            0
        }
        "update_kdeglobals" => {
            let file = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("kdeglobals"));
            let sec_json = args.get(2).map(String::as_str).unwrap_or("[]");
            let icon = args.get(3).map(String::as_str).unwrap_or("yet-another-monochrome-icon-set");
            let primary = args.get(4).map(String::as_str).unwrap_or("#fabd2f");

            let mut sections_storage: Vec<(String, Vec<(String, String)>)> = Vec::new();
            if let Ok(Json::Arr(items)) = json::parse(sec_json) {
                for item in items {
                    if let Json::Arr(pair) = item {
                        if pair.len() >= 2 {
                            let hdr = pair[0].as_str().unwrap_or("").to_string();
                            let mut fields = Vec::new();
                            if let Json::Obj(fmap) = &pair[1] {
                                for (k, v) in fmap {
                                    if let Some(vs) = v.as_str() {
                                        fields.push((k.clone(), vs.to_string()));
                                    }
                                }
                            }
                            sections_storage.push((hdr, fields));
                        }
                    }
                }
            }

            let mut sec_refs: Vec<(&str, Vec<(&str, &str)>)> = Vec::new();
            for (h, f) in &sections_storage {
                let f_refs: Vec<(&str, &str)> = f.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                sec_refs.push((h.as_str(), f_refs));
            }
            let final_secs: Vec<(&str, &[(&str, &str)])> = sec_refs.iter().map(|(h, f)| (*h, f.as_slice())).collect();

            render::update_kdeglobals(&file, &final_secs, icon, primary);
            0
        }
        "get_active_icon_theme" => {
            let is_dark = args.get(1).map(|s| s == "true" || s == "1").unwrap_or(true);
            println!("{}", render::get_active_icon_theme(is_dark));
            0
        }
        "render_zed" => {
            let pill_json = args.get(1).map(String::as_str).unwrap_or("{}");
            let b_json = args.get(2).map(String::as_str).unwrap_or("null");
            let zed_dir = PathBuf::from(args.get(3).map(String::as_str).unwrap_or("."));

            let mut pill = HashMap::new();
            if let Ok(Json::Obj(entries)) = json::parse(pill_json) {
                for (k, v) in entries {
                    if let Some(s) = v.as_str() {
                        pill.insert(k, s.to_string());
                    }
                }
            }
            let mut b = HashMap::new();
            let b_ptr = if b_json != "null" && b_json != "None" {
                if let Ok(Json::Obj(entries)) = json::parse(b_json) {
                    for (k, v) in entries {
                        if let Some(s) = v.as_str() {
                            b.insert(k, s.to_string());
                        }
                    }
                }
                Some(&b)
            } else {
                None
            };
            render::render_zed(&pill, b_ptr, Some(&zed_dir));
            0
        }
        "render_helix" => {
            let pill_json = args.get(1).map(String::as_str).unwrap_or("{}");
            let b_json = args.get(2).map(String::as_str).unwrap_or("{}");
            let h_dir = PathBuf::from(args.get(3).map(String::as_str).unwrap_or("."));

            let mut pill = HashMap::new();
            if let Ok(Json::Obj(entries)) = json::parse(pill_json) {
                for (k, v) in entries {
                    if let Some(s) = v.as_str() {
                        pill.insert(k, s.to_string());
                    }
                }
            }
            let mut b = HashMap::new();
            if let Ok(Json::Obj(entries)) = json::parse(b_json) {
                for (k, v) in entries {
                    if let Some(s) = v.as_str() {
                        b.insert(k, s.to_string());
                    }
                }
            }
            render::render_helix(&pill, &b, Some(&h_dir));
            0
        }
        "render_micro" => {
            let pill_json = args.get(1).map(String::as_str).unwrap_or("{}");
            let b_json = args.get(2).map(String::as_str).unwrap_or("{}");
            let m_dir = PathBuf::from(args.get(3).map(String::as_str).unwrap_or("."));

            let mut pill = HashMap::new();
            if let Ok(Json::Obj(entries)) = json::parse(pill_json) {
                for (k, v) in entries {
                    if let Some(s) = v.as_str() {
                        pill.insert(k, s.to_string());
                    }
                }
            }
            let mut b = HashMap::new();
            if let Ok(Json::Obj(entries)) = json::parse(b_json) {
                for (k, v) in entries {
                    if let Some(s) = v.as_str() {
                        b.insert(k, s.to_string());
                    }
                }
            }
            render::render_micro(&pill, &b, Some(&m_dir));
            0
        }
        other => {
            eprintln!("Unknown test-eval function: {other}");
            1
        }
    }
}
