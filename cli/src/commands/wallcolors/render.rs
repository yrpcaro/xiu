//! Template renderers for all apps configured by wallcolors:
//! fastfetch, starship, fish, foot, btop, htop, nvtop, cava, micro, helix,
//! bottom, yazi, userchrome, discord, vscode, zed, browser, qt, gtk, and user_templates.

use super::color::{gnome_accent_color, rel_luminance};
use crate::helpers::{cache_file, config_file, home_path, on_path};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn tool_dir(name: &str) -> Option<PathBuf> {
    let p = home_path(&[".config", name]);
    if p.is_dir() {
        return Some(p);
    }
    let is_installed = match name {
        "helix" => on_path("hx") || on_path("helix"),
        "bottom" => on_path("btm") || on_path("bottom"),
        "zed" => on_path("zed") || on_path("zed-editor"),
        other => on_path(other),
    };
    if is_installed {
        let _ = fs::create_dir_all(&p);
        return Some(p);
    }
    None
}

pub fn reload_tool(binary: &str) {
    let _ = Command::new("killall")
        .args(["-USR2", binary])
        .stderr(Stdio::null())
        .status();
}

pub fn render_fastfetch(pill: &HashMap<String, String>) {
    let ff = match tool_dir("fastfetch") {
        Some(d) => d,
        None => return,
    };
    let tmpl = ff.join("config.jsonc.in");
    if !tmpl.is_file() {
        eprintln!(
            "wallcolors: config.jsonc.in missing in ~/.config/fastfetch, skipping fastfetch recolour"
        );
        return;
    }
    let seq = |h: &str| -> String {
        let s = h.trim().trim_start_matches('#');
        if s.len() >= 6 {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
            format!("{r};{g};{b}")
        } else {
            "0;0;0".to_string()
        }
    };
    let primary = pill.get("primary").cloned().unwrap_or_default();
    let dim = pill.get("dim").cloned().unwrap_or_default();
    let on_primary_container = pill.get("on_primary_container").cloned().unwrap_or_default();
    let surface_container = pill.get("surface_container").cloned().unwrap_or_default();
    let surface_container_high = pill.get("surface_container_high").cloned().unwrap_or_default();
    let subtle = pill.get("subtle").cloned().unwrap_or_default();
    let outline = pill.get("outline").cloned().unwrap_or_default();
    let bright = pill.get("bright").cloned().unwrap_or_default();

    if let Ok(mut out) = fs::read_to_string(&tmpl) {
        out = out.replace("__LANTERN__", &ff.join("lantern.txt").to_string_lossy());
        out = out.replace("__KEYS__", &seq(&primary));
        out = out.replace("__SEP__", &seq(&dim));
        out = out.replace("__LOGO1__", &seq(&primary));
        out = out.replace("__LOGO2__", &seq(&on_primary_container));
        out = out.replace("__LOGO3__", &seq(&surface_container));
        out = out.replace("__LOGO4__", &seq(&surface_container_high));
        out = out.replace("__LOGO5__", &seq(&subtle));
        out = out.replace("__LOGO6__", &seq(&outline));
        out = out.replace("__LOGO7__", &seq(&bright));
        let _ = fs::write(ff.join("config.jsonc"), out);
    }
}

pub fn render_starship(pill: &HashMap<String, String>) {
    let fish = match tool_dir("fish") {
        Some(d) => d,
        None => return,
    };
    let tmpl = fish.join("starship.toml.in");
    if !tmpl.is_file() {
        eprintln!(
            "wallcolors: starship.toml.in missing in ~/.config/fish, skipping starship recolour"
        );
        return;
    }
    let primary = pill.get("primary").cloned().unwrap_or_default();
    let bright = pill.get("bright").cloned().unwrap_or_default();
    let cream = pill.get("cream").cloned().unwrap_or_default();
    let subtle = pill.get("subtle").cloned().unwrap_or_default();
    let dim = pill.get("dim").cloned().unwrap_or_default();
    let faint = pill.get("faint").cloned().unwrap_or_default();

    if let Ok(mut out) = fs::read_to_string(&tmpl) {
        out = out.replace("__CHAR_OK__", &primary);
        out = out.replace("__CHAR_ERR__", &bright);
        out = out.replace("__DIR__", &cream);
        out = out.replace("__BRANCH__", &subtle);
        out = out.replace("__STATUS__", &dim);
        out = out.replace("__DURATION__", &faint);
        let _ = fs::write(fish.join("starship.toml"), out);
    }
}

pub fn render_fish(pill: &HashMap<String, String>, ansi: &[String]) {
    let fish = match tool_dir("fish") {
        Some(d) => d,
        None => return,
    };
    let q = |s: &str| format!("\"{s}\"");
    let primary = pill.get("primary").cloned().unwrap_or_default();
    let cream = pill.get("cream").cloned().unwrap_or_default();
    let subtle = pill.get("subtle").cloned().unwrap_or_default();
    let on_primary_container = pill.get("on_primary_container").cloned().unwrap_or_default();
    let faint = pill.get("faint").cloned().unwrap_or_default();
    let dim = pill.get("dim").cloned().unwrap_or_default();
    let primary_container = pill.get("primary_container").cloned().unwrap_or_default();
    let surface_container_high = pill.get("surface_container_high").cloned().unwrap_or_default();
    let ansi1 = ansi.get(1).cloned().unwrap_or_default();
    let ansi3 = ansi.get(3).cloned().unwrap_or_default();

    let lines = vec![
        "# Written by wallcolors.py on every palette change; sourced by config.fish.".to_string(),
        format!("set -g fish_color_command {}", q(&primary)),
        format!("set -g fish_color_param {}", q(&cream)),
        format!("set -g fish_color_option {}", q(&subtle)),
        format!("set -g fish_color_quote {}", q(&on_primary_container)),
        format!("set -g fish_color_escape {}", q(&ansi3)),
        format!("set -g fish_color_redirection {}", q(&subtle)),
        format!("set -g fish_color_comment {}", q(&faint)),
        format!("set -g fish_color_error {}", q(&ansi1)),
        format!("set -g fish_color_operator {}", q(&on_primary_container)),
        format!("set -g fish_color_autosuggestion {}", q(&faint)),
        format!("set -g fish_color_cancel {}", q(&dim)),
        format!("set -g fish_color_search_match --background={}", q(&primary_container)),
        format!("set -g fish_color_selection --background={}", q(&surface_container_high)),
        format!("set -g fish_pager_color_prefix {}", q(&primary)),
        format!("set -g fish_pager_color_completion {}", q(&cream)),
        format!("set -g fish_pager_color_description {}", q(&subtle)),
        format!("set -g fish_pager_color_progress {}", q(&dim)),
        format!("set -g fish_pager_color_selected_background --background={}", q(&surface_container_high)),
    ];
    let _ = fs::write(fish.join("syntax.fish"), lines.join("\n") + "\n");
}

pub fn render_foot(pill: &HashMap<String, String>, b: &HashMap<String, String>, ansi: &[String]) {
    let clean = |hex: &str| hex.trim().trim_start_matches('#').to_string();
    let base00 = clean(b.get("base00").map(String::as_str).unwrap_or("141a20"));
    let base07 = clean(b.get("base07").map(String::as_str).unwrap_or("abb4bc"));
    let base02 = clean(b.get("base02").map(String::as_str).unwrap_or("2c333b"));
    let primary = clean(pill.get("primary").map(String::as_str).unwrap_or("ffaa00"));

    let mut lines = vec![
        "[colors-dark]".to_string(),
        "blur=yes".to_string(),
        "alpha=0.85".to_string(),
        format!("background={base00}"),
        format!("foreground={base07}"),
        format!("cursor={primary} {base07}"),
        format!("selection-background={base02}"),
        format!("selection-foreground={base07}"),
    ];
    for i in 0..8 {
        let val = ansi.get(i).map(String::as_str).unwrap_or("000000");
        lines.push(format!("regular{i}={}", clean(val)));
    }
    for i in 0..8 {
        let val = ansi.get(i + 8).map(String::as_str).unwrap_or("ffffff");
        lines.push(format!("bright{i}={}", clean(val)));
    }
    let body = lines.join("\n") + "\n";

    let foot = tool_dir("foot").unwrap_or_else(|| {
        let p = home_path(&[".config", "foot"]);
        let _ = fs::create_dir_all(&p);
        p
    });
    let _ = fs::write(foot.join("colors.ini"), &body);

    let cache_xiu = cache_file("xiu");
    let _ = fs::create_dir_all(&cache_xiu);
    let _ = fs::write(cache_xiu.join("foot-colors.ini"), &body);
}

pub fn render_alacritty(
    pill: &HashMap<String, String>,
    b: &HashMap<String, String>,
    ansi: &[String],
) {
    let clean = |hex: &str| -> String {
        let h = hex.trim().trim_start_matches('#');
        format!("#{h}")
    };
    let base00 = clean(b.get("base00").map(String::as_str).unwrap_or("141a20"));
    let base07 = clean(b.get("base07").map(String::as_str).unwrap_or("abb4bc"));
    let base02 = clean(b.get("base02").map(String::as_str).unwrap_or("2c333b"));
    let primary = clean(pill.get("primary").map(String::as_str).unwrap_or("e0563b"));
    let bright_sel = clean(pill.get("bright").map(String::as_str).unwrap_or("fff6f0"));

    let ansi_col = |idx: usize, def: &str| -> String {
        clean(ansi.get(idx).map(String::as_str).unwrap_or(def))
    };

    let body = format!(
        r#"# Written by wallcolors on every palette change.

[colors.primary]
background = "{base00}"
foreground = "{base07}"

[colors.cursor]
cursor = "{primary}"
text = "{base00}"

[colors.selection]
background = "{base02}"
text = "{bright_sel}"

[colors.normal]
black = "{c0}"
red = "{c1}"
green = "{c2}"
yellow = "{c3}"
blue = "{c4}"
magenta = "{c5}"
cyan = "{c6}"
white = "{c7}"

[colors.bright]
black = "{c8}"
red = "{c9}"
green = "{c10}"
yellow = "{c11}"
blue = "{c12}"
magenta = "{c13}"
cyan = "{c14}"
white = "{c15}"
"#,
        c0 = ansi_col(0, "2e231b"),
        c1 = ansi_col(1, "c0442b"),
        c2 = ansi_col(2, "8a9a5b"),
        c3 = ansi_col(3, "d89a5b"),
        c4 = ansi_col(4, "6f8da0"),
        c5 = ansi_col(5, "b06a78"),
        c6 = ansi_col(6, "7fa89a"),
        c7 = ansi_col(7, "e6d6cb"),
        c8 = ansi_col(8, "594636"),
        c9 = ansi_col(9, "e0563b"),
        c10 = ansi_col(10, "a8b56e"),
        c11 = ansi_col(11, "f0b85e"),
        c12 = ansi_col(12, "8fa9bb"),
        c13 = ansi_col(13, "d08a96"),
        c14 = ansi_col(14, "9fc3b4"),
        c15 = ansi_col(15, "fff6f0"),
    );

    let alacritty = tool_dir("alacritty").unwrap_or_else(|| {
        let p = home_path(&[".config", "alacritty"]);
        let _ = fs::create_dir_all(&p);
        p
    });
    let _ = fs::write(alacritty.join("colors.toml"), &body);

    for sub in &["ricelin", "xiu"] {
        let d = cache_file(sub);
        let _ = fs::create_dir_all(&d);
        let _ = fs::write(d.join("alacritty-colors.toml"), &body);
    }
}

pub fn render_btop(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let d = match tool_dir("btop") {
        Some(dir) => dir,
        None => return,
    };
    let themes_dir = d.join("themes");
    let _ = fs::create_dir_all(&themes_dir);

    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let keys: Vec<(&str, String)> = vec![
        ("main_bg", "".to_string()),
        ("main_fg", g(pill, "cream")),
        ("title", g(pill, "bright")),
        ("hi_fg", g(pill, "primary")),
        ("selected_bg", g(pill, "surface_container_high")),
        ("selected_fg", g(pill, "bright")),
        ("inactive_fg", g(pill, "faint")),
        ("graph_text", g(pill, "subtle")),
        ("meter_bg", g(pill, "outline_variant")),
        ("proc_misc", g(pill, "subtle")),
        ("cpu_box", g(b, "base0c")),
        ("mem_box", g(b, "base0b")),
        ("net_box", g(b, "base0d")),
        ("proc_box", g(b, "base0e")),
        ("div_line", g(pill, "outline_variant")),
        ("temp_start", g(b, "base0b")),
        ("temp_mid", g(b, "base0a")),
        ("temp_end", g(b, "base08")),
        ("cpu_start", g(b, "base0e")),
        ("cpu_mid", g(b, "base0c")),
        ("cpu_end", g(pill, "primary")),
        ("free_start", g(b, "base0d")),
        ("free_mid", g(b, "base0e")),
        ("free_end", g(pill, "primary")),
        ("cached_start", g(b, "base0c")),
        ("cached_mid", g(b, "base0e")),
        ("cached_end", g(b, "base0d")),
        ("available_start", g(b, "base0a")),
        ("available_mid", g(b, "base09")),
        ("available_end", g(b, "base08")),
        ("used_start", g(b, "base0b")),
        ("used_mid", g(b, "base0e")),
        ("used_end", g(b, "base0c")),
        ("download_start", g(b, "base0a")),
        ("download_mid", g(b, "base09")),
        ("download_end", g(b, "base08")),
        ("upload_start", g(b, "base0b")),
        ("upload_mid", g(b, "base0e")),
        ("upload_end", g(b, "base0c")),
        ("process_start", g(b, "base0c")),
        ("process_mid", g(b, "base0d")),
        ("process_end", g(pill, "primary")),
    ];

    let mut lines = vec!["# Written by wallcolors.py on every palette change.".to_string()];
    for (k, v) in keys {
        lines.push(format!("theme[{k}]=\"{v}\""));
    }
    let _ = fs::write(themes_dir.join("xiu.theme"), lines.join("\n") + "\n");
    reload_tool("btop");
}

pub fn render_htop(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let d = match tool_dir("htop") {
        Some(dir) => dir,
        None => return,
    };
    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let lines = vec![
        "fields=0 48 17 18 38 39 40 2 46 47 49 1".to_string(),
        "sort_key=46".to_string(),
        "sort_direction=-1".to_string(),
        "tree_sort_key=0".to_string(),
        "tree_sort_direction=1".to_string(),
        "hide_kernel_threads=1".to_string(),
        "hide_userland_threads=0".to_string(),
        "shadow_other_users=0".to_string(),
        "show_thread_names=0".to_string(),
        "show_program_path=1".to_string(),
        "highlight_base_name=0".to_string(),
        "highlight_deleted_exe=1".to_string(),
        "highlight_megabytes=1".to_string(),
        "highlight_threads=1".to_string(),
        "highlight_changes=0".to_string(),
        "highlight_changes_delay_secs=5".to_string(),
        "find_comm_in_cmdline=1".to_string(),
        "strip_exe_from_cmdline=1".to_string(),
        "show_merged_command=0".to_string(),
        "tree_view=0".to_string(),
        "tree_view_always_by_pid=0".to_string(),
        "all_branches_collapsed=0".to_string(),
        "header_margin=1".to_string(),
        "detailed_cpu_time=0".to_string(),
        "cpu_count_from_one=0".to_string(),
        "show_cpu_usage=1".to_string(),
        "show_cpu_frequency=0".to_string(),
        "show_cpu_temperature=0".to_string(),
        "degree_fahrenheit=0".to_string(),
        "update_process_names=0".to_string(),
        "account_guest_in_cpu_meter=0".to_string(),
        "color_scheme=6".to_string(),
        format!("color_background={}", g(pill, "surface")),
        format!("color_text={}", g(pill, "cream")),
        format!("color_highlight={}", g(pill, "primary")),
        format!("color_selected={}", g(pill, "surface_container_high")),
        format!("color_cpu_low={}", g(b, "base0b")),
        format!("color_cpu_med={}", g(b, "base0a")),
        format!("color_cpu_high={}", g(b, "base08")),
        format!("color_mem_used={}", g(b, "base0c")),
        format!("color_mem_buffers={}", g(b, "base0e")),
        format!("color_mem_cache={}", g(b, "base0d")),
        format!("color_mem_available={}", g(b, "base0b")),
        format!("color_process_normal={}", g(pill, "cream")),
        format!("color_process_running={}", g(b, "base0b")),
        format!("color_process_sleeping={}", g(pill, "dim")),
    ];
    let _ = fs::write(d.join("htoprc"), lines.join("\n") + "\n");
}

pub fn render_nvtop(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let d = match tool_dir("nvtop") {
        Some(dir) => dir,
        None => return,
    };
    let clean = |hex: &str| hex.trim().trim_start_matches('#').to_string();
    let g = |m: &HashMap<String, String>, k: &str| clean(m.get(k).map(String::as_str).unwrap_or(""));

    let keys = vec![
        ("background", g(pill, "surface")),
        ("selected_bg", g(pill, "surface_container_high")),
        ("header_bg", g(pill, "surface_container_highest")),
        ("text", g(pill, "cream")),
        ("selected_text", g(pill, "primary")),
        ("header_text", g(pill, "bright")),
        ("inactive_text", g(pill, "faint")),
        ("gpu_util_low", g(b, "base0b")),
        ("gpu_util_med", g(b, "base0a")),
        ("gpu_util_high", g(b, "base08")),
        ("memory_low", g(b, "base0b")),
        ("memory_med", g(b, "base0e")),
        ("memory_high", g(b, "base0c")),
        ("temp_cool", g(b, "base0b")),
        ("temp_warm", g(b, "base0a")),
        ("temp_hot", g(b, "base08")),
        ("power_low", g(b, "base0b")),
        ("power_med", g(b, "base0a")),
        ("power_high", g(b, "base08")),
        ("process_normal", g(pill, "cream")),
        ("process_highlight", g(pill, "primary")),
        ("process_killed", g(b, "base08")),
        ("border", g(pill, "outline_variant")),
        ("separator", g(pill, "outline_variant")),
        ("chart_line", g(pill, "subtle")),
        ("chart_fill", g(pill, "surface_container")),
        ("status_ok", g(b, "base0b")),
        ("status_warning", g(b, "base0a")),
        ("status_error", g(b, "base08")),
        ("status_info", g(b, "base0c")),
    ];

    let mut lines = vec!["# Written by wallcolors.py on every palette change.".to_string()];
    for (k, v) in keys {
        lines.push(format!("{k} = {v}"));
    }
    let _ = fs::write(d.join("nvtop.colors"), lines.join("\n") + "\n");
}

pub fn render_cava(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let d = match tool_dir("cava") {
        Some(dir) => dir,
        None => return,
    };
    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let gradient = vec![
        g(b, "base0b"),
        g(b, "base0e"),
        g(b, "base0c"),
        g(pill, "primary"),
        g(b, "base0d"),
        g(b, "base0a"),
        g(b, "base09"),
        g(b, "base08"),
    ];

    let mut lines = vec![
        "# Written by wallcolors.py on every palette change.".to_string(),
        "[general]".to_string(),
        "framerate = 60".to_string(),
        "".to_string(),
        "[input]".to_string(),
        "method = pulse".to_string(),
        "source = auto".to_string(),
        "".to_string(),
        "[output]".to_string(),
        "method = ncurses".to_string(),
        "style = stereo".to_string(),
        "".to_string(),
        "[color]".to_string(),
        "background = default".to_string(),
        format!("foreground = {}", g(pill, "primary")),
        "gradient = 1".to_string(),
        "gradient_count = 8".to_string(),
    ];
    for (i, c) in gradient.iter().enumerate() {
        lines.push(format!("gradient_color_{} = '{c}'", i + 1));
    }
    lines.extend(vec![
        "".to_string(),
        "[smoothing]".to_string(),
        "noise_reduction = 85".to_string(),
        "monstercat = 1".to_string(),
    ]);

    let _ = fs::write(d.join("config"), lines.join("\n") + "\n");
    let _ = Command::new("killall")
        .args(["-USR1", "cava"])
        .stderr(Stdio::null())
        .status();
}

pub fn render_micro(
    pill: &HashMap<String, String>,
    b: &HashMap<String, String>,
    override_dir: Option<&Path>,
) {
    let dir = match override_dir {
        Some(d) => d.to_path_buf(),
        None => match tool_dir("micro") {
            Some(d) => d,
            None => return,
        },
    };
    let cs_dir = dir.join("colorschemes");
    let _ = fs::create_dir_all(&cs_dir);

    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let cream = g(pill, "cream");
    let primary = g(pill, "primary");
    let faint = g(pill, "faint");
    let subtle = g(pill, "subtle");
    let surface_container_low = g(pill, "surface_container_low");
    let surface_container = g(pill, "surface_container");
    let surface_container_high = g(pill, "surface_container_high");
    let dim = g(pill, "dim");
    let outline_variant = g(pill, "outline_variant");

    let links: Vec<(&str, Option<String>, Option<String>)> = vec![
        ("default", Some(cream.clone()), None),
        ("cursor", Some(primary.clone()), None),
        ("line-number", Some(faint.clone()), None),
        ("current-line-number", Some(subtle.clone()), None),
        ("gutter", Some(faint.clone()), None),
        ("cursor-line", None, Some(surface_container_low.clone())),
        ("color-column", None, Some(surface_container_low.clone())),
        ("statusline", Some(cream.clone()), Some(surface_container.clone())),
        ("statusline.active", Some(g(pill, "bright")), Some(surface_container_high.clone())),
        ("tabbar", Some(dim.clone()), None),
        ("divider", Some(surface_container_high.clone()), None),
        ("indent-char", Some(outline_variant.clone()), None),
        ("comment", Some(format!("italic {dim}")), None),
        ("identifier", Some(cream.clone()), None),
        ("identifier.class", Some(g(b, "base0a")), None),
        ("identifier.var", Some(cream.clone()), None),
        ("identifier.macro", Some(g(b, "base0e")), None),
        ("identifier.function", Some(g(b, "base0d")), None),
        ("constant", Some(g(b, "base0a")), None),
        ("constant.bool", Some(g(b, "base09")), None),
        ("constant.number", Some(g(b, "base09")), None),
        ("constant.string", Some(g(b, "base0b")), None),
        ("constant.string.escape", Some(g(b, "base0e")), None),
        ("constant.specialChar", Some(g(b, "base0e")), None),
        ("statement", Some(primary.clone()), None),
        ("keyword", Some(primary.clone()), None),
        ("keyword.operator", Some(subtle.clone()), None),
        ("symbol", Some(subtle.clone()), None),
        ("symbol.brackets", Some(faint.clone()), None),
        ("symbol.tag", Some(g(b, "base0d")), None),
        ("preproc", Some(g(b, "base0e")), None),
        ("type", Some(g(b, "base0c")), None),
        ("type.keyword", Some(primary.clone()), None),
        ("special", Some(g(b, "base0e")), None),
        ("underlined", Some(primary.clone()), None),
        ("error", Some(g(b, "base08")), None),
        ("warning", Some(g(b, "base0a")), None),
        ("todo", Some(format!("bold {primary}")), None),
        ("diff-added", Some(g(b, "base0b")), None),
        ("diff-modified", Some(g(b, "base0a")), None),
        ("diff-deleted", Some(g(b, "base08")), None),
    ];

    let mut lines = vec!["# Written by wallcolors.py on every palette change.".to_string()];
    for (group, fg, bg) in links {
        if fg.is_none() {
            let bg_str = bg.unwrap_or_default();
            lines.push(format!("color-link {group} \",{bg_str}\""));
        } else {
            let fg_str = fg.unwrap();
            let bg_part = match bg {
                Some(b) => format!(",{b}"),
                None => "".to_string(),
            };
            lines.push(format!("color-link {group} \"{fg_str}{bg_part}\""));
        }
    }
    let _ = fs::write(cs_dir.join("xiu.micro"), lines.join("\n") + "\n");
}

pub fn render_helix(
    pill: &HashMap<String, String>,
    b: &HashMap<String, String>,
    override_dir: Option<&Path>,
) {
    let dir = match override_dir {
        Some(d) => d.to_path_buf(),
        None => match tool_dir("helix") {
            Some(d) => d,
            None => return,
        },
    };
    let themes_dir = dir.join("themes");
    let _ = fs::create_dir_all(&themes_dir);

    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let fg = |col: &str| format!("\"{col}\"");
    let p = pill;

    let lines = vec![
        "# Written by wallcolors.py on every palette change.".to_string(),
        "# ui.background stays empty so the terminal's transparency shows through.".to_string(),
        "\"ui.background\" = {}".to_string(),
        format!("\"ui.background.separator\" = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        "\"ui.gutter\" = {}".to_string(),
        "\"ui.gutter.selected\" = {}".to_string(),
        format!("\"ui.text\" = {}", fg(&g(p, "cream"))),
        format!("\"ui.text.focus\" = {}", fg(&g(p, "bright"))),
        format!("\"ui.text.info\" = {}", fg(&g(p, "subtle"))),
        format!("\"ui.selection\" = {{ bg = \"{}\" }}", g(p, "surface_container_high")),
        format!("\"ui.selection.primary\" = {{ bg = \"{}\" }}", g(p, "surface_container")),
        "\"ui.cursorline\" = {}".to_string(),
        format!("\"ui.cursorline.primary\" = {{ underline = {{ color = \"{}\", style = \"line\" }} }}", g(p, "outline_variant")),
        format!("\"ui.cursorline.secondary\" = {{ underline = {{ color = \"{}\", style = \"line\" }} }}", g(p, "outline_variant")),
        format!("\"ui.linenr\" = {{ fg = \"{}\" }}", g(p, "faint")),
        format!("\"ui.linenr.selected\" = {{ fg = \"{}\" }}", g(p, "subtle")),
        format!("\"ui.statusline\" = {{ fg = \"{}\" }}", g(p, "cream")),
        format!("\"ui.statusline.inactive\" = {{ fg = \"{}\" }}", g(p, "dim")),
        format!("\"ui.statusline.separator\" = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        format!("\"ui.statusline.normal\" = {{ fg = \"{}\", bg = \"{}\", modifiers = [\"bold\"] }}", g(p, "on_primary_container"), g(p, "primary_container")),
        format!("\"ui.statusline.insert\" = {{ fg = \"{}\", bg = \"{}\", modifiers = [\"bold\"] }}", g(p, "bright"), g(p, "primary")),
        format!("\"ui.statusline.select\" = {{ fg = \"{}\", bg = \"{}\", modifiers = [\"bold\"] }}", g(p, "bright"), g(p, "primary_container")),
        format!("\"ui.bufferline\" = {{ fg = \"{}\" }}", g(p, "dim")),
        format!("\"ui.bufferline.active\" = {{ fg = \"{}\", underline = {{ color = \"{}\", style = \"line\" }}, modifiers = [\"bold\"] }}", g(p, "bright"), g(p, "primary")),
        "\"ui.bufferline.background\" = {}".to_string(),
        format!("\"ui.popup\" = {{ bg = \"{}\" }}", g(p, "surface_container")),
        format!("\"ui.popup.info\" = {{ bg = \"{}\" }}", g(p, "surface_container_high")),
        format!("\"ui.window\" = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        format!("\"ui.help\" = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "cream"), g(p, "surface_container")),
        format!("\"ui.menu\" = {{ bg = \"{}\" }}", g(p, "surface_container")),
        format!("\"ui.menu.selected\" = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "bright"), g(p, "surface_container_high")),
        format!("\"ui.virtual\" = {}", fg(&g(p, "outline_variant"))),
        format!("\"ui.virtual.whitespace\" = {}", fg(&g(p, "outline_variant"))),
        format!("\"ui.virtual.indent-guide\" = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        "\"ui.virtual.ruler\" = {}".to_string(),
        // Syntax scopes
        format!("\"attribute\" = {}", fg(&g(b, "base0d"))),
        format!("\"type\" = {}", fg(&g(b, "base0a"))),
        format!("\"type.builtin\" = {}", fg(&g(b, "base0c"))),
        format!("\"type.enum\" = {}", fg(&g(b, "base0a"))),
        format!("\"type.enum.variant\" = {}", fg(&g(b, "base0c"))),
        format!("\"constructor\" = {}", fg(&g(b, "base0c"))),
        format!("\"constant\" = {}", fg(&g(b, "base0a"))),
        format!("\"constant.builtin\" = {}", fg(&g(b, "base0a"))),
        format!("\"constant.builtin.boolean\" = {}", fg(&g(b, "base09"))),
        format!("\"constant.character\" = {}", fg(&g(b, "base0e"))),
        format!("\"constant.character.escape\" = {}", fg(&g(b, "base0e"))),
        format!("\"constant.numeric\" = {}", fg(&g(b, "base09"))),
        format!("\"constant.numeric.integer\" = {}", fg(&g(b, "base09"))),
        format!("\"constant.numeric.float\" = {}", fg(&g(b, "base09"))),
        format!("\"string\" = {}", fg(&g(b, "base0b"))),
        format!("\"string.regexp\" = {}", fg(&g(b, "base0c"))),
        format!("\"string.special\" = {}", fg(&g(b, "base0e"))),
        format!("\"string.special.symbol\" = {}", fg(&g(b, "base0b"))),
        format!("\"comment\" = {{ fg = \"{}\", modifiers = [\"italic\"] }}", g(p, "dim")),
        format!("\"comment.line\" = {{ fg = \"{}\", modifiers = [\"italic\"] }}", g(p, "dim")),
        format!("\"comment.block\" = {{ fg = \"{}\", modifiers = [\"italic\"] }}", g(p, "dim")),
        format!("\"comment.block.documentation\" = {{ fg = \"{}\", modifiers = [\"italic\"] }}", g(p, "subtle")),
        format!("\"variable\" = {}", fg(&g(p, "cream"))),
        format!("\"variable.builtin\" = {}", fg(&g(p, "primary"))),
        format!("\"variable.parameter\" = {}", fg(&g(p, "subtle"))),
        format!("\"variable.other.member\" = {}", fg(&g(p, "bright"))),
        format!("\"label\" = {}", fg(&g(p, "primary"))),
        format!("\"punctuation\" = {}", fg(&g(p, "dim"))),
        format!("\"punctuation.bracket\" = {}", fg(&g(p, "faint"))),
        format!("\"punctuation.delimiter\" = {}", fg(&g(p, "dim"))),
        format!("\"punctuation.special\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.control\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.control.conditional\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.control.repeat\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.control.import\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.control.return\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.control.exception\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.operator\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.directive\" = {}", fg(&g(b, "base0e"))),
        format!("\"keyword.function\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.storage\" = {}", fg(&g(p, "primary"))),
        format!("\"keyword.storage.type\" = {}", fg(&g(p, "primary"))),
        format!("\"operator\" = {}", fg(&g(p, "subtle"))),
        format!("\"function\" = {}", fg(&g(b, "base0d"))),
        format!("\"function.builtin\" = {}", fg(&g(b, "base0c"))),
        format!("\"function.method\" = {}", fg(&g(b, "base0d"))),
        format!("\"function.macro\" = {}", fg(&g(b, "base0e"))),
        format!("\"tag\" = {}", fg(&g(p, "primary"))),
        format!("\"special\" = {}", fg(&g(b, "base0e"))),
        format!("\"markup.heading\" = {{ fg = \"{}\", modifiers = [\"bold\"] }}", g(p, "bright")),
        "\"markup.bold\" = { modifiers = [\"bold\"] }".to_string(),
        "\"markup.italic\" = { modifiers = [\"italic\"] }".to_string(),
        "\"markup.strikethrough\" = { modifiers = [\"crossed_out\"] }".to_string(),
        format!("\"markup.link.url\" = {{ fg = \"{}\", modifiers = [\"underlined\"] }}", g(p, "dim")),
        format!("\"markup.link.text\" = {}", fg(&g(p, "primary"))),
        format!("\"markup.raw\" = {}", fg(&g(b, "base0b"))),
        format!("\"markup.list\" = {}", fg(&g(p, "primary"))),
        format!("\"diff.plus\" = {}", fg(&g(b, "base0b"))),
        format!("\"diff.minus\" = {}", fg(&g(b, "base08"))),
        format!("\"diff.delta\" = {}", fg(&g(b, "base0c"))),
        format!("\"error\" = {}", fg(&g(b, "base08"))),
        format!("\"warning\" = {}", fg(&g(b, "base0a"))),
        format!("\"info\" = {}", fg(&g(b, "base0c"))),
        format!("\"hint\" = {}", fg(&g(p, "subtle"))),
        format!("\"diagnostic.error\" = {{ underline = {{ color = \"{}\", style = \"curl\" }} }}", g(b, "base08")),
        format!("\"diagnostic.warning\" = {{ underline = {{ color = \"{}\", style = \"curl\" }} }}", g(b, "base0a")),
        format!("\"diagnostic.info\" = {{ underline = {{ color = \"{}\", style = \"curl\" }} }}", g(b, "base0c")),
        format!("\"diagnostic.hint\" = {{ underline = {{ color = \"{}\", style = \"curl\" }} }}", g(p, "subtle")),
    ];
    let _ = fs::write(themes_dir.join("xiu.toml"), lines.join("\n") + "\n");
}

pub fn render_bottom(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let d = match tool_dir("bottom") {
        Some(dir) => dir,
        None => return,
    };
    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let p = pill;

    let fresh = format!(
        r#"# [styles] kept fresh by wallcolors.py on every palette change.
[styles.cpu]
all_entry_colour = "{primary}"
avg_entry_colour = "{on_primary_container}"
cpu_core_colours = ["{primary}", "{on_primary_container}", "{subtle}", "{bright}", "{dim}", "{faint}"]

[styles.temp_graph]
temp_graph_colour_styles = ["{on_primary_container}", "{subtle}", "{primary}"]

[styles.memory]
ram_colour = "{base0d}"
cache_colour = "{base0c}"
swap_colour = "{base0e}"
arc_colour = "{base0a}"
gpu_colours = ["{primary}", "{subtle}", "{base0c}", "{base0b}", "{dim}", "{base0e}"]

[styles.network]
rx_colour = "{base0d}"
tx_colour = "{base0b}"
rx_total_colour = "{base0c}"
tx_total_colour = "{base0e}"

[styles.battery]
high_battery_colour = "{base0b}"
medium_battery_colour = "{base0a}"
low_battery_colour = "{base08}"

[styles.tables]
headers = {{colour = "{bright}", bold = true}}

[styles.graphs]
graph_colour = "{outline_variant}"
legend_text = {{colour = "{dim}"}}

[styles.widgets]
border_colour = "{outline_variant}"
selected_border_colour = "{primary}"
widget_title = {{colour = "{subtle}"}}
text = {{colour = "{cream}"}}
selected_text = {{colour = "{bright}", bg_colour = "{surface_container_high}"}}
disabled_text = {{colour = "{faint}"}}
"#,
        primary = g(p, "primary"),
        on_primary_container = g(p, "on_primary_container"),
        subtle = g(p, "subtle"),
        bright = g(p, "bright"),
        dim = g(p, "dim"),
        faint = g(p, "faint"),
        cream = g(p, "cream"),
        surface_container_high = g(p, "surface_container_high"),
        outline_variant = g(p, "outline_variant"),
        base0d = g(b, "base0d"),
        base0c = g(b, "base0c"),
        base0e = g(b, "base0e"),
        base0a = g(b, "base0a"),
        base0b = g(b, "base0b"),
        base08 = g(b, "base08"),
    );

    let cfg = d.join("bottom.toml");
    let body = if cfg.is_file() {
        if let Ok(content) = fs::read_to_string(&cfg) {
            let mut kept = Vec::new();
            let mut inside = false;
            for line in content.lines() {
                if line.starts_with("[styles") {
                    inside = true;
                    continue;
                }
                if inside && line.starts_with('[') {
                    inside = false;
                }
                if !inside {
                    kept.push(line);
                }
            }
            let base = kept.join("\n").trim_end().to_string();
            if !base.is_empty() {
                format!("{base}\n\n{fresh}")
            } else {
                fresh
            }
        } else {
            fresh
        }
    } else {
        fresh
    };
    let _ = fs::write(cfg, body);
}

pub fn render_yazi(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let d = match tool_dir("yazi") {
        Some(dir) => dir,
        None => return,
    };
    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let p = pill;

    let lines = vec![
        "# Written by wallcolors.py on every palette change.".to_string(),
        "[manager]".to_string(),
        format!("cwd = {{ fg = \"{}\", bold = true }}", g(p, "cream")),
        format!("hovered = {{ fg = \"{}\", bg = \"{}\", bold = true }}", g(p, "bright"), g(p, "surface_container_high")),
        "preview_hovered = { underline = true }".to_string(),
        format!("border_style = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        format!("find_keyword = {{ fg = \"{}\", bold = true }}", g(p, "primary")),
        format!("find_position = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "bright"), g(p, "surface_container_high")),
        format!("marker_selected = {{ fg = \"{}\", bold = true }}", g(p, "primary")),
        format!("marker_copied = {{ fg = \"{}\" }}", g(b, "base0b")),
        format!("marker_cut = {{ fg = \"{}\" }}", g(b, "base08")),
        format!("marker_marked = {{ fg = \"{}\" }}", g(b, "base0e")),
        format!("tab_active = {{ fg = \"{}\", bg = \"{}\", bold = true }}", g(p, "bright"), g(p, "surface_container_high")),
        format!("tab_inactive = {{ fg = \"{}\" }}", g(p, "dim")),
        format!("count_copied = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "bright"), g(b, "base0b")),
        format!("count_cut = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "bright"), g(b, "base08")),
        format!("count_selected = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "bright"), g(p, "primary")),
        "border_symbol = \"│\"".to_string(),
        "syntect_theme = \"\"".to_string(),
        "".to_string(),
        "[tabs]".to_string(),
        format!("active = {{ fg = \"{}\", bg = \"{}\", bold = true }}", g(p, "bright"), g(p, "surface_container_high")),
        format!("inactive = {{ fg = \"{}\" }}", g(p, "dim")),
        format!("sep = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        "".to_string(),
        "[mode]".to_string(),
        format!("normal_main = {{ fg = \"{}\", bg = \"{}\", bold = true }}", g(p, "on_primary_container"), g(p, "primary")),
        format!("normal_alt = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "cream"), g(p, "surface_container")),
        format!("select_main = {{ fg = \"{}\", bg = \"{}\", bold = true }}", g(p, "bright"), g(b, "base0d")),
        format!("select_alt = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "cream"), g(p, "surface_container_high")),
        format!("unset_main = {{ fg = \"{}\", bg = \"{}\", bold = true }}", g(p, "cream"), g(b, "base08")),
        format!("unset_alt = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "cream"), g(p, "surface_container_high")),
        "".to_string(),
        "[status]".to_string(),
        "separator_open = \"\"".to_string(),
        "separator_close = \"\"".to_string(),
        format!("separator_style = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        "".to_string(),
        "[select]".to_string(),
        format!("border = {{ fg = \"{}\" }}", g(p, "primary")),
        format!("active = {{ fg = \"{}\", bg = \"{}\" }}", g(p, "bright"), g(p, "surface_container_high")),
        format!("inactive = {{ fg = \"{}\" }}", g(p, "cream")),
        "".to_string(),
        "[input]".to_string(),
        format!("border = {{ fg = \"{}\" }}", g(p, "primary")),
        format!("title = {{ fg = \"{}\", bold = true }}", g(p, "cream")),
        format!("value = {{ fg = \"{}\" }}", g(p, "bright")),
        format!("selected = {{ bg = \"{}\" }}", g(p, "surface_container_high")),
        "".to_string(),
        "[which]".to_string(),
        format!("mask = {{ bg = \"{}\" }}", g(p, "surface_container")),
        format!("cand = {{ fg = \"{}\" }}", g(b, "base0c")),
        format!("rest = {{ fg = \"{}\" }}", g(p, "subtle")),
        format!("desc = {{ fg = \"{}\" }}", g(p, "cream")),
        "separator = \"  \"".to_string(),
        format!("separator_style = {{ fg = \"{}\" }}", g(p, "outline_variant")),
        "".to_string(),
        "[filetype]".to_string(),
        "rules = [".to_string(),
        format!("  {{ mime = \"image/*\", fg = \"{}\" }},", g(b, "base0a")),
        format!("  {{ mime = \"{{audio,video}}/*\", fg = \"{}\" }},", g(b, "base0d")),
        format!("  {{ mime = \"application/{{zip,rar,7z*,tar*,gzip,xz}}\", fg = \"{}\" }},", g(b, "base0e")),
        format!("  {{ mime = \"application/{{pdf,doc*,epub*}}\", fg = \"{}\" }},", g(b, "base0b")),
        format!("  {{ mime = \"inode/empty\", fg = \"{}\" }},", g(p, "dim")),
        format!("  {{ url = \"*/\", fg = \"{}\", bold = true }},", g(b, "base0d")),
        format!("  {{ url = \"*\", fg = \"{}\" }},", g(p, "cream")),
        "]".to_string(),
    ];
    let _ = fs::write(d.join("theme.toml"), lines.join("\n") + "\n");
}

pub fn render_zathura(
    pill: &HashMap<String, String>,
    b: &HashMap<String, String>,
    override_dir: Option<&Path>,
) {
    let dir = match override_dir {
        Some(d) => d.to_path_buf(),
        None => match tool_dir("zathura") {
            Some(d) => d,
            None => {
                let p = home_path(&[".config", "zathura"]);
                let _ = fs::create_dir_all(&p);
                p
            }
        },
    };

    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let clean = |hex: &str| hex.trim().trim_start_matches('#').to_string();
    let base00 = clean(b.get("base00").map(String::as_str).unwrap_or("141a20"));
    let base07 = clean(b.get("base07").map(String::as_str).unwrap_or("abb4bc"));
    let primary = g(pill, "primary");
    let subtle = g(pill, "subtle");
    let bright = g(pill, "bright");
    let error = g(b, "base08");
    let warning = g(b, "base0a");

    let bg_hex = format!("#{base00}");
    let fg_hex = format!("#{base07}");

    let hex_to_rgba = |hex: &str, alpha: f32| -> String {
        let s = hex.trim().trim_start_matches('#');
        if s.len() >= 6 {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
            format!("rgba({r},{g},{b},{alpha:.2})")
        } else {
            format!("rgba(20,17,15,{alpha:.2})")
        }
    };

    let bg_rgba = hex_to_rgba(&bg_hex, 0.85);
    let highlight_rgba = hex_to_rgba(&primary, 0.35);
    let highlight_active_rgba = hex_to_rgba(&primary, 0.65);

    let err_col = if !error.is_empty() { error } else { "#c0442b".to_string() };
    let warn_col = if !warning.is_empty() { warning } else { "#d89a5b".to_string() };

    let content = format!(
        r#"# Written by wallcolors on every palette change.

set recolor "true"
set recolor-keephue "true"
set recolor-reverse-video "false"

# Page background and text recolor (matches terminal background, transparency & blur)
set recolor-lightcolor "{bg_rgba}"
set recolor-darkcolor "{fg_hex}"

# Window background and foreground
set default-bg "{bg_rgba}"
set default-fg "{fg_hex}"

# Statusbar
set statusbar-bg "{bg_rgba}"
set statusbar-fg "{fg_hex}"

# Inputbar
set inputbar-bg "{bg_rgba}"
set inputbar-fg "{fg_hex}"

# Notifications
set notification-bg "{bg_rgba}"
set notification-fg "{fg_hex}"
set notification-error-bg "{err_col}"
set notification-error-fg "{bright}"
set notification-warning-bg "{warn_col}"
set notification-warning-fg "{bright}"

# Highlight & Selection
set highlight-color "{highlight_rgba}"
set highlight-active-color "{highlight_active_rgba}"

# Completion
set completion-bg "{bg_rgba}"
set completion-fg "{fg_hex}"
set completion-highlight-bg "{primary}"
set completion-highlight-fg "{bg_hex}"
set completion-group-bg "{bg_rgba}"
set completion-group-fg "{subtle}"

# Index mode (table of contents)
set index-bg "{bg_rgba}"
set index-fg "{fg_hex}"
set index-active-bg "{primary}"
set index-active-fg "{bg_hex}"

# Render loading
set render-loading "true"
set render-loading-bg "{bg_rgba}"
set render-loading-fg "{subtle}"
"#
    );

    let _ = fs::write(dir.join("theme"), &content);

    let rc_file = dir.join("zathurarc");
    if !rc_file.is_file() {
        let rc_content = "# Xiu zathura configuration\nset adjust-open \"width\"\nset selection-clipboard \"clipboard\"\nset window-title-basename \"true\"\nset statusbar-home-tilde \"true\"\n\ninclude theme\n";
        let _ = fs::write(rc_file, rc_content);
    }

    for sub in &["ricelin", "xiu"] {
        let d = cache_file(sub);
        let _ = fs::create_dir_all(&d);
        let _ = fs::write(d.join("zathura-theme"), &content);
    }
}

pub fn render_mpv(
    pill: &HashMap<String, String>,
    b: &HashMap<String, String>,
    custom_dir: Option<&Path>,
) {
    let mpv_dir = match custom_dir {
        Some(d) => d.to_path_buf(),
        None => match tool_dir("mpv") {
            Some(d) => d,
            None => return,
        },
    };

    let clean = |hex: &str| hex.trim().trim_start_matches('#').to_string();
    let base00 = clean(b.get("base00").map(String::as_str).unwrap_or("141a20"));
    let base02 = clean(b.get("base02").map(String::as_str).unwrap_or("2c333b"));
    let base07 = clean(b.get("base07").map(String::as_str).unwrap_or("abb4bc"));
    let base08 = clean(b.get("base08").map(String::as_str).unwrap_or("c0442b"));
    let base0a = clean(b.get("base0a").map(String::as_str).unwrap_or("d89a5b"));
    let base0b = clean(b.get("base0b").map(String::as_str).unwrap_or("8a9a5b"));

    let primary = clean(pill.get("primary").map(String::as_str).unwrap_or(&base08));
    let bright = clean(pill.get("bright").map(String::as_str).unwrap_or("ffffff"));

    // 1. Write theme.conf
    let theme_content = format!(
        "# Written by wallcolors on every palette change.\n\nosd-color=\"#{base07}\"\nosd-border-color=\"#{base00}\"\nsub-color=\"#{base07}\"\nsub-border-color=\"#{base00}\"\nbackground-color=\"#{base00}\"\n"
    );
    let theme_path = mpv_dir.join("theme.conf");
    let _ = fs::write(&theme_path, theme_content);

    // 2. Ensure mpv.conf includes theme.conf and remove hardcoded subtitle color overrides
    let conf_path = mpv_dir.join("mpv.conf");
    if conf_path.is_file() {
        if let Ok(content) = fs::read_to_string(&conf_path) {
            let mut cleaned_lines: Vec<String> = Vec::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("sub-color=") || trimmed.starts_with("sub-border-color=") {
                    continue;
                }
                cleaned_lines.push(line.to_string());
            }
            let mut updated = cleaned_lines.join("\n") + "\n";
            if !updated.contains("include=~~/theme.conf") {
                updated = format!("include=~~/theme.conf\n{}", updated);
            }
            let _ = fs::write(&conf_path, updated);
        }
    }

    // 3. Ensure scripts/theme.lua exists for dynamic live reloading
    let scripts_dir = mpv_dir.join("scripts");
    let _ = fs::create_dir_all(&scripts_dir);
    let theme_lua_path = scripts_dir.join("theme.lua");
    if !theme_lua_path.is_file() {
        let theme_lua_code = r#"-- Xiu MPV theme reload script: monitors theme changes and applies them live
local opt = require 'mp.options'
local utils = require 'mp.utils'

local function get_config_dir()
    return mp.find_config_file("mpv.conf") and mp.command_native({"expand-path", "~~/"}) or nil
end

local theme_path = nil
local last_mtime = 0

local function reload_theme()
    if not theme_path then
        local dir = get_config_dir()
        if dir then
            theme_path = utils.join_path(dir, "theme.conf")
        else
            return
        end
    end

    local info = utils.file_info(theme_path)
    if not info or info.mtime == last_mtime then
        return
    end
    last_mtime = info.mtime

    local f = io.open(theme_path, "r")
    if not f then return end

    for line in f:lines() do
        line = line:gsub("^%s+", ""):gsub("%s+$", "")
        if not line:match("^#") and line:find("=") then
            local k, v = line:match("([^=]+)=(.*)")
            if k and v then
                k = k:gsub("^%s+", ""):gsub("%s+$", "")
                v = v:gsub("^[\"']", ""):gsub("[\"']$", "")
                pcall(function()
                    mp.set_property(k, v)
                end)
            end
        end
    end
    f:close()

    local dir = get_config_dir()
    if dir then
        local uosc_conf = utils.join_path(dir, "script-opts/uosc.conf")
        local uf = io.open(uosc_conf, "r")
        if uf then
            for line in uf:lines() do
                if line:match("^color=") then
                    local color_val = line:sub(7)
                    pcall(function()
                        mp.commandv("change-list", "script-opts", "append", "uosc-color=" .. color_val)
                    end)
                    break
                end
            end
            uf:close()
        end
    end
end

mp.register_event("file-loaded", reload_theme)
reload_theme()
mp.add_periodic_timer(2, reload_theme)
"#;
        let _ = fs::write(&theme_lua_path, theme_lua_code);
    }

    // 4. Update or generate script-opts/uosc.conf with the current colors
    let script_opts_dir = mpv_dir.join("script-opts");
    let _ = fs::create_dir_all(&script_opts_dir);
    let uosc_path = script_opts_dir.join("uosc.conf");

    let color_line = format!(
        "color=foreground={primary},foreground_text={bright},background={base00},background_text={base07},window_border={base02},curtain={base00},success={base0b},error={base08},match={base0a}"
    );

    if uosc_path.is_file() {
        if let Ok(content) = fs::read_to_string(&uosc_path) {
            let mut lines: Vec<String> = Vec::new();
            let mut found_color = false;
            for line in content.lines() {
                if line.starts_with("color=") {
                    lines.push(color_line.clone());
                    found_color = true;
                } else {
                    lines.push(line.to_string());
                }
            }
            if !found_color {
                lines.push(color_line.clone());
            }
            let _ = fs::write(&uosc_path, lines.join("\n") + "\n");
        }
    } else {
        let uosc_content = format!(
            "# Written by wallcolors\ntimeline_style=line\ntimeline_line_width=2\ntimeline_size=30\ntimeline_border=1\ntimeline_step=5\ntimeline_cache=yes\ntimeline_heatmap=overlay\n\nprogress=windowed\nprogress_size=2\nprogress_line_width=20\n\ncontrols=menu,gap,subtitles,<has_many_audio>audio,space,gap,prev,play-pause,next,gap,space,speed,gap,fullscreen\ncontrols_size=28\ncontrols_margin=10\ncontrols_spacing=4\n\nscale=1\nscale_fullscreen=1.2\nfont_scale=1\ntext_border=1.2\nborder_radius=8\n\n{color_line}\n\nopacity=timeline=0.85,top_bar=0.85,volume=0.85,menu=0.95\n\nanimation_duration=120\nflash_duration=800\nproximity_in=40\nproximity_out=120\ndestination_time=playtime-remaining\npause_indicator=flash\n\nchapter_ranges=openings:30abf964,endings:30abf964,ads:{primary}80\nchapter_range_patterns=openings:オープニング;endings:エンディング\n\nsubtitles_directory=~~/subtitles\n"
        );
        let _ = fs::write(&uosc_path, uosc_content);
    }
}

pub fn render_spicetify(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let d = config_file(&["spicetify"]);
    let theme_dir = d.join("Themes").join("xiu");
    if !theme_dir.is_dir() {
        if d.is_dir() || on_path("spicetify") {
            let _ = fs::create_dir_all(&theme_dir);
        } else {
            return;
        }
    }

    let h = |k: &str| -> String {
        pill.get(k)
            .map(|s| s.trim_start_matches('#').to_uppercase())
            .unwrap_or_else(|| "FFFFFF".to_string())
    };
    let hb = |k: &str| -> String {
        b.get(k)
            .map(|s| s.trim_start_matches('#').to_uppercase())
            .unwrap_or_else(|| "FFFFFF".to_string())
    };

    let lines = vec![
        "; Xiu Spotify theme — colors kept fresh by xiu wallcolors on every".to_string(),
        "; palette change. Selected with: spicetify config current_theme xiu color_scheme xiu".to_string(),
        "[xiu]".to_string(),
        format!("text               = {}", h("bright")),
        format!("subtext            = {}", h("subtle")),
        format!("main               = {}", h("surface")),
        format!("main-elevated      = {}", h("surface_container_high")),
        format!("highlight          = {}", h("surface_container")),
        format!("highlight-elevated = {}", h("surface_container_highest")),
        format!("sidebar            = {}", h("surface_container")),
        format!("player             = {}", h("surface_container")),
        format!("card               = {}", h("primary_container")),
        format!("shadow             = {}", h("surface_container")),
        format!("selected-row       = {}", h("bright")),
        format!("button             = {}", h("primary")),
        format!("button-active      = {}", h("primary_container")),
        format!("button-disabled    = {}", h("outline_variant")),
        format!("tab-active         = {}", h("surface_container_high")),
        format!("notification       = {}", h("primary")),
        format!("notification-error = {}", hb("base08")),
        format!("misc               = {}", h("subtle")),
    ];

    let _ = fs::write(theme_dir.join("color.ini"), lines.join("\n") + "\n");

    let prefs = d.join("config-xpui.ini");
    let mut should_refresh = false;
    if prefs.is_file() {
        if let Ok(content) = fs::read_to_string(&prefs) {
            if content.contains("current_theme = xiu")
                || content.contains("current_theme                 = xiu")
            {
                should_refresh = true;
            }
        }
    } else if on_path("spicetify") {
        should_refresh = true;
    }

    if should_refresh && on_path("spicetify") {
        let _ = Command::new("spicetify")
            .arg("refresh")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

pub fn render_userchrome(pill: &HashMap<String, String>) {
    let g = |k: &str| pill.get(k).cloned().unwrap_or_default();
    let subs = vec![
        ("--xiu-surface", g("surface")),
        ("--xiu-surface-high", g("surface_container_high")),
        ("--xiu-cream", g("cream")),
        ("--xiu-dim", g("dim")),
        ("--xiu-outline", g("outline_variant")),
        ("--xiu-primary", g("primary")),
    ];

    for root in &[home_path(&[".mozilla", "firefox"]), home_path(&[".zen"])] {
        if !root.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let profile_path = entry.path();
                let uc = profile_path.join("chrome").join("userChrome.css");
                if uc.is_file() {
                    if let Ok(mut text) = fs::read_to_string(&uc) {
                        if !text.contains("--xiu-surface:") {
                            continue;
                        }
                        for (key, val) in &subs {
                            // Replace e.g. --xiu-surface:\s*#[0-9a-fA-F]{3,8}; with --xiu-surface: <val>;
                            let re_pat = format!("{key}:");
                            let mut new_lines = Vec::new();
                            for line in text.lines() {
                                if let Some(idx) = line.find(&re_pat) {
                                    if let Some(semi) = line[idx..].find(';') {
                                        let before = &line[..idx];
                                        let after = &line[idx + semi + 1..];
                                        new_lines.push(format!("{before}{key}: {val};{after}"));
                                        continue;
                                    }
                                }
                                new_lines.push(line.to_string());
                            }
                            text = new_lines.join("\n");
                        }
                        let _ = fs::write(&uc, text);
                    }
                }
            }
        }
    }
}

pub fn render_discord(pill: &HashMap<String, String>) {
    let g = |k: &str| pill.get(k).cloned().unwrap_or_default();
    let v = |c: &str, a: &str| format!("{c}{a}");

    let css = format!(
        r#"/**
 * @name xiu
 * @author yrpcaro
 * @description The xiu palette, regenerated by wallcolors.py on every wallpaper change.
 * @version 1.0.0
 */
:root {{
    --background-primary: {surface};
    --background-secondary: {surface_container};
    --background-secondary-alt: {surface_container_high};
    --background-tertiary: {surface_container_low};
    --background-floating: {surface_container_highest};
    --channeltextarea-background: {surface_container};
    --background-modifier-hover: {hover};
    --background-modifier-active: {active};
    --background-modifier-selected: {selected};
    --background-modifier-accent: {outline_variant};
    --text-normal: {cream};
    --text-muted: {subtle};
    --text-link: {primary};
    --header-primary: {bright};
    --header-secondary: {subtle};
    --interactive-normal: {subtle};
    --interactive-hover: {cream};
    --interactive-active: {bright};
    --interactive-muted: {faint};
    --channels-default: {subtle};
    --brand-experiment: {primary};
    --brand-experiment-560: {primary_container};
    --button-secondary-background: {surface_container};
    --scrollbar-auto-thumb: {outline_variant};
    --scrollbar-auto-track: transparent;
}}
"#,
        surface = g("surface"),
        surface_container = g("surface_container"),
        surface_container_high = g("surface_container_high"),
        surface_container_low = g("surface_container_low"),
        surface_container_highest = g("surface_container_highest"),
        hover = v(&g("surface_container_high"), "26"),
        active = v(&g("surface_container_high"), "40"),
        selected = v(&g("primary"), "26"),
        outline_variant = g("outline_variant"),
        cream = g("cream"),
        subtle = g("subtle"),
        primary = g("primary"),
        bright = g("bright"),
        faint = g("faint"),
        primary_container = g("primary_container"),
    );

    for client in &["vesktop", "vencord", "equicord"] {
        let tdir = home_path(&[".config", client, "themes"]);
        if tdir.is_dir() {
            let _ = fs::write(tdir.join("xiu.css"), &css);
        }
    }
}

pub fn render_vscode(pill: &HashMap<String, String>) {
    let g = |k: &str| pill.get(k).cloned().unwrap_or_default();
    let cc_entries = vec![
        ("editor.background", g("surface")),
        ("editor.foreground", g("cream")),
        ("editorCursor.foreground", g("primary")),
        ("editor.lineHighlightBackground", g("surface_container")),
        ("editor.selectionBackground", g("primary_container")),
        ("editorGroup.border", g("outline_variant")),
        ("tab.activeBackground", g("surface")),
        ("tab.inactiveBackground", g("surface_container_low")),
        ("tab.activeBorderTop", g("primary")),
        ("sideBar.background", g("surface_container_low")),
        ("sideBar.foreground", g("subtle")),
        ("activityBar.background", g("surface")),
        ("activityBar.foreground", g("subtle")),
        ("activityBar.activeBorder", g("primary")),
        ("titleBar.activeBackground", g("surface")),
        ("titleBar.activeForeground", g("subtle")),
        ("titleBar.inactiveBackground", g("surface")),
        ("titleBar.inactiveForeground", g("faint")),
        ("editorGroup.border", "#00000000".to_string()),
        ("sideBar.border", "#00000000".to_string()),
        ("tab.border", "#00000000".to_string()),
        ("statusBar.background", g("surface_container")),
        ("statusBar.foreground", g("subtle")),
        ("terminal.background", g("surface")),
        ("terminal.foreground", g("cream")),
        ("input.background", g("surface_container")),
        ("dropdown.background", g("surface_container")),
        ("list.activeSelectionBackground", g("surface_container_high")),
        ("list.hoverBackground", g("surface_container")),
        ("notifications.background", g("surface_container_high")),
        ("widget.border", g("outline_variant")),
        ("scrollbarSlider.background", g("outline_variant")),
        ("focusBorder", g("primary")),
        ("badge.background", g("primary")),
        ("badge.foreground", g("bright")),
        ("button.background", g("primary")),
        ("button.foreground", g("bright")),
    ];

    for editor in &["Code", "VSCodium"] {
        let sdir = home_path(&[".config", editor, "User"]);
        if !sdir.is_dir() {
            continue;
        }
        let sfile = sdir.join("settings.json");
        let mut data = if sfile.is_file() {
            if let Ok(c) = fs::read_to_string(&sfile) {
                crate::json::parse(&c).unwrap_or(crate::json::Json::Obj(Vec::new()))
            } else {
                crate::json::Json::Obj(Vec::new())
            }
        } else {
            crate::json::Json::Obj(Vec::new())
        };

        if let crate::json::Json::Obj(ref mut root_entries) = data {
            let mut cc_obj = Vec::new();
            for (k, v) in cc_entries.clone() {
                cc_obj.push((k.to_string(), crate::json::Json::Str(v)));
            }
            if let Some(pos) = root_entries.iter().position(|(k, _)| k == "workbench.colorCustomizations") {
                root_entries[pos] = ("workbench.colorCustomizations".to_string(), crate::json::Json::Obj(cc_obj));
            } else {
                root_entries.push(("workbench.colorCustomizations".to_string(), crate::json::Json::Obj(cc_obj)));
            }
            let _ = fs::write(&sfile, crate::json::stringify_pretty(&data, 4) + "\n");
        }
    }
}

pub fn render_zed(
    pill: &HashMap<String, String>,
    b_opt: Option<&HashMap<String, String>>,
    override_dir: Option<&Path>,
) {
    let dir = match override_dir {
        Some(d) => d.to_path_buf(),
        None => match tool_dir("zed") {
            Some(d) => d,
            None => return,
        },
    };
    let themes_dir = dir.join("themes");
    let _ = fs::create_dir_all(&themes_dir);

    let g = |m: &HashMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let p = pill;

    let fallback_b;
    let b = match b_opt {
        Some(b_map) => b_map,
        None => {
            fallback_b = {
                let mut m = HashMap::new();
                m.insert("base00".to_string(), g(p, "surface"));
                m.insert("base01".to_string(), g(p, "surface_container_low"));
                m.insert("base02".to_string(), g(p, "surface_container"));
                m.insert("base03".to_string(), g(p, "surface_container_high"));
                m.insert("base04".to_string(), g(p, "outline_variant"));
                m.insert("base05".to_string(), g(p, "dim"));
                m.insert("base06".to_string(), g(p, "subtle"));
                m.insert("base07".to_string(), g(p, "cream"));
                m.insert("base08".to_string(), g(p, "primary"));
                m.insert("base09".to_string(), p.get("on_primary_container").cloned().unwrap_or_else(|| g(p, "primary")));
                m.insert("base0a".to_string(), p.get("tick_rest").cloned().unwrap_or_else(|| g(p, "primary")));
                m.insert("base0b".to_string(), p.get("tick_rest").cloned().unwrap_or_else(|| g(p, "cream")));
                m.insert("base0c".to_string(), p.get("subtle").cloned().unwrap_or_else(|| g(p, "cream")));
                m.insert("base0d".to_string(), p.get("primary").cloned().unwrap_or_else(|| g(p, "bright")));
                m.insert("base0e".to_string(), p.get("primary_container").cloned().unwrap_or_else(|| g(p, "primary")));
                m.insert("base0f".to_string(), p.get("outline").cloned().unwrap_or_else(|| g(p, "dim")));
                m
            };
            &fallback_b
        }
    };

    let lum = |c: &str| -> f64 {
        let s = c.trim().trim_start_matches('#');
        if s.len() >= 6 {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0) as f64;
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0) as f64;
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0) as f64;
            0.2126 * r + 0.7152 * g + 0.0722 * b
        } else {
            0.0
        }
    };

    let hex_a = |c: &str, al: &str| -> String {
        let s = c.trim().trim_start_matches('#');
        let base = if s.len() >= 6 { &s[0..6] } else { "000000" };
        format!("#{base}{al}")
    };

    let a = |c: &str| -> String {
        let s = c.trim().trim_start_matches('#');
        if s.len() <= 6 {
            let base = if s.len() >= 6 { &s[0..6] } else { "000000" };
            format!("#{base}ff")
        } else {
            format!("#{s}")
        }
    };

    let syntax_entry = |col: &str, italic: bool, bold: bool| -> crate::json::Json {
        let mut obj = Vec::new();
        obj.push(("color".to_string(), crate::json::Json::Str(a(col))));
        if italic {
            obj.push(("font_style".to_string(), crate::json::Json::Str("italic".to_string())));
        }
        if bold {
            obj.push(("font_weight".to_string(), crate::json::Json::Num(700.0)));
        }
        crate::json::Json::Obj(obj)
    };

    let mut syntax_tree = Vec::new();
    syntax_tree.push(("attribute".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("boolean".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("comment".to_string(), syntax_entry(&g(p, "dim"), true, false)));
    syntax_tree.push(("comment.doc".to_string(), syntax_entry(&g(p, "dim"), true, false)));
    syntax_tree.push(("constant".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("constructor".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("emphasis".to_string(), syntax_entry(&g(p, "cream"), true, false)));
    syntax_tree.push(("emphasis.strong".to_string(), syntax_entry(&g(p, "bright"), false, true)));
    syntax_tree.push(("function".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("function.builtin".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("function.method".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("function.macro".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("keyword".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("keyword.control".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("keyword.operator".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("label".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("link_text".to_string(), syntax_entry(&g(p, "cream"), false, false)));
    syntax_tree.push(("link_uri".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("number".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("operator".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("punctuation".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("punctuation.bracket".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("punctuation.delimiter".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("punctuation.list_marker".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("punctuation.special".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("string".to_string(), syntax_entry(&g(b, "base0b"), false, false)));
    syntax_tree.push(("string.escape".to_string(), syntax_entry(&g(p, "subtle"), false, false)));
    syntax_tree.push(("string.regex".to_string(), syntax_entry(b.get("base0c").unwrap_or(&g(p, "subtle")), false, false)));
    syntax_tree.push(("string.special".to_string(), syntax_entry(&g(b, "base0b"), false, false)));
    syntax_tree.push(("string.special.symbol".to_string(), syntax_entry(&g(b, "base0b"), false, false)));
    syntax_tree.push(("tag".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("text.literal".to_string(), syntax_entry(&g(b, "base0b"), false, false)));
    syntax_tree.push(("title".to_string(), syntax_entry(&g(p, "primary"), false, true)));
    syntax_tree.push(("type".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("type.builtin".to_string(), syntax_entry(&g(p, "primary"), false, false)));
    syntax_tree.push(("variable".to_string(), syntax_entry(&g(p, "cream"), false, false)));
    syntax_tree.push(("variable.special".to_string(), syntax_entry(&g(p, "subtle"), false, false)));

    let player_entry = |cur: &str, sel: &str, bg: &str| -> crate::json::Json {
        let mut obj = Vec::new();
        obj.push(("cursor".to_string(), crate::json::Json::Str(a(cur))));
        obj.push(("selection".to_string(), crate::json::Json::Str(hex_a(sel, "25"))));
        obj.push(("background".to_string(), crate::json::Json::Str(a(bg))));
        crate::json::Json::Obj(obj)
    };

    let players = vec![
        player_entry(&g(p, "primary"), &g(p, "bright"), &g(p, "primary")),
        player_entry(&g(b, "base0b"), &g(b, "base0b"), &g(b, "base0b")),
        player_entry(&g(b, "base08"), &g(b, "base08"), &g(b, "base08")),
        player_entry(&g(p, "subtle"), &g(p, "subtle"), &g(p, "subtle")),
    ];

    let mut base_elements: HashMap<String, String> = HashMap::new();
    base_elements.insert("border".into(), "#00000000".into());
    base_elements.insert("border.variant".into(), "#00000000".into());
    base_elements.insert("border.focused".into(), a(&g(p, "primary")));
    base_elements.insert("border.selected".into(), a(&g(p, "primary")));
    base_elements.insert("border.transparent".into(), "#00000000".into());
    base_elements.insert("border.disabled".into(), "#00000000".into());
    base_elements.insert("panel.focused_border".into(), "#00000000".into());
    base_elements.insert("pane.focused_border".into(), "#00000000".into());
    base_elements.insert("pane_group.border".into(), "#00000000".into());

    base_elements.insert("elevated_surface.background".into(), a(&g(p, "surface_container")));
    base_elements.insert("panel.overlay_background".into(), a(&g(p, "surface_container")));

    base_elements.insert("element.background".into(), a(&g(p, "surface_container_low")));
    base_elements.insert("element.hover".into(), a(&g(p, "surface_container")));
    base_elements.insert("element.active".into(), a(&g(p, "surface_container_high")));
    base_elements.insert("element.selected".into(), a(&g(p, "surface_container_high")));
    base_elements.insert("element.disabled".into(), a(&g(p, "surface_container_low")));
    base_elements.insert("ghost_element.background".into(), "#00000000".into());
    base_elements.insert("ghost_element.hover".into(), a(&g(p, "surface_container")));
    base_elements.insert("ghost_element.active".into(), a(&g(p, "surface_container_high")));
    base_elements.insert("ghost_element.selected".into(), a(&g(p, "surface_container_high")));
    base_elements.insert("ghost_element.disabled".into(), "#00000000".into());
    base_elements.insert("drop_target.background".into(), hex_a(&g(p, "primary"), "50"));

    base_elements.insert("text".into(), a(&g(p, "cream")));
    base_elements.insert("text.muted".into(), a(&g(p, "subtle")));
    base_elements.insert("text.placeholder".into(), a(&g(p, "dim")));
    base_elements.insert("text.disabled".into(), a(&g(p, "faint")));
    base_elements.insert("text.accent".into(), a(&g(p, "primary")));
    base_elements.insert("icon".into(), a(&g(p, "cream")));
    base_elements.insert("icon.muted".into(), a(&g(p, "subtle")));
    base_elements.insert("icon.disabled".into(), a(&g(p, "faint")));
    base_elements.insert("icon.placeholder".into(), a(&g(p, "dim")));
    base_elements.insert("icon.accent".into(), a(&g(p, "primary")));

    base_elements.insert("panel.indent_guide".into(), a(&g(p, "outline_variant")));
    base_elements.insert("panel.indent_guide_active".into(), a(&g(p, "outline")));
    base_elements.insert("panel.indent_guide_hover".into(), a(&g(p, "primary")));
    base_elements.insert("editor.indent_guide".into(), a(&g(p, "outline_variant")));
    base_elements.insert("editor.indent_guide_active".into(), a(&g(p, "outline")));
    base_elements.insert("editor.wrap_guide".into(), a(&g(p, "outline_variant")));
    base_elements.insert("editor.active_wrap_guide".into(), a(&g(p, "outline_variant")));
    base_elements.insert("editor.invisible".into(), a(&g(p, "faint")));

    base_elements.insert("scrollbar.thumb.background".into(), hex_a(&g(p, "outline_variant"), "80"));
    base_elements.insert("scrollbar.thumb.hover_background".into(), a(&g(p, "outline")));
    base_elements.insert("scrollbar.thumb.active_background".into(), a(&g(p, "outline")));
    base_elements.insert("scrollbar.thumb.border".into(), "#00000000".into());
    base_elements.insert("scrollbar.track.background".into(), "#00000000".into());
    base_elements.insert("scrollbar.track.border".into(), "#00000000".into());

    base_elements.insert("search.match_background".into(), hex_a(&g(p, "bright"), "25"));
    base_elements.insert("search.active_match_background".into(), hex_a(&g(p, "primary"), "50"));

    base_elements.insert("editor.foreground".into(), a(&g(p, "cream")));
    base_elements.insert("editor.line_number".into(), a(&g(p, "faint")));
    base_elements.insert("editor.active_line_number".into(), a(&g(p, "bright")));
    base_elements.insert("editor.highlighted_line.background".into(), hex_a(&g(p, "bright"), "10"));
    base_elements.insert("editor.document_highlight.read_background".into(), hex_a(&g(p, "bright"), "15"));
    base_elements.insert("editor.document_highlight.write_background".into(), hex_a(&g(p, "bright"), "15"));
    base_elements.insert("editor.subheader.background".into(), a(&g(p, "surface_container_low")));

    base_elements.insert("link_text.hover".into(), a(p.get("primary_container").unwrap_or(&g(p, "primary"))));

    base_elements.insert("conflict".into(), a(&g(b, "base0a")));
    base_elements.insert("conflict.background".into(), hex_a(&g(b, "base0a"), "15"));
    base_elements.insert("conflict.border".into(), a(&g(b, "base0a")));
    base_elements.insert("created".into(), a(&g(b, "base0b")));
    base_elements.insert("created.background".into(), hex_a(&g(b, "base0b"), "15"));
    base_elements.insert("created.border".into(), a(&g(b, "base0b")));
    base_elements.insert("deleted".into(), a(&g(b, "base08")));
    base_elements.insert("deleted.background".into(), hex_a(&g(b, "base08"), "15"));
    base_elements.insert("deleted.border".into(), a(&g(b, "base08")));
    base_elements.insert("error".into(), a(&g(b, "base08")));
    base_elements.insert("error.background".into(), hex_a(&g(b, "base08"), "25"));
    base_elements.insert("error.border".into(), a(&g(b, "base08")));
    base_elements.insert("hidden".into(), a(&g(p, "dim")));
    base_elements.insert("hidden.background".into(), a(&g(p, "surface")));
    base_elements.insert("hidden.border".into(), a(&g(p, "surface")));
    base_elements.insert("hint".into(), a(&g(p, "subtle")));
    base_elements.insert("hint.background".into(), a(&g(p, "surface_container_low")));
    base_elements.insert("hint.border".into(), a(&g(p, "surface_container")));
    base_elements.insert("ignored".into(), a(&g(p, "dim")));
    base_elements.insert("ignored.background".into(), a(&g(p, "surface")));
    base_elements.insert("ignored.border".into(), a(&g(p, "surface")));
    base_elements.insert("info".into(), a(&g(b, "base0c")));
    base_elements.insert("info.background".into(), a(&g(p, "surface_container_low")));
    base_elements.insert("info.border".into(), a(&g(p, "surface_container")));
    base_elements.insert("modified".into(), a(&g(b, "base0a")));
    base_elements.insert("modified.background".into(), hex_a(&g(b, "base0a"), "15"));
    base_elements.insert("modified.border".into(), a(&g(b, "base0a")));
    base_elements.insert("predictive".into(), a(&g(p, "dim")));
    base_elements.insert("predictive.background".into(), a(&g(p, "surface")));
    base_elements.insert("predictive.border".into(), a(&g(p, "surface")));
    base_elements.insert("renamed".into(), a(&g(b, "base0a")));
    base_elements.insert("renamed.background".into(), hex_a(&g(b, "base0a"), "15"));
    base_elements.insert("renamed.border".into(), a(&g(b, "base0a")));
    base_elements.insert("success".into(), a(&g(b, "base0b")));
    base_elements.insert("success.background".into(), hex_a(&g(b, "base0b"), "15"));
    base_elements.insert("success.border".into(), a(&g(b, "base0b")));
    base_elements.insert("unreachable".into(), a(&g(b, "base08")));
    base_elements.insert("unreachable.background".into(), hex_a(&g(b, "base08"), "15"));
    base_elements.insert("unreachable.border".into(), a(&g(b, "base08")));
    base_elements.insert("warning".into(), a(&g(b, "base0a")));
    base_elements.insert("warning.background".into(), hex_a(&g(b, "base0a"), "25"));
    base_elements.insert("warning.border".into(), a(&g(b, "base0a")));

    base_elements.insert("terminal.foreground".into(), a(&g(p, "cream")));
    base_elements.insert("terminal.bright_foreground".into(), a(&g(p, "bright")));
    base_elements.insert("terminal.dim_foreground".into(), a(&g(p, "dim")));
    base_elements.insert("terminal.ansi.background".into(), a(&g(p, "surface")));
    base_elements.insert("terminal.ansi.black".into(), a(&g(b, "base00")));
    base_elements.insert("terminal.ansi.bright_black".into(), a(&g(b, "base03")));
    base_elements.insert("terminal.ansi.red".into(), a(&g(b, "base08")));
    base_elements.insert("terminal.ansi.bright_red".into(), a(&g(b, "base08")));
    base_elements.insert("terminal.ansi.green".into(), a(&g(b, "base0b")));
    base_elements.insert("terminal.ansi.bright_green".into(), a(&g(b, "base0b")));
    base_elements.insert("terminal.ansi.yellow".into(), a(&g(b, "base0a")));
    base_elements.insert("terminal.ansi.bright_yellow".into(), a(&g(b, "base0a")));
    base_elements.insert("terminal.ansi.blue".into(), a(&g(b, "base0d")));
    base_elements.insert("terminal.ansi.bright_blue".into(), a(&g(b, "base0d")));
    base_elements.insert("terminal.ansi.magenta".into(), a(&g(b, "base0e")));
    base_elements.insert("terminal.ansi.bright_magenta".into(), a(&g(b, "base0e")));
    base_elements.insert("terminal.ansi.cyan".into(), a(&g(b, "base0c")));
    base_elements.insert("terminal.ansi.bright_cyan".into(), a(&g(b, "base0c")));
    base_elements.insert("terminal.ansi.white".into(), a(&g(b, "base07")));
    base_elements.insert("terminal.ansi.bright_white".into(), a(&g(p, "bright")));

    let make_style_obj = |overrides: &[(&str, String)]| -> crate::json::Json {
        let mut map = base_elements.clone();
        for (k, v) in overrides {
            map.insert(k.to_string(), v.clone());
        }
        let mut entries = Vec::new();
        // Sort keys for deterministic output
        let mut keys: Vec<String> = map.keys().cloned().collect();
        keys.sort();
        for k in keys {
            entries.push((k.clone(), crate::json::Json::Str(map.get(&k).unwrap().clone())));
        }
        entries.push(("players".to_string(), crate::json::Json::Arr(players.clone())));
        entries.push(("syntax".to_string(), crate::json::Json::Obj(syntax_tree.clone())));
        crate::json::Json::Obj(entries)
    };

    let flat_overrides = vec![
        ("background", a(&g(p, "surface"))),
        ("surface.background", a(&g(p, "surface"))),
        ("panel.background", a(&g(p, "surface"))),
        ("title_bar.background", a(&g(p, "surface"))),
        ("title_bar.inactive_background", a(&g(p, "surface"))),
        ("status_bar.background", a(&g(p, "surface"))),
        ("toolbar.background", a(&g(p, "surface"))),
        ("tab_bar.background", a(&g(p, "surface"))),
        ("tab.active_background", a(&g(p, "surface_container_low"))),
        ("tab.inactive_background", a(&g(p, "surface"))),
        ("editor.background", a(&g(p, "surface"))),
        ("editor.gutter.background", a(&g(p, "surface"))),
        ("editor.active_line.background", a(&g(p, "surface_container_low"))),
        ("terminal.background", a(&g(p, "surface"))),
    ];
    let flat_style = make_style_obj(&flat_overrides);

    let blur_overrides = vec![
        ("background.appearance", "blurred".to_string()),
        ("background", hex_a(&g(p, "surface"), "b8")),
        ("surface.background", hex_a(&g(p, "surface"), "b8")),
        ("panel.background", "#00000000".to_string()),
        ("title_bar.background", hex_a(&g(p, "surface"), "b8")),
        ("title_bar.inactive_background", hex_a(&g(p, "surface"), "b8")),
        ("status_bar.background", hex_a(&g(p, "surface"), "b8")),
        ("toolbar.background", "#00000000".to_string()),
        ("tab_bar.background", "#00000000".to_string()),
        ("tab.active_background", a(&g(p, "surface_container_low"))),
        ("tab.inactive_background", "#00000000".to_string()),
        ("editor.background", "#00000000".to_string()),
        ("editor.gutter.background", "#00000000".to_string()),
        ("editor.active_line.background", "#00000000".to_string()),
        ("terminal.background", "#00000000".to_string()),
    ];
    let blur_style = make_style_obj(&blur_overrides);

    let appearance = if lum(&g(p, "surface")) > 128.0 {
        "light"
    } else {
        "dark"
    };

    let theme_flat = crate::json::Json::Obj(vec![
        ("name".to_string(), crate::json::Json::Str("xiu".to_string())),
        ("appearance".to_string(), crate::json::Json::Str(appearance.to_string())),
        ("style".to_string(), flat_style),
    ]);
    let theme_blur = crate::json::Json::Obj(vec![
        ("name".to_string(), crate::json::Json::Str("xiu blur".to_string())),
        ("appearance".to_string(), crate::json::Json::Str(appearance.to_string())),
        ("style".to_string(), blur_style),
    ]);

    let full_theme = crate::json::Json::Obj(vec![
        ("$schema".to_string(), crate::json::Json::Str("https://zed.dev/schema/themes/v0.2.0.json".to_string())),
        ("name".to_string(), crate::json::Json::Str("xiu".to_string())),
        ("author".to_string(), crate::json::Json::Str("yrpcaro".to_string())),
        ("themes".to_string(), crate::json::Json::Arr(vec![theme_flat, theme_blur])),
    ]);

    let _ = fs::write(themes_dir.join("xiu.json"), crate::json::stringify_pretty(&full_theme, 2) + "\n");

    let settings = dir.join("settings.json");
    if settings.is_file() {
        if let Ok(mut text) = fs::read_to_string(&settings) {
            if !text.contains("\"theme\"") {
                let trimmed = text.trim_end();
                if trimmed.ends_with('}') {
                    let head = trimmed[..trimmed.len() - 1].trim_end();
                    let joiner = if head.ends_with(',') { "" } else { "," };
                    text = format!("{head}{joiner}\n  \"theme\": {{ \"mode\": \"system\", \"dark\": \"xiu\", \"light\": \"xiu\" }}\n}}\n");
                    let _ = fs::write(&settings, text);
                }
            }
        }
    }
}

pub fn render_browser(pill: &HashMap<String, String>) {
    let d = config_file(&["xiu"]);
    let _ = fs::create_dir_all(&d);

    let surface = pill.get("surface").cloned().unwrap_or_else(|| "#141a20".to_string());
    let surface_container = pill.get("surface_container").cloned().unwrap_or_else(|| "#252538".to_string());
    let surface_container_low = pill.get("surface_container_low").cloned().unwrap_or_else(|| "#181825".to_string());
    let surface_container_high = pill.get("surface_container_high").cloned().unwrap_or_else(|| "#313244".to_string());
    let primary = pill.get("primary").cloned().unwrap_or_else(|| "#f38ba8".to_string());
    let cream = pill.get("cream").cloned().unwrap_or_else(|| "#cdd6f4".to_string());
    let subtle = pill.get("subtle").cloned().unwrap_or_else(|| "#bac2de".to_string());
    let dim = pill.get("dim").cloned().unwrap_or_else(|| "#a6adc8".to_string());
    let faint = pill.get("faint").cloned().unwrap_or_else(|| "#6c7086".to_string());

    // 1. Write ~/.config/xiu/browser-theme.json policy payload
    let mut theme_json = Vec::new();
    theme_json.push(("BrowserThemeColor".to_string(), crate::json::Json::Str(surface.clone())));
    theme_json.push(("BrowserColorScheme".to_string(), crate::json::Json::Str("device".to_string())));
    let body = crate::json::stringify_pretty(&crate::json::Json::Obj(theme_json), 2) + "\n";
    let payload_path = d.join("browser-theme.json");
    let _ = fs::write(&payload_path, &body);

    // 2. Write to browser managed policies
    let home = std::env::var("HOME").unwrap_or_default();
    let user_targets = [
        format!("{home}/.config/BraveSoftware/Brave-Browser/policies/managed/xiu.json"),
        format!("{home}/.config/chromium/policies/managed/xiu.json"),
        format!("{home}/.config/brave/policies/managed/xiu.json"),
        format!("{home}/.config/google-chrome/policies/managed/xiu.json"),
    ];
    for target in &user_targets {
        let p = Path::new(target);
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
            let _ = fs::write(p, &body);
        }
    }

    // 3. Update Chromium Preferences files with SkColor
    let hex = surface.trim().trim_start_matches('#');
    if hex.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            let val: u32 = (0xFF << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            let sk_color: i32 = val as i32;
            let pref_py = format!(
                r#"import json, glob, os
home = os.path.expanduser('~')
for pref in glob.glob(f'{{home}}/.config/**/Preferences', recursive=True):
    if any(b in pref for b in ('BraveSoftware', 'chromium', 'google-chrome', 'brave')):
        try:
            with open(pref, 'r') as f:
                prefs = json.load(f)
            if 'autogenerated' not in prefs or not isinstance(prefs['autogenerated'], dict):
                prefs['autogenerated'] = {{}}
            if 'theme' not in prefs['autogenerated'] or not isinstance(prefs['autogenerated']['theme'], dict):
                prefs['autogenerated']['theme'] = {{}}
            prefs['autogenerated']['theme']['color'] = {sk_color}
            if 'browser' not in prefs or not isinstance(prefs['browser'], dict):
                prefs['browser'] = {{}}
            if 'theme' not in prefs['browser'] or not isinstance(prefs['browser']['theme'], dict):
                prefs['browser']['theme'] = {{}}
            prefs['browser']['theme']['color'] = {sk_color}
            with open(pref, 'w') as f:
                json.dump(prefs, f, indent=2)
        except Exception:
            pass
"#
            );
            let _ = Command::new("python3").args(["-c", &pref_py]).status();
        }
    }

    // 4. Generate dynamic brave-theme/manifest.json matching active palette
    let parse_rgb = |hex_str: &str| -> (u8, u8, u8) {
        let s = hex_str.trim().trim_start_matches('#');
        if s.len() >= 6 {
            (
                u8::from_str_radix(&s[0..2], 16).unwrap_or(0),
                u8::from_str_radix(&s[2..4], 16).unwrap_or(0),
                u8::from_str_radix(&s[4..6], 16).unwrap_or(0),
            )
        } else {
            (0, 0, 0)
        }
    };

    let (r_surf, g_surf, b_surf) = parse_rgb(&surface);
    let (r_cont, g_cont, b_cont) = parse_rgb(&surface_container);
    let (r_surflow, g_surflow, b_surflow) = parse_rgb(&surface_container_low);
    let (r_conthigh, g_conthigh, b_conthigh) = parse_rgb(&surface_container_high);
    let (r_pri, g_pri, b_pri) = parse_rgb(&primary);
    let (r_cream, g_cream, b_cream) = parse_rgb(&cream);
    let (r_subtle, g_subtle, b_subtle) = parse_rgb(&subtle);
    let (r_dim, g_dim, b_dim) = parse_rgb(&dim);
    let (r_faint, g_faint, b_faint) = parse_rgb(&faint);

    let brave_manifest = format!(
        r#"{{
  "manifest_version": 3,
  "version": "1.0.0",
  "name": "Xiu",
  "description": "Theme matching the active xiu desktop palette.",
  "theme": {{
    "colors": {{
      "frame": [{r_surf}, {g_surf}, {b_surf}],
      "frame_inactive": [{r_surflow}, {g_surflow}, {b_surflow}],
      "frame_incognito": [{r_surf}, {g_surf}, {b_surf}],
      "frame_incognito_inactive": [{r_surflow}, {g_surflow}, {b_surflow}],
      "toolbar": [{r_cont}, {g_cont}, {b_cont}],
      "ntp_background": [{r_surf}, {g_surf}, {b_surf}],
      "ntp_text": [{r_cream}, {g_cream}, {b_cream}],
      "ntp_link": [{r_pri}, {g_pri}, {b_pri}],
      "ntp_header": [{r_conthigh}, {g_conthigh}, {b_conthigh}],
      "tab_text": [{r_cream}, {g_cream}, {b_cream}],
      "tab_background_text": [{r_dim}, {g_dim}, {b_dim}],
      "tab_background_text_inactive": [{r_faint}, {g_faint}, {b_faint}],
      "background_tab": [{r_surflow}, {g_surflow}, {b_surflow}],
      "bookmark_text": [{r_cream}, {g_cream}, {b_cream}],
      "toolbar_text": [{r_cream}, {g_cream}, {b_cream}],
      "toolbar_button_icon": [{r_subtle}, {g_subtle}, {b_subtle}],
      "button_background": [{r_cont}, {g_cont}, {b_cont}],
      "omnibox_background": [{r_conthigh}, {g_conthigh}, {b_conthigh}],
      "omnibox_text": [{r_cream}, {g_cream}, {b_cream}]
    }},
    "tints": {{
      "frame": [-1, -1, -1],
      "background_tab": [-1, -1, -1],
      "buttons": [-1, -1, -1]
    }},
    "properties": {{
      "ntp_background_alignment": "center",
      "ntp_background_repeat": "no-repeat"
    }}
  }}
}}
"#
    );

    let brave_dirs = [
        d.join("brave-theme"),
        home_path(&["xiu", "configs", "brave-theme"]),
    ];
    for b_dir in &brave_dirs {
        let _ = fs::create_dir_all(b_dir);
        let _ = fs::write(b_dir.join("manifest.json"), &brave_manifest);
        let _ = fs::remove_file(b_dir.join("Cached Theme.pak"));
    }

    // 5. Ensure native messaging host manifests are present for Firefox, Zen, and Chromium/Brave
    let host_py_candidate = d.join("browser-integration").join("xiufox-host.py");
    let host_py = if host_py_candidate.is_file() {
        host_py_candidate
    } else {
        d.join("xiufox-host.py")
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if host_py.is_file() {
            let _ = fs::set_permissions(&host_py, fs::Permissions::from_mode(0o755));
        }
    }

    let gecko_manifest = format!(
        r#"{{
  "name": "io.github.yrpcaro.xiu",
  "description": "Xiu live theme host",
  "path": "{}",
  "type": "stdio",
  "allowed_extensions": ["xiu-theme@yrpcaro"]
}}
"#,
        host_py.to_string_lossy()
    );

    let chrome_manifest = format!(
        r#"{{
  "name": "io.github.yrpcaro.xiu",
  "description": "Xiu live theme host",
  "path": "{}",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://*/*"]
}}
"#,
        host_py.to_string_lossy()
    );

    let gecko_target_dirs = [
        home_path(&[".mozilla", "native-messaging-hosts"]),
        home_path(&[".config", "mozilla", "native-messaging-hosts"]),
        home_path(&[".zen", "native-messaging-hosts"]),
        home_path(&[".config", "zen", "native-messaging-hosts"]),
    ];
    for dir in &gecko_target_dirs {
        let _ = fs::create_dir_all(dir);
        let _ = fs::write(dir.join("io.github.yrpcaro.xiu.json"), &gecko_manifest);
    }

    let chrome_target_dirs = [
        home_path(&[".config", "BraveSoftware", "Brave-Browser", "NativeMessagingHosts"]),
        home_path(&[".config", "chromium", "NativeMessagingHosts"]),
        home_path(&[".config", "google-chrome", "NativeMessagingHosts"]),
    ];
    for dir in &chrome_target_dirs {
        let _ = fs::create_dir_all(dir);
        let _ = fs::write(dir.join("io.github.yrpcaro.xiu.json"), &chrome_manifest);
    }

    // 6. Refresh running browsers
    for cmd in &["brave", "chromium"] {
        if on_path(cmd) {
            let _ = Command::new("timeout")
                .args(["10", cmd, "--refresh-platform-policy", "--no-startup-window"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
}

pub fn rgb_tuple(hex_str: &str) -> String {
    let s = hex_str.trim().trim_start_matches('#');
    if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
        format!("{r},{g},{b}")
    } else {
        "0,0,0".to_string()
    }
}

pub fn get_active_icon_theme(_is_dark: bool) -> String {
    let mut theme = String::new();
    if on_path("gsettings") {
        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "icon-theme"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').trim_matches('"').to_string();
            if !s.is_empty() {
                theme = s;
            }
        }
    }
    if theme.is_empty() {
        let kde_cfg = home_path(&[".config", "kdeglobals"]);
        if kde_cfg.is_file() {
            if let Ok(c) = fs::read_to_string(&kde_cfg) {
                for line in c.lines() {
                    if let Some(rest) = line.strip_prefix("Theme=") {
                        theme = rest.trim().to_string();
                        break;
                    }
                }
            }
        }
    }
    if theme.is_empty() {
        for ver in &["gtk-3.0", "gtk-4.0"] {
            let ini = home_path(&[".config", ver, "settings.ini"]);
            if ini.is_file() {
                if let Ok(c) = fs::read_to_string(&ini) {
                    for line in c.lines() {
                        if let Some(rest) = line.strip_prefix("gtk-icon-theme-name=") {
                            theme = rest.trim().to_string();
                            break;
                        }
                    }
                }
            }
            if !theme.is_empty() {
                break;
            }
        }
    }

    let yamis_name = "yet-another-monochrome-icon-set";
    let local_icons = home_path(&[".local", "share", "icons", yamis_name]);
    let usr_icons = Path::new("/usr/share/icons").join(yamis_name);
    let has_yamis = local_icons.is_dir() || usr_icons.is_dir();

    if has_yamis || theme == yamis_name {
        return yamis_name.to_string();
    }

    let is_generic = theme.is_empty()
        || matches!(
            theme.to_lowercase().as_str(),
            "breeze"
                | "breeze-dark"
                | "breeze-light"
                | "breeze_light"
                | "adwaita"
                | "adwaitalegacy"
                | "hicolor"
                | "breeze-round-chameleon dark icons"
                | "breeze-round-chameleon light icons"
                | "papirus"
                | "papirus-dark"
        );
    if is_generic {
        yamis_name.to_string()
    } else {
        theme
    }
}

pub fn update_gtk_settings(
    settings_file: &Path,
    theme_name: &str,
    icon_theme: &str,
    is_dark: bool,
) {
    let mut lines = Vec::new();
    if settings_file.is_file() {
        if let Ok(c) = fs::read_to_string(settings_file) {
            lines = c.lines().map(String::from).collect();
        }
    }

    let mut in_settings = false;
    let mut settings_found = false;
    let mut has_theme = false;
    let mut has_icon = false;
    let mut has_dark = false;
    let mut new_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_settings {
                if !has_theme {
                    new_lines.push(format!("gtk-theme-name={theme_name}"));
                }
                if !has_icon {
                    new_lines.push(format!("gtk-icon-theme-name={icon_theme}"));
                }
                if !has_dark {
                    new_lines.push(format!("gtk-application-prefer-dark-theme={}", if is_dark { "true" } else { "false" }));
                }
                in_settings = false;
            }
            if trimmed == "[Settings]" {
                in_settings = true;
                settings_found = true;
            }
            new_lines.push(line);
            continue;
        }

        if in_settings {
            if trimmed.starts_with("gtk-theme-name=") {
                new_lines.push(format!("gtk-theme-name={theme_name}"));
                has_theme = true;
                continue;
            }
            if trimmed.starts_with("gtk-icon-theme-name=") {
                new_lines.push(format!("gtk-icon-theme-name={icon_theme}"));
                has_icon = true;
                continue;
            }
            if trimmed.starts_with("gtk-application-prefer-dark-theme=") {
                new_lines.push(format!("gtk-application-prefer-dark-theme={}", if is_dark { "true" } else { "false" }));
                has_dark = true;
                continue;
            }
            if trimmed.starts_with("gtk-cursor-theme-name=breeze_cursors") {
                let cur = crate::commands::cursor::read_cursor_theme().unwrap_or_else(|| "Bibata-Modern-Ice".to_string());
                new_lines.push(format!("gtk-cursor-theme-name={cur}"));
                continue;
            }
            if trimmed.starts_with("gtk-modules=") {
                let val = trimmed.strip_prefix("gtk-modules=").unwrap_or("");
                let mods: Vec<&str> = val
                    .split(':')
                    .map(str::trim)
                    .filter(|m| !m.is_empty() && *m != "colorreload-gtk-module")
                    .collect();
                if !mods.is_empty() {
                    new_lines.push(format!("gtk-modules={}", mods.join(":")));
                }
                continue;
            }
        }
        new_lines.push(line);
    }

    if in_settings {
        if !has_theme {
            new_lines.push(format!("gtk-theme-name={theme_name}"));
        }
        if !has_icon {
            new_lines.push(format!("gtk-icon-theme-name={icon_theme}"));
        }
        if !has_dark {
            new_lines.push(format!("gtk-application-prefer-dark-theme={}", if is_dark { "true" } else { "false" }));
        }
    } else if !settings_found {
        if !new_lines.is_empty() && !new_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
            new_lines.push("".to_string());
        }
        new_lines.push("[Settings]".to_string());
        new_lines.push(format!("gtk-theme-name={theme_name}"));
        new_lines.push(format!("gtk-icon-theme-name={icon_theme}"));
        new_lines.push(format!("gtk-application-prefer-dark-theme={}", if is_dark { "true" } else { "false" }));
    }

    if let Some(parent) = settings_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(settings_file, new_lines.join("\n") + "\n");
}

pub fn update_xsettingsd(conf_path: &Path, theme_name: &str, icon_theme: &str) {
    let mut lines = Vec::new();
    if conf_path.is_file() {
        if let Ok(c) = fs::read_to_string(conf_path) {
            lines = c.lines().map(String::from).collect();
        }
    }

    let mut new_lines = Vec::new();
    let mut has_theme = false;
    let mut has_icon = false;

    for line in lines {
        let stripped = line.trim();
        if stripped.starts_with("Net/ThemeName") {
            new_lines.push(format!("Net/ThemeName \"{theme_name}\""));
            has_theme = true;
        } else if stripped.starts_with("Net/IconThemeName") {
            new_lines.push(format!("Net/IconThemeName \"{icon_theme}\""));
            has_icon = true;
        } else if stripped.starts_with("Gtk/CursorThemeName") && stripped.contains("breeze_cursors") {
            let cur = crate::commands::cursor::read_cursor_theme().unwrap_or_else(|| "Bibata-Modern-Ice".to_string());
            new_lines.push(format!("Gtk/CursorThemeName \"{cur}\""));
        } else {
            new_lines.push(line);
        }
    }

    if !has_theme {
        new_lines.push(format!("Net/ThemeName \"{theme_name}\""));
    }
    if !has_icon {
        new_lines.push(format!("Net/IconThemeName \"{icon_theme}\""));
    }

    if let Some(parent) = conf_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(conf_path, new_lines.join("\n") + "\n");
}

pub fn update_kdeglobals(
    kdeglobals: &Path,
    sections: &[(&str, &[(&str, &str)])],
    icon_theme: &str,
    primary_hex: &str,
) {
    let mut lines = Vec::new();
    if kdeglobals.is_file() {
        if let Ok(c) = fs::read_to_string(kdeglobals) {
            lines = c.lines().map(String::from).collect();
        }
    }

    // Parse existing sections into a structured map: section -> ordered key-values
    let mut sec_order: Vec<String> = Vec::new();
    let mut sec_data: HashMap<String, Vec<(String, String)>> = HashMap::new();

    let mut current_sec = String::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_sec = trimmed[1..trimmed.len() - 1].to_string();
            if !sec_order.contains(&current_sec) {
                sec_order.push(current_sec.clone());
            }
            sec_data.entry(current_sec.clone()).or_default();
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            let val = v.trim().to_string();
            if !current_sec.is_empty() {
                let list = sec_data.entry(current_sec.clone()).or_default();
                if let Some(pos) = list.iter().position(|(ek, _)| ek == &key) {
                    list[pos] = (key, val);
                } else {
                    list.push((key, val));
                }
            }
        }
    }

    // Apply sections from argument
    for (sec_hdr, fields) in sections {
        let sec_name = sec_hdr.trim_matches(|c| c == '[' || c == ']').to_string();
        if !sec_order.contains(&sec_name) {
            sec_order.push(sec_name.clone());
        }
        let list = sec_data.entry(sec_name).or_default();
        for (k, v) in *fields {
            let val_str = if v.starts_with('#') {
                rgb_tuple(v)
            } else {
                v.to_string()
            };
            if let Some(pos) = list.iter().position(|(ek, _)| ek == k) {
                list[pos] = (k.to_string(), val_str);
            } else {
                list.push((k.to_string(), val_str));
            }
        }
    }

    // General: ColorScheme=Xiu, AccentColor=rgb_tuple(primary_hex)
    let gen_sec = "General".to_string();
    if !sec_order.contains(&gen_sec) {
        sec_order.push(gen_sec.clone());
    }
    let gen_list = sec_data.entry(gen_sec).or_default();
    let prim_rgb = rgb_tuple(primary_hex);
    for (k, v) in &[("ColorScheme", "Xiu"), ("AccentColor", &prim_rgb)] {
        if let Some(pos) = gen_list.iter().position(|(ek, _)| ek == k) {
            gen_list[pos] = (k.to_string(), v.to_string());
        } else {
            gen_list.push((k.to_string(), v.to_string()));
        }
    }

    // Icons: Theme=icon_theme
    let icon_sec = "Icons".to_string();
    if !sec_order.contains(&icon_sec) {
        sec_order.push(icon_sec.clone());
    }
    let icon_list = sec_data.entry(icon_sec).or_default();
    if let Some(pos) = icon_list.iter().position(|(ek, _)| ek == "Theme") {
        icon_list[pos] = ("Theme".to_string(), icon_theme.to_string());
    } else {
        icon_list.push(("Theme".to_string(), icon_theme.to_string()));
    }

    // Write back
    let mut out_lines = Vec::new();
    for (idx, sec_name) in sec_order.iter().enumerate() {
        if idx > 0 {
            out_lines.push("".to_string());
        }
        out_lines.push(format!("[{sec_name}]"));
        if let Some(pairs) = sec_data.get(sec_name) {
            for (k, v) in pairs {
                out_lines.push(format!("{k}={v}"));
            }
        }
    }

    if let Some(parent) = kdeglobals.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(kdeglobals, out_lines.join("\n") + "\n");
}

pub fn render_gtk(pill: &HashMap<String, String>) {
    let surface = pill.get("surface").cloned().unwrap_or_else(|| "#141a20".to_string());
    let is_dark = rel_luminance(&surface) < 0.40;
    let theme_name = if is_dark { "adw-gtk3-dark" } else { "adw-gtk3" };
    let color_scheme = if is_dark { "prefer-dark" } else { "prefer-light" };
    let icon_theme = get_active_icon_theme(is_dark);
    let primary = pill.get("primary").cloned().unwrap_or_else(|| "#ffaa00".to_string());
    let gnome_accent = gnome_accent_color(&primary);
    let accent_fg = if rel_luminance(&primary) > 0.45 {
        "#000000"
    } else {
        "#ffffff"
    };
    let cream = pill.get("cream").cloned().unwrap_or_else(|| "#abb4bc".to_string());
    let surface_container = pill.get("surface_container").cloned().unwrap_or_else(|| "#20262d".to_string());
    let surface_container_high = pill.get("surface_container_high").cloned().unwrap_or_else(|| "#2c333b".to_string());

    let css = format!(
        r#"/* Written by wallcolors.py on every palette change. */
@define-color accent_color {primary};
@define-color accent_bg_color {primary};
@define-color accent_fg_color {accent_fg};
@define-color window_bg_color {surface};
@define-color window_fg_color {cream};
@define-color headerbar_bg_color {surface_container};
@define-color headerbar_fg_color {cream};
@define-color popover_bg_color {surface_container_high};
@define-color popover_fg_color {cream};
@define-color view_bg_color {surface_container};
@define-color view_fg_color {cream};
@define-color card_bg_color {surface_container};
@define-color card_fg_color {cream};
@define-color sidebar_bg_color @window_bg_color;
@define-color sidebar_fg_color @window_fg_color;
@define-color sidebar_border_color @window_bg_color;
@define-color theme_selected_bg_color alpha(@accent_color, 0.25);
@define-color theme_selected_fg_color {primary};
@define-color theme_bg_color @window_bg_color;
@define-color theme_fg_color @window_fg_color;
@define-color theme_base_color @view_bg_color;
@define-color theme_text_color @view_fg_color;

/* Concrete widget selectors to guarantee recoloring across GTK engines */
window, .background {{
    background-color: @window_bg_color;
    color: @window_fg_color;
}}
headerbar, .titlebar {{
    background-color: @headerbar_bg_color;
    color: @headerbar_fg_color;
}}
view, .view, textview text {{
    background-color: @view_bg_color;
    color: @view_fg_color;
}}
popover, .popover, menu, .menu {{
    background-color: @popover_bg_color;
    color: @popover_fg_color;
}}
card, .card {{
    background-color: @card_bg_color;
    color: @card_fg_color;
}}
button.suggested-action {{
    background-color: @accent_bg_color;
    color: @accent_fg_color;
}}
button.suggested-action:hover {{
    background-color: alpha(@accent_bg_color, 0.85);
}}
switch:checked {{
    background-color: @accent_bg_color;
    color: @accent_fg_color;
}}
selection, *:selected {{
    background-color: @theme_selected_bg_color;
    color: @window_fg_color;
}}
"#
    );

    for ver in &["gtk-3.0", "gtk-4.0"] {
        let d = home_path(&[".config", ver]);
        let _ = fs::create_dir_all(&d);
        let _ = fs::write(d.join("gtk.css"), &css);
        update_gtk_settings(&d.join("settings.ini"), theme_name, &icon_theme, is_dark);
    }

    let xsettings_conf = home_path(&[".config", "xsettingsd", "xsettingsd.conf"]);
    update_xsettingsd(&xsettings_conf, theme_name, &icon_theme);
    let xset_hup = Command::new("killall")
        .args(["-HUP", "xsettingsd"])
        .stderr(Stdio::null())
        .status();
    if (xset_hup.is_err() || !xset_hup.as_ref().unwrap().success()) && on_path("xsettingsd") {
        let _ = Command::new("xsettingsd")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    if on_path("gsettings") {
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "gtk-theme", ""])
            .stderr(Stdio::null())
            .status();
        for (key, val) in &[
            ("color-scheme", color_scheme),
            ("gtk-theme", theme_name),
            ("icon-theme", &icon_theme),
            ("accent-color", gnome_accent),
        ] {
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.desktop.interface", key, val])
                .stderr(Stdio::null())
                .status();
        }
    }

    let (c_theme, c_size) = crate::commands::cursor::resolve_cursor(None, None);
    let _ = crate::commands::cursor::sync_all(&c_theme, c_size);
}

pub fn render_qt(pill: &HashMap<String, String>) {
    let surface = pill.get("surface").cloned().unwrap_or_else(|| "#141a20".to_string());
    let is_dark = rel_luminance(&surface) < 0.40;
    let icon_theme = get_active_icon_theme(is_dark);
    let p = pill;
    let g = |k: &str| p.get(k).cloned().unwrap_or_default();

    let view_fields: [(&str, &str); 5] = [
        ("BackgroundNormal", &g("surface_container")),
        ("ForegroundNormal", &g("cream")),
        ("BackgroundAlternate", &g("surface_container_low")),
        ("DecorationFocus", &g("primary")),
        ("ForegroundActive", &g("primary")),
    ];
    let win_fields: [(&str, &str); 5] = [
        ("BackgroundNormal", &g("surface")),
        ("ForegroundNormal", &g("cream")),
        ("BackgroundAlternate", &g("surface_container")),
        ("DecorationFocus", &g("primary")),
        ("ForegroundActive", &g("primary")),
    ];
    let btn_fields: [(&str, &str); 5] = [
        ("BackgroundNormal", &g("surface_container_high")),
        ("ForegroundNormal", &g("cream")),
        ("BackgroundAlternate", &g("surface_container_highest")),
        ("DecorationFocus", &g("primary")),
        ("ForegroundActive", &g("primary")),
    ];
    let sel_fields: [(&str, &str); 4] = [
        ("BackgroundNormal", &g("primary_container")),
        ("ForegroundNormal", &g("on_primary_container")),
        ("DecorationFocus", &g("primary")),
        ("ForegroundActive", &g("cream")),
    ];
    let tip_fields: [(&str, &str); 3] = [
        ("BackgroundNormal", &g("surface_container_highest")),
        ("ForegroundNormal", &g("cream")),
        ("DecorationFocus", &g("primary")),
    ];
    let hdr_fields: [(&str, &str); 3] = [
        ("BackgroundNormal", &g("surface_container")),
        ("ForegroundNormal", &g("cream")),
        ("DecorationFocus", &g("primary")),
    ];
    let comp_fields: [(&str, &str); 3] = [
        ("BackgroundNormal", &g("surface_container_low")),
        ("ForegroundNormal", &g("cream")),
        ("DecorationFocus", &g("primary")),
    ];
    let wm_fields: [(&str, &str); 4] = [
        ("activeBackground", &g("surface")),
        ("activeForeground", &g("cream")),
        ("inactiveBackground", &g("surface_container_low")),
        ("inactiveForeground", &g("dim")),
    ];

    let sections: [(&str, &[(&str, &str)]); 8] = [
        ("Colors:View", &view_fields),
        ("Colors:Window", &win_fields),
        ("Colors:Button", &btn_fields),
        ("Colors:Selection", &sel_fields),
        ("Colors:Tooltip", &tip_fields),
        ("Colors:Header", &hdr_fields),
        ("Colors:Complementary", &comp_fields),
        ("WM", &wm_fields),
    ];

    let mut lines = vec!["# Written by wallcolors.py on every palette change.".to_string()];
    for (header, fields) in &sections {
        lines.push("".to_string());
        lines.push(format!("[{header}]"));
        for (k, v) in *fields {
            let val_str = if v.starts_with('#') {
                rgb_tuple(v)
            } else {
                v.to_string()
            };
            lines.push(format!("{k}={val_str}"));
        }
    }
    lines.push("".to_string());
    lines.push("[General]".to_string());
    lines.push("ColorScheme=Xiu".to_string());
    lines.push(format!("AccentColor={}", rgb_tuple(&g("primary"))));
    lines.push("".to_string());
    lines.push("[Icons]".to_string());
    lines.push(format!("Theme={icon_theme}"));

    let colors_content = lines.join("\n") + "\n";

    // 1. QtEngine config and colors
    let d_qtengine = home_path(&[".config", "qtengine"]);
    let _ = fs::create_dir_all(&d_qtengine);
    let _ = fs::write(d_qtengine.join("xiu.colors"), &colors_content);
    let config_path = d_qtengine.join("config.json");
        let mut cfg_data = if config_path.is_file() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|c| crate::json::parse(&c).ok())
                .unwrap_or(crate::json::Json::Obj(Vec::new()))
        } else {
            crate::json::Json::Obj(Vec::new())
        };
        if let crate::json::Json::Obj(ref mut root) = cfg_data {
            let mut theme_map = Vec::new();
            theme_map.push(("colorScheme".to_string(), crate::json::Json::Str(d_qtengine.join("xiu.colors").to_string_lossy().to_string())));
            theme_map.push(("iconTheme".to_string(), crate::json::Json::Str(icon_theme.clone())));
            theme_map.push(("style".to_string(), crate::json::Json::Str("Darkly".to_string())));

            let mut misc_map = Vec::new();
            misc_map.push(("menusHaveIcons".to_string(), crate::json::Json::Bool(true)));
            misc_map.push(("singleClickActivate".to_string(), crate::json::Json::Bool(false)));

            if let Some(pos) = root.iter().position(|(k, _)| k == "theme") {
                root[pos] = ("theme".to_string(), crate::json::Json::Obj(theme_map));
            } else {
                root.push(("theme".to_string(), crate::json::Json::Obj(theme_map)));
            }
            if let Some(pos) = root.iter().position(|(k, _)| k == "misc") {
                root[pos] = ("misc".to_string(), crate::json::Json::Obj(misc_map));
            } else {
                root.push(("misc".to_string(), crate::json::Json::Obj(misc_map)));
            }
            let _ = fs::write(&config_path, crate::json::stringify_pretty(&cfg_data, 4) + "\n");
        }
        let _ = Command::new("dbus-send")
            .args(["--session", "--type=signal", "/", "org.qtengine.ConfigWatcher.configChanged"])
            .stderr(Stdio::null())
            .status();

    // 2. KDE color scheme & kdeglobals
    let d_kde_schemes = home_path(&[".local", "share", "color-schemes"]);
    let _ = fs::create_dir_all(&d_kde_schemes);
    let _ = fs::write(d_kde_schemes.join("Xiu.colors"), &colors_content);

    let kdeglobals = home_path(&[".config", "kdeglobals"]);
    update_kdeglobals(&kdeglobals, &sections, &icon_theme, &g("primary"));

    if on_path("kwriteconfig6") {
        let _ = Command::new("kwriteconfig6")
            .args(["--file", "kdeglobals", "--group", "General", "--key", "ColorScheme", "Xiu", "--notify"])
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("kwriteconfig6")
            .args(["--file", "kdeglobals", "--group", "Icons", "--key", "Theme", &icon_theme, "--notify"])
            .stderr(Stdio::null())
            .status();
    }

    if on_path("plasma-apply-colorscheme") {
        let fallback_theme = if is_dark { "BreezeDark" } else { "BreezeLight" };
        let _ = Command::new("plasma-apply-colorscheme")
            .arg(fallback_theme)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("plasma-apply-colorscheme")
            .arg("Xiu")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("plasma-apply-colorscheme")
            .args(["-a", &g("primary")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    let _ = Command::new("dbus-send")
        .args(["--session", "--type=signal", "/KGlobalSettings", "org.kde.KGlobalSettings.notifyChange", "int32:0", "int32:0"])
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("dbus-send")
        .args(["--session", "--type=signal", "/KGlobalSettings", "org.kde.KGlobalSettings.notifyChange", "int32:1", "int32:0"])
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("dbus-send")
        .args(["--session", "--type=signal", "/KGlobalSettings", "org.kde.KGlobalSettings.notifyChange", "int32:4", "int32:0"])
        .stderr(Stdio::null())
        .status();

    let _ = Command::new("busctl")
        .args(["--user", "emit", "/", "org.qtengine.ConfigWatcher", "configChanged"])
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("busctl")
        .args(["--user", "emit", "/KGlobalSettings", "org.kde.KGlobalSettings", "notifyChange", "ii", "0", "0"])
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("busctl")
        .args(["--user", "emit", "/KGlobalSettings", "org.kde.KGlobalSettings", "notifyChange", "ii", "4", "0"])
        .stderr(Stdio::null())
        .status();
}

pub fn render_user_templates(pill: &HashMap<String, String>, b: &HashMap<String, String>) {
    let src = config_file(&["xiu", "templates"]);
    if !src.is_dir() {
        return;
    }
    let mut tokens = pill.clone();
    for (k, v) in b {
        tokens.insert(k.clone(), v.clone());
    }
    if let Ok(entries) = fs::read_dir(&src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(base_name) = name.strip_suffix(".in") {
                        if let Ok(mut text) = fs::read_to_string(&path) {
                            for (k, v) in &tokens {
                                let pat = format!("{{{{ ${k} }}}}");
                                let clean_val = v.trim().trim_start_matches('#');
                                text = text.replace(&pat, clean_val);
                            }
                            let out_path = path.with_file_name(base_name);
                            let _ = fs::write(out_path, text);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_alacritty_generates_valid_config() {
        let mut pill = HashMap::new();
        pill.insert("primary".into(), "#e0563b".into());
        pill.insert("bright".into(), "#fff6f0".into());
        let mut b = HashMap::new();
        b.insert("base00".into(), "#141a20".into());
        b.insert("base07".into(), "#abb4bc".into());
        b.insert("base02".into(), "#2c333b".into());
        let ansi: Vec<String> = (0..16).map(|i| format!("#{:02x}{:02x}{:02x}", i, i, i)).collect();
        render_alacritty(&pill, &b, &ansi);

        let colors_toml = home_path(&[".config", "alacritty", "colors.toml"]);
        if colors_toml.is_file() {
            let content = fs::read_to_string(colors_toml).unwrap();
            assert!(content.contains("[colors.primary]"));
            assert!(content.contains("background = \"#141a20\""));
            assert!(content.contains("cursor = \"#e0563b\""));
        }
    }
}
