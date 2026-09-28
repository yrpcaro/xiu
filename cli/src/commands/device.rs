//! Device and external mount management.
//!
//! Replaces mount-phone.sh.

use crate::helpers::{home_path, on_path};
use std::fs;
use std::process::Command;

pub fn is_phone_mounted() -> bool {
    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            if line.contains("simple-mtpfs") || line.contains("jmtpfs") {
                return true;
            }
        }
    }
    false
}

pub fn mount_phone(action: &str) -> i32 {
    let action = match action {
        "" => "toggle",
        other => other,
    };

    if !matches!(action, "up" | "down" | "toggle") {
        eprintln!("xiu mount-phone: usage: xiu mount-phone [up|down|toggle]");
        return 2;
    }

    let mnt = home_path(&["mnt", "phone"]);
    let mnt_str = mnt.to_string_lossy();

    let mounted = is_phone_mounted();
    if mounted && action != "up" {
        let status = Command::new("fusermount")
            .arg("-u")
            .arg(&*mnt_str)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("phone unmounted");
                return 0;
            }
            _ => {
                eprintln!("xiu mount-phone: failed to unmount {mnt_str}");
                return 1;
            }
        }
    }

    if action == "down" {
        return 0;
    }

    let fs_tool = if on_path("simple-mtpfs") {
        "simple-mtpfs"
    } else if on_path("jmtpfs") {
        "jmtpfs"
    } else {
        eprintln!("no MTP filesystem tool (install simple-mtpfs)");
        return 1;
    };

    let _ = fs::create_dir_all(&mnt);
    if is_phone_mounted() {
        println!("already mounted at {mnt_str}");
        return 0;
    }

    let status = Command::new(fs_tool).arg(&*mnt_str).status();
    match status {
        Ok(s) if s.success() => {
            println!("phone mounted at {mnt_str}");
            0
        }
        _ => {
            eprintln!("xiu mount-phone: failed to mount at {mnt_str}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_action_rejected() {
        assert_eq!(mount_phone("invalid-verb"), 2);
    }

    #[test]
    fn test_down_when_not_mounted() {
        // Calling down when not mounted should succeed with 0
        if !is_phone_mounted() {
            assert_eq!(mount_phone("down"), 0);
        }
    }
}
