//! Window management: special workspaces, scratchpad, and minimize toggles.
//!
//! Replaces special-toggle.sh and minimize-toggle.sh.

use crate::helpers::ipc_call;
use crate::json;
use std::process::Command;

/// Returns (address, workspace_name) of the currently active window.
pub fn active_window_info() -> Option<(String, String)> {
    let out = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let parsed = json::parse(&text).ok()?;

    let addr = parsed.get("address").and_then(json::Json::as_str)?.to_string();
    if addr.is_empty() {
        return None;
    }

    let ws = parsed
        .get("workspace")
        .and_then(|w| w.get("name"))
        .and_then(json::Json::as_str)
        .unwrap_or("")
        .to_string();

    Some((addr, ws))
}

/// Returns the active workspace ID of the focused monitor (default 1).
pub fn focused_monitor_workspace_id() -> i64 {
    if let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Ok(json::Json::Arr(monitors)) = json::parse(&text) {
            for mon in monitors {
                if mon.get("focused").and_then(json::Json::as_bool) == Some(true) {
                    if let Some(ws_id) = mon
                        .get("activeWorkspace")
                        .and_then(|w| w.get("id"))
                        .and_then(json::Json::as_i64)
                    {
                        return ws_id;
                    }
                }
            }
        }
    }
    1
}

/// Toggle the focused window in and out of a special workspace.
pub fn special(name: Option<&str>) -> i32 {
    let name = match name {
        Some(n) if !n.is_empty() => n,
        _ => return 0,
    };

    let Some((addr, ws)) = active_window_info() else {
        return 0;
    };

    let target_special = format!("special:{name}");
    if ws == target_special {
        let real = focused_monitor_workspace_id();
        let dispatch_arg = format!(
            "hl.dsp.window.move({{ workspace = {real}, window = \"address:{addr}\" }})"
        );
        let _ = Command::new("hyprctl")
            .args(["dispatch", &dispatch_arg])
            .status();
    } else {
        let dispatch_arg = format!(
            "hl.dsp.window.move({{ workspace = \"special:{name}\", follow = false, window = \"address:{addr}\" }})"
        );
        let _ = Command::new("hyprctl")
            .args(["dispatch", &dispatch_arg])
            .status();
    }

    0
}

/// SUPER+M minimize toggle.
pub fn minimize() -> i32 {
    let Some((addr, ws)) = active_window_info() else {
        return 0;
    };

    if ws == "special:minimized" {
        let real = focused_monitor_workspace_id();
        let payload = format!("{addr}|{real}");
        ipc_call("pill", &["restoreWindow", &payload])
    } else {
        ipc_call("pill", &["minimizeWindow", &addr])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_empty_name_returns_zero() {
        assert_eq!(special(None), 0);
        assert_eq!(special(Some("")), 0);
    }

    #[test]
    fn test_focused_monitor_workspace_fallback() {
        // Without hyprctl, falls back safely to 1
        let ws = focused_monitor_workspace_id();
        assert!(ws >= 1);
    }
}
