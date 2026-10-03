//! The shell-facing commands: everything that talks to the pill's IPC socket
//! or shells out to the rice's own tools.

use crate::helpers::{focused_monitor, ipc_call, run_status};
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
    let mon = focused_monitor();
    let pages = [
        "user", "appearance", "look", "display", "input",
        "animation", "keybinds", "workspaces", "idlelock",
        "defaultapps", "updates",
    ];
    if pages.contains(&surface) {
        ipc_call("pill", &["page", &mon, surface])
    } else {
        ipc_call("pill", &[surface, &mon])
    }
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
    let mut mode = "region";
    if let Some(first) = args.first() {
        match first.as_str() {
            "monitor" | "--monitor" | "-m" => mode = "monitor",
            "region" | "--region" | "-r" | "" => mode = "region",
            "-h" | "--help" => {
                println!(
                    "rishot - screenshot + annotate\n\n\
usage: rishot [mode]\n\n\
modes:\n  \
(none), region   drag a region, or click a window to grab it (default)\n  \
monitor          click a monitor to grab the whole output\n\n\
options:\n  \
-h, --help       show this help\n\n\
env:\n  \
RISHOT_CONFIG_DIR  override the Quickshell config dir (must hold shell.qml)\n  \
RISHOT_UPLOAD      override the upload endpoint (https form-post target)\n  \
RISHOT_SAVEDIR     override the auto-save directory\n  \
RISHOT_KEYBIND_FILE  write the in-app rebind here instead of autodetecting"
                );
                return 0;
            }
            other => {
                eprintln!("rishot: unknown argument: {other}");
                eprintln!(
                    "rishot - screenshot + annotate\n\n\
usage: rishot [mode]\n\n\
modes:\n  \
(none), region   drag a region, or click a window to grab it (default)\n  \
monitor          click a monitor to grab the whole output\n\n\
options:\n  \
-h, --help       show this help"
                );
                return 2;
            }
        }
    }

    let config_dir = if let Ok(dir) = std::env::var("RISHOT_CONFIG_DIR") {
        if !dir.is_empty() {
            Some(std::path::PathBuf::from(dir))
        } else {
            find_rishot_dir()
        }
    } else {
        find_rishot_dir()
    };

    let dir = match config_dir {
        Some(d) if d.join("shell.qml").is_file() => d,
        _ => {
            eprintln!("rishot: could not locate the config dir (set RISHOT_CONFIG_DIR)");
            return 1;
        }
    };

    if !crate::helpers::on_path("qs") {
        eprintln!("rishot: 'qs' (quickshell) not found in PATH");
        return 1;
    }

    let rundir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.cache")
    });
    let _ = std::fs::create_dir_all(&rundir);
    let lock_path = std::path::PathBuf::from(&rundir).join("rishot.lock");
    let lock_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .or_else(|_| {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open("/tmp/rishot.lock")
        });

    let _lock = if let Ok(file) = lock_file {
        use std::os::unix::io::AsRawFd;
        let fd = file.as_raw_fd();
        extern "C" {
            fn flock(fd: i32, operation: i32) -> i32;
        }
        let lock_res = unsafe { flock(fd, 2 | 4) }; // LOCK_EX | LOCK_NB
        if lock_res != 0 {
            eprintln!("rishot: already running");
            return 0;
        }
        Some(file)
    } else {
        None
    };

    let mut cmd = Command::new("qs");
    cmd.env("RISHOT_MODE", mode)
        .arg("-p")
        .arg(dir);
    run_status(&mut cmd)
}

fn find_rishot_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let xdg_config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));
    let mut candidates = vec![
        format!("{xdg_config}/quickshell/rishot"),
        format!("{home}/xiu/configs/quickshell/rishot"),
        format!("{home}/.local/share/rishot/src"),
        "/usr/share/rishot/src".to_string(),
        "/usr/lib/rishot/src".to_string(),
    ];
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("configs/quickshell/rishot").to_string_lossy().to_string());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("../../configs/quickshell/rishot").to_string_lossy().to_string());
            candidates.push(parent.join("../configs/quickshell/rishot").to_string_lossy().to_string());
        }
    }
    for c in &candidates {
        let p = std::path::PathBuf::from(c);
        if p.join("shell.qml").is_file() {
            return Some(p);
        }
    }
    None
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

/// The palette engine is wallcolors; the CLI is its front door. Scheme
/// state survives wallpaper changes in its own state file, and an explicit
/// change flips the pill's paletteMode so the shell actually listens.
pub fn scheme(action: &str, value: Option<&str>, variant: Option<&str>) -> i32 {
    let run = |flags: &[&str]| {
        let args: Vec<String> = flags.iter().map(|s| s.to_string()).collect();
        crate::commands::wallcolors::wallcolors(&args)
    };

    match action {
        "list" => run(&["--list-presets"]),
        "get" => run(&["--state"]),
        "preview" => {
            let Some(wallpaper) = value else {
                eprintln!("xiu scheme preview: needs a wallpaper path");
                return 2;
            };
            run(&["--preview", wallpaper])
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
            let res = run(&refs);
            if res == 0 {
                let _ = crate::commands::theme::live_reload();
            }
            res
        }
        "dark" => {
            let res = run(&["--mode", "dark"]);
            if res == 0 {
                let _ = crate::commands::theme::live_reload();
            }
            res
        }
        "light" => {
            let res = run(&["--mode", "light"]);
            if res == 0 {
                let _ = crate::commands::theme::live_reload();
            }
            res
        }
        "toggle" => {
            let res = run(&["--toggle"]);
            if res == 0 {
                let _ = crate::commands::theme::live_reload();
            }
            res
        }
        other => {
            eprintln!("xiu scheme: unknown action '{other}' (list, get, set, preview, dark, light, toggle)");
            2
        }
    }
}

/// Brave and Chromium read their toolbar color from a managed policy or user profile.
/// The palette pipeline keeps the payload fresh in ~/.config/xiu/browser-theme.json.
/// Applies the theme for the current user in ~/.config without requiring root,
/// updates browser profile preferences if present, and updates system /etc policies
/// as an optional fallback if root access happens to be available.
pub fn browser() -> i32 {
    let home = std::env::var("HOME").unwrap_or_default();
    let payload = format!("{home}/.config/xiu/browser-theme.json");
    let payload_path = std::path::Path::new(&payload);
    if !payload_path.is_file() {
        eprintln!("xiu browser: no palette payload yet (run a wallpaper change or `xiu scheme set` first)");
        return 1;
    }

    // User-level policy targets under ~/.config
    let user_targets = [
        format!("{home}/.config/BraveSoftware/Brave-Browser/policies/managed/xiu.json"),
        format!("{home}/.config/chromium/policies/managed/xiu.json"),
        format!("{home}/.config/brave/policies/managed/xiu.json"),
        format!("{home}/.config/google-chrome/policies/managed/xiu.json"),
    ];

    let mut applied_any = false;
    for target in &user_targets {
        let p = std::path::Path::new(target);
        if let Some(parent) = p.parent() {
            if let Ok(()) = std::fs::create_dir_all(parent) {
                if let Ok(_) = std::fs::copy(&payload, p) {
                    println!("applied → {target}");
                    applied_any = true;
                }
            }
        }
    }

    // Update user profile Preferences if present
    let update_py = r#"
import json, glob, os
home = os.path.expanduser('~')
theme_file = os.path.join(home, '.config/xiu/browser-theme.json')
if os.path.isfile(theme_file):
    try:
        data = json.load(open(theme_file))
        color_hex = data.get('BrowserThemeColor', '')
        if color_hex.startswith('#') and len(color_hex) == 7:
            r = int(color_hex[1:3], 16)
            g = int(color_hex[3:5], 16)
            b = int(color_hex[5:7], 16)
            val = (0xFF << 24) | (r << 16) | (g << 8) | b
            sk_color = val if val < 0x80000000 else val - 0x100000000
            for pref in glob.glob(f'{home}/.config/**/Preferences', recursive=True):
                if any(b in pref for b in ('BraveSoftware', 'chromium', 'google-chrome', 'brave')):
                    try:
                        with open(pref, 'r') as f:
                            prefs = json.load(f)
                        if 'autogenerated' not in prefs or not isinstance(prefs['autogenerated'], dict):
                            prefs['autogenerated'] = {}
                        if 'theme' not in prefs['autogenerated'] or not isinstance(prefs['autogenerated']['theme'], dict):
                            prefs['autogenerated']['theme'] = {}
                        prefs['autogenerated']['theme']['color'] = sk_color
                        if 'browser' not in prefs or not isinstance(prefs['browser'], dict):
                            prefs['browser'] = {}
                        if 'theme' not in prefs['browser'] or not isinstance(prefs['browser']['theme'], dict):
                            prefs['browser']['theme'] = {}
                        prefs['browser']['theme']['color'] = sk_color
                        with open(pref, 'w') as f:
                            json.dump(prefs, f, indent=2)
                    except Exception:
                        pass
    except Exception:
        pass
"#;
    let _ = Command::new("python3").args(["-c", update_py]).status();

    // Best-effort optional system-level install if passwordless sudo is available
    let etc_targets = [
        "/etc/brave/policies/managed/xiu.json",
        "/etc/chromium/policies/managed/xiu.json",
    ];
    for target in etc_targets {
        if let Ok(s) = Command::new("sudo")
            .args(["-n", "install", "-m", "644", "-D", &payload, target])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            if s.success() {
                println!("applied → {target}");
            }
        }
    }

    // Refresh running browsers
    for cmd in ["brave", "chromium", "google-chrome"] {
        if crate::helpers::on_path(cmd) {
            let _ = Command::new("timeout")
                .args(["10", cmd, "--refresh-platform-policy", "--no-startup-window"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }

    if applied_any {
        println!("restart the browser to pick the new color up");
        0
    } else {
        eprintln!("xiu browser: failed to apply theme to user config");
        1
    }
}
