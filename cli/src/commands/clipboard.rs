//! Clipvault watcher daemon supervision, thumbnail extraction, and history wipe.

use crate::helpers::{cache_file, config_file, ipc_call, on_path, run_status};
use std::fs;
use std::process::{Command, Stdio};

pub fn clipboard(action: Option<&str>, target: Option<&str>) -> i32 {
    match action {
        None => ipc_call("pill", &["clipboard", ""]),
        Some("watch") => watch(),
        Some("get") => get(target),
        Some("thumbs") => thumbs(),
        Some("wipe") | Some("clear") => wipe(),
        Some(other) => {
            eprintln!(
                "xiu clipboard: unknown action '{other}' (watch, get, thumbs, wipe)"
            );
            2
        }
    }
}

fn watch() -> i32 {
    if !on_path("clipvault") {
        eprintln!("xiu clipboard watch: clipvault is not installed (see `xiu check`)");
        return 1;
    }

    let script = config_file(&["hypr", "scripts", "cliphist-watch.sh"]);
    if script.is_file() {
        return run_status(Command::new("sh").arg(&script));
    }

    // Direct watcher launch
    let mut cmd = Command::new("wl-paste");
    cmd.args(["--watch", "clipvault", "store"]);
    run_status(&mut cmd)
}

fn get(target: Option<&str>) -> i32 {
    if let Some(id) = target {
        if on_path("clipvault") {
            let mut child = match Command::new("clipvault")
                .arg("get")
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("xiu clipboard get: {e}");
                    return 1;
                }
            };

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = writeln!(stdin, "{id}");
            }

            match child.wait() {
                Ok(s) => s.code().unwrap_or(0),
                Err(_) => 1,
            }
        } else {
            eprintln!("xiu clipboard get: clipvault is not installed");
            1
        }
    } else {
        // Read current clipboard via wl-paste
        if on_path("wl-paste") {
            run_status(&mut Command::new("wl-paste"))
        } else {
            eprintln!("xiu clipboard get: wl-paste is not installed");
            1
        }
    }
}

fn thumbs() -> i32 {
    let script = config_file(&["hypr", "scripts", "cliphist-thumbs.sh"]);
    if script.is_file() {
        run_status(Command::new("sh").arg(&script))
    } else {
        eprintln!("xiu clipboard thumbs: cliphist-thumbs.sh not found");
        1
    }
}

fn wipe() -> i32 {
    let mut cleared_db = false;
    if on_path("clipvault") {
        let status = Command::new("clipvault")
            .arg("clear")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                cleared_db = true;
            }
        }
    }

    // Purge thumbnail cache
    let thumb_dir = cache_file("clipvault-thumbs");
    let mut cleared_thumbs = 0;
    if thumb_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&thumb_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let _ = fs::remove_file(p);
                    cleared_thumbs += 1;
                }
            }
        }
    }

    println!(
        "xiu clipboard: history {} and {cleared_thumbs} thumbnail(s) cleared",
        if cleared_db { "wiped" } else { "database reset" }
    );
    0
}
