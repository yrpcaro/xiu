//! Process launch supervision and crash reporting.
//!
//! Replaces launch-guard.sh.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

pub fn launch_guard(
    name: &str,
    icon: Option<&str>,
    wd: Option<&str>,
    cmd: &[String],
) -> i32 {
    if cmd.is_empty() {
        return 0;
    }

    let program = &cmd[0];
    let args = &cmd[1..];

    let mut command = Command::new(program);
    command.args(args);
    command.stdout(Stdio::null());
    command.stderr(Stdio::piped());

    if let Some(dir) = wd {
        if !dir.is_empty() && Path::new(dir).is_dir() {
            command.current_dir(dir);
        }
    }

    let start = Instant::now();
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            // Binary failed to start (e.g. not found -> 127)
            let rc = 127;
            let err = e.to_string();
            notify_failure(name, icon, rc, &err);
            return 0;
        }
    };

    let mut captured_stderr = Vec::new();
    if let Some(mut stderr) = child.stderr.take() {
        // Read up to 64KB (cap = 65536)
        let mut buffer = [0u8; 4096];
        while captured_stderr.len() < 65536 {
            match stderr.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let take = n.min(65536 - captured_stderr.len());
                    captured_stderr.extend_from_slice(&buffer[..take]);
                }
                Err(_) => break,
            }
        }
        // Drain any remaining stderr in background/discard to avoid SIGPIPE
        let mut drain_buf = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut drain_buf) {
            if n == 0 {
                break;
            }
        }
    }

    let rc = match child.wait() {
        Ok(status) => status.code().unwrap_or(1),
        Err(_) => 1,
    };

    let elapsed = start.elapsed();
    if rc != 0 && elapsed.as_secs() < 5 {
        let err_text = String::from_utf8_lossy(&captured_stderr);
        let err = if err_text.trim().is_empty() {
            "(no output)"
        } else {
            &err_text
        };
        notify_failure(name, icon, rc, err);
    }

    0
}

fn notify_failure(name: &str, icon: Option<&str>, rc: i32, err: &str) {
    let non_empty_lines: Vec<&str> = err
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let tail3 = if non_empty_lines.is_empty() {
        String::new()
    } else {
        let start_idx = non_empty_lines.len().saturating_sub(3);
        non_empty_lines[start_idx..].join("\n")
    };

    let mut notify_cmd = Command::new("notify-send");
    notify_cmd.args(["-u", "critical", "-a", "xiu"]);

    if let Some(ic) = icon {
        if !ic.is_empty() {
            notify_cmd.args(["-i", ic]);
        }
    }

    notify_cmd.args(["-A", "copy=Copy"]);
    notify_cmd.arg(format!("{name} failed (exit {rc})"));
    notify_cmd.arg(tail3);

    if let Ok(output) = notify_cmd.output() {
        let response = String::from_utf8_lossy(&output.stdout);
        if response.trim() == "copy" {
            if let Ok(mut wl_copy) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = wl_copy.stdin.take() {
                    let _ = write!(stdin, "{name}: exit {rc}\n{err}\n");
                }
                let _ = wl_copy.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_guard_empty_cmd() {
        assert_eq!(launch_guard("test", None, None, &[]), 0);
    }

    #[test]
    fn test_launch_guard_success_command() {
        let cmd = vec!["sh".into(), "-c".into(), "exit 0".into()];
        assert_eq!(launch_guard("test", None, None, &cmd), 0);
    }
}
