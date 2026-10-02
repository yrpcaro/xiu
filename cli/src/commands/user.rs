//! User profile, account management and session user operations.
//!
//! Replaces user management scripts, providing avatar set/remove,
//! password changing, login shell switching, and profile status.

use crate::helpers::on_path;
use crate::ui;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn resolve_terminal() -> String {
    if let Ok(term) = std::env::var("TERMINAL") {
        if !term.trim().is_empty() {
            return term;
        }
    }
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

pub fn run_in_term(title: &str, cmd_args: &[&str]) -> i32 {
    let term = resolve_terminal();
    let mut cmd = Command::new(&term);

    if term.contains("foot") {
        cmd.args(["--app-id=xiu-user-dialog", &format!("--title={title}")]);
        cmd.args(cmd_args);
    } else if term.contains("ghostty") {
        cmd.args(["--class=xiu-user-dialog", &format!("--title={title}"), "-e"]);
        cmd.args(cmd_args);
    } else if term.contains("kitty") {
        cmd.args(["--class=xiu-user-dialog", "--title", title]);
        cmd.args(cmd_args);
    } else if term.contains("alacritty") {
        cmd.args(["--class=xiu-user-dialog", "-t", title, "-e"]);
        cmd.args(cmd_args);
    } else {
        cmd.args(["-e"]);
        cmd.args(cmd_args);
    }

    match cmd.status() {
        Ok(s) => s.code().unwrap_or(0),
        Err(e) => {
            eprintln!("xiu user: failed to launch terminal '{term}': {e}");
            1
        }
    }
}

fn pick_avatar_interactive() -> Option<PathBuf> {
    // 1. zenity
    if on_path("zenity") {
        if let Ok(out) = Command::new("zenity")
            .args([
                "--file-selection",
                "--title=Select Profile Picture",
                "--file-filter=Image Files (*.png *.jpg *.jpeg *.webp *.svg *.gif) | *.png *.jpg *.jpeg *.webp *.svg *.gif",
            ])
            .output()
        {
            if out.status.success() {
                let chosen = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !chosen.is_empty() {
                    let p = PathBuf::from(chosen);
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }

    // 2. kdialog
    if on_path("kdialog") {
        if let Ok(out) = Command::new("kdialog")
            .args([
                "--getopenfilename",
                "~",
                "*.png *.jpg *.jpeg *.webp *.svg *.gif | Image files",
            ])
            .output()
        {
            if out.status.success() {
                let chosen = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !chosen.is_empty() {
                    let p = PathBuf::from(chosen);
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }

    // 3. yazi chooser wrapper
    let tmp_path = std::env::temp_dir().join(format!("xiu-avatar-pick-{}.tmp", std::process::id()));
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let _ = fs::remove_file(&tmp_path);
    let code = crate::commands::yazi::yazi_chooser(
        Some("0"),
        Some("0"),
        Some("0"),
        Some(&home),
        tmp_path.to_str(),
        None,
        &[],
    );
    if code == 0 && tmp_path.is_file() {
        if let Ok(content) = fs::read_to_string(&tmp_path) {
            let _ = fs::remove_file(&tmp_path);
            let first = content.lines().next().unwrap_or("").trim();
            if !first.is_empty() {
                let p = PathBuf::from(first);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    let _ = fs::remove_file(&tmp_path);

    None
}

pub fn set_avatar(path: Option<&str>) -> i32 {
    let k = ui::skin();
    let img_path = match path {
        Some(p) => PathBuf::from(p),
        None => match pick_avatar_interactive() {
            Some(p) => p,
            None => {
                ui::note(&k, "No image selected");
                return 0;
            }
        },
    };

    if !img_path.is_file() {
        eprintln!("xiu user avatar: file not found: {}", img_path.display());
        return 1;
    }

    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => {
            eprintln!("xiu user avatar: HOME environment variable not set");
            return 1;
        }
    };

    let face_file = home.join(".face");
    let icon_file = home.join(".face.icon");

    let mut success = false;

    // Try magick/convert to center-crop square thumbnail
    if on_path("magick") {
        let status = Command::new("magick")
            .arg(&img_path)
            .args(["-thumbnail", "256x256^", "-gravity", "center", "-extent", "256x256"])
            .arg(&face_file)
            .status();
        if let Ok(s) = status {
            if s.success() {
                success = true;
            }
        }
    } else if on_path("convert") {
        let status = Command::new("convert")
            .arg(&img_path)
            .args(["-thumbnail", "256x256^", "-gravity", "center", "-extent", "256x256"])
            .arg(&face_file)
            .status();
        if let Ok(s) = status {
            if s.success() {
                success = true;
            }
        }
    }

    if !success {
        if let Err(e) = fs::copy(&img_path, &face_file) {
            eprintln!("xiu user avatar: failed to copy image to {}: {e}", face_file.display());
            return 1;
        }
    }

    // Also mirror to .face.icon
    let _ = fs::copy(&face_file, &icon_file);

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&face_file, fs::Permissions::from_mode(0o644));
        let _ = fs::set_permissions(&icon_file, fs::Permissions::from_mode(0o644));
    }

    ui::act(&k, "updated", &format!("user profile picture ({})", face_file.display()));
    0
}

pub fn remove_avatar() -> i32 {
    let k = ui::skin();
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => {
            eprintln!("xiu user avatar: HOME environment variable not set");
            return 1;
        }
    };

    let face_file = home.join(".face");
    let icon_file = home.join(".face.icon");

    let mut removed = false;
    if face_file.exists() {
        if let Ok(()) = fs::remove_file(&face_file) {
            removed = true;
        }
    }
    if icon_file.exists() {
        if let Ok(()) = fs::remove_file(&icon_file) {
            removed = true;
        }
    }

    if removed {
        ui::act(&k, "removed", "user profile picture (~/.face)");
    } else {
        ui::note(&k, "no custom profile picture was set");
    }
    0
}

pub fn avatar_status() -> i32 {
    let k = ui::skin();
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let face_file = Path::new(&home).join(".face");
    let icon_file = Path::new(&home).join(".face.icon");

    if face_file.is_file() {
        let size = fs::metadata(&face_file).map(|m| m.len()).unwrap_or(0);
        ui::act(&k, "custom avatar set", &format!("{} ({} bytes)", face_file.display(), size));
    } else if icon_file.is_file() {
        let size = fs::metadata(&icon_file).map(|m| m.len()).unwrap_or(0);
        ui::act(&k, "custom avatar set", &format!("{} ({} bytes)", icon_file.display(), size));
    } else {
        ui::note(&k, "default avatar icon in use (no ~/.face)");
    }
    0
}

pub fn change_passwd() -> i32 {
    if std::io::stdin().is_terminal() {
        match Command::new("passwd").status() {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("xiu user: failed to run passwd: {e}");
                1
            }
        }
    } else {
        run_in_term("Change Password", &["passwd"])
    }
}

pub fn change_shell(target_shell: Option<&str>) -> i32 {
    if let Some(sh) = target_shell {
        match Command::new("chsh").args(["-s", sh]).status() {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("xiu user: failed to run chsh: {e}");
                1
            }
        }
    } else if std::io::stdin().is_terminal() {
        match Command::new("chsh").status() {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("xiu user: failed to run chsh: {e}");
                1
            }
        }
    } else {
        run_in_term("Change Login Shell", &["chsh"])
    }
}

pub fn change_gecos(name: Option<&str>) -> i32 {
    if let Some(n) = name {
        match Command::new("chfn").args(["-f", n]).status() {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("xiu user: failed to run chfn: {e}");
                1
            }
        }
    } else if std::io::stdin().is_terminal() {
        match Command::new("chfn").status() {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("xiu user: failed to run chfn: {e}");
                1
            }
        }
    } else {
        run_in_term("Change User Details", &["chfn"])
    }
}

pub fn user_info() -> i32 {
    let k = ui::skin();
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let host = std::env::var("HOSTNAME")
        .or_else(|_| fs::read_to_string("/proc/sys/kernel/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "localhost".to_string());

    let gecos = get_gecos_name(&user).unwrap_or_default();
    let face = Path::new(&home).join(".face");
    let avatar_str = if face.is_file() {
        format!("{} (set)", face.display())
    } else {
        "none (default glyph)".to_string()
    };

    ui::sec(&k, "User & Session");
    ui::gap(&k);
    ui::row(&k, &format!("{:<14} {}", "username", user));
    if !gecos.is_empty() {
        ui::row(&k, &format!("{:<14} {}", "full name", gecos));
    }
    ui::row(&k, &format!("{:<14} {}@{}", "host", user, host));
    ui::row(&k, &format!("{:<14} {}", "home", home));
    ui::row(&k, &format!("{:<14} {}", "shell", shell));
    ui::row(&k, &format!("{:<14} {}", "avatar", avatar_str));
    ui::gap(&k);

    0
}

fn get_gecos_name(username: &str) -> Option<String> {
    if let Ok(out) = Command::new("getent").args(["passwd", username]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        let fields: Vec<&str> = text.trim().split(':').collect();
        if fields.len() >= 5 {
            let gecos_part = fields[4].split(',').next().unwrap_or("").trim();
            if !gecos_part.is_empty() {
                return Some(gecos_part.to_string());
            }
        }
    }
    None
}

pub fn user(action: &str, target: Option<&str>, value: Option<&str>) -> i32 {
    match action {
        "avatar" => match target.unwrap_or("status") {
            "set" => set_avatar(value),
            "remove" | "rm" | "delete" => remove_avatar(),
            "status" | "get" => avatar_status(),
            other => {
                if Path::new(other).exists() {
                    set_avatar(Some(other))
                } else {
                    eprintln!("xiu user avatar: unknown action '{other}'. Valid: set, remove, status");
                    1
                }
            }
        },
        "passwd" | "password" => change_passwd(),
        "shell" | "chsh" => change_shell(target.or(value)),
        "gecos" | "name" | "fullname" => change_gecos(target.or(value)),
        "info" | "status" => user_info(),
        other => {
            eprintln!("xiu user: unknown command '{other}'. Valid: avatar, passwd, shell, gecos, info");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_terminal() {
        let t = resolve_terminal();
        assert!(!t.is_empty());
    }

    #[test]
    fn test_avatar_operations_safe() {
        assert_eq!(avatar_status(), 0);
    }

    #[test]
    fn test_user_info_safe() {
        assert_eq!(user_info(), 0);
    }

    #[test]
    fn test_user_dispatch_unknown() {
        assert_eq!(user("unknown_subcommand", None, None), 1);
    }
}
