//! Quickshell surface watchdog daemon.
//!
//! Supervises quickshell surfaces (pill, lock) and automatically restarts
//! them when they stop responding. Replaces watchdog.sh.

use std::fs;
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
const LOCK_UN: std::os::raw::c_int = 8;

pub fn is_watchdog_running(surface: &str) -> bool {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    let lock_file = dir.join(format!("{surface}-watchdog.lock"));
    if lock_file.exists() {
        if let Ok(file) = fs::File::open(&lock_file) {
            let fd = file.as_raw_fd();
            let res = unsafe { flock(fd, LOCK_EX | LOCK_NB) };
            if res != 0 {
                return true;
            }
            unsafe { flock(fd, LOCK_UN) };
        }
    }
    false
}

pub fn is_surface_alive(surface: &str) -> bool {
    Command::new("qs")
        .args(["-c", surface, "ipc", "show"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn launch_and_wait(surface: &str) {
    let _ = Command::new("qs")
        .args(["-c", surface, "-d"])
        .env("QS_ICON_THEME", "yet-another-monochrome-icon-set")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    for _ in 0..30 {
        if is_surface_alive(surface) {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

pub fn watchdog(surface: &str) -> i32 {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    let lock_file = dir.join(format!("{surface}-watchdog.lock"));

    let file = match fs::File::create(&lock_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("xiu watchdog: unable to open lock file: {e}");
            return 1;
        }
    };

    let fd = file.as_raw_fd();
    let res = unsafe { flock(fd, LOCK_EX | LOCK_NB) };
    if res != 0 {
        // Another watchdog is already holding the lock
        return 0;
    }

    loop {
        if !is_surface_alive(surface) {
            launch_and_wait(surface);
        }
        thread::sleep(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_surface_alive_false_in_test() {
        assert!(!is_surface_alive("nonexistent_test_surface_xyz"));
    }
}
