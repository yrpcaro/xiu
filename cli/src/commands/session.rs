//! Session locking and control.
//!
//! Replaces lock.sh with integrated idle management and suspension guards.

use crate::json;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub const VIDEO_PLAYERS: &[&str] = &[
    "mpv",
    "vlc",
    "celluloid",
    "totem",
    "kodi",
    "plex",
    "freetube",
    "clapper",
    "haruna",
    "smplayer",
    "firefox",
    "chromium",
    "brave",
    "chrome",
];

pub const GUARDED_COMMANDS: &[&str] = &[
    "antigravity",
    "agy",
    "claude",
    "cargo",
    "rustc",
    "npm",
    "yarn",
    "pnpm",
    "ninja",
    "cmake",
    "git",
    "ffmpeg",
    "make",
    "gcc",
    "g++",
    "clang",
    "clang++",
    "yay",
    "pacman",
    "paru",
];

/// Helper to parse Hyprland clients JSON and check if any client is inhibiting idle.
pub fn is_inhibiting_idle_in_json(text: &str) -> bool {
    if let Ok(json::Json::Arr(clients)) = json::parse(text) {
        for c in clients {
            if let Some(true) = c.get("inhibitingIdle").and_then(json::Json::as_bool) {
                return true;
            }
        }
    }
    false
}

/// Helper to parse playerctl output (format: `playerName:status`) and check if video is playing.
pub fn is_video_player_active_in_output(output: &str) -> bool {
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 2 {
            let player = parts[0].to_lowercase();
            let status = parts[1].to_lowercase();
            if status == "playing" {
                for &vp in VIDEO_PLAYERS {
                    if player.contains(vp) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Helper to check if a process's comm or cmdline matches guarded background tasks.
pub fn matches_guarded_task(comm: &str, cmdline: &str) -> bool {
    let comm_clean = comm.trim().to_lowercase();
    let cmdline_clean = cmdline.trim().to_lowercase();

    // Ignore self invocations or wrapper commands
    if cmdline_clean.contains("xiu suspend") || cmdline_clean.contains("xiu lock") {
        return false;
    }

    for &guard in GUARDED_COMMANDS {
        if comm_clean == guard || comm_clean.starts_with(guard) {
            return true;
        }
        // In cmdline, check for binary invocation or arguments
        if cmdline_clean.split_whitespace().any(|token| {
            token == guard
                || token.ends_with(&format!("/{guard}"))
                || (guard == "antigravity" && token.contains("antigravity"))
                || (guard == "claude" && token.contains("claude"))
        }) {
            return true;
        }
    }
    false
}

/// Check if video playback is active via Hyprland idle-inhibit or active video players.
pub fn is_video_playing() -> bool {
    // 1. Check Hyprland clients inhibiting idle
    if let Ok(out) = Command::new("hyprctl").args(["clients", "-j"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if is_inhibiting_idle_in_json(&text) {
            return true;
        }
    }

    // 2. Check playerctl for video players playing
    if let Ok(out) = Command::new("playerctl")
        .args(["-a", "status", "-f", "{{playerName}}:{{status}}"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        if is_video_player_active_in_output(&text) {
            return true;
        }
    } else if let Ok(out) = Command::new("playerctl")
        .args(["-a", "metadata", "--format", "{{playerName}}:{{status}}"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        if is_video_player_active_in_output(&text) {
            return true;
        }
    }

    false
}

/// Check if system suspension should be inhibited due to active background tasks or video.
pub fn check_suspend_inhibition() -> Option<String> {
    if is_video_playing() {
        return Some("video playback is active".to_string());
    }

    // Inspect /proc for guarded background tasks
    let my_pid = std::process::id();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if let Ok(pid) = name_str.parse::<u32>() {
                if pid == my_pid {
                    continue;
                }
                let proc_path = entry.path();
                let comm = fs::read_to_string(proc_path.join("comm"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let cmdline = fs::read_to_string(proc_path.join("cmdline"))
                    .unwrap_or_default()
                    .replace('\0', " ")
                    .trim()
                    .to_string();

                if matches_guarded_task(&comm, &cmdline) {
                    return Some(format!("active background task '{comm}' (pid {pid})"));
                }
            }
        }
    }

    None
}

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

/// Lock command with optional --idle flag.
pub fn lock_cmd(idle: bool) -> i32 {
    if idle && is_video_playing() {
        eprintln!("xiu: lock inhibited: video playback is active");
        return 0;
    }
    lock()
}

/// Suspend command with optional --idle flag.
pub fn suspend_cmd(idle: bool) -> i32 {
    if idle {
        if let Some(reason) = check_suspend_inhibition() {
            eprintln!("xiu: suspend inhibited: {reason}");
            return 0;
        }
    }
    let status = Command::new("systemctl").arg("suspend").status();
    match status {
        Ok(s) => s.code().unwrap_or(0),
        Err(e) => {
            eprintln!("xiu: failed to dispatch systemctl suspend: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_execution_safe_without_hyprland() {
        // Must succeed with exit code 0 even if hyprctl/grim are not present
        assert_eq!(lock(), 0);
    }

    #[test]
    fn test_inhibiting_idle_json_parsing() {
        let empty_json = "[]";
        assert!(!is_inhibiting_idle_in_json(empty_json));

        let non_inhibiting = r#"[{"class": "foot", "inhibitingIdle": false}]"#;
        assert!(!is_inhibiting_idle_in_json(non_inhibiting));

        let inhibiting = r#"[{"class": "mpv", "inhibitingIdle": true}]"#;
        assert!(is_inhibiting_idle_in_json(inhibiting));
    }

    #[test]
    fn test_video_player_active_detection() {
        let spotify_only = "spotify:Playing\n";
        assert!(!is_video_player_active_in_output(spotify_only));

        let mpv_paused = "mpv:Paused\nspotify:Playing\n";
        assert!(!is_video_player_active_in_output(mpv_paused));

        let mpv_playing = "mpv:Playing\n";
        assert!(is_video_player_active_in_output(mpv_playing));

        let vlc_playing = "vlc:Playing\n";
        assert!(is_video_player_active_in_output(vlc_playing));

        let brave_playing = "brave.instance1:Playing\n";
        assert!(is_video_player_active_in_output(brave_playing));
    }

    #[test]
    fn test_matches_guarded_task() {
        assert!(matches_guarded_task("antigravity", "/usr/bin/antigravity"));
        assert!(matches_guarded_task("agy", "agy --dangerously-sk ~/xiu"));
        assert!(matches_guarded_task("claude", "claude code"));
        assert!(matches_guarded_task("cargo", "cargo build --release"));
        assert!(matches_guarded_task("ffmpeg", "ffmpeg -i in.mp4 out.mp4"));
        assert!(!matches_guarded_task("bash", "xiu suspend --idle"));
        assert!(!matches_guarded_task("ls", "ls -la"));
    }
}
