//! Yazi file chooser wrapper for xdg-desktop-portal-termfilechooser.
//!
//! Replaces yazi-chooser.sh.

use crate::helpers::on_path;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn resolve_yazi() -> String {
    if on_path("yazi") {
        "yazi".to_string()
    } else {
        "/usr/bin/yazi".to_string()
    }
}

pub fn resolve_terminal() -> String {
    if let Ok(term) = std::env::var("TERMCMD") {
        if !term.trim().is_empty() {
            return term;
        }
    }

    for t in ["foot", "ghostty", "kitty", "alacritty"] {
        if on_path(t) {
            return t.to_string();
        }
    }

    "foot".to_string()
}

fn run_term(term: &str, yazi_args: &[&str]) -> i32 {
    let mut cmd = Command::new(term);

    if term.contains("foot") {
        cmd.args(["--app-id=termfilechooser", "--title=File Chooser"]);
        cmd.args(yazi_args);
    } else if term.contains("ghostty") {
        cmd.args(["--class=termfilechooser", "--title=File Chooser", "-e"]);
        cmd.args(yazi_args);
    } else if term.contains("kitty") {
        cmd.args(["--class=termfilechooser", "--title=File Chooser"]);
        cmd.args(yazi_args);
    } else if term.contains("alacritty") {
        cmd.args(["--class=termfilechooser", "-t", "File Chooser", "-e"]);
        cmd.args(yazi_args);
    } else {
        cmd.args(yazi_args);
    }

    match cmd.status() {
        Ok(s) => s.code().unwrap_or(0),
        Err(e) => {
            eprintln!("xiu yazi-chooser: failed to launch terminal '{term}': {e}");
            1
        }
    }
}

fn file_has_content(path: &str) -> bool {
    fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false)
}

pub fn yazi_chooser(
    multiple: Option<&str>,
    directory: Option<&str>,
    save: Option<&str>,
    path: Option<&str>,
    out: Option<&str>,
    _debug: Option<&str>,
    _extra: &[String],
) -> i32 {
    let _multiple = multiple.unwrap_or("0");
    let directory = directory.unwrap_or("0");
    let save = save.unwrap_or("0");
    let path = path.unwrap_or("");
    let out = out.unwrap_or("");

    let yazi = resolve_yazi();
    let term = resolve_terminal();

    let chooser_flag = format!("--chooser-file={out}");

    if save == "1" {
        let p = Path::new(path);
        let dir = p.parent().unwrap_or_else(|| Path::new("."));
        let _ = fs::create_dir_all(dir);
        let dir_str = dir.to_string_lossy();

        let code = run_term(&term, &[&yazi, &chooser_flag, &dir_str]);
        if !file_has_content(out) {
            let _ = fs::remove_file(path);
        }
        code
    } else if directory == "1" {
        let tmp_file = std::env::temp_dir().join(format!(
            "xiu-yazi-cwd-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let tmp_str = tmp_file.to_string_lossy().to_string();
        let cwd_flag = format!("--cwd-file={tmp_str}");

        let code = run_term(&term, &[&yazi, &chooser_flag, &cwd_flag, path]);
        if !file_has_content(out) && file_has_content(&tmp_str) {
            let _ = fs::copy(&tmp_file, out);
        }
        let _ = fs::remove_file(&tmp_file);
        code
    } else {
        run_term(&term, &[&yazi, &chooser_flag, path])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_terminal_returns_valid() {
        let term = resolve_terminal();
        assert!(!term.is_empty());
    }

    #[test]
    fn test_file_has_content() {
        let tmp = std::env::temp_dir().join("xiu_test_empty.txt");
        let _ = fs::write(&tmp, "");
        assert!(!file_has_content(&tmp.to_string_lossy()));
        let _ = fs::write(&tmp, "data");
        assert!(file_has_content(&tmp.to_string_lossy()));
        let _ = fs::remove_file(&tmp);
    }
}
