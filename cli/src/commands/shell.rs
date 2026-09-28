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

pub fn gamemode(action: &str, target: Option<&str>) -> i32 {
    match action {
        "strip" | "apply" => gamemode_strip(target.unwrap_or("on")),
        "status" => ipc_call("gamemode", &["status"]),
        "on" => {
            if target.is_some() {
                gamemode_strip("on")
            } else {
                ipc_call("gamemode", &["on"])
            }
        }
        "off" => {
            if target.is_some() {
                gamemode_strip("off")
            } else {
                ipc_call("gamemode", &["off"])
            }
        }
        "toggle" => ipc_call("gamemode", &["toggle"]),
        other => {
            eprintln!("xiu gamemode: unknown action '{other}'");
            2
        }
    }
}

pub fn gamemode_strip(mode: &str) -> i32 {
    let state_dir = std::env::var("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| crate::helpers::home_path(&[".local", "state"]))
        .join("ricelin");
    let _ = std::fs::create_dir_all(&state_dir);
    let snap_path = state_dir.join("gamemode-snapshot.json");

    match mode {
        "on" => {
            if !snap_path.is_file() {
                let get_int = |opt: &str| -> i64 {
                    Command::new("hyprctl")
                        .args(["getoption", opt, "-j"])
                        .output()
                        .ok()
                        .and_then(|o| crate::json::parse(&String::from_utf8_lossy(&o.stdout)).ok())
                        .and_then(|j| j.get("int").and_then(crate::json::Json::as_i64))
                        .unwrap_or(0)
                };
                let get_bool = |opt: &str| -> bool {
                    Command::new("hyprctl")
                        .args(["getoption", opt, "-j"])
                        .output()
                        .ok()
                        .and_then(|o| crate::json::parse(&String::from_utf8_lossy(&o.stdout)).ok())
                        .and_then(|j| j.get("bool").and_then(crate::json::Json::as_bool))
                        .unwrap_or(false)
                };
                let get_gap = |opt: &str| -> String {
                    Command::new("hyprctl")
                        .args(["getoption", opt, "-j"])
                        .output()
                        .ok()
                        .and_then(|o| crate::json::parse(&String::from_utf8_lossy(&o.stdout)).ok())
                        .and_then(|j| {
                            j.get("css")
                                .and_then(crate::json::Json::as_str)
                                .map(|s| s.split_whitespace().next().unwrap_or("0").to_string())
                        })
                        .unwrap_or_else(|| "0".to_string())
                };

                let gi = get_gap("general:gaps_in");
                let go = get_gap("general:gaps_out");
                let bs = get_int("general:border_size");
                let rd = get_int("decoration:rounding");
                let bl = get_bool("decoration:blur:enabled");
                let sh = get_bool("decoration:shadow:enabled");
                let an = get_bool("animations:enabled");

                let snap_json = format!(
                    "{{\"gaps_in\":\"{gi}\",\"gaps_out\":\"{go}\",\"border_size\":{bs},\"rounding\":{rd},\"blur\":{bl},\"shadow\":{sh},\"anim\":{an}}}"
                );
                let _ = std::fs::write(&snap_path, snap_json);
            }

            let _ = Command::new("hyprctl")
                .args([
                    "eval",
                    "hl.config({ general = { gaps_in = 0, gaps_out = 0, border_size = 0 }, decoration = { rounding = 0, blur = { enabled = false }, shadow = { enabled = false } }, animations = { enabled = false } })",
                ])
                .status();
            0
        }
        "off" => {
            if snap_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&snap_path) {
                    if let Ok(j) = crate::json::parse(&content) {
                        let gi = j.get("gaps_in").and_then(crate::json::Json::as_str).unwrap_or("0");
                        let go = j.get("gaps_out").and_then(crate::json::Json::as_str).unwrap_or("0");
                        let bs = j.get("border_size").and_then(crate::json::Json::as_i64).unwrap_or(0);
                        let rd = j.get("rounding").and_then(crate::json::Json::as_i64).unwrap_or(0);
                        let bl = j.get("blur").and_then(crate::json::Json::as_bool).unwrap_or(false);
                        let sh = j.get("shadow").and_then(crate::json::Json::as_bool).unwrap_or(false);
                        let an = j.get("anim").and_then(crate::json::Json::as_bool).unwrap_or(false);

                        let restore_cmd = format!(
                            "hl.config({{ general = {{ gaps_in = {gi}, gaps_out = {go}, border_size = {bs} }}, decoration = {{ rounding = {rd}, blur = {{ enabled = {bl} }}, shadow = {{ enabled = {sh} }} }}, animations = {{ enabled = {an} }} }})"
                        );
                        let _ = Command::new("hyprctl").args(["eval", &restore_cmd]).status();
                    }
                }
                let _ = std::fs::remove_file(&snap_path);
            } else {
                let _ = Command::new("hyprctl").arg("reload").status();
            }
            0
        }
        _ => {
            eprintln!("xiu gamemode strip: usage: xiu gamemode strip on|off");
            1
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
