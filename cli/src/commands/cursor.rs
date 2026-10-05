//! Cursor theme and size management and system-wide synchronization.
//! Synchronizes Hyprland, XWayland (xrdb), ~/.icons/default, ~/.local/share/icons/default,
//! GTK 3 & 4 settings, xsettingsd, and gsettings so all applications (including
//! Electron, Chromium, CEF, Spotify, and X11 apps) use the identical mouse cursor.

use crate::helpers::{config_file, home_path, on_path};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn cursor(theme: Option<&str>, size: Option<u32>) -> i32 {
    let (active_theme, active_size) = resolve_cursor(theme, size);
    sync_all(&active_theme, active_size)
}

pub fn resolve_cursor(theme: Option<&str>, size: Option<u32>) -> (String, u32) {
    if let Some(t) = theme {
        if !t.is_empty() && t != "sync" {
            let s = size.unwrap_or_else(|| read_cursor_size().unwrap_or(24));
            return (t.to_string(), s);
        }
    }
    let current_theme = read_cursor_theme().unwrap_or_else(|| "Bibata-Modern-Ice".to_string());
    let current_size = size.unwrap_or_else(|| read_cursor_size().unwrap_or(24));
    (current_theme, current_size)
}

pub fn read_cursor_theme() -> Option<String> {
    // 1. Check ~/.config/hypr/modules/env.lua
    let env_file = config_file(&["hypr", "modules", "env.lua"]);
    if env_file.is_file() {
        if let Ok(c) = fs::read_to_string(&env_file) {
            for line in c.lines() {
                let trimmed = line.trim();
                for key in &["XCURSOR_THEME", "HYPRCURSOR_THEME"] {
                    let pat = format!("hl.env(\"{key}\"");
                    if trimmed.contains(&pat) {
                        if let Some(val) = extract_env_val(trimmed) {
                            if !val.is_empty() {
                                return Some(val);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Check gsettings
    if on_path("gsettings") {
        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-theme"])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
}

pub fn read_cursor_size() -> Option<u32> {
    // 1. Check ~/.config/hypr/modules/env.lua
    let env_file = config_file(&["hypr", "modules", "env.lua"]);
    if env_file.is_file() {
        if let Ok(c) = fs::read_to_string(&env_file) {
            for line in c.lines() {
                let trimmed = line.trim();
                for key in &["XCURSOR_SIZE", "HYPRCURSOR_SIZE"] {
                    let pat = format!("hl.env(\"{key}\"");
                    if trimmed.contains(&pat) {
                        if let Some(val) = extract_env_val(trimmed) {
                            if let Ok(num) = val.parse::<u32>() {
                                return Some(num);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Check gsettings
    if on_path("gsettings") {
        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-size"])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(num) = s.parse::<u32>() {
                    return Some(num);
                }
            }
        }
    }

    None
}

fn extract_env_val(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split('"').collect();
    if parts.len() >= 4 {
        Some(parts[3].to_string())
    } else {
        None
    }
}

pub fn sync_all(theme: &str, size: u32) -> i32 {
    let size_str = size.to_string();

    // 1. Hyprland live setcursor
    if on_path("hyprctl") {
        let _ = Command::new("hyprctl")
            .args(["setcursor", theme, &size_str])
            .stderr(Stdio::null())
            .status();
    }

    // 2. ~/.icons/default/index.theme and ~/.local/share/icons/default/index.theme
    let index_theme_content = format!(
        "[Icon Theme]\nName=Default\nComment=Default Cursor Theme\nInherits={theme}\n"
    );
    let icon_targets = [
        home_path(&[".icons", "default", "index.theme"]),
        home_path(&[".local", "share", "icons", "default", "index.theme"]),
    ];
    for target in &icon_targets {
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(target, &index_theme_content);
    }

    // 3. ~/.config/gtk-3.0/settings.ini & ~/.config/gtk-4.0/settings.ini
    for ver in &["gtk-3.0", "gtk-4.0"] {
        let ini_path = config_file(&[ver, "settings.ini"]);
        update_gtk_cursor(&ini_path, theme, size);
    }

    // 4. ~/.config/xsettingsd/xsettingsd.conf & ~/.xsettingsd
    let xset_paths = [
        config_file(&["xsettingsd", "xsettingsd.conf"]),
        home_path(&[".xsettingsd"]),
    ];
    for xset in &xset_paths {
        update_xsettingsd_cursor(xset, theme, size);
    }
    if on_path("killall") {
        let _ = Command::new("killall")
            .args(["-HUP", "xsettingsd"])
            .stderr(Stdio::null())
            .status();
    }

    // 5. X resources (xrdb)
    if on_path("xrdb") {
        let xrdb_data = format!(
            "Xcursor.theme: {theme}\nXcursor.size: {size}\nXcursor.theme_core: true\n"
        );
        if let Ok(mut child) = Command::new("xrdb")
            .arg("-merge")
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(xrdb_data.as_bytes());
            }
            let _ = child.wait();
        }
    }

    // 6. GSettings
    if on_path("gsettings") {
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "cursor-theme", theme])
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "cursor-size", &size_str])
            .stderr(Stdio::null())
            .status();
    }

    // 7. Update env.lua
    let env_path = config_file(&["hypr", "modules", "env.lua"]);
    update_env_cursor(&env_path, theme, size);

    println!("✓ cursor synced: {theme} ({size}px)");
    0
}

pub fn update_gtk_cursor(ini_path: &Path, theme: &str, size: u32) {
    let mut lines = Vec::new();
    if ini_path.is_file() {
        if let Ok(c) = fs::read_to_string(ini_path) {
            lines = c.lines().map(String::from).collect();
        }
    }

    let mut in_settings = false;
    let mut settings_found = false;
    let mut has_theme = false;
    let mut has_size = false;
    let mut new_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_settings {
                if !has_theme {
                    new_lines.push(format!("gtk-cursor-theme-name={theme}"));
                }
                if !has_size {
                    new_lines.push(format!("gtk-cursor-theme-size={size}"));
                }
                in_settings = false;
            }
            if trimmed == "[Settings]" {
                in_settings = true;
                settings_found = true;
            }
            new_lines.push(line);
            continue;
        }

        if in_settings {
            if trimmed.starts_with("gtk-cursor-theme-name=") {
                new_lines.push(format!("gtk-cursor-theme-name={theme}"));
                has_theme = true;
                continue;
            }
            if trimmed.starts_with("gtk-cursor-theme-size=") {
                new_lines.push(format!("gtk-cursor-theme-size={size}"));
                has_size = true;
                continue;
            }
        }
        new_lines.push(line);
    }

    if in_settings {
        if !has_theme {
            new_lines.push(format!("gtk-cursor-theme-name={theme}"));
        }
        if !has_size {
            new_lines.push(format!("gtk-cursor-theme-size={size}"));
        }
    } else if !settings_found {
        if !new_lines.is_empty() && !new_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
            new_lines.push("".to_string());
        }
        new_lines.push("[Settings]".to_string());
        new_lines.push(format!("gtk-cursor-theme-name={theme}"));
        new_lines.push(format!("gtk-cursor-theme-size={size}"));
    }

    if let Some(parent) = ini_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(ini_path, new_lines.join("\n") + "\n");
}

pub fn update_xsettingsd_cursor(conf_path: &Path, theme: &str, size: u32) {
    let mut lines = Vec::new();
    if conf_path.is_file() {
        if let Ok(c) = fs::read_to_string(conf_path) {
            lines = c.lines().map(String::from).collect();
        }
    }

    let mut new_lines = Vec::new();
    let mut has_theme = false;
    let mut has_size = false;

    for line in lines {
        let stripped = line.trim();
        if stripped.starts_with("Gtk/CursorThemeName") {
            new_lines.push(format!("Gtk/CursorThemeName \"{theme}\""));
            has_theme = true;
        } else if stripped.starts_with("Gtk/CursorThemeSize") {
            new_lines.push(format!("Gtk/CursorThemeSize {size}"));
            has_size = true;
        } else {
            new_lines.push(line);
        }
    }

    if !has_theme {
        new_lines.push(format!("Gtk/CursorThemeName \"{theme}\""));
    }
    if !has_size {
        new_lines.push(format!("Gtk/CursorThemeSize {size}"));
    }

    if let Some(parent) = conf_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(conf_path, new_lines.join("\n") + "\n");
}

pub fn update_env_cursor(env_path: &Path, theme: &str, size: u32) {
    if !env_path.is_file() {
        return;
    }
    let Ok(c) = fs::read_to_string(env_path) else {
        return;
    };
    let mut lines: Vec<String> = c.lines().map(String::from).collect();
    let mut has_hc_theme = false;
    let mut has_hc_size = false;

    for line in lines.iter_mut() {
        let trimmed = line.trim();
        if trimmed.starts_with("hl.env(\"XCURSOR_THEME\"") {
            *line = format!("hl.env(\"XCURSOR_THEME\",    \"{theme}\")");
        } else if trimmed.starts_with("hl.env(\"XCURSOR_SIZE\"") {
            *line = format!("hl.env(\"XCURSOR_SIZE\",     \"{size}\")");
        } else if trimmed.starts_with("hl.env(\"HYPRCURSOR_THEME\"") {
            *line = format!("hl.env(\"HYPRCURSOR_THEME\", \"{theme}\")");
            has_hc_theme = true;
        } else if trimmed.starts_with("hl.env(\"HYPRCURSOR_SIZE\"") {
            *line = format!("hl.env(\"HYPRCURSOR_SIZE\",  \"{size}\")");
            has_hc_size = true;
        }
    }

    if !has_hc_theme {
        let mut idx = 0;
        for (i, l) in lines.iter().enumerate() {
            if l.contains("XCURSOR_SIZE") {
                idx = i + 1;
                break;
            }
        }
        lines.insert(idx, format!("hl.env(\"HYPRCURSOR_THEME\", \"{theme}\")"));
    }

    if !has_hc_size {
        let mut idx = 0;
        for (i, l) in lines.iter().enumerate() {
            if l.contains("HYPRCURSOR_THEME") {
                idx = i + 1;
                break;
            }
        }
        lines.insert(idx, format!("hl.env(\"HYPRCURSOR_SIZE\",  \"{size}\")"));
    }

    let _ = fs::write(env_path, lines.join("\n") + "\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_gtk_cursor() {
        let dir = std::env::temp_dir().join(format!("xiu_test_gtk_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let ini = dir.join("settings.ini");
        fs::write(&ini, "[Settings]\ngtk-theme-name=adw-gtk3-dark\n").unwrap();
        update_gtk_cursor(&ini, "Bibata-Modern-Ice", 24);
        let content = fs::read_to_string(&ini).unwrap();
        assert!(content.contains("gtk-cursor-theme-name=Bibata-Modern-Ice"));
        assert!(content.contains("gtk-cursor-theme-size=24"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_update_xsettingsd_cursor() {
        let dir = std::env::temp_dir().join(format!("xiu_test_xset_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let conf = dir.join("xsettingsd.conf");
        fs::write(&conf, "Gtk/CursorThemeName \"breeze_cursors\"\nGtk/CursorThemeSize 24\n").unwrap();
        update_xsettingsd_cursor(&conf, "Bibata-Modern-Ice", 28);
        let content = fs::read_to_string(&conf).unwrap();
        assert!(content.contains("Gtk/CursorThemeName \"Bibata-Modern-Ice\""));
        assert!(content.contains("Gtk/CursorThemeSize 28"));
        let _ = fs::remove_dir_all(&dir);
    }
}
