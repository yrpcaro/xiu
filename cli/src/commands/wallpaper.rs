//! Wallpaper lifecycle, bag history, and backend communication.

use crate::helpers::{config_file, home_path, ipc_call, state_file};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VALID_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "mp4", "webm", "mkv", "mov",
];

pub fn wallpaper(
    action: Option<&str>,
    target: Option<&str>,
    print: bool,
    list: bool,
    file: Option<&str>,
) -> i32 {
    if print {
        return query();
    }
    if list {
        return list_wallpapers();
    }
    if let Some(f) = file {
        return set_wallpaper(f, target);
    }

    match action {
        None => next_wallpaper(),
        Some("query") | Some("get") => query(),
        Some("list") => list_wallpapers(),
        Some("set") => {
            if let Some(path) = target {
                set_wallpaper(path, None)
            } else {
                eprintln!("xiu wallpaper set: missing image path");
                2
            }
        }
        Some("next") | Some("random") => next_wallpaper(),
        Some("prev") | Some("previous") => prev_wallpaper(),
        Some("init") => init_wallpaper(),
        Some(other) => {
            if Path::new(other).is_file() {
                set_wallpaper(other, target)
            } else {
                eprintln!(
                    "xiu wallpaper: unknown action '{other}' (init, set, next, prev, query, list)"
                );
                2
            }
        }
    }
}

fn query() -> i32 {
    let xiu_state = state_file("xiu/wallpaper");
    if let Ok(content) = fs::read_to_string(&xiu_state) {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            println!("{trimmed}");
            return 0;
        }
    }

    let legacy_state = state_file("ricelin-wallpaper");
    if let Ok(content) = fs::read_to_string(&legacy_state) {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            println!("{trimmed}");
            return 0;
        }
    }

    // Fall back to quickshell IPC
    ipc_call("wallpaper", &["get"])
}

fn set_wallpaper(path: &str, output: Option<&str>) -> i32 {
    let p = Path::new(path);
    if !p.is_file() {
        eprintln!("xiu wallpaper set: file not found: {path}");
        return 1;
    }

    let canonical = match fs::canonicalize(p) {
        Ok(c) => c.to_string_lossy().to_string(),
        Err(_) => path.to_string(),
    };

    // Push current wallpaper to history before updating
    record_history();

    // 1. Try Quickshell IPC if available
    if let Some(out) = output {
        let _ = ipc_call("wallpaper", &["set", &canonical, out]);
    } else {
        let _ = ipc_call("wallpaper", &["set", &canonical]);
    }

    // 2. Invoke wallpaper.sh to synchronize visual layers, SDDM, and theme palette
    let script = config_file(&["hypr", "scripts", "wallpaper.sh"]);
    if script.is_file() {
        let mut cmd = Command::new("bash");
        cmd.arg(&script).arg("set").arg(&canonical);
        if let Some(out) = output {
            cmd.arg(out);
        }
        let _ = cmd.status();
    }

    // Ensure state files are kept in sync
    update_state_files(&canonical);

    0
}

fn next_wallpaper() -> i32 {
    record_history();

    let script = config_file(&["hypr", "scripts", "wallpaper.sh"]);
    if script.is_file() {
        let status = Command::new("bash").arg(&script).status();
        if let Ok(s) = status {
            if s.success() {
                return 0;
            }
        }
    }

    // Fall back to Quickshell IPC random
    ipc_call("wallpaper", &["random"])
}

fn prev_wallpaper() -> i32 {
    let hist_file = state_file("xiu/wallpaper-history");
    if !hist_file.is_file() {
        eprintln!("xiu wallpaper prev: no wallpaper history found");
        return 1;
    }

    let Ok(content) = fs::read_to_string(&hist_file) else {
        eprintln!("xiu wallpaper prev: unable to read history");
        return 1;
    };

    let mut lines: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let current = get_current_wallpaper().unwrap_or_default();

    // Pop entries matching the current wallpaper
    while let Some(&last) = lines.last() {
        if last == current {
            lines.pop();
        } else {
            break;
        }
    }

    let Some(prev) = lines.pop() else {
        eprintln!("xiu wallpaper prev: no previous wallpaper in history");
        return 1;
    };

    let prev_str = prev.to_string();

    // Rewrite history without the popped item
    let new_content = lines.join("\n") + if lines.is_empty() { "" } else { "\n" };
    let _ = fs::write(&hist_file, new_content);

    set_wallpaper(&prev_str, None)
}

fn init_wallpaper() -> i32 {
    let script = config_file(&["hypr", "scripts", "wallpaper.sh"]);
    if script.is_file() {
        let status = Command::new("bash").arg(&script).arg("init").status();
        match status {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("xiu wallpaper init failed: {e}");
                1
            }
        }
    } else {
        eprintln!("xiu wallpaper init: wallpaper.sh not found");
        1
    }
}

fn list_wallpapers() -> i32 {
    // 1. If quickshell is up, let it return the entries
    let res = ipc_call("wallpaper", &["list"]);
    if res == 0 {
        return 0;
    }

    // 2. Otherwise inspect the wallpaper directory
    let dir = resolve_wallpaper_dir();
    if !dir.is_dir() {
        eprintln!("xiu wallpaper list: directory does not exist: {}", dir.display());
        return 1;
    }

    if let Ok(entries) = fs::read_dir(&dir) {
        let mut paths = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_ascii_lowercase();
                    if VALID_EXTENSIONS.contains(&ext_lower.as_str()) {
                        paths.push(path);
                    }
                }
            }
        }
        paths.sort();
        for p in paths {
            println!("{}", p.display());
        }
    }
    0
}

fn get_current_wallpaper() -> Option<String> {
    for name in &["xiu/wallpaper", "ricelin-wallpaper"] {
        let f = state_file(name);
        if let Ok(c) = fs::read_to_string(&f) {
            let t = c.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn record_history() {
    if let Some(curr) = get_current_wallpaper() {
        if curr.is_empty() || !Path::new(&curr).is_file() {
            return;
        }
        let hist_file = state_file("xiu/wallpaper-history");
        if let Some(parent) = hist_file.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut lines = Vec::new();
        if let Ok(content) = fs::read_to_string(&hist_file) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    lines.push(trimmed.to_string());
                }
            }
        }

        // Avoid duplicate consecutive entries
        if lines.last().map(String::as_str) != Some(&curr) {
            lines.push(curr);
        }

        // Keep last 50 wallpapers
        if lines.len() > 50 {
            lines = lines.split_off(lines.len() - 50);
        }

        let _ = fs::write(&hist_file, lines.join("\n") + "\n");
    }
}

fn update_state_files(path: &str) {
    for name in &["xiu/wallpaper", "ricelin-wallpaper"] {
        let f = state_file(name);
        if let Some(parent) = f.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&f, format!("{path}\n"));
    }
}

fn resolve_wallpaper_dir() -> PathBuf {
    let resolved_state = state_file("ricelin-wallpaper-dir");
    if let Ok(content) = fs::read_to_string(&resolved_state) {
        let t = content.trim();
        if !t.is_empty() && Path::new(t).is_dir() {
            return PathBuf::from(t);
        }
    }

    for cand in &[
        &["Pictures", "xiu", "wallpapers"][..],
        &["Pictures", "Wallpapers"][..],
        &["Pictures", "wallpapers"][..],
        &["Wallpapers"][..],
        &["wallpapers"][..],
    ] {
        let p = home_path(cand);
        if p.is_dir() {
            return p;
        }
    }

    home_path(&["Pictures", "xiu", "wallpapers"])
}
