//! Palette generation, 24-bit TrueColor token computation, template rendering, and live OSC broadcast.

use crate::helpers::cache_file;
use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;

const O_NONBLOCK_NOCTTY: i32 = 0x800 | 0x100;

pub fn theme(
    action: &str,
    target: Option<&str>,
    preset: Option<&str>,
    variant: Option<&str>,
) -> i32 {
    match action {
        "live" => live_reload(),
        "generate" | "preview" => generate(target, preset, variant),
        "dark" => set_mode("dark"),
        "light" => set_mode("light"),
        "toggle" => toggle_mode(),
        "apply" => apply(target, preset, variant),
        other => {
            eprintln!("xiu theme: unknown action '{other}' (generate, apply, live, dark, light, toggle)");
            2
        }
    }
}

fn set_mode(mode: &str) -> i32 {
    let status = crate::commands::wallcolors::wallcolors(&["--mode".to_string(), mode.to_string()]);
    if status == 0 {
        let _ = live_reload();
        let _ = Command::new("hyprctl").arg("reload").status();
    }
    status
}

fn toggle_mode() -> i32 {
    let status = crate::commands::wallcolors::wallcolors(&["--toggle".to_string()]);
    if status == 0 {
        let _ = live_reload();
        let _ = Command::new("hyprctl").arg("reload").status();
    }
    status
}

pub fn live_reload() -> i32 {
    let mut seq = None;
    for cand in &["xiu/sequences.txt", "ricelin/sequences.txt"] {
        let f = cache_file(cand);
        if f.is_file() {
            if let Ok(bytes) = fs::read(&f) {
                if !bytes.is_empty() {
                    seq = Some(bytes);
                    break;
                }
            }
        }
    }

    let data = match seq {
        Some(d) => d,
        None => {
            eprintln!("xiu theme live: no cached sequences found; apply a theme first");
            return 1;
        }
    };

    let mut count = 0;
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
                    if file.write_all(&data).is_ok() {
                        count += 1;
                    }
                }
            }
        }
    }

    println!("xiu theme live: broadcast 24-bit TrueColor OSC sequences to {count} pty(s)");
    0
}

fn generate(target: Option<&str>, preset: Option<&str>, variant: Option<&str>) -> i32 {
    let mut args = Vec::new();
    if let Some(p) = preset {
        args.push("--preset".to_string());
        args.push(p.to_string());
        if let Some(v) = variant {
            args.push("--variant".to_string());
            args.push(v.to_string());
        }
    } else if let Some(t) = target {
        if Path::new(t).is_file() {
            args.push("--preview".to_string());
            args.push(t.to_string());
        } else {
            args.push("--preset".to_string());
            args.push(t.to_string());
            if let Some(v) = variant {
                args.push("--variant".to_string());
                args.push(v.to_string());
            }
        }
    } else {
        args.push("--state".to_string());
    }
    crate::commands::wallcolors::wallcolors(&args)
}

fn apply(target: Option<&str>, preset: Option<&str>, variant: Option<&str>) -> i32 {
    let mut args = Vec::new();
    if let Some(p) = preset {
        args.push("--preset".to_string());
        args.push(p.to_string());
        if let Some(v) = variant {
            args.push("--variant".to_string());
            args.push(v.to_string());
        }
    } else if let Some(t) = target {
        if Path::new(t).is_file() {
            args.push(t.to_string());
        } else {
            args.push("--preset".to_string());
            args.push(t.to_string());
            if let Some(v) = variant {
                args.push("--variant".to_string());
                args.push(v.to_string());
            }
        }
    } else {
        args.push("--apply".to_string());
    }
    let status = crate::commands::wallcolors::wallcolors(&args);
    if status == 0 {
        let _ = live_reload();
        let _ = Command::new("hyprctl").arg("reload").status();
    }
    status
}
