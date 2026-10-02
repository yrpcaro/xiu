//! Wallpaper lifecycle, search, downloads, thumbnails, and backend communication.
//!
//! Native Rust implementation replacing wallpaper.sh, wallpaper-search.sh, and wallpaper-thumbs.sh.

use crate::helpers::{home_path, ipc_call, on_path, state_file};
use crate::json::{self, Json};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub const VALID_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "mp4", "webm", "mkv", "mov",
];

pub fn wallpaper(
    action: Option<&str>,
    target: Option<&str>,
    print: bool,
    list: bool,
    file: Option<&str>,
    extra: Option<&str>,
) -> i32 {
    if print {
        return query();
    }
    if list {
        return list_wallpapers();
    }
    if let Some(f) = file {
        return set_wallpaper(f, target);
    }

    match action {
        None => next_wallpaper(),
        Some("query") | Some("get") => query(),
        Some("list") => list_wallpapers(),
        Some("resolve") => resolve(),
        Some("thumbs") => thumbs(),
        Some("search") => {
            if let Some(q) = target {
                search(q, extra)
            } else {
                println!("[]");
                0
            }
        }
        Some("download") => {
            if let Some(url) = target {
                download(url)
            } else {
                1
            }
        }
        Some("set") => {
            if let Some(path) = target {
                set_wallpaper(path, extra)
            } else {
                eprintln!("xiu wallpaper set: missing image path");
                2
            }
        }
        Some("next") | Some("random") => next_wallpaper(),
        Some("prev") | Some("previous") => prev_wallpaper(),
        Some("init") => init_wallpaper(),
        Some(other) => {
            if Path::new(other).is_file() {
                set_wallpaper(other, target)
            } else {
                eprintln!(
                    "xiu wallpaper: unknown action '{other}' (init, set, next, prev, query, list, search, download, thumbs, resolve)"
                );
                2
            }
        }
    }
}

pub fn query() -> i32 {
    let xiu_state = state_file("xiu/wallpaper");
    if let Ok(content) = fs::read_to_string(&xiu_state) {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            println!("{trimmed}");
            return 0;
        }
    }

    let legacy_state = state_file("ricelin-wallpaper");
    if let Ok(content) = fs::read_to_string(&legacy_state) {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            println!("{trimmed}");
            return 0;
        }
    }

    // Fall back to quickshell IPC
    ipc_call("wallpaper", &["get"])
}

fn ensure_daemon() -> bool {
    let bin = if on_path("awww") { "awww" } else { "swww" };
    let daemon = if on_path("awww") { "awww-daemon" } else { "swww-daemon" };
    if Command::new(bin).arg("query").output().map(|o| o.status.success()).unwrap_or(false) {
        return true;
    }
    for _ in 1..=5 {
        let _ = Command::new(daemon).spawn();
        for _ in 0..15 {
            if Command::new(bin).arg("query").output().map(|o| o.status.success()).unwrap_or(false) {
                return true;
            }
            thread::sleep(Duration::from_millis(200));
        }
    }
    false
}

fn outputs() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Ok(Json::Arr(mons)) = json::parse(&text) {
            for m in mons {
                if let Some(n) = m.get("name").and_then(Json::as_str) {
                    if !n.is_empty() {
                        names.push(n.to_string());
                    }
                }
            }
        }
    }
    names
}

fn focused_output() -> Option<String> {
    if let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Ok(Json::Arr(mons)) = json::parse(&text) {
            for m in mons {
                if m.get("focused").and_then(Json::as_bool) == Some(true) {
                    return m.get("name").and_then(Json::as_str).map(|s| s.to_string());
                }
            }
        }
    }
    None
}

fn cursor_output() -> Option<String> {
    if let Ok(out) = Command::new("hyprctl").arg("cursorpos").output() {
        let text = String::from_utf8_lossy(&out.stdout);
        let trimmed = text.trim();
        if let Some((xs, ys)) = trimmed.split_once(',') {
            if let (Ok(cx), Ok(cy)) = (xs.trim().parse::<f64>(), ys.trim().parse::<f64>()) {
                if let Ok(mout) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
                    let mtext = String::from_utf8_lossy(&mout.stdout);
                    if let Ok(Json::Arr(mons)) = json::parse(&mtext) {
                        for m in mons {
                            let mx = m.get("x").and_then(Json::as_f64).unwrap_or(0.0);
                            let my = m.get("y").and_then(Json::as_f64).unwrap_or(0.0);
                            let mw = m.get("width").and_then(Json::as_f64).unwrap_or(1920.0);
                            let mh = m.get("height").and_then(Json::as_f64).unwrap_or(1080.0);
                            let scale = m.get("scale").and_then(Json::as_f64).unwrap_or(1.0);
                            let transform = m.get("transform").and_then(Json::as_i64).unwrap_or(0);
                            let (w, h) = if transform % 2 == 1 {
                                (mh / scale, mw / scale)
                            } else {
                                (mw / scale, mh / scale)
                            };
                            if cx >= mx && cx < mx + w && cy >= my && cy < my + h {
                                return m.get("name").and_then(Json::as_str).map(|s| s.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    focused_output()
}

fn map_file() -> PathBuf {
    state_file("ricelin-wallpaper-map")
}

fn map_get(output: &str) -> Option<String> {
    let mf = map_file();
    if let Ok(content) = fs::read_to_string(&mf) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 && parts[0] == output {
                return Some(parts[1].trim().to_string());
            }
        }
    }
    None
}

fn map_put(output: &str, path: &str) {
    let mf = map_file();
    let mut entries = Vec::new();
    if let Ok(content) = fs::read_to_string(&mf) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 && parts[0] != output && !parts[0].is_empty() {
                entries.push((parts[0].to_string(), parts[1].to_string()));
            }
        }
    }
    entries.push((output.to_string(), path.to_string()));
    let mut out = String::new();
    for (o, p) in entries {
        out.push_str(&format!("{o}\t{p}\n"));
    }
    let _ = fs::write(&mf, out);
}

fn map_put_all(path: &str) {
    let mf = map_file();
    let mut out = String::new();
    for o in outputs() {
        out.push_str(&format!("{o}\t{path}\n"));
    }
    let _ = fs::write(&mf, out);
}

fn is_video(path: &str) -> bool {
    let p = Path::new(path);
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        let low = ext.to_ascii_lowercase();
        matches!(low.as_str(), "mp4" | "webm" | "mkv" | "mov")
    } else {
        false
    }
}

fn is_animated(path: &str) -> bool {
    let p = Path::new(path);
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        let low = ext.to_ascii_lowercase();
        matches!(low.as_str(), "mp4" | "webm" | "mkv" | "mov" | "gif")
    } else {
        false
    }
}

fn make_still(src: &str, dst: &Path) -> bool {
    let tmp = dst.with_extension("png.tmp");
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-i",
            src,
            "-frames:v",
            "1",
            "-f",
            "image2",
            "-c:v",
            "png",
        ])
        .arg(&tmp)
        .status();
    if let Ok(s) = status {
        if s.success() && tmp.is_file() {
            let _ = fs::rename(tmp, dst);
            return true;
        }
    }
    let _ = fs::remove_file(&tmp);
    false
}

fn apply_visual(pic: &str, output: Option<&str>) {
    let mut show = pic.to_string();
    if is_animated(pic) {
        let still = if let Some(o) = output {
            state_file(&format!("ricelin-wallpaper-still-{o}.png"))
        } else {
            state_file("ricelin-wallpaper-still.png")
        };
        if make_still(pic, &still) {
            show = still.to_string_lossy().to_string();
        }
    }

    let bin = if on_path("awww") { "awww" } else { "swww" };
    let mut cmd = Command::new(bin);
    cmd.arg("img");
    if let Some(o) = output {
        if o != "all" && !o.is_empty() {
            cmd.args(["--outputs", o]);
        }
    }
    cmd.arg(&show);
    cmd.args([
        "--transition-type",
        "wave",
        "--transition-angle",
        "30",
        "--transition-wave",
        "60,30",
        "--transition-fps",
        "60",
        "--transition-step",
        "90",
    ]);
    let _ = cmd.status();

    if show != pic && !is_video(pic) {
        thread::sleep(Duration::from_millis(900));
        let mut c2 = Command::new(bin);
        c2.arg("img");
        if let Some(o) = output {
            if o != "all" && !o.is_empty() {
                c2.args(["--outputs", o]);
            }
        }
        c2.arg(pic);
        c2.args(["--transition-type", "none"]);
        let _ = c2.status();
    }
}

fn sync_videos() {
    let outs = outputs();
    let mut desired = Vec::new();
    for o in &outs {
        if let Some(pic) = map_get(o) {
            if is_video(&pic) && Path::new(&pic).is_file() {
                desired.push((o.clone(), pic));
            }
        }
    }

    let mpv_running = Command::new("pgrep")
        .args(["-x", "mpvpaper"])
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if mpv_running {
        let _ = Command::new("pkill").args(["-x", "mpvpaper"]).status();
        for _ in 0..10 {
            let still_running = Command::new("pgrep")
                .args(["-x", "mpvpaper"])
                .stdout(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !still_running {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    if desired.is_empty() {
        return;
    }

    thread::sleep(Duration::from_millis(800));

    for (o, pic) in desired {
        let _ = Command::new("mpvpaper")
            .args(["-p", "-o", "no-audio loop-file=inf hwdec=auto panscan=1.0", &o, &pic])
            .spawn();
    }
}

fn sddm_sync(show: &str) {
    let dst = Path::new("/usr/share/sddm/themes/washi");
    let cache_colors = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".cache"]))
        .join("ricelin")
        .join("colors.json");

    if !dst.is_dir() || !cache_colors.is_file() {
        return;
    }

    if let Ok(content) = fs::read_to_string(&cache_colors) {
        if let Ok(parsed) = json::parse(&content) {
            let bright = parsed.get("bright").and_then(Json::as_str).unwrap_or("#ffffff");
            let cream = parsed.get("cream").and_then(Json::as_str).unwrap_or("#f0f0f0");
            let dim = parsed.get("dim").and_then(Json::as_str).unwrap_or("#888888");
            let primary = parsed.get("primary").and_then(Json::as_str).unwrap_or("#ffaa00");

            let conf_content = format!(
                "[General]\nbackground=current/wallpaper\nbright={bright}\ncream={cream}\ndim={dim}\nmark={primary}\nerror={primary}\n"
            );
            let tmp = std::env::temp_dir().join(format!("sddm-conf-{}", std::process::id()));
            if fs::write(&tmp, conf_content).is_ok() {
                let _ = Command::new("sudo")
                    .args([
                        "-n",
                        "install",
                        "-m644",
                        show,
                        "/usr/share/sddm/themes/washi/current/wallpaper",
                    ])
                    .status();
                let _ = Command::new("sudo")
                    .args([
                        "-n",
                        "install",
                        "-m644",
                        tmp.to_str().unwrap_or(""),
                        "/usr/share/sddm/themes/washi/theme.conf.user",
                    ])
                    .status();
                let face = home_path(&[".face"]);
                if face.is_file() {
                    let _ = Command::new("sudo")
                        .args([
                            "-n",
                            "install",
                            "-m644",
                            face.to_str().unwrap_or(""),
                            "/usr/share/sddm/themes/washi/current/face",
                        ])
                        .status();
                }
                let _ = fs::remove_file(&tmp);
            }
        }
    }
}

fn palette_update() {
    let focused = focused_output();
    let pic = focused.as_deref().and_then(map_get).or_else(get_current_wallpaper);
    let Some(pic) = pic else {
        return;
    };
    if !Path::new(&pic).is_file() {
        return;
    }

    let mut show = pic.clone();
    if is_video(&pic) {
        let still = state_file("ricelin-wallpaper-still.png");
        if make_still(&pic, &still) {
            show = still.to_string_lossy().to_string();
        }
    }

    update_state_files(&pic);

    let flags_file = state_file("ricelin/flags.json");
    let mut palette_mode = "static".to_string();
    let mut manual_hue = "30".to_string();
    let mut manual_dark = "dark".to_string();

    if let Ok(c) = fs::read_to_string(&flags_file) {
        if let Ok(parsed) = json::parse(&c) {
            if let Some(m) = parsed.get("paletteMode").and_then(Json::as_str) {
                palette_mode = m.to_string();
            }
            if let Some(h) = parsed.get("manualHue") {
                if let Some(num) = h.as_i64() {
                    manual_hue = num.to_string();
                }
            }
            if let Some(d) = parsed.get("manualDark").and_then(Json::as_bool) {
                manual_dark = if d { "dark".to_string() } else { "light".to_string() };
            }
        }
    }

    let wallcolors = crate::helpers::wallcolors_script();
    if wallcolors.is_file() {
        if palette_mode == "manual" {
            let _ = Command::new("python3")
                .arg(&wallcolors)
                .args(["--hue", &manual_hue, &manual_dark])
                .status();
        } else {
            let _ = Command::new("python3")
                .arg(&wallcolors)
                .arg(&show)
                .status();
        }
    }

    let _ = Command::new("hyprctl").arg("reload").status();
    let _ = Command::new("busctl")
        .args([
            "--user",
            "call",
            "com.mitchellh.ghostty",
            "/com/mitchellh/ghostty",
            "org.gtk.Actions",
            "Activate",
            "sava{sv}",
            "reload-config",
            "0",
            "0",
        ])
        .status();

    sddm_sync(&show);
}

pub fn set_wallpaper(path: &str, output: Option<&str>) -> i32 {
    let p = Path::new(path);
    if !p.is_file() {
        eprintln!("xiu wallpaper set: file not found: {path}");
        return 1;
    }

    let canonical = match fs::canonicalize(p) {
        Ok(c) => c.to_string_lossy().to_string(),
        Err(_) => path.to_string(),
    };

    record_history();
    ensure_daemon();

    let target = output.unwrap_or("");
    if !target.is_empty() && target != "all" {
        map_put(target, &canonical);
    } else {
        map_put_all(&canonical);
    }

    apply_visual(&canonical, output);
    sync_videos();
    palette_update();

    update_state_files(&canonical);
    let _ = ipc_call("wallpaper", &["refresh"]);

    0
}

pub fn next_wallpaper() -> i32 {
    record_history();
    ensure_daemon();

    let flags_file = state_file("ricelin/flags.json");
    let mut scope = "all".to_string();
    if let Ok(c) = fs::read_to_string(&flags_file) {
        if let Ok(parsed) = json::parse(&c) {
            if let Some(s) = parsed.get("randomScope").and_then(Json::as_str) {
                scope = s.to_string();
            }
        }
    }

    let target = if scope == "cursor" {
        cursor_output()
    } else {
        None
    };

    let bag = state_file("ricelin-wallpaper-bag");
    let mut lines = Vec::new();
    if let Ok(c) = fs::read_to_string(&bag) {
        for l in c.lines() {
            let t = l.trim();
            if !t.is_empty() {
                lines.push(t.to_string());
            }
        }
    }

    if lines.is_empty() {
        let dir = resolve_wallpaper_dir();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        let low = ext.to_ascii_lowercase();
                        if VALID_EXTENSIONS.contains(&low.as_str()) {
                            lines.push(p.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as usize)
            .unwrap_or(42);
        for i in (1..lines.len()).rev() {
            let j = (seed + i * 31) % (i + 1);
            lines.swap(i, j);
        }
    }

    if lines.is_empty() {
        return ipc_call("wallpaper", &["random"]);
    }

    let picked = lines.remove(0);
    let _ = fs::write(&bag, lines.join("\n") + "\n");

    set_wallpaper(&picked, target.as_deref())
}

pub fn prev_wallpaper() -> i32 {
    let hist_file = state_file("xiu/wallpaper-history");
    if !hist_file.is_file() {
        eprintln!("xiu wallpaper prev: no wallpaper history found");
        return 1;
    }

    let Ok(content) = fs::read_to_string(&hist_file) else {
        eprintln!("xiu wallpaper prev: unable to read history");
        return 1;
    };

    let mut lines: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let current = get_current_wallpaper().unwrap_or_default();

    // Pop entries matching the current wallpaper
    while let Some(&last) = lines.last() {
        if last == current {
            lines.pop();
        } else {
            break;
        }
    }

    let Some(prev) = lines.pop() else {
        eprintln!("xiu wallpaper prev: no previous wallpaper in history");
        return 1;
    };

    let prev_str = prev.to_string();

    // Rewrite history without the popped item
    let new_content = lines.join("\n") + if lines.is_empty() { "" } else { "\n" };
    let _ = fs::write(&hist_file, new_content);

    set_wallpaper(&prev_str, None)
}

pub fn init_wallpaper() -> i32 {
    let daemon_was_running = ensure_daemon();

    let mf = map_file();
    let state_xiu = state_file("xiu/wallpaper");
    let state_ricelin = state_file("ricelin-wallpaper");

    let current = fs::read_to_string(&state_xiu)
        .or_else(|_| fs::read_to_string(&state_ricelin))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if !mf.is_file() || fs::metadata(&mf).map(|m| m.len() == 0).unwrap_or(true) {
        if !current.is_empty() && Path::new(&current).is_file() {
            map_put_all(&current);
        }
    }

    if daemon_was_running {
        let has_video = outputs().into_iter().any(|o| {
            map_get(&o).map(|p| is_video(&p)).unwrap_or(false)
        });
        let mpv_running = Command::new("pgrep")
            .args(["-x", "mpvpaper"])
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if has_video && !mpv_running {
            sync_videos();
        }
        return 0;
    }

    let outs = outputs();
    let mut any = false;
    for o in &outs {
        let pic = map_get(o).or_else(|| {
            if !current.is_empty() && Path::new(&current).is_file() {
                Some(current.clone())
            } else {
                None
            }
        });
        if let Some(p) = pic {
            map_put(o, &p);
            apply_visual(&p, Some(o));
            any = true;
        }
    }

    if any {
        sync_videos();
        palette_update();
    }
    0
}

pub fn list_wallpapers() -> i32 {
    // 1. If quickshell is up, let it return the entries
    let res = ipc_call("wallpaper", &["list"]);
    if res == 0 {
        return 0;
    }

    // 2. Otherwise inspect the wallpaper directory
    let dir = resolve_wallpaper_dir();
    if !dir.is_dir() {
        eprintln!("xiu wallpaper list: directory does not exist: {}", dir.display());
        return 1;
    }

    if let Ok(entries) = fs::read_dir(&dir) {
        let mut paths = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_ascii_lowercase();
                    if VALID_EXTENSIONS.contains(&ext_lower.as_str()) {
                        paths.push(path);
                    }
                }
            }
        }
        paths.sort();
        for p in paths {
            println!("{}", p.display());
        }
    }
    0
}

pub fn resolve() -> i32 {
    let dir = resolve_wallpaper_dir();
    let resolved_state = state_file("ricelin-wallpaper-dir");
    if let Some(parent) = resolved_state.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&resolved_state, format!("{}\n", dir.display()));
    println!("{}", dir.display());
    0
}

pub fn thumbs() -> i32 {
    let wpdir = resolve_wallpaper_dir();
    let cache = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".cache"]))
        .join("ricelin-wp-thumbs");
    let _ = fs::create_dir_all(&cache);

    // 1. Prune stale thumbnails
    if wpdir.is_dir() {
        let mut valid_names = HashSet::new();
        if let Ok(entries) = fs::read_dir(&wpdir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    valid_names.insert(name.to_string());
                }
            }
        }
        if let Ok(entries) = fs::read_dir(&cache) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("png") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        if !valid_names.contains(stem) {
                            let _ = fs::remove_file(p);
                        }
                    }
                }
            }
        }
    }

    // 2. Generate thumbnails
    if wpdir.is_dir() {
        if let Ok(entries) = fs::read_dir(&wpdir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let Some(ext) = p.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                let ext_lower = ext.to_ascii_lowercase();
                if !VALID_EXTENSIONS.contains(&ext_lower.as_str()) {
                    continue;
                }

                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let thumb_path = cache.join(format!("{name}.png"));
                let is_stale = if let (Ok(src_meta), Ok(dst_meta)) = (p.metadata(), thumb_path.metadata()) {
                    if let (Ok(src_mod), Ok(dst_mod)) = (src_meta.modified(), dst_meta.modified()) {
                        src_mod > dst_mod
                    } else {
                        false
                    }
                } else {
                    true
                };

                if !thumb_path.exists() || is_stale {
                    let tmp = cache.join(format!("{name}.tmp.png"));
                    let mut cmd = if matches!(ext_lower.as_str(), "mp4" | "webm" | "mkv" | "mov") {
                        let mut c = Command::new("ffmpeg");
                        c.args([
                            "-y",
                            "-loglevel",
                            "quiet",
                            "-i",
                            p.to_str().unwrap_or(""),
                            "-frames:v",
                            "1",
                            "-vf",
                            "scale=512:-2",
                            "-f",
                            "image2",
                            "-c:v",
                            "png",
                        ]);
                        c.arg(&tmp);
                        c
                    } else {
                        let mut c = Command::new("magick");
                        c.arg(format!("{}[0]", p.to_str().unwrap_or("")));
                        c.args(["-strip", "-resize", "512x"]);
                        c.arg(format!("png:{}", tmp.to_str().unwrap_or("")));
                        c
                    };

                    if let Ok(s) = cmd.status() {
                        if s.success() && tmp.exists() {
                            let _ = fs::rename(tmp, thumb_path);
                        } else {
                            let _ = fs::remove_file(tmp);
                        }
                    }
                }
            }
        }
    }

    0
}

pub fn search(query: &str, _kind: Option<&str>) -> i32 {
    if query.trim().is_empty() {
        println!("[]");
        return 0;
    }

    let script = r#"
import concurrent.futures, json, os, re, sys, urllib.parse, urllib.request

ua = "Mozilla/5.0 (X11; Linux x86_64) Gecko/20100101 Firefox/126.0"

def fetch(url, timeout=10):
    req = urllib.request.Request(url, headers={"User-Agent": ua, "Referer": "https://moewalls.com/"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read().decode("utf-8", "ignore")

def post_entry(url):
    try:
        html = fetch(url)
        prev = re.search(r'<source src="(/wp-content/uploads/preview/[^"]+)"', html)
        token = re.search(r'id="moe-download"[^>]*data-url="([^"]+)"', html)
        thumb = re.search(r'poster="([^"]+)"', html)
        if not prev or not token: return None
        res = re.search(r'resolutions-(\d+)x(\d+)', html)
        return {
            "image": "https://go.moewalls.com/download.php?video=" + token.group(1),
            "thumb": urllib.parse.urljoin("https://moewalls.com/", thumb.group(1)) if thumb else "",
            "preview": urllib.parse.urljoin("https://moewalls.com/", prev.group(1)),
            "w": int(res.group(1)) if res else 0,
            "h": int(res.group(2)) if res else 0,
        }
    except Exception: return None

try:
    q = urllib.parse.quote(sys.argv[1])
    page = fetch("https://moewalls.com/?s=" + q, timeout=12)
    posts = []
    for m in re.finditer(r'href="(https://moewalls\.com/[a-z0-9-]+/[a-z0-9-]+-live-wallpaper/)"', page):
        if m.group(1) not in posts: posts.append(m.group(1))
    out = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as ex:
        for entry in ex.map(post_entry, posts[:24]):
            if entry: out.append(entry)
    print(json.dumps(out))
except Exception:
    print("[]")
"#;

    let output = Command::new("python3")
        .args(["-c", script, query])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            print!("{text}");
            0
        }
        _ => {
            println!("[]");
            0
        }
    }
}

pub fn download(url: &str) -> i32 {
    let wpdir = resolve_wallpaper_dir();
    let _ = fs::create_dir_all(&wpdir);

    let mut fname = url.split('?').next().unwrap_or(url);
    if let Some(pos) = fname.rfind('/') {
        fname = &fname[pos + 1..];
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let clean_fname = if fname.is_empty() || fname == "download.php" {
        format!("wallpaper_{now}.mp4")
    } else {
        fname.to_string()
    };

    let dest = wpdir.join(&clean_fname);

    let res = Command::new("curl")
        .args(["-sSL", "-o", dest.to_str().unwrap_or(""), url])
        .status();

    match res {
        Ok(s) if s.success() && dest.is_file() => {
            println!("{}", dest.display());
            0
        }
        _ => {
            eprintln!("xiu wallpaper download: failed to download {url}");
            1
        }
    }
}

fn get_current_wallpaper() -> Option<String> {
    for name in &["xiu/wallpaper", "ricelin-wallpaper"] {
        let f = state_file(name);
        if let Ok(c) = fs::read_to_string(&f) {
            let t = c.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn record_history() {
    if let Some(curr) = get_current_wallpaper() {
        if curr.is_empty() || !Path::new(&curr).is_file() {
            return;
        }
        let hist_file = state_file("xiu/wallpaper-history");
        if let Some(parent) = hist_file.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut lines = Vec::new();
        if let Ok(content) = fs::read_to_string(&hist_file) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    lines.push(trimmed.to_string());
                }
            }
        }

        // Avoid duplicate consecutive entries
        if lines.last().map(String::as_str) != Some(&curr) {
            lines.push(curr);
            let new_content = lines.join("\n") + "\n";
            let _ = fs::write(&hist_file, new_content);
        }
    }
}

fn update_state_files(path: &str) {
    for name in &["xiu/wallpaper", "ricelin-wallpaper"] {
        let f = state_file(name);
        if let Some(parent) = f.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&f, format!("{path}\n"));
    }
}

pub fn resolve_wallpaper_dir() -> PathBuf {
    let resolved_state = state_file("ricelin-wallpaper-dir");
    if let Ok(content) = fs::read_to_string(&resolved_state) {
        let t = content.trim();
        if !t.is_empty() && Path::new(t).is_dir() {
            return PathBuf::from(t);
        }
    }

    for cand in &[
        &["Pictures", "xiu", "wallpapers"][..],
        &["Pictures", "Wallpapers"][..],
        &["Pictures", "wallpapers"][..],
        &["Wallpapers"][..],
        &["wallpapers"][..],
    ] {
        let p = home_path(cand);
        if p.is_dir() {
            return p;
        }
    }

    home_path(&["Pictures", "xiu", "wallpapers"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_wallpaper_dir() {
        let dir = resolve_wallpaper_dir();
        assert!(!dir.to_string_lossy().is_empty());
    }

    #[test]
    fn test_search_empty_query() {
        assert_eq!(wallpaper(Some("search"), None, false, false, None, None), 0);
    }
}
