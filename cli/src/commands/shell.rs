//! The shell-facing commands: everything that talks to the pill's IPC socket
//! or shells out to the rice's own tools.

use crate::helpers::{focused_monitor, ipc_call, passthrough, run_status};
use std::process::Command;

pub fn shell(kill: bool, target: Option<&str>, args: &[String]) -> i32 {
    if kill {
        return run_status(Command::new("qs").args(["-c", "pill", "kill"]));
    }
    if target.is_none() && args.is_empty() {
        return run_status(Command::new("qs").args(["-c", "pill", "ipc", "show"]));
    }
    let mut argv: Vec<&str> = vec![];
    if let Some(t) = target {
        argv.push(t);
    }
    argv.extend(args.iter().map(String::as_str));
    run_status(Command::new("qs").args(["-c", "pill", "ipc", "call"]).args(&argv))
}

pub fn open(surface: &str) -> i32 {
    ipc_call("pill", &[surface, ""])
}

pub fn wallpaper(print: bool, list: bool, file: Option<&str>) -> i32 {
    if list {
        return ipc_call("wallpaper", &["list"]);
    }
    if let Some(path) = &file {
        return ipc_call("wallpaper", &["set", path]);
    }
    if print {
        return ipc_call("wallpaper", &["get"]);
    }
    ipc_call("wallpaper", &["random"])
}

pub fn mpris(action: &str) -> i32 {
    match action {
        "play" | "pause" | "playPause" | "play-pause" | "toggle" => ipc_call("mpris", &["playPause"]),
        "next" => ipc_call("mpris", &["next"]),
        "prev" | "previous" => ipc_call("mpris", &["previous"]),
        "stop" => ipc_call("mpris", &["stop"]),
        "list" => ipc_call("mpris", &["list"]),
        "active" | "status" => ipc_call("mpris", &["active"]),
        other => {
            eprintln!("xiu mpris: unknown action '{other}'");
            2
        }
    }
}

pub fn record(stop: bool) -> i32 {
    if stop {
        return ipc_call("recorder", &["stop"]);
    }
    let mon = focused_monitor();
    ipc_call("pill", &["quickRecord", &mon])
}

pub fn screenshot(args: &[String]) -> i32 {
    passthrough("rishot", args)
}

pub fn clipboard() -> i32 {
    ipc_call("pill", &["clipboard", ""])
}

pub fn notifs(action: &str) -> i32 {
    match action {
        "clear" => ipc_call("notifs", &["clear"]),
        "seen" => ipc_call("notifs", &["seen"]),
        other => {
            eprintln!("xiu notifs: unknown action '{other}'");
            2
        }
    }
}

pub fn gamemode(action: &str) -> i32 {
    match action {
        "status" => ipc_call("gamemode", &["status"]),
        "on" => ipc_call("gamemode", &["on"]),
        "off" => ipc_call("gamemode", &["off"]),
        "toggle" => ipc_call("gamemode", &["toggle"]),
        other => {
            eprintln!("xiu gamemode: unknown action '{other}'");
            2
        }
    }
}

/// The palette engine is wallcolors.py; the CLI is its front door. Scheme
/// state survives wallpaper changes in its own state file, and an explicit
/// change flips the pill's paletteMode so the shell actually listens.
pub fn scheme(action: &str, value: Option<&str>, variant: Option<&str>) -> i32 {
    let home = std::env::var("HOME").unwrap_or_default();
    let script = format!("{home}/.config/hypr/scripts/wallcolors.py");
    let run = |flags: Vec<&str>| run_status(Command::new("python3").arg(&script).args(&flags));

    match action {
        "list" => run(vec!["--list-presets"]),
        "get" => run(vec!["--state"]),
        "preview" => {
            let Some(wallpaper) = value else {
                eprintln!("xiu scheme preview: needs a wallpaper path");
                return 2;
            };
            run(vec!["--preview", wallpaper])
        }
        "set" => {
            let Some(preset) = value else {
                eprintln!("xiu scheme set: needs a preset name, `dynamic` or -v VARIANT");
                return 2;
            };
            let mut flags: Vec<String> = vec!["--preset".into(), preset.to_string()];
            if let Some(v) = variant {
                flags.push("--variant".into());
                flags.push(v.to_string());
            }
            let refs: Vec<&str> = flags.iter().map(String::as_str).collect();
            run(refs)
        }
        other => {
            eprintln!("xiu scheme: unknown action '{other}' (list, get, set, preview)");
            2
        }
    }
}

/// Brave and Chromium read their toolbar color from a managed policy under
/// /etc, which needs root. The palette pipeline keeps the payload fresh in
/// ~/.config/xiu/browser-theme.json; this copies it out with a
/// non-interactive sudo when possible and prints the commands otherwise.
pub fn browser() -> i32 {
    let home = std::env::var("HOME").unwrap_or_default();
    let payload = format!("{home}/.config/xiu/browser-theme.json");
    if !std::path::Path::new(&payload).is_file() {
        eprintln!("xiu browser: no palette payload yet (run a wallpaper change or `xiu scheme set` first)");
        return 1;
    }
    let targets = ["/etc/brave/policies/managed/xiu.json", "/etc/chromium/policies/managed/xiu.json"];
    let mut failed = false;
    for target in targets {
        let status = Command::new("sudo")
            .args(["-n", "install", "-m", "644", "-D", &payload, target])
            .status();
        match status {
            Ok(s) if s.success() => println!("applied → {target}"),
            _ => {
                failed = true;
                eprintln!("needs root; run: sudo install -m 644 -D {payload} {target}");
            }
        }
    }
    if failed {
        1
    } else {
        println!("restart the browser to pick the new color up");
        0
    }
}
