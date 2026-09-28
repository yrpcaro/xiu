//! Display settings apply, keep, and revert watchdog.
//!
//! Replaces display-apply.sh.

use crate::json;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn snapshot_old(out: &str, old_file: &PathBuf) -> Result<(), String> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .map_err(|e| e.to_string())?;

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed = json::parse(&text).map_err(|e| e.to_string())?;

    if let json::Json::Arr(mons) = parsed {
        for m in mons {
            if m.get("name").and_then(json::Json::as_str) == Some(out) {
                let w = m.get("width").and_then(json::Json::as_i64).unwrap_or(1920);
                let h = m.get("height").and_then(json::Json::as_i64).unwrap_or(1080);
                let rate = match m.get("refreshRate") {
                    Some(json::Json::Num(n)) => {
                        let rounded = (*n * 1000.0).round() / 1000.0;
                        rounded.to_string()
                    }
                    _ => "60".to_string(),
                };
                let x = m.get("x").and_then(json::Json::as_i64).unwrap_or(0);
                let y = m.get("y").and_then(json::Json::as_i64).unwrap_or(0);
                let scale = match m.get("scale") {
                    Some(json::Json::Num(n)) => n.to_string(),
                    Some(json::Json::Str(s)) => s.clone(),
                    _ => "1".to_string(),
                };

                let spec = format!(
                    "hl.monitor({{ output = \"{out}\", mode = \"{w}x{h}@{rate}\", position = \"{x}x{y}\", scale = {scale} }}) return \"ok\""
                );
                fs::write(old_file, spec).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    Err(format!("monitor '{out}' not found in hyprctl monitors"))
}

pub fn display(
    verb: &str,
    out: Option<&str>,
    mode: Option<&str>,
    position: Option<&str>,
    scale: Option<&str>,
) -> i32 {
    let Some(out) = out else {
        eprintln!("xiu display: missing monitor output name");
        return 2;
    };

    let tmp_dir = std::env::temp_dir();
    let old_file = tmp_dir.join(format!("ricelin-display-{out}.old"));
    let pending_file = tmp_dir.join(format!("ricelin-display-{out}.pending"));

    match verb {
        "apply" => {
            let (Some(m), Some(pos), Some(sc)) = (mode, position, scale) else {
                eprintln!("xiu display apply: requires mode, position, and scale");
                return 2;
            };

            if !old_file.exists() {
                if let Err(e) = snapshot_old(out, &old_file) {
                    eprintln!("xiu display apply: failed to snapshot old mode: {e}");
                    return 1;
                }
            }

            let token = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos().to_string())
                .unwrap_or_else(|_| "0".to_string());

            if let Err(e) = fs::write(&pending_file, &token) {
                eprintln!("xiu display apply: failed to write pending token: {e}");
                return 1;
            }

            let new_spec = format!(
                "hl.monitor({{ output = \"{out}\", mode = \"{m}\", position = \"{pos}\", scale = {sc} }}) return \"ok\""
            );
            let _ = Command::new("hyprctl")
                .args(["eval", &new_spec])
                .status();

            let pending_str = pending_file.to_string_lossy().to_string();
            let old_str = old_file.to_string_lossy().to_string();
            let script = format!(
                "sleep 14; if [ \"$(cat '{pending_str}' 2>/dev/null)\" = '{token}' ]; then hyprctl eval \"$(cat '{old_str}')\" >/dev/null 2>&1; rm -f '{pending_str}' '{old_str}'; fi"
            );

            let _ = Command::new("sh")
                .args(["-c", &script])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            0
        }
        "keep" => {
            let _ = fs::remove_file(&pending_file);
            let _ = fs::remove_file(&old_file);
            0
        }
        "revert" => {
            if old_file.is_file() {
                if let Ok(content) = fs::read_to_string(&old_file) {
                    let _ = Command::new("hyprctl")
                        .args(["eval", &content])
                        .status();
                }
            }
            let _ = fs::remove_file(&pending_file);
            let _ = fs::remove_file(&old_file);
            0
        }
        other => {
            eprintln!("xiu display: unknown verb '{other}' (apply, keep, revert)");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_verb_returns_2() {
        assert_eq!(display("unknown", Some("DP-1"), None, None, None), 2);
    }

    #[test]
    fn test_missing_out_returns_2() {
        assert_eq!(display("keep", None, None, None, None), 2);
    }

    #[test]
    fn test_keep_cleans_files() {
        let tmp = std::env::temp_dir();
        let old = tmp.join("ricelin-display-TEST-1.old");
        let pend = tmp.join("ricelin-display-TEST-1.pending");
        fs::write(&old, "test").unwrap();
        fs::write(&pend, "test").unwrap();
        assert_eq!(display("keep", Some("TEST-1"), None, None, None), 0);
        assert!(!old.exists());
        assert!(!pend.exists());
    }
}
