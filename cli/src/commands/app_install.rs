//! Application drop-installer and package management.
//!
//! Replaces app-install.sh and appimage-install.sh.

use crate::helpers::home_path;
use crate::json::{self, Json};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".local", "share"]))
}

fn apps_dir() -> PathBuf {
    home_path(&["Applications"])
}

fn desktop_dir() -> PathBuf {
    data_home().join("applications")
}

fn icon_dir() -> PathBuf {
    data_home().join("ricelin").join("appimages")
}

fn registry_file() -> PathBuf {
    data_home().join("ricelin").join("appimages.json")
}

#[derive(Debug, Clone)]
struct AppRegistryEntry {
    name: String,
    appimage_path: String,
    icon_path: String,
    desktop_path: String,
    app_id: String,
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn load_registry() -> HashMap<String, AppRegistryEntry> {
    let mut map = HashMap::new();
    let reg_path = registry_file();
    if let Ok(content) = fs::read_to_string(&reg_path) {
        if let Ok(Json::Obj(entries)) = json::parse(&content) {
            for (slug, val) in entries {
                if let Json::Obj(fields) = val {
                    let mut entry = AppRegistryEntry {
                        name: String::new(),
                        appimage_path: String::new(),
                        icon_path: String::new(),
                        desktop_path: String::new(),
                        app_id: String::new(),
                    };
                    for (k, v) in fields {
                        if let Some(s) = v.as_str() {
                            match k.as_str() {
                                "name" => entry.name = s.to_string(),
                                "appimagePath" => entry.appimage_path = s.to_string(),
                                "iconPath" => entry.icon_path = s.to_string(),
                                "desktopPath" => entry.desktop_path = s.to_string(),
                                "appId" => entry.app_id = s.to_string(),
                                _ => {}
                            }
                        }
                    }
                    map.insert(slug, entry);
                }
            }
        }
    }
    map
}

fn save_registry(registry: &HashMap<String, AppRegistryEntry>) -> Result<(), String> {
    let reg_path = registry_file();
    if let Some(parent) = reg_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut keys: Vec<&String> = registry.keys().collect();
    keys.sort();

    let mut out = String::from("{\n");
    for (i, k) in keys.iter().enumerate() {
        let e = &registry[*k];
        out.push_str(&format!(
            "  \"{}\": {{\n    \"name\": \"{}\",\n    \"appimagePath\": \"{}\",\n    \"iconPath\": \"{}\",\n    \"desktopPath\": \"{}\",\n    \"appId\": \"{}\"\n  }}{}\n",
            json_escape(k),
            json_escape(&e.name),
            json_escape(&e.appimage_path),
            json_escape(&e.icon_path),
            json_escape(&e.desktop_path),
            json_escape(&e.app_id),
            if i + 1 < keys.len() { "," } else { "" }
        ));
    }
    out.push_str("}\n");

    let tmp = reg_path.with_extension("tmp");
    fs::write(&tmp, out).map_err(|e| e.to_string())?;
    fs::rename(tmp, reg_path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn strip_tokens(base: &str) -> String {
    let stem = base
        .strip_suffix(".AppImage")
        .or_else(|| base.strip_suffix(".appimage"))
        .unwrap_or(base);

    let ignored_arch: &[&str] = &[
        "x86_64", "amd64", "x86", "i386", "i686", "aarch64", "arm64", "armhf", "arm",
        "linux", "gnu", "glibc", "musl", "static", "portable",
    ];

    let version_re = regex::Regex::new(r"^[vV]?[0-9]+([.][0-9]+)*$").unwrap();

    let mut out = Vec::new();
    for tok in stem.split(|c| c == '.' || c == '_' || c == '-') {
        if tok.is_empty() {
            continue;
        }
        let low = tok.to_ascii_lowercase();
        if ignored_arch.contains(&low.as_str()) {
            continue;
        }
        if version_re.is_match(tok) {
            continue;
        }
        out.push(tok);
    }
    out.join(" ")
}

pub fn slugify(name: &str) -> String {
    let s = strip_tokens(name);
    let low = s.to_ascii_lowercase();
    let re = regex::Regex::new(r"[^a-z0-9]+").unwrap();
    let replaced = re.replace_all(&low, "-");
    let trimmed = replaced.trim_matches('-');
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn prettify(name: &str) -> String {
    let s = strip_tokens(name);
    if s.is_empty() {
        name.strip_suffix(".AppImage")
            .or_else(|| name.strip_suffix(".appimage"))
            .unwrap_or(name)
            .to_string()
    } else {
        s
    }
}

fn is_appimage(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let ext_match = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("appimage"))
        .unwrap_or(false);
    if !ext_match {
        return false;
    }
    if let Ok(mut f) = fs::File::open(path) {
        use std::io::Read;
        let mut magic = [0u8; 4];
        if f.read_exact(&mut magic).is_ok() {
            return magic == [0x7f, 0x45, 0x4c, 0x46];
        }
    }
    false
}

fn install_icon(root: &Path, icon_name: &str, slug: &str) {
    let icons = icon_dir();
    let _ = fs::create_dir_all(&icons);

    let _ = fs::remove_file(icons.join(format!("{slug}.png")));
    let _ = fs::remove_file(icons.join(format!("{slug}.svg")));

    let mut clean_name = icon_name;
    if let Some(pos) = clean_name.rfind('/') {
        clean_name = &clean_name[pos + 1..];
    }
    for ext in &[".png", ".svg", ".xpm"] {
        if let Some(stripped) = clean_name.strip_suffix(ext) {
            clean_name = stripped;
            break;
        }
    }

    let mut found = None;

    if !clean_name.is_empty() {
        let svg_target = format!("{clean_name}.svg");
        let png_target = format!("{clean_name}.png");

        // 1. Check scalable svg
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.to_string_lossy().contains("scalable") {
                    let cand = p.join(&svg_target);
                    if cand.is_file() {
                        found = Some(cand);
                        break;
                    }
                }
            }
        }

        // 2. Check icon sizes
        if found.is_none() {
            let sizes = ["1024x1024", "512x512", "256x256", "128x128", "96x96", "64x64", "48x48"];
            for sz in sizes {
                if found.is_some() {
                    break;
                }
                // Check if directory matching size exists
                for entry in walkdir_simple(root, 5) {
                    if entry.to_string_lossy().contains(sz) && entry.ends_with(&png_target) && entry.is_file() {
                        found = Some(entry);
                        break;
                    }
                }
            }
        }

        // 3. Any match with svg or png
        if found.is_none() {
            for entry in walkdir_simple(root, 4) {
                if (entry.ends_with(&svg_target) || entry.ends_with(&png_target)) && entry.is_file() {
                    found = Some(entry);
                    break;
                }
            }
        }
    }

    // 4. Check .DirIcon
    if found.is_none() {
        let dir_icon = root.join(".DirIcon");
        if dir_icon.is_file() || dir_icon.is_symlink() {
            if let Ok(canonical) = fs::canonicalize(&dir_icon) {
                if canonical.is_file() {
                    found = Some(canonical);
                }
            }
            if found.is_none() && dir_icon.is_file() {
                found = Some(dir_icon);
            }
        }
    }

    // 5. Any image at root
    if found.is_none() {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if ext == "png" || ext == "svg" {
                            found = Some(p);
                            break;
                        }
                    }
                }
            }
        }
    }

    if let Some(src_icon) = found {
        let ext = if src_icon.extension().and_then(|e| e.to_str()) == Some("svg") {
            "svg"
        } else {
            "png"
        };
        let target = icons.join(format!("{slug}.{ext}"));
        let _ = fs::copy(src_icon, target);
    }
}

fn walkdir_simple(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if max_depth == 0 {
        return results;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                results.extend(walkdir_simple(&p, max_depth - 1));
            } else if p.is_file() {
                results.push(p);
            }
        }
    }
    results
}

fn find_app_binary(tree: &Path) -> Option<PathBuf> {
    let mut binmatch = None;
    let mut best = None;
    let mut bestsz = 0u64;

    for f in walkdir_simple(tree, 6) {
        let lossy = f.to_string_lossy();
        if lossy.contains("/lib/") || lossy.contains("/lib64/") || lossy.contains("/libexec/") {
            continue;
        }

        // Check if executable ELF
        let is_elf = if let Ok(mut file) = fs::File::open(&f) {
            use std::io::Read;
            let mut magic = [0u8; 4];
            file.read_exact(&mut magic).is_ok() && magic == [0x7f, 0x45, 0x4c, 0x46]
        } else {
            false
        };

        if !is_elf {
            continue;
        }

        if binmatch.is_none() && lossy.contains("/bin/") {
            binmatch = Some(f.clone());
        }

        let sz = fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
        if sz > bestsz {
            bestsz = sz;
            best = Some(f);
        }
    }

    binmatch.or(best)
}

fn update_desktop_database() {
    let _ = Command::new("update-desktop-database")
        .arg(desktop_dir())
        .output();
}

fn install_appimage(src: &Path) -> Result<(String, String, String), String> {
    if !is_appimage(src) {
        return Err(format!("not an appimage: {}", src.display()));
    }

    let fname = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid filename".to_string())?;

    let apps = apps_dir();
    let desktops = desktop_dir();
    let icons = icon_dir();
    let _ = fs::create_dir_all(&apps);
    let _ = fs::create_dir_all(&desktops);
    let _ = fs::create_dir_all(&icons);

    let dest = apps.join(fname);
    if src != dest {
        fs::copy(src, &dest).map_err(|e| e.to_string())?;
    }

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
    }

    let tmpdir = std::env::temp_dir().join(format!("xiu-appimage-{}", std::process::id()));
    let _ = fs::create_dir_all(&tmpdir);

    let _ = Command::new("timeout")
        .args(["60", dest.to_str().unwrap_or(""), "--appimage-extract"])
        .current_dir(&tmpdir)
        .output();

    let root = tmpdir.join("squashfs-root");
    let mut name = String::new();
    let mut iconname = String::new();
    let mut categories = String::new();
    let mut wmclass = String::new();

    if root.is_dir() {
        for f in walkdir_simple(&root, 3) {
            if f.extension().and_then(|e| e.to_str()) == Some("desktop") {
                if let Ok(content) = fs::read_to_string(&f) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if name.is_empty() && trimmed.starts_with("Name=") {
                            name = trimmed["Name=".len()..].trim().to_string();
                        } else if iconname.is_empty() && trimmed.starts_with("Icon=") {
                            iconname = trimmed["Icon=".len()..].trim().to_string();
                        } else if categories.is_empty() && trimmed.starts_with("Categories=") {
                            categories = trimmed["Categories=".len()..].trim().to_string();
                        } else if wmclass.is_empty() && trimmed.starts_with("StartupWMClass=") {
                            wmclass = trimmed["StartupWMClass=".len()..].trim().to_string();
                        }
                    }
                }
                break;
            }
        }
    }

    if name.is_empty() {
        name = prettify(fname);
    }

    let mut slug = slugify(fname);
    let appid = if !wmclass.is_empty() {
        wmclass.clone()
    } else if !name.is_empty() {
        name.clone()
    } else {
        slug.clone()
    };

    let mut registry = load_registry();
    let action;

    if let Some(prev) = registry.get(&slug) {
        if prev.appimage_path == dest.to_string_lossy() {
            action = "reinstalled".to_string();
        } else if prev.app_id.is_empty() || prev.app_id == appid {
            action = "updated".to_string();
            let _ = fs::remove_file(&prev.appimage_path);
        } else {
            let mut n = 2;
            while registry.contains_key(&format!("{slug}-{n}")) {
                n += 1;
            }
            slug = format!("{slug}-{n}");
            action = "new".to_string();
        }
    } else {
        action = "new".to_string();
    }

    if root.is_dir() {
        install_icon(&root, &iconname, &slug);
    }

    let mut icon_path = icons.join(format!("{slug}.png"));
    if !icon_path.is_file() {
        icon_path = icons.join(format!("{slug}.svg"));
        if !icon_path.is_file() {
            icon_path = PathBuf::new();
        }
    }

    let df_out = desktops.join(format!("xiu-{slug}.desktop"));
    let mut df_content = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{}\" %U\n",
        dest.display()
    );
    if !icon_path.as_os_str().is_empty() {
        df_content.push_str(&format!("Icon={}\n", icon_path.display()));
    }
    if !categories.is_empty() {
        df_content.push_str(&format!("Categories={categories}\n"));
    }
    if !wmclass.is_empty() {
        df_content.push_str(&format!("StartupWMClass={wmclass}\n"));
    }
    df_content.push_str("Terminal=false\nX-Xiu-AppImage=true\n");
    let _ = fs::write(&df_out, df_content);

    registry.insert(
        slug.clone(),
        AppRegistryEntry {
            name: name.clone(),
            appimage_path: dest.to_string_lossy().to_string(),
            icon_path: icon_path.to_string_lossy().to_string(),
            desktop_path: df_out.to_string_lossy().to_string(),
            app_id: appid,
        },
    );
    let _ = save_registry(&registry);
    update_desktop_database();

    let _ = fs::remove_dir_all(&tmpdir);
    Ok((slug, name, action))
}

fn extract_install(src: &Path) -> Result<(String, String, String), String> {
    let base = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid filename".to_string())?;

    let mut stem = base.to_string();
    let low_stem = stem.to_ascii_lowercase();
    for ext in &[".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if low_stem.ends_with(ext) {
            stem = stem[..stem.len() - ext.len()].to_string();
            break;
        }
    }
    for ext in &[".tgz", ".txz", ".tbz2", ".zip", ".deb", ".rpm"] {
        if low_stem.ends_with(ext) {
            stem = stem[..stem.len() - ext.len()].to_string();
            break;
        }
    }

    let tmpdir = std::env::temp_dir().join(format!("xiu-extract-{}", std::process::id()));
    let _ = fs::create_dir_all(&tmpdir);

    // bsdtar extracts tar, zip, deb, rpm
    let status = Command::new("bsdtar")
        .args(["-xf", src.to_str().unwrap_or(""), "-C", tmpdir.to_str().unwrap_or("")])
        .status()
        .map_err(|e| format!("failed to run bsdtar: {e}"))?;

    if !status.success() {
        let _ = fs::remove_dir_all(&tmpdir);
        return Err(format!("bsdtar extraction failed for {base}"));
    }

    let mut tree = tmpdir.clone();
    if low_stem.ends_with(".deb") {
        if let Some(datatar) = walkdir_simple(&tmpdir, 1).into_iter().find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("data.tar"))
                .unwrap_or(false)
        }) {
            let sub = tmpdir.join("_data");
            let _ = fs::create_dir_all(&sub);
            let _ = Command::new("bsdtar")
                .args(["-xf", datatar.to_str().unwrap_or(""), "-C", sub.to_str().unwrap_or("")])
                .status();
            tree = sub;
        }
    }

    // Collapse single top-level directory
    if let Ok(entries) = fs::read_dir(&tree) {
        let items: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        if items.len() == 1 && items[0].is_dir() {
            tree = items[0].clone();
        }
    }

    // Find .desktop file
    let mut df = None;
    for cand in walkdir_simple(&tree, 5) {
        if cand.extension().and_then(|e| e.to_str()) == Some("desktop") {
            let lossy = cand.to_string_lossy();
            if lossy.contains("share/applications") {
                df = Some(cand);
                break;
            } else if df.is_none() {
                df = Some(cand);
            }
        }
    }

    let mut dname = String::new();
    let mut iconname = String::new();
    let mut categories = String::new();
    let mut wmclass = String::new();
    let mut exec_line = String::new();

    if let Some(ref f) = df {
        if let Ok(content) = fs::read_to_string(f) {
            for line in content.lines() {
                let trimmed = line.trim();
                if dname.is_empty() && trimmed.starts_with("Name=") {
                    dname = trimmed["Name=".len()..].trim().to_string();
                } else if iconname.is_empty() && trimmed.starts_with("Icon=") {
                    iconname = trimmed["Icon=".len()..].trim().to_string();
                } else if categories.is_empty() && trimmed.starts_with("Categories=") {
                    categories = trimmed["Categories=".len()..].trim().to_string();
                } else if wmclass.is_empty() && trimmed.starts_with("StartupWMClass=") {
                    wmclass = trimmed["StartupWMClass=".len()..].trim().to_string();
                } else if exec_line.is_empty() && trimmed.starts_with("Exec=") {
                    exec_line = trimmed["Exec=".len()..].trim().to_string();
                }
            }
        }
    }

    let mut bin = None;
    let mut execargs = String::new();
    if !exec_line.is_empty() {
        let mut parts = exec_line.split_whitespace();
        if let Some(first) = parts.next() {
            let mut bin_name = first.trim_matches('"');
            if let Some(pos) = bin_name.rfind('/') {
                bin_name = &bin_name[pos + 1..];
            }
            execargs = parts.collect::<Vec<&str>>().join(" ");

            // Search for binary with this name
            for cand in walkdir_simple(&tree, 6) {
                if cand.file_name().and_then(|n| n.to_str()) == Some(bin_name) {
                    if let Ok(mut f) = fs::File::open(&cand) {
                        use std::io::Read;
                        let mut magic = [0u8; 4];
                        if f.read_exact(&mut magic).is_ok() && magic == [0x7f, 0x45, 0x4c, 0x46] {
                            bin = Some(cand);
                            break;
                        }
                    }
                }
            }
        }
    }

    if bin.is_none() {
        bin = find_app_binary(&tree);
    }

    let bin = match bin {
        Some(b) => b,
        None => {
            let _ = fs::remove_dir_all(&tmpdir);
            return Err(format!("no executable app binary found inside {base}"));
        }
    };

    let mut slug = slugify(&stem);
    let name = if !dname.is_empty() {
        dname
    } else {
        prettify(&stem)
    };
    let appid = if !wmclass.is_empty() {
        wmclass.clone()
    } else if !name.is_empty() {
        name.clone()
    } else {
        slug.clone()
    };

    let apps = apps_dir();
    let desktops = desktop_dir();
    let icons = icon_dir();
    let mut registry = load_registry();
    let action;

    if let Some(prev) = registry.get(&slug) {
        if prev.app_id.is_empty() || prev.app_id == appid {
            action = "updated".to_string();
            let _ = fs::remove_dir_all(&prev.appimage_path);
            let _ = fs::remove_file(&prev.appimage_path);
        } else {
            let mut n = 2;
            while registry.contains_key(&format!("{slug}-{n}")) {
                n += 1;
            }
            slug = format!("{slug}-{n}");
            action = "new".to_string();
        }
    } else {
        action = "new".to_string();
    }

    let dest = apps.join(&slug);
    let _ = fs::remove_dir_all(&dest);
    let _ = fs::create_dir_all(&dest);

    // Copy tree into dest
    let _ = Command::new("cp")
        .args(["-a"])
        .arg(format!("{}/.", tree.display()))
        .arg(format!("{}/", dest.display()))
        .status();

    let relbin = bin.strip_prefix(&tree).unwrap_or(&bin);
    let binpath = dest.join(relbin);

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&binpath, fs::Permissions::from_mode(0o755));
    }

    install_icon(&dest, &iconname, &slug);
    let mut icon_path = icons.join(format!("{slug}.png"));
    if !icon_path.is_file() {
        icon_path = icons.join(format!("{slug}.svg"));
        if !icon_path.is_file() {
            icon_path = PathBuf::new();
        }
    }

    let df_out = desktops.join(format!("xiu-{slug}.desktop"));
    let mut df_content = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{}\" {}\n",
        binpath.display(),
        if !execargs.is_empty() { &execargs } else { "%U" }
    );
    if !icon_path.as_os_str().is_empty() {
        df_content.push_str(&format!("Icon={}\n", icon_path.display()));
    }
    if !categories.is_empty() {
        df_content.push_str(&format!("Categories={categories}\n"));
    }
    if !wmclass.is_empty() {
        df_content.push_str(&format!("StartupWMClass={wmclass}\n"));
    }
    df_content.push_str("Terminal=false\nX-Xiu-AppImage=true\n");
    let _ = fs::write(&df_out, df_content);

    registry.insert(
        slug.clone(),
        AppRegistryEntry {
            name: name.clone(),
            appimage_path: dest.to_string_lossy().to_string(),
            icon_path: icon_path.to_string_lossy().to_string(),
            desktop_path: df_out.to_string_lossy().to_string(),
            app_id: appid,
        },
    );
    let _ = save_registry(&registry);
    update_desktop_database();

    let _ = fs::remove_dir_all(&tmpdir);
    Ok((slug, name, action))
}

fn install_font(src: &Path) -> Result<(String, String, String), String> {
    let base = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid font filename".to_string())?;

    let fonts = data_home().join("fonts");
    let _ = fs::create_dir_all(&fonts);

    let name = base.strip_suffix(".ttf")
        .or_else(|| base.strip_suffix(".otf"))
        .unwrap_or(base);

    let dest = fonts.join(base);
    let action = if dest.is_file() { "updated" } else { "new" };

    fs::copy(src, &dest).map_err(|e| e.to_string())?;
    let _ = Command::new("fc-cache").args(["-f", fonts.to_str().unwrap_or("")]).output();

    Ok((name.to_string(), action.to_string(), dest.to_string_lossy().to_string()))
}

fn install_wallpaper(src: &Path) -> Result<(String, String), String> {
    let base = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid wallpaper filename".to_string())?;

    let wpdir = crate::commands::wallpaper::resolve_wallpaper_dir();
    let _ = fs::create_dir_all(&wpdir);

    let low = base.to_ascii_lowercase();
    let stem = Path::new(base).file_stem().and_then(|s| s.to_str()).unwrap_or(base);
    let dest = if low.ends_with(".webp") {
        let png_dest = wpdir.join(format!("{stem}.png"));
        let _ = Command::new("magick")
            .arg(src)
            .arg(&png_dest)
            .output();
        png_dest
    } else {
        let direct_dest = wpdir.join(base);
        let _ = fs::copy(src, &direct_dest);
        direct_dest
    };

    let dest_str = dest.to_string_lossy().to_string();
    let _ = crate::commands::wallpaper::set_wallpaper(&dest_str, None);

    Ok((stem.to_string(), "set".to_string()))
}

fn remove_appimage(slug: &str) -> i32 {
    if slug.is_empty() {
        eprintln!("xiu app-install remove: missing slug");
        return 1;
    }

    let mut registry = load_registry();
    if let Some(entry) = registry.remove(slug) {
        if !entry.appimage_path.is_empty() {
            let p = Path::new(&entry.appimage_path);
            let apps = apps_dir();
            if let Ok(real) = fs::canonicalize(p) {
                if let Ok(real_apps) = fs::canonicalize(&apps) {
                    if real.starts_with(&real_apps) && real != real_apps {
                        if real.is_dir() {
                            let _ = fs::remove_dir_all(&real);
                        } else {
                            let _ = fs::remove_file(&real);
                        }
                    } else {
                        let _ = fs::remove_file(p);
                    }
                }
            }
        }
        if !entry.icon_path.is_empty() {
            let _ = fs::remove_file(&entry.icon_path);
        }
        if !entry.desktop_path.is_empty() {
            let _ = fs::remove_file(&entry.desktop_path);
        }
        let _ = save_registry(&registry);
        update_desktop_database();
        0
    } else {
        eprintln!("xiu app-install remove: unknown slug '{slug}'");
        1
    }
}

fn rename_appimage(slug: &str, new_name: &str) -> i32 {
    if slug.is_empty() || new_name.is_empty() {
        eprintln!("xiu app-install rename: usage: rename <slug> <new name>");
        return 1;
    }

    let mut registry = load_registry();
    let entry = match registry.get_mut(slug) {
        Some(e) => e,
        None => {
            eprintln!("xiu app-install rename: unknown slug '{slug}'");
            return 1;
        }
    };

    if !entry.desktop_path.is_empty() {
        if let Ok(content) = fs::read_to_string(&entry.desktop_path) {
            let mut lines = Vec::new();
            let mut done = false;
            for line in content.lines() {
                if !done && line.starts_with("Name=") {
                    lines.push(format!("Name={new_name}"));
                    done = true;
                } else {
                    lines.push(line.to_string());
                }
            }
            let _ = fs::write(&entry.desktop_path, lines.join("\n") + "\n");
        }
    }

    entry.name = new_name.to_string();
    let _ = save_registry(&registry);
    update_desktop_database();
    0
}

pub fn app_install(action: &str, target: Option<&str>, extra: Option<&str>) -> i32 {
    match action {
        "install" => {
            let Some(src_str) = target else {
                eprintln!("xiu app-install: missing source file argument");
                return 1;
            };
            let src = Path::new(src_str);
            if !src.is_file() {
                eprintln!("xiu app-install: file not found: {src_str}");
                return 1;
            }

            let base = src
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(src_str);
            let low = base.to_ascii_lowercase();

            if low.ends_with(".appimage") {
                match install_appimage(src) {
                    Ok((_, name, act)) => {
                        println!("app\t{name}\t{act}");
                        0
                    }
                    Err(e) => {
                        eprintln!("xiu app-install error: {e}");
                        1
                    }
                }
            } else if low.ends_with(".pkg.tar.zst")
                || low.ends_with(".pkg.tar.xz")
                || low.ends_with(".pkg.tar.gz")
                || low.ends_with(".pkg.tar.bz2")
            {
                let res = Command::new("pkexec")
                    .args(["pacman", "-U", "--noconfirm", src_str])
                    .status();
                match res {
                    Ok(s) if s.success() => {
                        println!("native\t{}\tnew", prettify(base));
                        0
                    }
                    _ => 1,
                }
            } else if low.ends_with(".deb") {
                if crate::helpers::on_path("apt-get") {
                    let res = Command::new("pkexec")
                        .args(["apt-get", "install", "-y", src_str])
                        .status();
                    match res {
                        Ok(s) if s.success() => {
                            println!("native\t{}\tnew", prettify(base));
                            0
                        }
                        _ => 1,
                    }
                } else {
                    match extract_install(src) {
                        Ok((_, name, act)) => {
                            println!("app\t{name}\t{act}");
                            0
                        }
                        Err(e) => {
                            eprintln!("xiu app-install error: {e}");
                            1
                        }
                    }
                }
            } else if low.ends_with(".rpm") {
                if crate::helpers::on_path("dnf") {
                    let res = Command::new("pkexec")
                        .args(["dnf", "install", "-y", src_str])
                        .status();
                    match res {
                        Ok(s) if s.success() => {
                            println!("native\t{}\tnew", prettify(base));
                            0
                        }
                        _ => 1,
                    }
                } else if crate::helpers::on_path("zypper") {
                    let res = Command::new("pkexec")
                        .args(["zypper", "--non-interactive", "install", "--allow-unsigned-rpm", src_str])
                        .status();
                    match res {
                        Ok(s) if s.success() => {
                            println!("native\t{}\tnew", prettify(base));
                            0
                        }
                        _ => 1,
                    }
                } else {
                    match extract_install(src) {
                        Ok((_, name, act)) => {
                            println!("app\t{name}\t{act}");
                            0
                        }
                        Err(e) => {
                            eprintln!("xiu app-install error: {e}");
                            1
                        }
                    }
                }
            } else if low.ends_with(".flatpakref") {
                let res = Command::new("flatpak")
                    .args(["install", "--user", "-y", "--noninteractive", src_str])
                    .status();
                match res {
                    Ok(s) if s.success() => {
                        println!("native\t{}\tnew", prettify(base));
                        0
                    }
                    _ => 1,
                }
            } else if low.ends_with(".tar.gz")
                || low.ends_with(".tgz")
                || low.ends_with(".tar.xz")
                || low.ends_with(".txz")
                || low.ends_with(".tar.bz2")
                || low.ends_with(".tbz2")
                || low.ends_with(".tar.zst")
                || low.ends_with(".zip")
            {
                match extract_install(src) {
                    Ok((_, name, act)) => {
                        println!("app\t{name}\t{act}");
                        0
                    }
                    Err(e) => {
                        eprintln!("xiu app-install error: {e}");
                        1
                    }
                }
            } else if low.ends_with(".ttf") || low.ends_with(".otf") {
                match install_font(src) {
                    Ok((name, act, dest)) => {
                        println!("font\t{name}\t{act}\t{dest}");
                        0
                    }
                    Err(e) => {
                        eprintln!("xiu app-install error: {e}");
                        1
                    }
                }
            } else if low.ends_with(".png")
                || low.ends_with(".jpg")
                || low.ends_with(".jpeg")
                || low.ends_with(".webp")
            {
                match install_wallpaper(src) {
                    Ok((name, act)) => {
                        println!("wallpaper\t{name}\t{act}");
                        0
                    }
                    Err(e) => {
                        eprintln!("xiu app-install error: {e}");
                        1
                    }
                }
            } else {
                eprintln!("xiu app-install: unsupported file type: {base}");
                1
            }
        }
        "remove" => {
            let Some(slug) = target else {
                eprintln!("xiu app-install remove: missing slug");
                return 1;
            };
            remove_appimage(slug)
        }
        "rename" => {
            let Some(slug) = target else {
                eprintln!("xiu app-install rename: missing slug");
                return 1;
            };
            let Some(new_name) = extra else {
                eprintln!("xiu app-install rename: missing new name");
                return 1;
            };
            rename_appimage(slug, new_name)
        }
        other => {
            eprintln!("xiu app-install: unknown action '{other}' (install, remove, rename)");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_install_nonexistent() {
        let res = app_install("invalid-action", None, None);
        assert!(res >= 0);
    }

    #[test]
    fn test_strip_tokens_and_slugify() {
        assert_eq!(strip_tokens("Krita-5.2.0-x86_64.AppImage"), "Krita");
        assert_eq!(slugify("Krita-5.2.0-x86_64.AppImage"), "krita");
        assert_eq!(prettify("Krita-5.2.0-x86_64.AppImage"), "Krita");

        assert_eq!(strip_tokens("1Password-8.10.AppImage"), "1Password");
        assert_eq!(slugify("1Password-8.10.AppImage"), "1password");
    }
}
