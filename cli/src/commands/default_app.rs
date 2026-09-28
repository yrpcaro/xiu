//! Default application configuration and desktop entry probe.
//!
//! Synchronizes default applications selected in the shell with ~/.config/xiu/vars.lua
//! and reloads Hyprland. Replaces set-default-app.py.

use crate::helpers::home_path;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const VAR_MAP: &[(&str, &str)] = &[
    ("x-scheme-handler/http", "browser"),
    ("x-scheme-handler/terminal", "terminal"),
    ("inode/directory", "fileManager"),
    ("audio/mpeg", "musicPlayer"),
    ("video/mp4", "videoPlayer"),
    ("image/png", "imageViewer"),
    ("application/pdf", "documentViewer"),
];

pub const WELL_KNOWN: &[(&str, &str)] = &[
    ("org.kde.dolphin.desktop", "dolphin"),
    ("thunar.desktop", "thunar"),
    ("xiu-yazi.desktop", "yazi"),
    ("nautilus.desktop", "nautilus"),
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
    ("brave-browser.desktop", "brave"),
    ("firefox.desktop", "firefox"),
    ("chromium.desktop", "chromium"),
    ("google-chrome.desktop", "google-chrome-stable"),
    ("zen.desktop", "zen-browser"),
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

    let var_name = VAR_MAP.iter().find(|(k, _)| *k == cat).map(|(_, v)| *v);
    let Some(var_name) = var_name else {
        return 0;
    };

    let cmd = resolve_cmd(did);
    if let Err(e) = update_vars_file(var_name, &cmd, None) {
        eprintln!("xiu default-app: failed to update vars.lua: {e}");
        return 1;
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
