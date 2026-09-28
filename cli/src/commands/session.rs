//! Session locking and control.
//!
//! Replaces lock.sh.

use crate::json;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Lock the session: capture grabs of all active monitors and trigger the lock daemon.
pub fn lock() -> i32 {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));

    let mut monitors = Vec::new();
    if let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Ok(json::Json::Arr(mons)) = json::parse(&text) {
            for m in mons {
                if let Some(name) = m.get("name").and_then(json::Json::as_str) {
                    if !name.is_empty() {
                        monitors.push(name.to_string());
                    }
                }
            }
        }
    }

    let mut children = Vec::new();
    for out in &monitors {
        let target = dir.join(format!("ricelin-lock-{out}.png"));
        let _ = fs::remove_file(&target);
        if let Ok(child) = Command::new("grim")
            .arg("-o")
            .arg(out)
            .arg(&target)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            children.push(child);
        }
    }

    for mut child in children {
        let _ = child.wait();
    }

    let trigger_file = dir.join("ricelin-lock-trigger");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let _ = fs::write(trigger_file, format!("{now}\n"));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_execution_safe_without_hyprland() {
        // Must succeed with exit code 0 even if hyprctl/grim are not present
        assert_eq!(lock(), 0);
    }
}
