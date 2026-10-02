//! Pure Rust update engine for the xiu rice.
//! Three-way merges user configuration against upstream updates, tracks sync manifests,
//! updates code files wholesale, rebuilds the CLI when modified, and checks missing core dependencies.

use crate::json::{self, Json};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const DEFAULT_REMOTE: &str = "https://github.com/yrpcaro/xiu.git";

pub const PROTECTED: &[&str] = &[
    "hypr/modules/decoration.lua",
    "hypr/modules/binds.lua",
    "hypr/modules/monitors.lua",
    "hypr/modules/input.lua",
    "hypr/modules/env.lua",
    "hypr/modules/autostart.lua",
    "hypr/modules/animations.lua",
    "hypr/modules/stash-apps.lua",
    "hypr/modules/spaces.lua",
    "hypr/hypridle.conf",
];

pub const ARCH_IDS: &[&str] = &[
    "arch", "cachyos", "endeavouros", "manjaro", "garuda", "artix",
    "arcolinux", "archcraft", "rebornos", "athena", "blackarch", "archbang",
    "crystal", "snigdha", "parabola", "obarun", "arch32", "hyperbola", "steamos",
    "omarchy", "xerolinux", "archman", "biglinux", "ctlos", "tromjaro",
    "bluestar", "arkane", "blendos", "acreetionos", "mabox",
];
pub const DEBIAN_IDS: &[&str] = &[
    "debian", "ubuntu", "linuxmint", "pop", "elementary", "zorin", "raspbian",
];
pub const FEDORA_IDS: &[&str] = &[
    "fedora", "nobara", "rhel", "centos", "rocky", "almalinux",
];
pub const SUSE_IDS: &[&str] = &[
    "suse", "opensuse", "sles", "sled", "tumbleweed", "leap",
];
pub const GENTOO_IDS: &[&str] = &[
    "gentoo", "funtoo", "calculate", "pentoo", "redcore",
];

pub fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".local/share")
        });
    base.join("xiu-update")
}

pub fn manifest_path() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".local/state")
        });
    base.join("xiu/update.json")
}

pub fn migrate_legacy() {
    let share = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".local/share")
        });
    let new_clone = share.join("xiu-update");
    if !new_clone.join(".git").exists() {
        let old_clone = share.join("ricelin-update");
        if old_clone.join(".git").exists() {
            let _ = fs::rename(&old_clone, &new_clone);
        }
    }

    let new_man = manifest_path();
    if !new_man.exists() {
        let state = std::env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                PathBuf::from(home).join(".local/state")
            });
        let old_man = state.join("ricelin/update.json");
        if old_man.exists() {
            if let Some(parent) = new_man.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::rename(&old_man, &new_man);
        }
    }
}

pub fn load_manifest() -> Result<Json, String> {
    migrate_legacy();
    let path = manifest_path();
    if !path.exists() {
        return Ok(Json::Obj(vec![
            ("syncedSha".to_string(), Json::Null),
            ("modules".to_string(), Json::Obj(Vec::new())),
        ]));
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    json::parse(&content)
}

pub fn save_manifest(manifest: &Json) -> Result<(), String> {
    let serialized = manifest.to_json();
    atomic_write_bytes(&manifest_path(), serialized.as_bytes()).map_err(|e| e.to_string())
}

pub fn atomic_write_bytes(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    let tmp_name = format!(".tmp-{}-{}", std::process::id(), file_name);
    let tmp_path = parent.join(tmp_name);
    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn target_path(config_root: &Path, rel: &str) -> PathBuf {
    if rel == "kde/kdeglobals" {
        config_root.join("kdeglobals")
    } else if let Some(stripped) = rel.strip_prefix("portals/") {
        config_root.join("xdg-desktop-portal").join(stripped)
    } else if let Some(stripped) = rel.strip_prefix("browser-integration/") {
        config_root.join("xiu/browser-integration").join(stripped)
    } else {
        config_root.join(rel)
    }
}

pub fn backup_protected(config_root: &Path) -> Option<PathBuf> {
    let stamp = Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "backup".to_string());

    let dest_root = data_dir().parent()?.join("xiu-update-backup").join(&stamp);
    let mut made = false;

    for &rel in PROTECTED {
        let live = target_path(config_root, rel);
        if !live.exists() {
            continue;
        }
        let dest = dest_root.join(rel);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::copy(&live, &dest).is_ok() {
            made = true;
        }
    }

    if made {
        Some(dest_root)
    } else {
        None
    }
}

pub fn in_git_worktree(path: &Path) -> bool {
    Command::new("git")
        .args(["-C", &path.to_string_lossy(), "rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false)
}

pub fn is_devmode(config_root: &Path) -> bool {
    for name in &["hypr", "quickshell"] {
        let sub = config_root.join(name);
        if sub.is_symlink() {
            if let Ok(target) = fs::canonicalize(&sub) {
                if in_git_worktree(&target) {
                    return true;
                }
            }
        }
    }
    false
}

fn git_cmd(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git command failed: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn classify_git_failure(stderr: &str) -> &'static str {
    let lower = stderr.to_lowercase();
    let offline = [
        "could not resolve",
        "couldn't resolve",
        "network",
        "timed out",
        "connection",
        "unable to access",
        "failed to connect",
    ]
    .iter()
    .any(|s| lower.contains(s));
    if offline {
        return "offline";
    }
    if !data_dir().join(".git").exists() {
        return "noclone";
    }
    "error"
}

pub fn ensure_clone(remote: &str, do_fetch: bool) -> Result<PathBuf, (String, String)> {
    let clone = data_dir();
    if clone.join(".git").exists() {
        let current = git_cmd(&clone, &["remote", "get-url", "origin"]).unwrap_or_default();
        let current = current.trim();
        if !current.is_empty() && current != remote {
            let _ = git_cmd(&clone, &["remote", "set-url", "origin", remote]);
        }
        if do_fetch {
            if let Err(e) = git_cmd(&clone, &["fetch", "origin"]) {
                return Err((classify_git_failure(&e).to_string(), e));
            }
        }
        return Ok(clone);
    }
    if let Some(parent) = clone.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let res = Command::new("git")
        .args(["clone", "--quiet", remote])
        .arg(&clone)
        .output();
    match res {
        Ok(out) if out.status.success() => Ok(clone),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err((classify_git_failure(&err).to_string(), err))
        }
        Err(e) => Err(("error".to_string(), e.to_string())),
    }
}

pub fn origin_head(clone: &Path) -> Result<String, String> {
    for r in &["origin/xiu", "origin/main"] {
        if let Ok(out) = git_cmd(clone, &["rev-parse", "--verify", &format!("{r}^{{commit}}")]) {
            let sha = out.trim().to_string();
            if !sha.is_empty() {
                return Ok(sha);
            }
        }
    }
    git_cmd(clone, &["rev-parse", "HEAD"]).map(|s| s.trim().to_string())
}

pub fn sha_known(clone: &Path, sha: &str) -> bool {
    if sha.is_empty() {
        return false;
    }
    Command::new("git")
        .args(["-C", &clone.to_string_lossy(), "cat-file", "-e", &format!("{sha}^{{commit}}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn commit_date(clone: &Path, sha: &str) -> String {
    git_cmd(clone, &["show", "-s", "--format=%cs", sha])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub fn short_sha(clone: &Path, sha: &str) -> String {
    git_cmd(clone, &["rev-parse", "--short", sha])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub fn behind_count(clone: &Path, base: Option<&str>, head: &str) -> i64 {
    let base = match base {
        Some(b) => b,
        None => return 0,
    };
    let range = format!("{base}..{head}");
    git_cmd(clone, &["rev-list", "--count", &range])
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

pub fn extract_changelog(clone: &Path, base: Option<&str>, head: &str) -> Vec<String> {
    let base = match base {
        Some(b) => b,
        None => return Vec::new(),
    };
    let range = format!("{base}..{head}");
    let out = match git_cmd(clone, &["log", "--format=%B%x00", &range]) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut lines = Vec::new();
    for commit in out.split('\0') {
        for line in commit.lines() {
            let trimmed = line.trim();
            if trimmed.to_lowercase().starts_with("changelog:") {
                let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let text = parts[1].trim();
                    if !text.is_empty() {
                        lines.push(text.to_string());
                    }
                }
            }
        }
    }
    lines
}

pub fn tracked_config_files(clone: &Path, r#ref: &str) -> Vec<String> {
    let out = git_cmd(clone, &["ls-tree", "-r", "--name-only", r#ref, "configs/"]).unwrap_or_default();
    let mut rels = Vec::new();
    for line in out.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("configs/") {
            rels.push(stripped.to_string());
        }
    }
    rels
}

pub fn show_at(clone: &Path, sha: &str, rel: &str) -> Option<Vec<u8>> {
    let target = format!("{sha}:configs/{rel}");
    let out = Command::new("git")
        .args(["-C", &clone.to_string_lossy(), "show", &target])
        .output()
        .ok()?;
    if out.status.success() {
        Some(out.stdout)
    } else {
        None
    }
}

pub fn merge_file(theirs: &[u8], base: &[u8], new: &[u8]) -> (Vec<u8>, bool) {
    let tmp_dir = std::env::temp_dir().join(format!("xiu-merge-{}", std::process::id()));
    let _ = fs::create_dir_all(&tmp_dir);
    let tp = tmp_dir.join("theirs");
    let bp = tmp_dir.join("base");
    let np = tmp_dir.join("new");
    let _ = fs::write(&tp, theirs);
    let _ = fs::write(&bp, base);
    let _ = fs::write(&np, new);
    let res = Command::new("git")
        .args(["merge-file", "-p"])
        .arg(&tp)
        .arg(&bp)
        .arg(&np)
        .output();
    let _ = fs::remove_file(&tp);
    let _ = fs::remove_file(&bp);
    let _ = fs::remove_file(&np);
    let _ = fs::remove_dir(&tmp_dir);
    match res {
        Ok(out) => (out.stdout, out.status.success()),
        Err(_) => (new.to_vec(), false),
    }
}

pub fn module_name(rel: &str) -> &str {
    Path::new(rel).file_stem().and_then(|s| s.to_str()).unwrap_or("")
}

pub fn reconcile_protected(
    clone: &Path,
    config_root: &Path,
    manifest: &Json,
    head: &str,
    apply: bool,
    take: &HashSet<String>,
) -> (Vec<Json>, Vec<String>, Vec<(String, String)>) {
    let mut rows = Vec::new();
    let mut conflicts = Vec::new();
    let mut sha_updates = Vec::new();

    let modules_obj = manifest.get("modules");

    for &rel in PROTECTED {
        let new_bytes = match show_at(clone, head, rel) {
            Some(b) => b,
            None => continue,
        };
        let stem = module_name(rel);
        let live_path = target_path(config_root, rel);
        if !live_path.exists() {
            if apply {
                let _ = atomic_write_bytes(&live_path, &new_bytes);
                sha_updates.push((rel.to_string(), head.to_string()));
            }
            rows.push(Json::Obj(vec![
                ("name".to_string(), Json::Str(stem.to_string())),
                ("path".to_string(), Json::Str(rel.to_string())),
                ("state".to_string(), Json::Str("update".to_string())),
            ]));
            continue;
        }

        let base_sha = modules_obj
            .and_then(|m| m.get(rel))
            .and_then(Json::as_str)
            .or_else(|| manifest.get("syncedSha").and_then(Json::as_str));

        let theirs = match fs::read(&live_path) {
            Ok(b) => b,
            Err(_) => Vec::new(),
        };

        if base_sha.is_none() {
            if theirs == new_bytes {
                rows.push(Json::Obj(vec![
                    ("name".to_string(), Json::Str(stem.to_string())),
                    ("path".to_string(), Json::Str(rel.to_string())),
                    ("state".to_string(), Json::Str("clean".to_string())),
                ]));
            } else if take.contains(rel) || apply {
                if apply {
                    let _ = atomic_write_bytes(&live_path, &new_bytes);
                    sha_updates.push((rel.to_string(), head.to_string()));
                }
                rows.push(Json::Obj(vec![
                    ("name".to_string(), Json::Str(stem.to_string())),
                    ("path".to_string(), Json::Str(rel.to_string())),
                    ("state".to_string(), Json::Str("update".to_string())),
                ]));
            } else {
                rows.push(Json::Obj(vec![
                    ("name".to_string(), Json::Str(stem.to_string())),
                    ("path".to_string(), Json::Str(rel.to_string())),
                    ("state".to_string(), Json::Str("conflict".to_string())),
                ]));
                conflicts.push(rel.to_string());
            }
            continue;
        }

        let b_sha = base_sha.unwrap();
        let base_bytes = show_at(clone, b_sha, rel).unwrap_or_else(|| new_bytes.clone());

        if new_bytes == base_bytes {
            rows.push(Json::Obj(vec![
                ("name".to_string(), Json::Str(stem.to_string())),
                ("path".to_string(), Json::Str(rel.to_string())),
                ("state".to_string(), Json::Str("clean".to_string())),
            ]));
            continue;
        }

        if theirs == base_bytes {
            if apply {
                let _ = atomic_write_bytes(&live_path, &new_bytes);
                sha_updates.push((rel.to_string(), head.to_string()));
            }
            rows.push(Json::Obj(vec![
                ("name".to_string(), Json::Str(stem.to_string())),
                ("path".to_string(), Json::Str(rel.to_string())),
                ("state".to_string(), Json::Str("update".to_string())),
            ]));
            continue;
        }

        let (merged, clean) = merge_file(&theirs, &base_bytes, &new_bytes);
        if clean {
            if apply {
                let _ = atomic_write_bytes(&live_path, &merged);
                sha_updates.push((rel.to_string(), head.to_string()));
            }
            rows.push(Json::Obj(vec![
                ("name".to_string(), Json::Str(stem.to_string())),
                ("path".to_string(), Json::Str(rel.to_string())),
                ("state".to_string(), Json::Str("merged".to_string())),
            ]));
        } else if take.contains(rel) {
            if apply {
                let _ = atomic_write_bytes(&live_path, &new_bytes);
                sha_updates.push((rel.to_string(), head.to_string()));
            }
            rows.push(Json::Obj(vec![
                ("name".to_string(), Json::Str(stem.to_string())),
                ("path".to_string(), Json::Str(rel.to_string())),
                ("state".to_string(), Json::Str("update".to_string())),
            ]));
        } else {
            rows.push(Json::Obj(vec![
                ("name".to_string(), Json::Str(stem.to_string())),
                ("path".to_string(), Json::Str(rel.to_string())),
                ("state".to_string(), Json::Str("conflict".to_string())),
            ]));
            conflicts.push(rel.to_string());
        }
    }

    (rows, conflicts, sha_updates)
}

pub fn sync_code(clone: &Path, config_root: &Path, head: &str, apply: bool) -> bool {
    let protected_set: HashSet<&str> = PROTECTED.iter().copied().collect();
    let mut changed = false;
    for rel in tracked_config_files(clone, head) {
        if protected_set.contains(rel.as_str()) {
            continue;
        }
        let new_bytes = match show_at(clone, head, &rel) {
            Some(b) => b,
            None => continue,
        };
        let live_path = target_path(config_root, &rel);
        let current_bytes = fs::read(&live_path).ok();
        if current_bytes.as_deref() == Some(&new_bytes) {
            continue;
        }
        changed = true;
        if apply {
            let _ = atomic_write_bytes(&live_path, &new_bytes);
        }
    }
    changed
}

pub fn baseline_modules(manifest: &mut Json, head: &str) {
    if let Json::Obj(pairs) = manifest {
        if let Some((_, modules_val)) = pairs.iter_mut().find(|(k, _)| k == "modules") {
            if let Json::Obj(mods) = modules_val {
                for &rel in PROTECTED {
                    if let Some((_, v)) = mods.iter_mut().find(|(k, _)| k == rel) {
                        *v = Json::Str(head.to_string());
                    } else {
                        mods.push((rel.to_string(), Json::Str(head.to_string())));
                    }
                }
            }
        }
    }
}

pub fn baseline(config_root: &Path, mut sha: Option<String>) -> Json {
    if sha.is_none() {
        let clone = data_dir();
        if clone.join(".git").exists() {
            if let Ok(head) = origin_head(&clone) {
                sha = Some(head);
            }
        }
    }
    let sha = match sha {
        Some(s) if !s.is_empty() => s,
        _ => return error_result("error", "baseline needs --sha"),
    };
    if is_devmode(config_root) {
        return Json::Obj(vec![
            ("status".to_string(), Json::Str("devmode".to_string())),
            ("syncedSha".to_string(), Json::Str(String::new())),
        ]);
    }
    if manifest_path().exists() {
        if let Ok(m) = load_manifest() {
            let s = m.get("syncedSha").and_then(Json::as_str).unwrap_or(&sha).to_string();
            return Json::Obj(vec![
                ("status".to_string(), Json::Str("kept".to_string())),
                ("syncedSha".to_string(), Json::Str(s)),
            ]);
        }
        return Json::Obj(vec![
            ("status".to_string(), Json::Str("kept".to_string())),
            ("syncedSha".to_string(), Json::Str(sha)),
        ]);
    }
    let mut modules = Vec::new();
    for &rel in PROTECTED {
        modules.push((rel.to_string(), Json::Str(sha.clone())));
    }
    let manifest = Json::Obj(vec![
        ("syncedSha".to_string(), Json::Str(sha.clone())),
        ("modules".to_string(), Json::Obj(modules)),
    ]);
    let _ = save_manifest(&manifest);
    Json::Obj(vec![
        ("status".to_string(), Json::Str("baselined".to_string())),
        ("syncedSha".to_string(), Json::Str(sha)),
    ])
}

// ── Dependency detection & installation ──────────────────────────────────────

pub fn detect_family() -> &'static str {
    let content = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut ids = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let clean_v = v.trim().trim_matches('"').trim_matches('\'').to_lowercase();
            if k == "ID" {
                ids.push(clean_v);
            } else if k == "ID_LIKE" {
                for token in clean_v.split_whitespace() {
                    ids.push(token.to_string());
                }
            }
        }
    }
    for id in &ids {
        if ARCH_IDS.contains(&id.as_str()) {
            return "arch";
        }
        if DEBIAN_IDS.contains(&id.as_str()) {
            return "debian";
        }
        if FEDORA_IDS.contains(&id.as_str()) {
            return "fedora";
        }
        if SUSE_IDS.contains(&id.as_str()) {
            return "suse";
        }
        if GENTOO_IDS.contains(&id.as_str()) {
            return "gentoo";
        }
    }
    "unknown"
}

pub fn pkg_manifest_at(clone: &Path, sha: &str) -> Option<Json> {
    let out = Command::new("git")
        .args(["-C", &clone.to_string_lossy(), "show", &format!("{sha}:installer/packages.json")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    json::parse(&text).ok()
}

pub fn pkg_installed(name: &str, family: &str) -> bool {
    match family {
        "arch" => {
            let res = Command::new("pacman").args(["-T", name]).output();
            if let Ok(o) = res {
                if o.status.success() {
                    return true;
                }
            }
            let res_bin = Command::new("pacman").args(["-T", &format!("{name}-bin")]).output();
            if let Ok(o) = res_bin {
                if o.status.success() {
                    return true;
                }
            }
            false
        }
        "debian" => {
            let res = Command::new("dpkg-query")
                .args(["-W", "-f=${Status}", name])
                .output();
            if let Ok(o) = res {
                String::from_utf8_lossy(&o.stdout).contains("install ok installed")
            } else {
                false
            }
        }
        "gentoo" => {
            Command::new("portageq")
                .args(["has_version", "/", name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
        "fedora" | "suse" => {
            Command::new("rpm")
                .args(["-q", name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
        _ => false,
    }
}

pub fn detect_missing_deps(clone: &Path, head: &str) -> Vec<Json> {
    let family = detect_family();
    if family == "unknown" {
        return Vec::new();
    }
    let manifest = match pkg_manifest_at(clone, head) {
        Some(m) => m,
        None => return Vec::new(),
    };
    let packages = match manifest.get("packages").and_then(Json::as_arr) {
        Some(pkgs) => pkgs,
        None => return Vec::new(),
    };
    let mut missing = Vec::new();
    for pkg in packages {
        if pkg.get("group").and_then(Json::as_str) != Some("core") {
            continue;
        }
        let names = pkg.get("names");
        let name = names.and_then(|n| n.get(family)).and_then(Json::as_str);
        let name = match name {
            Some(n) => n,
            None => continue,
        };
        if pkg_installed(name, family) {
            continue;
        }
        let id = pkg.get("id").and_then(Json::as_str).unwrap_or("");
        let desc = pkg.get("desc").and_then(Json::as_str).unwrap_or("");
        let group = pkg.get("group").and_then(Json::as_str).unwrap_or("core");
        missing.push(Json::Obj(vec![
            ("id".to_string(), Json::Str(id.to_string())),
            ("name".to_string(), Json::Str(name.to_string())),
            ("desc".to_string(), Json::Str(desc.to_string())),
            ("group".to_string(), Json::Str(group.to_string())),
        ]));
    }
    missing
}

pub fn rebuild_cli(clone: &Path, base: Option<&str>, head: &str, apply: bool) -> Option<String> {
    if !apply || base.is_none() {
        return None;
    }
    let b = base.unwrap();
    let diff = Command::new("git")
        .args(["-C", &clone.to_string_lossy(), "diff", "--quiet", b, head, "--", "cli/"])
        .status();
    if let Ok(st) = diff {
        if st.success() {
            return None;
        }
    }
    if Command::new("cargo").arg("--version").output().is_err() {
        return Some(
            "this update ships CLI changes but cargo is not on PATH; \
             re-run the installer (it bootstraps rust) or install rustup, \
             then update again".to_string()
        );
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let target = Path::new(&home).join(".cache/ricelin/build/xiu-target");
    let bin_dir = Path::new(&home).join(".local/bin");
    let build = Command::new("cargo")
        .args(["build", "--release", "--manifest-path"])
        .arg(clone.join("cli/Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .output();
    let build_out = match build {
        Ok(o) => o,
        Err(e) => return Some(format!("cli rebuild failed: {e}")),
    };
    if !build_out.status.success() {
        let stderr = String::from_utf8_lossy(&build_out.stderr);
        let tail = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("cargo build error");
        return Some(format!("cli rebuild failed: {tail}"));
    }
    let _ = fs::create_dir_all(&bin_dir);
    let install = Command::new("install")
        .args(["-m755"])
        .arg(target.join("release/xiu"))
        .arg(bin_dir.join("xiu"))
        .output();
    match install {
        Ok(o) if o.status.success() => None,
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            Some(format!("cli install failed: {stderr}"))
        }
        Err(e) => Some(format!("cli install failed: {e}")),
    }
}

pub fn error_result(status: &str, message: &str) -> Json {
    Json::Obj(vec![
        ("status".to_string(), Json::Str(status.to_string())),
        ("behind".to_string(), Json::Num(0.0)),
        ("fromDate".to_string(), Json::Str(String::new())),
        ("toDate".to_string(), Json::Str(String::new())),
        ("version".to_string(), Json::Str(String::new())),
        ("changelog".to_string(), Json::Arr(Vec::new())),
        ("codeChanged".to_string(), Json::Bool(false)),
        ("modules".to_string(), Json::Arr(Vec::new())),
        ("conflicts".to_string(), Json::Arr(Vec::new())),
        ("missingDeps".to_string(), Json::Arr(Vec::new())),
        ("depFailures".to_string(), Json::Arr(Vec::new())),
        ("applied".to_string(), Json::Bool(false)),
        ("restartNeeded".to_string(), Json::Bool(false)),
        ("error".to_string(), Json::Str(message.to_string())),
    ])
}

pub fn run(
    mode: &str,
    remote: &str,
    config_root: &Path,
    take: HashSet<String>,
    _install_ids: HashSet<String>,
) -> Json {
    if is_devmode(config_root) {
        return Json::Obj(vec![
            ("status".to_string(), Json::Str("devmode".to_string())),
            ("behind".to_string(), Json::Num(0.0)),
            ("fromDate".to_string(), Json::Str(String::new())),
            ("toDate".to_string(), Json::Str(String::new())),
            ("version".to_string(), Json::Str(String::new())),
            ("changelog".to_string(), Json::Arr(Vec::new())),
            ("codeChanged".to_string(), Json::Bool(false)),
            ("modules".to_string(), Json::Arr(Vec::new())),
            ("conflicts".to_string(), Json::Arr(Vec::new())),
            ("missingDeps".to_string(), Json::Arr(Vec::new())),
            ("depFailures".to_string(), Json::Arr(Vec::new())),
            ("applied".to_string(), Json::Bool(false)),
            ("restartNeeded".to_string(), Json::Bool(false)),
            ("error".to_string(), Json::Null),
        ]);
    }

    let apply = mode == "apply";
    let clone = match ensure_clone(remote, true) {
        Ok(c) => c,
        Err((st, err)) => return error_result(&st, &err),
    };

    let mut manifest = match load_manifest() {
        Ok(m) => m,
        Err(e) => {
            return error_result(
                "error",
                &format!("update manifest is corrupt and was left untouched: {e}"),
            )
        }
    };

    let head = match origin_head(&clone) {
        Ok(h) => h,
        Err(e) => return error_result("error", &e),
    };

    let mut base = manifest
        .get("syncedSha")
        .and_then(Json::as_str)
        .map(|s| s.to_string());
    if let Some(ref b) = base {
        if !sha_known(&clone, b) {
            base = None;
        }
    }
    let first_run = base.is_none();

    let changelog: Vec<Json> = extract_changelog(&clone, base.as_deref(), &head)
        .into_iter()
        .map(Json::Str)
        .collect();
    let behind = behind_count(&clone, base.as_deref(), &head);
    let from_date = base
        .as_deref()
        .map(|b| commit_date(&clone, b))
        .unwrap_or_else(|| commit_date(&clone, &head));
    let to_date = commit_date(&clone, &head);
    let version = format!("{} {}", short_sha(&clone, &head), to_date);

    let missing = detect_missing_deps(&clone, &head);
    let mut dep_failures: Vec<Json> = Vec::new();

    if first_run {
        let code_changed = sync_code(&clone, config_root, &head, apply);
        let (rows, conflicts, sha_updates) =
            reconcile_protected(&clone, config_root, &manifest, &head, apply, &take);
        if apply {
            baseline_modules(&mut manifest, &head);
            if let Json::Obj(ref mut pairs) = manifest {
                if let Some((_, modules_val)) = pairs.iter_mut().find(|(k, _)| k == "modules") {
                    if let Json::Obj(mods) = modules_val {
                        for (k, v) in sha_updates {
                            if let Some((_, ex_v)) = mods.iter_mut().find(|(mk, _)| mk == &k) {
                                *ex_v = Json::Str(v);
                            } else {
                                mods.push((k, Json::Str(v)));
                            }
                        }
                    }
                }
                if let Some((_, sync_val)) = pairs.iter_mut().find(|(k, _)| k == "syncedSha") {
                    *sync_val = Json::Str(head.clone());
                }
            }
            let _ = save_manifest(&manifest);
        }
        let protected_changed = rows.iter().any(|r| {
            r.get("state")
                .and_then(Json::as_str)
                .map(|s| s == "update" || s == "merged")
                .unwrap_or(false)
        });
        return Json::Obj(vec![
            ("status".to_string(), Json::Str("ok".to_string())),
            ("behind".to_string(), Json::Num(behind as f64)),
            ("fromDate".to_string(), Json::Str(from_date)),
            ("toDate".to_string(), Json::Str(to_date)),
            ("version".to_string(), Json::Str(version)),
            ("changelog".to_string(), Json::Arr(changelog)),
            ("codeChanged".to_string(), Json::Bool(code_changed)),
            ("modules".to_string(), Json::Arr(rows)),
            (
                "conflicts".to_string(),
                Json::Arr(conflicts.into_iter().map(Json::Str).collect()),
            ),
            ("missingDeps".to_string(), Json::Arr(missing)),
            ("depFailures".to_string(), Json::Arr(dep_failures)),
            ("applied".to_string(), Json::Bool(apply)),
            (
                "restartNeeded".to_string(),
                Json::Bool(code_changed || protected_changed),
            ),
            ("error".to_string(), Json::Null),
        ]);
    }

    if apply {
        backup_protected(config_root);
    }
    let (rows, conflicts, sha_updates) =
        reconcile_protected(&clone, config_root, &manifest, &head, apply, &take);
    let code_changed = sync_code(&clone, config_root, &head, apply);

    if let Some(cli_err) = rebuild_cli(&clone, base.as_deref(), &head, apply) {
        dep_failures.push(Json::Obj(vec![
            ("id".to_string(), Json::Str("xiu-cli".to_string())),
            ("error".to_string(), Json::Str(cli_err)),
        ]));
    }

    if apply {
        if let Json::Obj(ref mut pairs) = manifest {
            if let Some((_, modules_val)) = pairs.iter_mut().find(|(k, _)| k == "modules") {
                if let Json::Obj(mods) = modules_val {
                    for (k, v) in sha_updates {
                        if let Some((_, ex_v)) = mods.iter_mut().find(|(mk, _)| mk == &k) {
                            *ex_v = Json::Str(v);
                        } else {
                            mods.push((k, Json::Str(v)));
                        }
                    }
                }
            }
            if let Some((_, sync_val)) = pairs.iter_mut().find(|(k, _)| k == "syncedSha") {
                *sync_val = Json::Str(head.clone());
            }
        }
        let _ = save_manifest(&manifest);
    }

    let protected_changed = rows.iter().any(|r| {
        r.get("state")
            .and_then(Json::as_str)
            .map(|s| s == "update" || s == "merged")
            .unwrap_or(false)
    });

    Json::Obj(vec![
        ("status".to_string(), Json::Str("ok".to_string())),
        ("behind".to_string(), Json::Num(behind as f64)),
        ("fromDate".to_string(), Json::Str(from_date)),
        ("toDate".to_string(), Json::Str(to_date)),
        ("version".to_string(), Json::Str(version)),
        ("changelog".to_string(), Json::Arr(changelog)),
        ("codeChanged".to_string(), Json::Bool(code_changed)),
        ("modules".to_string(), Json::Arr(rows)),
        (
            "conflicts".to_string(),
            Json::Arr(conflicts.into_iter().map(Json::Str).collect()),
        ),
        ("missingDeps".to_string(), Json::Arr(missing)),
        ("depFailures".to_string(), Json::Arr(dep_failures)),
        ("applied".to_string(), Json::Bool(apply)),
        (
            "restartNeeded".to_string(),
            Json::Bool(code_changed || protected_changed),
        ),
        ("error".to_string(), Json::Null),
    ])
}
