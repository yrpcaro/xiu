//! Clipvault watcher daemon supervision, thumbnail extraction, history wipe,
//! and paste-latest automation.
//!
//! Replaces cliphist-watch.sh, cliphist-thumbs.sh, and paste-latest.sh.

use crate::helpers::{cache_file, config_file, home_path, ipc_call, on_path, run_status};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

extern "C" {
    fn flock(fd: std::os::raw::c_int, operation: std::os::raw::c_int) -> std::os::raw::c_int;
}

const LOCK_EX: std::os::raw::c_int = 2;
const LOCK_NB: std::os::raw::c_int = 4;

pub fn clipboard(action: Option<&str>, target: Option<&str>) -> i32 {
    match action {
        None => ipc_call("pill", &["clipboard", ""]),
        Some("watch") => watch(),
        Some("get") => get(target),
        Some("thumbs") => thumbs(),
        Some("wipe") | Some("clear") => wipe(),
        Some("paste-latest") | Some("paste") => paste_latest(),
        Some(other) => {
            eprintln!(
                "xiu clipboard: unknown action '{other}' (watch, get, thumbs, wipe, paste-latest)"
            );
            2
        }
    }
}

pub fn watch() -> i32 {
    if !on_path("clipvault") {
        return 0;
    }

    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    let lock_path = dir.join("xiu-clipvault-watch.lock");

    let file = match fs::File::create(&lock_path) {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let fd = file.as_raw_fd();
    let res = unsafe { flock(fd, LOCK_EX | LOCK_NB) };
    if res != 0 {
        return 0; // Already running
    }

    let mut cmd = Command::new("wl-paste");
    cmd.args(["--watch", "clipvault", "store"]);
    run_status(&mut cmd)
}

pub fn get(target: Option<&str>) -> i32 {
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
        if on_path("wl-paste") {
            run_status(&mut Command::new("wl-paste"))
        } else {
            eprintln!("xiu clipboard get: wl-paste is not installed");
            1
        }
    }
}

pub fn thumbs() -> i32 {
    if !on_path("clipvault") {
        return 0;
    }

    let cache = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".cache"]))
        .join("clipvault-thumbs");
    let _ = fs::create_dir_all(&cache);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&cache, fs::Permissions::from_mode(0o700));
    }

    let list_out = match Command::new("clipvault").arg("list").output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return 0,
    };

    if list_out.trim().is_empty() {
        return 0;
    }

    let mut valid_ids = HashSet::new();
    let mut image_lines = Vec::new();

    for line in list_out.lines() {
        let mut parts = line.split('\t');
        if let Some(id_str) = parts.next() {
            if let Ok(id) = id_str.parse::<u64>() {
                valid_ids.insert(id.to_string());
            }
            if line.contains("[[ binary data") && line.contains("image/") {
                image_lines.push((id_str.to_string(), line.to_string()));
            }
        }
    }

    // 1. Prune stale thumbnails
    if let Ok(entries) = fs::read_dir(&cache) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("png") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    if stem.chars().all(|c| c.is_ascii_digit()) && !valid_ids.contains(stem) {
                        let _ = fs::remove_file(p);
                    }
                }
            }
        }
    }

    let policy_path = config_file(&["hypr", "scripts", "magick-policy"]);

    // 2. Generate missing thumbnails
    for (id, line_content) in image_lines {
        let thumb = cache.join(format!("{id}.png"));
        let needs_gen = match fs::metadata(&thumb) {
            Ok(m) => m.len() == 0,
            Err(_) => true,
        };

        if needs_gen {
            let raw = cache.join(format!(".raw.{id}"));
            if let Ok(mut get_proc) = Command::new("clipvault")
                .arg("get")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = get_proc.stdin.take() {
                    let _ = stdin.write_all(line_content.as_bytes());
                }
                if let Ok(out) = get_proc.wait_with_output() {
                    let _ = fs::write(&raw, &out.stdout);
                }
            }

            if raw.is_file() && fs::metadata(&raw).map(|m| m.len() > 0).unwrap_or(false) {
                let tmp_thumb = cache.join(format!("{id}.png.tmp"));
                let raw_arg = format!("{}[0]", raw.to_string_lossy());
                let out_arg = format!("png:{}", tmp_thumb.to_string_lossy());

                let mut magick_cmd = Command::new("magick");
                if policy_path.is_dir() {
                    magick_cmd.env("MAGICK_CONFIGURE_PATH", &policy_path);
                }
                let status = magick_cmd
                    .args([&raw_arg, "-resize", "256x256>", &out_arg])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();

                if status.map(|s| s.success()).unwrap_or(false)
                    && fs::metadata(&tmp_thumb).map(|m| m.len() > 0).unwrap_or(false)
                {
                    let _ = fs::rename(&tmp_thumb, &thumb);
                } else {
                    let _ = fs::remove_file(&tmp_thumb);
                }
            }
            let _ = fs::remove_file(&raw);
        }
    }

    0
}

pub fn wipe() -> i32 {
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

pub fn paste_latest() -> i32 {
    thread::sleep(Duration::from_millis(300));

    let out = Command::new("clipvault")
        .args(["get", "--index", "0"])
        .output();

    let entry = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return 0,
    };

    if entry.is_empty() {
        return 0;
    }

    // Put on clipboard
    if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(entry.as_bytes());
        }
        let _ = child.wait();
    }

    // Type via dotool line protocol
    let mut dotool_input = String::new();
    for (i, line) in entry.lines().enumerate() {
        if i > 0 {
            dotool_input.push_str("key Return\n");
        }
        dotool_input.push_str(&format!("type {line}\n"));
    }

    if let Ok(mut child) = Command::new("dotool").stdin(Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(dotool_input.as_bytes());
        }
        let _ = child.wait();
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_clipboard_action() {
        assert_eq!(clipboard(Some("invalid"), None), 2);
    }
}
