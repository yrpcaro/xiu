//! Default application configuration and desktop entry probe.
//!
//! Synchronizes default applications across the entire system:
//! - vars.lua (Hyprland application variables)
//! - mimeapps.list (XDG default applications and MIME associations)
//! - xdg-desktop-portal (portal FileChooser routing to yazi/termfilechooser, dolphin/kde, or gtk)
//! - environment variables (uwsm/env, environment.d, and fish universal variables)
//! - KDE (kdeglobals and kwriteconfig6)
//! - GNOME (gio and gsettings)

use crate::helpers::{config_file, home_path, on_path};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)]
pub const VAR_MAP: &[(&str, &str)] = &[
    ("x-scheme-handler/http", "browser"),
    ("x-scheme-handler/terminal", "terminal"),
    ("text/plain", "editor"),
    ("inode/directory", "fileManager"),
    ("audio/mpeg", "musicPlayer"),
    ("video/mp4", "videoPlayer"),
    ("image/png", "imageViewer"),
    ("application/pdf", "documentViewer"),
];

pub const MIMES_BROWSER: &[&str] = &[
    "x-scheme-handler/http",
    "x-scheme-handler/https",
    "text/html",
    "application/xhtml+xml",
    "x-scheme-handler/about",
    "x-scheme-handler/unknown",
];

pub const MIMES_TERMINAL: &[&str] = &[
    "x-scheme-handler/terminal",
];

pub const MIMES_EDITOR: &[&str] = &[
    "text/plain",
    "text/markdown",
    "text/x-c",
    "text/x-c++",
    "text/x-python",
    "text/x-rust",
    "application/json",
];

pub const MIMES_FILE_MANAGER: &[&str] = &[
    "inode/directory",
];

pub const MIMES_IMAGE_VIEWER: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/avif",
    "image/bmp",
    "image/tiff",
    "image/svg+xml",
];

pub const MIMES_DOCUMENT_VIEWER: &[&str] = &[
    "application/pdf",
    "application/epub+zip",
    "application/oxps",
    "application/x-fictionbook",
    "application/x-mobipocket-ebook",
];

pub const MIMES_MUSIC_PLAYER: &[&str] = &[
    "audio/mpeg",
    "audio/flac",
    "audio/ogg",
    "audio/mp4",
    "audio/x-wav",
    "audio/wav",
    "audio/aac",
];

pub const MIMES_VIDEO_PLAYER: &[&str] = &[
    "video/mp4",
    "video/x-matroska",
    "video/webm",
    "video/quicktime",
    "video/x-msvideo",
    "video/mkv",
];

#[derive(Debug, Clone, Copy)]
pub struct CategoryInfo {
    pub canonical: &'static str,
    pub var_name: &'static str,
    pub mimes: &'static [&'static str],
}

pub fn lookup_category(cat: &str) -> Option<CategoryInfo> {
    match cat.trim().to_lowercase().as_str() {
        "x-scheme-handler/http" | "x-scheme-handler/https" | "browser" | "web" => Some(CategoryInfo {
            canonical: "browser",
            var_name: "browser",
            mimes: MIMES_BROWSER,
        }),
        "x-scheme-handler/terminal" | "terminal" | "term" => Some(CategoryInfo {
            canonical: "terminal",
            var_name: "terminal",
            mimes: MIMES_TERMINAL,
        }),
        "text/plain" | "editor" | "text" => Some(CategoryInfo {
            canonical: "editor",
            var_name: "editor",
            mimes: MIMES_EDITOR,
        }),
        "inode/directory" | "filemanager" | "file_manager" | "files" => Some(CategoryInfo {
            canonical: "fileManager",
            var_name: "fileManager",
            mimes: MIMES_FILE_MANAGER,
        }),
        "image/png" | "image" | "images" | "imageviewer" | "image_viewer" => Some(CategoryInfo {
            canonical: "imageViewer",
            var_name: "imageViewer",
            mimes: MIMES_IMAGE_VIEWER,
        }),
        "application/pdf" | "pdf" | "document" | "documents" | "documentviewer" | "document_viewer" => Some(CategoryInfo {
            canonical: "documentViewer",
            var_name: "documentViewer",
            mimes: MIMES_DOCUMENT_VIEWER,
        }),
        "audio/mpeg" | "audio" | "music" | "musicplayer" | "music_player" => Some(CategoryInfo {
            canonical: "musicPlayer",
            var_name: "musicPlayer",
            mimes: MIMES_MUSIC_PLAYER,
        }),
        "video/mp4" | "video" | "videoplayer" | "video_player" => Some(CategoryInfo {
            canonical: "videoPlayer",
            var_name: "videoPlayer",
            mimes: MIMES_VIDEO_PLAYER,
        }),
        _ => None,
    }
}

pub const WELL_KNOWN: &[(&str, &str)] = &[
    ("org.kde.dolphin.desktop", "dolphin"),
    ("thunar.desktop", "thunar"),
    ("xiu-yazi.desktop", "yazi"),
    ("nautilus.desktop", "nautilus"),
    ("org.gnome.Nautilus.desktop", "nautilus"),
    ("foot.desktop", "foot"),
    ("footclient.desktop", "footclient"),
    ("org.kde.konsole.desktop", "konsole"),
    ("com.mitchellh.ghostty.desktop", "ghostty"),
    ("ghostty.desktop", "ghostty"),
    ("kitty.desktop", "kitty"),
    ("alacritty.desktop", "alacritty"),
    ("Alacritty.desktop", "alacritty"),
    ("wezterm.desktop", "wezterm"),
    ("org.wezfurlong.wezterm.desktop", "wezterm"),
    ("code.desktop", "code"),
    ("visual-studio-code.desktop", "code"),
    ("code-oss.desktop", "code-oss"),
    ("dev.zed.Zed.desktop", "zeditor"),
    ("zed.desktop", "zeditor"),
    ("helix.desktop", "helix"),
    ("Helix.desktop", "helix"),
    ("nvim.desktop", "nvim"),
    ("neovim.desktop", "nvim"),
    ("micro.desktop", "micro"),
    ("nano.desktop", "nano"),
    ("org.gnome.TextEditor.desktop", "gnome-text-editor"),
    ("gedit.desktop", "gedit"),
    ("kate.desktop", "kate"),
    ("org.kde.kate.desktop", "kate"),
    ("brave-browser.desktop", "brave"),
    ("brave.desktop", "brave"),
    ("firefox.desktop", "firefox"),
    ("chromium.desktop", "chromium"),
    ("google-chrome.desktop", "google-chrome-stable"),
    ("zen.desktop", "zen-browser"),
    ("zen-browser.desktop", "zen-browser"),
    ("zen-browser-bin.desktop", "zen-browser"),
    ("spotify.desktop", "spotify"),
    ("spotify-launcher.desktop", "spotify-launcher"),
    ("amberol.desktop", "amberol"),
    ("io.bassi.Amberol.desktop", "amberol"),
    ("org.gnome.Lollypop.desktop", "lollypop"),
    ("rhythmbox.desktop", "rhythmbox"),
    ("clementine.desktop", "clementine"),
    ("imv.desktop", "imv"),
    ("imv-dir.desktop", "imv-dir"),
    ("feh.desktop", "feh"),
    ("org.gnome.Loupe.desktop", "loupe"),
    ("org.kde.gwenview.desktop", "gwenview"),
    ("viewnior.desktop", "viewnior"),
    ("mpv.desktop", "mpv"),
    ("vlc.desktop", "vlc"),
    ("org.gnome.Totem.desktop", "totem"),
    ("io.github.celluloid_player.Celluloid.desktop", "celluloid"),
    ("org.pwmt.zathura.desktop", "zathura"),
    ("org.pwmt.zathura-pdf-mupdf.desktop", "zathura"),
    ("org.gnome.Papers.desktop", "papers"),
    ("org.kde.okular.desktop", "okular"),
    ("evince.desktop", "evince"),
];

pub fn resolve_cmd(desktop_id: &str) -> String {
    for (id, cmd) in WELL_KNOWN {
        if *id == desktop_id {
            return cmd.to_string();
        }
    }

    let candidate_dirs = [
        home_path(&[".local", "share", "applications"]),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/usr/share/applications"),
    ];

    for app_dir in &candidate_dirs {
        let f = app_dir.join(desktop_id);
        if f.is_file() {
            if let Ok(content) = fs::read_to_string(&f) {
                for line in content.lines() {
                    if let Some(rest) = line.strip_prefix("Exec=") {
                        let trimmed = rest.trim();
                        if let Some(first_word) = trimmed.split_whitespace().next() {
                            let p = Path::new(first_word);
                            if let Some(file_name) = p.file_name() {
                                return file_name.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    desktop_id.strip_suffix(".desktop").unwrap_or(desktop_id).to_string()
}

pub fn update_vars_file(var_name: &str, cmd: &str, path: Option<&Path>) -> Result<(), std::io::Error> {
    let p = match path {
        Some(custom) => custom.to_path_buf(),
        None => {
            let cfg = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_path(&[".config"]));
            cfg.join("xiu").join("vars.lua")
        }
    };

    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }

    if !p.is_file() {
        let content = format!(
            "-- xiu personalization overrides\nreturn {{\n    {var_name} = \"{cmd}\",\n}}\n"
        );
        return fs::write(&p, content);
    }

    let text = fs::read_to_string(&p)?;
    let mut replaced = false;
    let mut new_lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(var_name) {
            let rest = trimmed[var_name.len()..].trim_start();
            if rest.starts_with('=') {
                let indent_len = line.len() - trimmed.len();
                let indent = &line[..indent_len];
                new_lines.push(format!("{indent}{var_name} = \"{cmd}\","));
                replaced = true;
                continue;
            }
        }
        new_lines.push(line.to_string());
    }

    let mut new_text = new_lines.join("\n");
    if text.ends_with('\n') {
        new_text.push('\n');
    }

    if !replaced {
        if let Some(idx) = new_text.rfind('}') {
            let (before, after) = new_text.split_at(idx);
            let insertion = format!("    {var_name} = \"{cmd}\",\n");
            new_text = format!("{before}{insertion}{after}");
        } else {
            new_text.push_str(&format!("\n    {var_name} = \"{cmd}\",\n"));
        }
    }

    fs::write(&p, new_text)
}

pub fn update_mimeapps_file(
    mimes: &[&str],
    desktop_id: &str,
    path: Option<&Path>,
) -> Result<(), std::io::Error> {
    let p = match path {
        Some(custom) => custom.to_path_buf(),
        None => config_file(&["mimeapps.list"]),
    };

    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = if p.is_file() {
        fs::read_to_string(&p)?
    } else {
        String::new()
    };

    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_section: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let name = trimmed[1..trimmed.len() - 1].to_string();
            current_section = Some(name.clone());
            sections.push((name, Vec::new()));
        } else if let Some(sec_name) = &current_section {
            if let Some((_, lines)) = sections.iter_mut().find(|(s, _)| s == sec_name) {
                lines.push(line.to_string());
            }
        }
    }

    let default_apps_idx = sections.iter().position(|(s, _)| s == "Default Applications");
    let default_apps_idx = match default_apps_idx {
        Some(idx) => idx,
        None => {
            sections.push(("Default Applications".to_string(), Vec::new()));
            sections.len() - 1
        }
    };

    let (_, default_lines) = &mut sections[default_apps_idx];
    let mut remaining_mimes: HashSet<&str> = mimes.iter().copied().collect();

    for line in default_lines.iter_mut() {
        let key_match = {
            let trimmed = line.trim();
            trimmed.split_once('=').map(|(k, _)| k.trim().to_string())
        };
        if let Some(key) = key_match {
            if remaining_mimes.contains(key.as_str()) {
                *line = format!("{key}={desktop_id}");
                remaining_mimes.remove(key.as_str());
            }
        }
    }

    for mime in remaining_mimes {
        default_lines.push(format!("{mime}={desktop_id}"));
    }

    let mut out = String::new();
    for (sec, lines) in &sections {
        out.push_str(&format!("[{sec}]\n"));
        for l in lines {
            out.push_str(l);
            out.push('\n');
        }
    }

    fs::write(&p, out)?;

    if path.is_none() {
        for mime in mimes {
            let _ = Command::new("xdg-mime").args(["default", desktop_id, mime]).status();
            let _ = Command::new("gio").args(["mime", mime, desktop_id]).status();
        }
    }

    Ok(())
}

pub fn update_key_value_file(path: &Path, key: &str, value: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = if path.is_file() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    let mut replaced = false;
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(key) {
            let rest = trimmed[key.len()..].trim_start();
            if rest.starts_with('=') {
                lines.push(format!("{key}={value}"));
                replaced = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }

    if !replaced {
        lines.push(format!("{key}={value}"));
    }

    let mut out = lines.join("\n");
    out.push('\n');
    fs::write(path, out)
}

pub fn update_kdeglobals_file(
    group: &str,
    key: &str,
    value: &str,
    path: Option<&Path>,
) -> Result<(), std::io::Error> {
    let p = match path {
        Some(custom) => custom.to_path_buf(),
        None => config_file(&["kdeglobals"]),
    };

    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = if p.is_file() {
        fs::read_to_string(&p)?
    } else {
        String::new()
    };

    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_section: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let name = trimmed[1..trimmed.len() - 1].to_string();
            current_section = Some(name.clone());
            sections.push((name, Vec::new()));
        } else if let Some(sec_name) = &current_section {
            if let Some((_, lines)) = sections.iter_mut().find(|(s, _)| s == sec_name) {
                lines.push(line.to_string());
            }
        }
    }

    let group_idx = sections.iter().position(|(s, _)| s == group);
    let group_idx = match group_idx {
        Some(idx) => idx,
        None => {
            sections.push((group.to_string(), Vec::new()));
            sections.len() - 1
        }
    };

    let (_, lines) = &mut sections[group_idx];
    let mut found = false;

    for line in lines.iter_mut() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(key) {
            let rest = trimmed[key.len()..].trim_start();
            if rest.starts_with('=') {
                *line = format!("{key}={value}");
                found = true;
                break;
            }
        }
    }

    if !found {
        lines.push(format!("{key}={value}"));
    }

    let mut out = String::new();
    for (sec, lines) in &sections {
        out.push_str(&format!("[{sec}]\n"));
        for l in lines {
            out.push_str(l);
            out.push('\n');
        }
    }

    fs::write(&p, out)?;

    if path.is_none() {
        let _ = Command::new("kwriteconfig6")
            .args(["--file", "kdeglobals", "--group", group, "--key", key, value])
            .status();
        let _ = Command::new("kwriteconfig5")
            .args(["--file", "kdeglobals", "--group", group, "--key", key, value])
            .status();
    }

    Ok(())
}

pub fn update_portal_config_file(backend: &str, path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = if path.is_file() {
        fs::read_to_string(path)?
    } else {
        "[preferred]\ndefault=gtk\norg.freedesktop.impl.portal.ScreenCast=hyprland\norg.freedesktop.impl.portal.Screenshot=hyprland\norg.freedesktop.impl.portal.Secret=gnome-keyring\n".to_string()
    };

    let target_key = "org.freedesktop.impl.portal.FileChooser";
    let target_line = format!("{target_key}={backend}");

    let mut lines = Vec::new();
    let mut in_preferred = false;
    let mut replaced = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[preferred]" {
            in_preferred = true;
            lines.push(line.to_string());
            continue;
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_preferred = false;
        }

        if in_preferred && trimmed.starts_with(target_key) {
            lines.push(target_line.clone());
            replaced = true;
            continue;
        }

        lines.push(line.to_string());
    }

    if !replaced {
        let mut new_lines = Vec::new();
        let mut inserted = false;
        for line in &lines {
            new_lines.push(line.clone());
            if line.trim() == "[preferred]" && !inserted {
                new_lines.push(target_line.clone());
                inserted = true;
            }
        }
        if !inserted {
            new_lines.push("[preferred]".to_string());
            new_lines.push(target_line.clone());
        }
        lines = new_lines;
    }

    let mut out = lines.join("\n");
    out.push('\n');
    fs::write(path, out)
}

pub fn update_portal_file_chooser(
    file_manager_cmd: &str,
    desktop_id: &str,
    custom_portal_dir: Option<&Path>,
) -> Result<(), std::io::Error> {
    let backend = if desktop_id == "xiu-yazi.desktop" || file_manager_cmd == "yazi" {
        "termfilechooser;gtk"
    } else if desktop_id == "org.kde.dolphin.desktop" || file_manager_cmd == "dolphin" {
        "kde;gtk"
    } else {
        "gtk"
    };

    let pconfs = match custom_portal_dir {
        Some(dir) => vec![dir.join("portals.conf"), dir.join("hyprland-portals.conf")],
        None => vec![
            config_file(&["xdg-desktop-portal", "portals.conf"]),
            config_file(&["xdg-desktop-portal", "hyprland-portals.conf"]),
        ],
    };

    for pconf in pconfs {
        update_portal_config_file(backend, &pconf)?;
    }

    if backend.starts_with("termfilechooser") {
        let term = resolve_active_terminal();
        update_termfilechooser_configs(&term, custom_portal_dir)?;
        ensure_xiu_yazi_desktop(custom_portal_dir)?;
    }

    if custom_portal_dir.is_none() {
        let _ = Command::new("systemctl")
            .args(["--user", "restart", "xdg-desktop-portal"])
            .status();
    }

    Ok(())
}

pub fn update_termfilechooser_configs(
    term: &str,
    custom_dir: Option<&Path>,
) -> Result<(), std::io::Error> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let cfg_dir = match custom_dir {
        Some(dir) => dir.join("xdg-desktop-portal-termfilechooser"),
        None => config_file(&["xdg-desktop-portal-termfilechooser"]),
    };

    fs::create_dir_all(&cfg_dir)?;
    let content = format!(
        "[filechooser]\ncmd=xiu yazi-chooser\ndefault_dir={home}\nenv=TERMCMD={term}\n"
    );

    fs::write(cfg_dir.join("config.toml"), &content)?;
    fs::write(cfg_dir.join("config"), &content)?;
    Ok(())
}

pub fn ensure_xiu_yazi_desktop(custom_dir: Option<&Path>) -> Result<(), std::io::Error> {
    let dest = match custom_dir {
        Some(dir) => dir.join("applications").join("xiu-yazi.desktop"),
        None => home_path(&[".local", "share", "applications", "xiu-yazi.desktop"]),
    };

    if dest.is_file() {
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = "[Desktop Entry]\nType=Application\nName=Yazi (xiu)\nIcon=yazi\nComment=The file manager in foot, xiu's terminal\nTryExec=foot\nExec=foot -e yazi %f\nMimeType=inode/directory;\nCategories=System;FileManager;FileTools;\nKeywords=File;Manager;Explorer;\n";
    fs::write(dest, content)?;
    Ok(())
}

pub fn resolve_active_terminal() -> String {
    let p = config_file(&["xiu", "vars.lua"]);
    if let Ok(content) = fs::read_to_string(&p) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("terminal") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    let cleaned = val.trim().trim_matches(|c| c == '"' || c == ',' || c == '\'');
                    if !cleaned.is_empty() {
                        return cleaned.to_string();
                    }
                }
            }
        }
    }
    if let Ok(t) = std::env::var("TERMCMD") {
        if !t.trim().is_empty() {
            return t.trim().to_string();
        }
    }
    for t in ["foot", "ghostty", "kitty", "alacritty"] {
        if on_path(t) {
            return t.to_string();
        }
    }
    "foot".to_string()
}

pub fn update_fish_editor(cmd: &str, custom_fish_dir: Option<&Path>) -> Result<(), std::io::Error> {
    let conf_d = match custom_fish_dir {
        Some(dir) => dir.join("fish").join("conf.d"),
        None => config_file(&["fish", "conf.d"]),
    };

    fs::create_dir_all(&conf_d)?;
    let p = conf_d.join("00-editor.fish");
    let content = format!(
        "# Configured by xiu default-app\nset -gx EDITOR {cmd}\nset -gx VISUAL {cmd}\nif not type -q editor; and type -q {cmd}\n    alias editor {cmd}\nend\n"
    );
    fs::write(p, content)?;

    if custom_fish_dir.is_none() {
        let _ = Command::new("fish")
            .args(["-c", &format!("set -Ux EDITOR {cmd}; set -Ux VISUAL {cmd}")])
            .status();
    }

    Ok(())
}

pub fn probe_desktop_entries(dirs: Option<&[PathBuf]>) -> Vec<String> {
    let default_dirs = [
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        home_path(&[".local", "share", "applications"]),
    ];
    let search_dirs = dirs.unwrap_or(&default_dirs);
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }
        let mut items = Vec::new();
        if let Ok(read_dir) = fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("desktop") {
                    items.push(p);
                }
            }
        }
        items.sort();

        for file_path in items {
            let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if seen.contains(file_name) || file_name == "foot-server.desktop" {
                continue;
            }
            seen.insert(file_name.to_string());

            let Ok(content) = fs::read_to_string(&file_path) else {
                continue;
            };

            let mut mimes = String::new();
            let mut is_term = false;
            let mut no_disp = false;

            for line in content.lines() {
                let line_trimmed = line.trim();
                if let Some(rest) = line_trimmed.strip_prefix("MimeType=") {
                    mimes = rest.to_string();
                } else if let Some(rest) = line_trimmed.strip_prefix("NoDisplay=") {
                    if rest.trim().eq_ignore_ascii_case("true") {
                        no_disp = true;
                    }
                } else if (line_trimmed.contains("TerminalEmulator")
                    || line_trimmed.contains("x-scheme-handler/terminal"))
                    && !line_trimmed.contains("Terminal=true")
                {
                    is_term = true;
                }
            }

            let in_well_known = WELL_KNOWN.iter().any(|(k, _)| *k == file_name);
            if no_disp && mimes.is_empty() && !is_term && !in_well_known {
                continue;
            }

            let term = if is_term { " terminal" } else { "" };
            entries.push(format!("{file_name}: {mimes}{term}"));
        }
    }

    entries
}

pub fn default_app(probe: bool, category: Option<&str>, desktop_id: Option<&str>) -> i32 {
    let is_probe = probe
        || matches!(
            category,
            Some("--probe") | Some("-p") | Some("probe")
        );

    if is_probe {
        for line in probe_desktop_entries(None) {
            println!("{line}");
        }
        return 0;
    }

    let (Some(cat), Some(did)) = (category, desktop_id) else {
        eprintln!("xiu default-app: usage: xiu default-app [--probe] [category] [desktop_id]");
        return 1;
    };

    let cat_info = lookup_category(cat);
    let Some(info) = cat_info else {
        return 0;
    };

    let cmd = resolve_cmd(did);
    if let Err(e) = update_vars_file(info.var_name, &cmd, None) {
        eprintln!("xiu default-app: failed to update vars.lua: {e}");
        return 1;
    }

    // 1. Update MIME handlers in ~/.config/mimeapps.list
    let _ = update_mimeapps_file(info.mimes, did, None);

    // 2. Category-specific settings, portal, and environment propagation
    match info.canonical {
        "editor" => {
            let uwsm_env = config_file(&["uwsm", "env"]);
            let _ = update_key_value_file(&uwsm_env, "EDITOR", &cmd);
            let _ = update_key_value_file(&uwsm_env, "VISUAL", &cmd);

            let env_d = config_file(&["environment.d", "10-editor.conf"]);
            let _ = update_key_value_file(&env_d, "EDITOR", &cmd);
            let _ = update_key_value_file(&env_d, "VISUAL", &cmd);

            let _ = update_fish_editor(&cmd, None);
            let _ = update_kdeglobals_file("General", "Editor", &cmd, None);
        }
        "fileManager" => {
            let _ = update_portal_file_chooser(&cmd, did, None);
            let _ = update_kdeglobals_file("General", "FileManager", &cmd, None);
        }
        "terminal" => {
            let uwsm_env = config_file(&["uwsm", "env"]);
            let _ = update_key_value_file(&uwsm_env, "TERMINAL", &cmd);

            let env_d = config_file(&["environment.d", "10-default-apps.conf"]);
            let _ = update_key_value_file(&env_d, "TERMINAL", &cmd);

            let _ = update_termfilechooser_configs(&cmd, None);

            let _ = update_kdeglobals_file("General", "TerminalApplication", &cmd, None);
            let _ = update_kdeglobals_file("General", "TerminalService", did, None);

            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.desktop.default-applications.terminal", "exec", &cmd])
                .status();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.desktop.default-applications.terminal", "exec-arg", "-e"])
                .status();
        }
        "browser" => {
            let uwsm_env = config_file(&["uwsm", "env"]);
            let _ = update_key_value_file(&uwsm_env, "BROWSER", &cmd);

            let env_d = config_file(&["environment.d", "10-default-apps.conf"]);
            let _ = update_key_value_file(&env_d, "BROWSER", &cmd);

            let _ = update_kdeglobals_file("General", "BrowserApplication", did, None);
        }
        _ => {}
    }

    let _ = Command::new("hyprctl").arg("reload").status();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_well_known_resolutions() {
        assert_eq!(resolve_cmd("xiu-yazi.desktop"), "yazi");
        assert_eq!(resolve_cmd("org.kde.dolphin.desktop"), "dolphin");
        assert_eq!(resolve_cmd("foot.desktop"), "foot");
        assert_eq!(resolve_cmd("footclient.desktop"), "footclient");
        assert_eq!(resolve_cmd("org.kde.konsole.desktop"), "konsole");
        assert_eq!(resolve_cmd("com.mitchellh.ghostty.desktop"), "ghostty");
        assert_eq!(resolve_cmd("ghostty.desktop"), "ghostty");
        assert_eq!(resolve_cmd("alacritty.desktop"), "alacritty");
        assert_eq!(resolve_cmd("Alacritty.desktop"), "alacritty");
        assert_eq!(resolve_cmd("wezterm.desktop"), "wezterm");
        assert_eq!(resolve_cmd("org.wezfurlong.wezterm.desktop"), "wezterm");
        assert_eq!(resolve_cmd("brave-browser.desktop"), "brave");
        assert_eq!(resolve_cmd("spotify.desktop"), "spotify");
        assert_eq!(resolve_cmd("spotify-launcher.desktop"), "spotify-launcher");
        assert_eq!(resolve_cmd("imv.desktop"), "imv");
        assert_eq!(resolve_cmd("mpv.desktop"), "mpv");
        assert_eq!(resolve_cmd("dev.zed.Zed.desktop"), "zeditor");
        assert_eq!(resolve_cmd("code.desktop"), "code");
        assert_eq!(resolve_cmd("helix.desktop"), "helix");
        assert_eq!(resolve_cmd("Helix.desktop"), "helix");
        assert_eq!(resolve_cmd("nvim.desktop"), "nvim");
    }

    #[test]
    fn test_vars_file_lifecycle() {
        let tmp_dir = tempfile();
        let p = tmp_dir.join("vars.lua");

        // 1. Fresh file creation
        update_vars_file("browser", "firefox", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("browser = \"firefox\""), "content: {content}");

        // 2. Key update in existing file
        update_vars_file("browser", "brave", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("browser = \"brave\""), "content: {content}");
        assert!(!content.contains("browser = \"firefox\""), "content: {content}");

        // 3. Add second key
        update_vars_file("terminal", "ghostty", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("browser = \"brave\""), "content: {content}");
        assert!(content.contains("terminal = \"ghostty\""), "content: {content}");

        // 4. Add editor key
        update_vars_file("editor", "zeditor", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("editor = \"zeditor\""), "content: {content}");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_lookup_category() {
        assert_eq!(lookup_category("x-scheme-handler/http").unwrap().canonical, "browser");
        assert_eq!(lookup_category("browser").unwrap().canonical, "browser");
        assert_eq!(lookup_category("text/plain").unwrap().canonical, "editor");
        assert_eq!(lookup_category("editor").unwrap().canonical, "editor");
        assert_eq!(lookup_category("inode/directory").unwrap().canonical, "fileManager");
        assert_eq!(lookup_category("file_manager").unwrap().canonical, "fileManager");
        assert_eq!(lookup_category("x-scheme-handler/terminal").unwrap().canonical, "terminal");
        assert_eq!(lookup_category("image/png").unwrap().canonical, "imageViewer");
        assert_eq!(lookup_category("application/pdf").unwrap().canonical, "documentViewer");
        assert_eq!(lookup_category("audio/mpeg").unwrap().canonical, "musicPlayer");
        assert_eq!(lookup_category("video/mp4").unwrap().canonical, "videoPlayer");
    }

    #[test]
    fn test_mimeapps_update() {
        let tmp_dir = tempfile();
        let p = tmp_dir.join("mimeapps.list");

        // Fresh file
        update_mimeapps_file(MIMES_EDITOR, "Helix.desktop", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("[Default Applications]"), "missing header: {content}");
        assert!(content.contains("text/plain=Helix.desktop"), "missing text/plain: {content}");
        assert!(content.contains("application/json=Helix.desktop"), "missing json: {content}");

        // Update to nvim
        update_mimeapps_file(MIMES_EDITOR, "nvim.desktop", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("text/plain=nvim.desktop"), "updated text/plain: {content}");
        assert!(!content.contains("Helix.desktop"), "stale Helix in: {content}");

        // Add browser mimes
        update_mimeapps_file(MIMES_BROWSER, "firefox.desktop", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("text/plain=nvim.desktop"), "nvim missing after browser: {content}");
        assert!(content.contains("x-scheme-handler/http=firefox.desktop"), "http missing: {content}");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_key_value_file_update() {
        let tmp_dir = tempfile();
        let p = tmp_dir.join("env");

        update_key_value_file(&p, "EDITOR", "helix").unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert_eq!(content.trim(), "EDITOR=helix");

        update_key_value_file(&p, "VISUAL", "helix").unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("EDITOR=helix"));
        assert!(content.contains("VISUAL=helix"));

        update_key_value_file(&p, "EDITOR", "nvim").unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("EDITOR=nvim"));
        assert!(!content.contains("EDITOR=helix"));
        assert!(content.contains("VISUAL=helix"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_kdeglobals_update() {
        let tmp_dir = tempfile();
        let p = tmp_dir.join("kdeglobals");

        update_kdeglobals_file("General", "BrowserApplication", "firefox.desktop", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("[General]"));
        assert!(content.contains("BrowserApplication=firefox.desktop"));

        update_kdeglobals_file("General", "TerminalApplication", "ghostty", Some(&p)).unwrap();
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("BrowserApplication=firefox.desktop"));
        assert!(content.contains("TerminalApplication=ghostty"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_portal_file_chooser_update() {
        let tmp_dir = tempfile();

        // 1. yazi selection -> termfilechooser;gtk
        update_portal_file_chooser("yazi", "xiu-yazi.desktop", Some(&tmp_dir)).unwrap();
        let pconf = fs::read_to_string(tmp_dir.join("portals.conf")).unwrap();
        assert!(pconf.contains("org.freedesktop.impl.portal.FileChooser=termfilechooser;gtk"), "pconf: {pconf}");
        let hconf = fs::read_to_string(tmp_dir.join("hyprland-portals.conf")).unwrap();
        assert!(hconf.contains("org.freedesktop.impl.portal.FileChooser=termfilechooser;gtk"), "hconf: {hconf}");

        let tf_cfg = fs::read_to_string(tmp_dir.join("xdg-desktop-portal-termfilechooser").join("config.toml")).unwrap();
        assert!(tf_cfg.contains("cmd=xiu yazi-chooser"), "tf_cfg: {tf_cfg}");
        assert!(tf_cfg.contains("env=TERMCMD="), "tf_cfg: {tf_cfg}");

        // 2. dolphin selection -> kde;gtk
        update_portal_file_chooser("dolphin", "org.kde.dolphin.desktop", Some(&tmp_dir)).unwrap();
        let pconf = fs::read_to_string(tmp_dir.join("portals.conf")).unwrap();
        assert!(pconf.contains("org.freedesktop.impl.portal.FileChooser=kde;gtk"), "pconf: {pconf}");
        assert!(!pconf.contains("termfilechooser"), "pconf still has termfilechooser: {pconf}");

        // 3. thunar selection -> gtk
        update_portal_file_chooser("thunar", "thunar.desktop", Some(&tmp_dir)).unwrap();
        let pconf = fs::read_to_string(tmp_dir.join("portals.conf")).unwrap();
        assert!(pconf.contains("org.freedesktop.impl.portal.FileChooser=gtk"), "pconf: {pconf}");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_fish_editor_update() {
        let tmp_dir = tempfile();
        update_fish_editor("nvim", Some(&tmp_dir)).unwrap();
        let fish_file = tmp_dir.join("fish").join("conf.d").join("00-editor.fish");
        assert!(fish_file.is_file());
        let content = fs::read_to_string(fish_file).unwrap();
        assert!(content.contains("set -gx EDITOR nvim"));
        assert!(content.contains("set -gx VISUAL nvim"));
        assert!(content.contains("alias editor nvim"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_desktop_entries_probe() {
        let tmp_dir = tempfile();
        let app_dir = tmp_dir.join("applications");
        fs::create_dir_all(&app_dir).unwrap();

        fs::write(
            app_dir.join("imv.desktop"),
            "[Desktop Entry]\nName=imv\nNoDisplay=true\nMimeType=image/png;image/jpeg;\n",
        )
        .unwrap();

        fs::write(
            app_dir.join("background-daemon.desktop"),
            "[Desktop Entry]\nName=daemon\nNoDisplay=true\n",
        )
        .unwrap();

        fs::write(
            app_dir.join("terminal.desktop"),
            "[Desktop Entry]\nName=terminal\nCategories=System;TerminalEmulator;\n",
        )
        .unwrap();

        let probed = probe_desktop_entries(Some(&[app_dir]));
        assert!(
            probed.iter().any(|l| l.contains("imv.desktop") && l.contains("image/png")),
            "imv missing from probed: {probed:?}"
        );
        assert!(
            !probed.iter().any(|l| l.contains("background-daemon.desktop")),
            "daemon incorrectly probed: {probed:?}"
        );
        assert!(
            probed.iter().any(|l| l.contains("terminal.desktop") && l.contains("terminal")),
            "terminal missing from probed: {probed:?}"
        );

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    fn tempfile() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("xiu-test-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&p).unwrap();
        p
    }
}
