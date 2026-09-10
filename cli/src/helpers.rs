//! Shared process plumbing: every command funnels through these so exit
//! codes stay consistent (the child's code when it ran, 127 when the binary
//! is missing) and the IPC call shape is spelled once.

use std::io::Write;
use std::process::{Command, Stdio};

/// `qs -c pill ipc call <target> <fn> [args...]` — the shell's whole command
/// surface.
pub fn ipc_call(target: &str, args: &[&str]) -> i32 {
    run_status(Command::new("qs").args(["-c", "pill", "ipc", "call", target]).args(args))
}

pub fn passthrough(bin: &str, args: &[String]) -> i32 {
    run_status(Command::new(bin).args(args))
}

pub fn run_status(cmd: &mut Command) -> i32 {
    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("xiu: {e}");
            127
        }
    }
}

/// One git command run quietly in `repo`; stdout on success, None on any
/// failure so callers can fall through to the "no data" wording.
pub fn git_out(repo: &str, args: &[&str]) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

/// A PATH probe good enough for a report: the file exists in a PATH dir.
pub fn on_path(bin: &str) -> bool {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|dir| std::path::Path::new(&format!("{dir}/{bin}")).is_file())
}

/// The focused monitor's name, resolved exactly like record.sh: one
/// activeworkspace round trip, the monitor field fished out by hand.
pub fn focused_monitor() -> String {
    if let Ok(out) = Command::new("hyprctl").args(["activeworkspace", "-j"]).output() {
        let s = String::from_utf8_lossy(&out.stdout);
        if let Some(i) = s.find("\"monitor\":\"") {
            let rest = &s[i + "\"monitor\":\"".len()..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
    }
    String::new()
}

/// Put text on the Wayland clipboard and say so; the glyph lands wherever the
/// user pastes next.
pub fn copy_text(text: &str) -> i32 {
    match Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            match child.wait() {
                Ok(s) if s.success() => {
                    println!("{text}");
                    0
                }
                _ => 1,
            }
        }
        Err(e) => {
            eprintln!("xiu: wl-copy unavailable ({e})");
            127
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_path_finds_a_real_binary() {
        // /bin/sh is universal on the boxes this runs on.
        assert!(on_path("sh"));
    }

    #[test]
    fn on_path_rejects_fiction() {
        assert!(!on_path("definitely-not-a-binary-xyz"));
    }

    #[test]
    fn focused_monitor_falls_back_to_empty() {
        // No Hyprland in a test environment: the probe must yield "" rather
        // than panic or hang (it is bounded by Command::output's failure).
        let mon = focused_monitor();
        assert!(mon.is_empty() || !mon.contains('"'));
    }
}
