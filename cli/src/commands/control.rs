//! The control verbs: the old `ricelin` wrapper as subcommands. The pill and
//! the lock are quickshell instances kept alive by watchdogs — the watchdog
//! respawns `qs -c <name> -d` whenever the instance dies. So `stop` has to
//! take the watchdog down too or the surface just comes back, while `restart`
//! leaves the watchdog running and only cycles the instance.

use crate::helpers::{git_out, run_status};
use crate::json;
use crate::ui::{act, ctl_die, gap, note, row, sec, skin};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub const SURFACES: [&str; 2] = ["pill", "lock"];

pub fn home_path(parts: &[&str]) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut path = PathBuf::from(home);
    for part in parts {
        path.push(part);
    }
    path
}

pub fn state_file(name: &str) -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".local", "state"]));
    base.join(name)
}

fn engine_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".config"]));
    base.join("hypr/scripts/xiu-update.py")
}

/// Expand a target argument into the surfaces it names, falling back to
/// `default` when no argument was given.
fn surface_targets(arg: Option<&str>, default: &str) -> Result<Vec<&'static str>, String> {
    let name = match arg {
        Some(a) => a,
        None => default,
    };
    match name {
        "all" => Ok(SURFACES.to_vec()),
        "pill" => Ok(vec!["pill"]),
        "lock" => Ok(vec!["lock"]),
        other => Err(format!("unknown target '{other}' (pill, lock or all)")),
    }
}

fn surface_up(surface: &str) -> bool {
    Command::new("qs")
        .args(["-c", surface, "ipc", "show"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn watchdog_up(surface: &str) -> bool {
    Command::new("pgrep")
        .arg("-f")
        .arg(format!("watchdog.sh {surface}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Launch a detached watchdog. A second one for the same surface hits the
/// flock in watchdog.sh and exits at once, so this is safe when one runs.
fn start_watchdog(surface: &str) {
    let watchdog = home_path(&[".config", "hypr", "scripts", "watchdog.sh"]);
    let _ = Command::new("setsid")
        .arg("sh")
        .arg(watchdog)
        .arg(surface)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// The installed version: a symlinked quickshell dir means a dev checkout
/// (git short sha); otherwise the update engine's manifest records the last
/// applied commit. The manifest moved to state/xiu at the rename; a box that
/// has not run an update since still has it under state/ricelin.
fn installed_version() -> String {
    let qsdir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".config"]))
        .join("quickshell");
    if let Ok(target) = qsdir.read_link() {
        if let Some(sha) = git_out(&target.to_string_lossy(), &["rev-parse", "--short", "HEAD"]) {
            return format!("dev {}", sha.trim());
        }
    }
    for manifest in ["xiu/update.json", "ricelin/update.json"] {
        let path = state_file(manifest);
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(value) = json::parse(&text) {
                if let Some(sha) = value.get("syncedSha").and_then(json::Json::as_str) {
                    if !sha.is_empty() {
                        return sha.chars().take(7).collect();
                    }
                }
            }
        }
    }
    "unknown".to_string()
}

pub fn start(target: Option<&str>) -> i32 {
    let k = skin();
    let targets = match surface_targets(target, "all") {
        Ok(t) => t,
        Err(e) => return ctl_die(&k, &e),
    };
    for surface in targets {
        if watchdog_up(surface) {
            note(&k, &format!("{surface} already running"));
        } else {
            start_watchdog(surface);
            act(&k, "started", surface);
        }
    }
    0
}

pub fn stop(target: Option<&str>) -> i32 {
    let k = skin();
    let targets = match surface_targets(target, "all") {
        Ok(t) => t,
        Err(e) => return ctl_die(&k, &e),
    };
    for surface in targets {
        let _ = Command::new("pkill")
            .arg("-f")
            .arg(format!("watchdog.sh {surface}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("qs")
            .args(["-c", surface, "kill"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        act(&k, "stopped", surface);
    }
    0
}

pub fn restart(target: Option<&str>) -> i32 {
    let k = skin();
    let targets = match surface_targets(target, "pill") {
        Ok(t) => t,
        Err(e) => return ctl_die(&k, &e),
    };
    for surface in targets {
        let had_watchdog = watchdog_up(surface);
        let _ = Command::new("qs")
            .args(["-c", surface, "kill"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        thread::sleep(Duration::from_millis(400));
        if had_watchdog {
            if !surface_up(surface) {
                let _ = Command::new("qs").args(["-c", surface, "-d"]).status();
            }
        } else {
            start_watchdog(surface);
        }
        act(&k, "restarted", surface);
    }
    0
}

pub fn log(target: Option<&str>, rest: &[String]) -> i32 {
    // The target positional merges with the viewer's own flags: a leading
    // "pill"/"lock" selects the surface, anything else defaults to pill.
    let mut rest: Vec<String> = rest.to_vec();
    if let Some(t) = target {
        if t == "pill" || t == "lock" {
            rest.insert(0, t.to_string());
        }
    }
    let surface = match rest.first().map(String::as_str) {
        Some("pill") | Some("lock") => rest.remove(0),
        _ => "pill".to_string(),
    };
    let mut c = Command::new("qs");
    c.args(["-c", &surface, "log", "-f"]);
    for arg in &rest {
        c.arg(arg);
    }
    run_status(&mut c)
}

pub fn status() -> i32 {
    let k = skin();
    sec(&k, "xiu");
    gap(&k);
    for surface in SURFACES {
        let (mark, label, state) = if surface_up(surface) {
            (k.flame, k.cream, "running")
        } else {
            (k.faint, k.dim, "stopped")
        };
        let rst = k.rst;
        let wd = if watchdog_up(surface) {
            format!("{}watchdog up{}", k.dim, rst)
        } else {
            format!("{}no watchdog{}", k.faint, rst)
        };
        row(&k, &format!("{mark}●{rst} {label}{surface}{rst}   {label}{state}{rst}   {wd}"));
    }
    gap(&k);
    let rst = k.rst;
    row(
        &k,
        &format!("{}version{rst}   {}{}{}", k.dim, k.cream, installed_version(), rst),
    );
    gap(&k);
    0
}

/// Run the update engine and parse its single-JSON-object output. The engine
/// always exits 0 with an error object for handled failures, so a non-zero
/// exit here is a hard failure (python missing, engine crashed).
fn engine_call(mode: &str, extra: &[&str]) -> Result<json::Json, String> {
    let engine = engine_path();
    if !engine.is_file() {
        return Err(format!("update engine missing ({})", engine.display()));
    }
    let output = Command::new("python3")
        .arg(&engine)
        .arg(mode)
        .args(extra)
        .output()
        .map_err(|e| format!("python3 unavailable ({e})"))?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        if let Ok(value) = json::parse(&text) {
            return Ok(value);
        }
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("update {mode} produced no result")
    } else {
        stderr
    })
}

pub fn update() -> i32 {
    let k = skin();
    let check = match engine_call("check", &[]) {
        Ok(v) => v,
        Err(e) => return ctl_die(&k, &e),
    };
    let status = check.get("status").and_then(json::Json::as_str).unwrap_or("error");
    match status {
        "devmode" => {
            note(&k, "dev install, update with git pull");
            return 0;
        }
        "offline" => return ctl_die(&k, "offline, can't reach the update remote"),
        "noclone" => {
            return ctl_die(&k, "no update clone yet, open the in-app updater once first")
        }
        "ok" => {}
        other => {
            let error = check.get("error").and_then(json::Json::as_str).unwrap_or("update check failed");
            return ctl_die(&k, &format!("update {other}: {error}"));
        }
    }

    let behind = check.get("behind").and_then(json::Json::as_i64).unwrap_or(0);
    let version = check.get("version").and_then(json::Json::as_str).unwrap_or("");
    if behind == 0 {
        act(&k, "up to date", version);
        return 0;
    }

    sec(&k, &format!("{behind} commit(s) behind"));
    gap(&k);
    let from = check.get("fromDate").and_then(json::Json::as_str).unwrap_or("");
    let to = check.get("toDate").and_then(json::Json::as_str).unwrap_or("");
    if !from.is_empty() || !to.is_empty() {
        row(&k, &format!("{}{from} -> {to}{}", k.faint, k.rst));
    }
    if let Some(lines) = check.get("changelog").and_then(json::Json::as_arr) {
        for line in lines {
            if let Some(text) = line.as_str() {
                row(&k, &format!("{}·{} {}{}{}", k.flame, k.rst, k.cream, text, k.rst));
            }
        }
    }
    let deps: Vec<&str> = check
        .get("missingDeps")
        .and_then(json::Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|d| d.get("name").and_then(json::Json::as_str))
                .collect()
        })
        .unwrap_or_default();
    if !deps.is_empty() {
        row(&k, &format!("{}new packages{}   {}{}{}", k.dim, k.rst, k.cream, deps.join(", "), k.rst));
    }
    let conflicts = check.get("conflicts").and_then(json::Json::as_arr).map(<[json::Json]>::len).unwrap_or(0);
    if conflicts > 0 {
        row(
            &k,
            &format!("{}{conflicts} edited file(s) clash with upstream, kept as you have them{}", k.faint, k.rst),
        );
    }
    gap(&k);

    print!("  {}▌{} {}apply update?{} {}[Y/n]{} ", k.verm, k.rst, k.cream, k.rst, k.dim, k.rst);
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        answer.clear();
    }
    match answer.trim() {
        "" | "y" | "Y" | "yes" | "YES" => {}
        _ => {
            note(&k, "skipped");
            return 0;
        }
    }

    let ids: Vec<String> = check
        .get("missingDeps")
        .and_then(json::Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|d| d.get("id").and_then(json::Json::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let joined = ids.join(",");
    let result = if !joined.is_empty() {
        engine_call("apply", &["--install-deps", &joined])
    } else {
        engine_call("apply", &[])
    };
    let result = match result {
        Ok(v) => v,
        Err(e) => return ctl_die(&k, &format!("apply failed: {e}")),
    };

    if let Some(failures) = result.get("depFailures").and_then(json::Json::as_arr) {
        if !failures.is_empty() {
            note(&k, "some packages didn't install:");
            for failure in failures {
                let id = failure.get("id").and_then(json::Json::as_str).unwrap_or("?");
                let error = failure.get("error").and_then(json::Json::as_str).unwrap_or("");
                row(&k, &format!("{}·{} {}{}: {}{}", k.faint, k.rst, k.dim, id, error, k.rst));
            }
        }
    }

    if result.get("restartNeeded").and_then(json::Json::as_bool) == Some(true) {
        // No arg: surface_targets takes its default ("all"), not an empty
        // name — an empty string used to be rejected as an unknown target so
        // the post-update restart never ran.
        let _ = restart(None);
    }
    act(&k, "updated to", version);
    0
}

pub fn uninstall() -> i32 {
    let k = skin();
    let share = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".local", "share"]));
    let candidates = [
        share.join("xiu/installer/xiu_install.py"),
        share.join("ricelin/installer/xiu_install.py"),
        share.join("ricelin/installer/ricelin_install.py"),
    ];
    let installer = candidates.iter().find(|p| p.is_file());
    let installer = match installer {
        Some(p) => p,
        None => {
            return ctl_die(
                &k,
                "installer not found; remove ~/.config/hypr and ~/.config/quickshell by hand, your originals sit next to them as .bak",
            )
        }
    };
    let _ = Command::new("python3").arg(installer).arg("--uninstall").status();
    0
}
