//! xiu — the shell's command line.
//!
//! One binary wrapping the surfaces that already exist: the pill's Quickshell
//! IPC targets, the hypr scripts and the small tools the rice ships. No
//! dependency on anything outside stdlib so `cargo build` works offline; the
//! installer puts the binary on PATH and the keybinds call it.
//!
//! `xiu shell` is a raw passthrough to `qs -c pill ipc call`, every other
//! subcommand is a thin, typed convenience over the same socket. The control
//! verbs (restart/start/stop/log/status/update/uninstall) are the old `ricelin`
//! shell wrapper reborn as subcommands, so one command owns the whole rice.

mod json;

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(run(&args));
}

fn run(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            help();
            0
        }
        Some("version") | Some("--version") | Some("-v") => {
            println!("xiu {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some("shell") => shell(&args[1..]),
        Some("open") => open(&args[1..]),
        Some("wallpaper") => wallpaper(&args[1..]),
        Some("mpris") => mpris(&args[1..]),
        Some("record") => record(&args[1..]),
        Some("screenshot") => passthrough("rishot", &args[1..]),
        Some("clipboard") => ipc_call("pill", &["clipboard", ""]),
        Some("notifs") => notifs(&args[1..]),
        Some("gamemode") => gamemode(&args[1..]),
        Some("scheme") => scheme(&args[1..]),
        Some("browser") => browser(&args[1..]),
        Some("emoji") => emoji(&args[1..]),
        Some("check") => check(&args[1..]),
        Some("restart") => ctl_restart(&args[1..]),
        Some("start") => ctl_start(&args[1..]),
        Some("stop") => ctl_stop(&args[1..]),
        Some("log") => ctl_log(&args[1..]),
        Some("status") => ctl_status(),
        Some("update") => ctl_update(&args[1..]),
        Some("uninstall") => ctl_uninstall(),
        Some(other) => {
            eprintln!("xiu: unknown command '{other}' (see `xiu help`)");
            2
        }
    }
}

fn help() {
    println!(
        "xiu — the shell's command line

USAGE:
    xiu <COMMAND> [OPTIONS...]

SHELL:
    shell [TARGET FN [ARGS...]]   raw passthrough to `qs -c pill ipc call`
                                  (bare: ipc show, -k: kill the shell)
    open <SURFACE>                open a pill surface (launcher, power, link,
                                  mixer, wallpaper, clipboard, gameMode, ...)
    wallpaper [-p] [-l] [-f PATH] current wallpaper (-p), list (-l), set (-f), random
    mpris [ACTION]                active (default), play, next, prev, stop, list
    record [-s]                   quick-record on the focused monitor (-s: stop)
    screenshot [ARGS...]          rishot
    clipboard                     open the pill's clipboard surface
    notifs [clear|seen]           clear (default) or mark notifications seen
    gamemode [ACTION]             status (default), on, off, toggle
    scheme [ACTION]               list, get, set <preset|dynamic> [-v VARIANT],
                                  preview <wallpaper> (engine: wallcolors.py)
    browser                       apply the palette policy to Brave/Chromium
    emoji [-p] [-l] [QUERY...]    copy the matching emoji (-p: ask, -l: list)
    check                         drift and health report for the install

CONTROL (the old ricelin wrapper, as subcommands):
    restart [pill|lock|all]       cycle a surface               (default: pill)
    start   [pill|lock|all]       start the watchdog            (default: all)
    stop    [pill|lock|all]       stop watchdog and surface     (default: all)
    log     [pill|lock] [ARGS...] follow the quickshell log     (default: pill)
    update                        check, show changelog, apply, restart
    status                        what's running and the installed version
    uninstall                     remove the configs, restore backups

    version                       print the version"
    );
}

/// `xiu check` — drift and health report for the install. Three sections:
/// which of the deploy set's main config dirs are in place and ours, how far
/// the xiu clone drifts from its remotes (as of the last fetch — checking
/// never touches the network), and whether the binaries the keybinds and the
/// palette pipeline shell out to are on PATH. Exits 1 when a core tool is
/// missing or a tracked config is absent, so scripts can gate on it.
fn check(_args: &[String]) -> i32 {
    let home = std::env::var("HOME").unwrap_or_default();
    let cfg = format!("{home}/.config");

    println!("configs:");
    let dirs = [
        ("hypr", "hypr"),
        ("quickshell", "quickshell"),
        ("foot", "foot"),
        ("fish", "fish"),
        ("fastfetch", "fastfetch"),
    ];
    let mut configs_ok = true;
    for (name, dir) in dirs {
        let path = format!("{cfg}/{dir}");
        // Either marker counts: .xiu-managed is what deploys stamp now, a
        // pre-rename .ricelin-managed still marks a box we own.
        let managed = [".xiu-managed", ".ricelin-managed"]
            .iter()
            .any(|m| std::path::Path::new(&format!("{path}/{m}")).is_file());
        let state = if managed {
            "managed"
        } else if std::path::Path::new(&path).is_dir() {
            configs_ok = false;
            "present (not ours)"
        } else {
            configs_ok = false;
            "absent"
        };
        println!("  {name:<12} {state}");
    }

    println!("drift:");
    let candidates: Vec<String> = [
        std::env::var("XIU_REPO").ok().filter(|s| !s.is_empty()),
        Some(format!("{home}/.local/share/xiu")),
        Some(format!("{home}/.local/share/ricelin")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let repo = candidates.iter().find(|p| {
        std::path::Path::new(&format!("{p}/.git")).exists()
            || std::path::Path::new(&format!("{p}/installer/deploy.py")).exists()
    });
    match repo {
        Some(repo) => {
            println!("  repo: {repo}");
            for remote in ["origin", "upstream"] {
                let behind = git_out(repo, &["rev-list", "--count", &format!("HEAD..{remote}/main")]);
                if let Some(b) = behind.filter(|s| !s.trim().is_empty()) {
                    let ahead = git_out(
                        repo,
                        &["rev-list", "--count", &format!("{remote}/main..HEAD")],
                    )
                    .unwrap_or_else(|| "?".to_string());
                    println!(
                        "  vs {remote:<9} behind {}, ahead {} (as of last fetch)",
                        b.trim(),
                        ahead.trim()
                    );
                } else {
                    println!("  vs {remote:<9} no tracking data (run: git fetch {remote})");
                }
            }
        }
        None => println!("  no xiu clone found (set XIU_REPO to point at one)"),
    }

    println!("health:");
    let tools = [
        "Hyprland", "qs", "foot", "clipvault", "matugen", "swww", "rishot",
    ];
    let missing: Vec<&str> = tools.iter().copied().filter(|t| !on_path(t)).collect();
    if missing.is_empty() {
        println!("  all core tools on PATH");
    } else {
        for t in &missing {
            println!("  {t} not found on PATH");
        }
    }

    if configs_ok && missing.is_empty() {
        0
    } else {
        1
    }
}

// ── Control verbs: the old `ricelin` wrapper as subcommands ──────────────────
//
// The pill and the lock are quickshell instances kept alive by watchdogs: the
// watchdog respawns `qs -c <name> -d` whenever the instance dies. So `stop`
// has to take the watchdog down too or the surface just comes back, while
// `restart` leaves the watchdog running and only cycles the instance.

const SURFACES: [&str; 2] = ["pill", "lock"];

fn home_path(parts: &[&str]) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut path = PathBuf::from(home);
    for part in parts {
        path.push(part);
    }
    path
}

fn state_file(name: &str) -> PathBuf {
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

/// The ember skin the installer's TUI uses: a vermilion edge marker, a quiet
/// connector gutter, the rice's palette. Empty strings when stdout is not a
/// terminal or NO_COLOR is set, so a pipe gets clean plain text.
struct Skin {
    verm: &'static str,
    flame: &'static str,
    cream: &'static str,
    bright: &'static str,
    dim: &'static str,
    faint: &'static str,
    rst: &'static str,
}

fn skin() -> Skin {
    let plain = Skin {
        verm: "",
        flame: "",
        cream: "",
        bright: "",
        dim: "",
        faint: "",
        rst: "",
    };
    if !std::io::stdout().is_terminal() || std::env::var_os("NO_COLOR").is_some() {
        return plain;
    }
    Skin {
        verm: "\x1b[38;2;192;68;43m",
        flame: "\x1b[38;2;255;154;100m",
        cream: "\x1b[38;2;230;214;203m",
        bright: "\x1b[38;2;255;246;240m",
        dim: "\x1b[38;2;138;125;116m",
        faint: "\x1b[38;2;111;99;91m",
        rst: "\x1b[0m",
    }
}

fn sec(k: &Skin, title: &str) {
    println!("\n  {}▌{} {}{}{}", k.verm, k.rst, k.cream, title, k.rst);
}

fn gap(k: &Skin) {
    println!("  {}▏{}", k.faint, k.rst);
}

fn row(k: &Skin, colored: &str) {
    println!("  {}▏{}  {}", k.faint, k.rst, colored);
}

fn act(k: &Skin, verb: &str, what: &str) {
    println!("  {}▫{} {}{}{} {}{}{}", k.flame, k.rst, k.cream, verb, k.rst, k.bright, what, k.rst);
}

fn note(k: &Skin, text: &str) {
    println!("  {}▫{} {}{}{}", k.faint, k.rst, k.dim, text, k.rst);
}

fn ctl_die(k: &Skin, text: &str) -> i32 {
    eprintln!("  {}▌{} {}{}{}", k.verm, k.rst, k.verm, text, k.rst);
    1
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

fn ctl_start(args: &[String]) -> i32 {
    let k = skin();
    let targets = match surface_targets(args.first().map(String::as_str), "all") {
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

fn ctl_stop(args: &[String]) -> i32 {
    let k = skin();
    let targets = match surface_targets(args.first().map(String::as_str), "all") {
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

fn ctl_restart(args: &[String]) -> i32 {
    let k = skin();
    let targets = match surface_targets(args.first().map(String::as_str), "pill") {
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

fn ctl_log(args: &[String]) -> i32 {
    let mut rest: Vec<String> = args.to_vec();
    let surface = match rest.first().map(String::as_str) {
        Some("pill") | Some("lock") => rest.remove(0),
        _ => "pill".to_string(),
    };
    let mut cmd = Command::new("qs");
    cmd.args(["-c", &surface, "log", "-f"]);
    for arg in &rest {
        cmd.arg(arg);
    }
    run_status(&mut cmd)
}

fn ctl_status() -> i32 {
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

fn ctl_update(_args: &[String]) -> i32 {
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
        let _ = ctl_restart(&[String::new()]);
    }
    act(&k, "updated to", version);
    0
}

fn ctl_uninstall() -> i32 {
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

/// `xiu open <surface>` — the generic surface opener the keybinds use,
/// replacing open-surface.sh: one IPC call, the focused monitor's pill.
fn open(args: &[String]) -> i32 {
    let surface = match args.first().map(String::as_str) {
        Some(s) if !s.is_empty() => s,
        _ => {
            eprintln!("xiu open: needs a surface name (launcher, power, link, mixer, wallpaper, clipboard, gameMode)");
            return 2;
        }
    };
    ipc_call("pill", &[surface, ""])
}

/// One git command run quietly in `repo`; stdout on success, None on any
/// failure so callers can fall through to the "no data" wording.
fn git_out(repo: &str, args: &[&str]) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

/// A PATH probe good enough for a report: the file exists in a PATH dir.
fn on_path(bin: &str) -> bool {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|dir| std::path::Path::new(&format!("{dir}/{bin}")).is_file())
}

/// `qs -c pill ipc call <target> <fn> [args...]` — the shell's whole command surface.
fn ipc_call(target: &str, args: &[&str]) -> i32 {
    run_status(Command::new("qs").args(["-c", "pill", "ipc", "call", target]).args(args))
}

fn passthrough(bin: &str, args: &[String]) -> i32 {
    run_status(Command::new(bin).args(args))
}

fn run_status(cmd: &mut Command) -> i32 {
    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("xiu: {e}");
            127
        }
    }
}

/// The focused monitor's name, resolved exactly like record.sh: one
/// activeworkspace round trip, the monitor field fished out by hand.
fn focused_monitor() -> String {
    if let Ok(out) = Command::new("hyprctl").args(["activeworkspace", "-j"]).output() {
        let s = String::from_utf8_lossy(&out.stdout);
        if let Some(i) = s.find("\"monitor\":\"") {
            let rest = &s[i + "\"monitor\":\"".len()..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
    }
    String::new()
}

fn shell(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-k" || a == "--kill") {
        return run_status(Command::new("qs").args(["-c", "pill", "kill"]));
    }
    if args.is_empty() {
        return run_status(Command::new("qs").args(["-c", "pill", "ipc", "show"]));
    }
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    run_status(Command::new("qs").args(["-c", "pill", "ipc", "call"]).args(&rest))
}

fn wallpaper(args: &[String]) -> i32 {
    let mut path: Option<String> = None;
    let mut print_current = false;
    let mut list = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--print" => print_current = true,
            "-l" | "--list" => list = true,
            "-r" | "--random" => {}
            "-f" | "--file" => {
                i += 1;
                match args.get(i) {
                    Some(p) => path = Some(p.clone()),
                    None => {
                        eprintln!("xiu wallpaper: -f needs a path");
                        return 2;
                    }
                }
            }
            other => {
                eprintln!("xiu wallpaper: unexpected argument '{other}'");
                return 2;
            }
        }
        i += 1;
    }
    if list {
        return ipc_call("wallpaper", &["list"]);
    }
    if let Some(p) = path {
        return ipc_call("wallpaper", &["set", &p]);
    }
    if print_current {
        return ipc_call("wallpaper", &["get"]);
    }
    ipc_call("wallpaper", &["random"])
}

fn mpris(args: &[String]) -> i32 {
    match args.first().map(String::as_str).unwrap_or("active") {
        "play" | "pause" | "playPause" | "play-pause" | "toggle" => ipc_call("mpris", &["playPause"]),
        "next" => ipc_call("mpris", &["next"]),
        "prev" | "previous" => ipc_call("mpris", &["previous"]),
        "stop" => ipc_call("mpris", &["stop"]),
        "list" => ipc_call("mpris", &["list"]),
        "active" | "status" => ipc_call("mpris", &["active"]),
        other => {
            eprintln!("xiu mpris: unknown action '{other}'");
            2
        }
    }
}

fn record(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-s" || a == "--stop") {
        return ipc_call("recorder", &["stop"]);
    }
    let mon = focused_monitor();
    ipc_call("pill", &["quickRecord", &mon])
}

fn notifs(args: &[String]) -> i32 {
    match args.first().map(String::as_str).unwrap_or("clear") {
        "clear" => ipc_call("notifs", &["clear"]),
        "seen" => ipc_call("notifs", &["seen"]),
        other => {
            eprintln!("xiu notifs: unknown action '{other}'");
            2
        }
    }
}

fn gamemode(args: &[String]) -> i32 {
    match args.first().map(String::as_str).unwrap_or("status") {
        "status" => ipc_call("gamemode", &["status"]),
        "on" => ipc_call("gamemode", &["on"]),
        "off" => ipc_call("gamemode", &["off"]),
        "toggle" => ipc_call("gamemode", &["toggle"]),
        other => {
            eprintln!("xiu gamemode: unknown action '{other}'");
            2
        }
    }
}

/// Brave and Chromium read their toolbar color from a managed policy under
/// /etc, which needs root. The palette pipeline keeps the payload fresh in
/// ~/.config/xiu/browser-theme.json; this copies it out with a
/// non-interactive sudo when possible and prints the commands otherwise.
fn browser(_args: &[String]) -> i32 {
    let home = std::env::var("HOME").unwrap_or_default();
    let payload = format!("{home}/.config/xiu/browser-theme.json");
    if !std::path::Path::new(&payload).is_file() {
        eprintln!("xiu browser: no palette payload yet (run a wallpaper change or `xiu scheme set` first)");
        return 1;
    }
    let targets = ["/etc/brave/policies/managed/xiu.json", "/etc/chromium/policies/managed/xiu.json"];
    let mut failed = false;
    for target in targets {
        let status = Command::new("sudo")
            .args(["-n", "install", "-m", "644", "-D", &payload, target])
            .status();
        match status {
            Ok(s) if s.success() => println!("applied → {target}"),
            _ => {
                failed = true;
                eprintln!("needs root; run: sudo install -m 644 -D {payload} {target}");
            }
        }
    }
    if failed {
        1
    } else {
        println!("restart the browser to pick the new color up");
        0
    }
}

/// The palette engine is wallcolors.py; the CLI is its front door. Scheme
/// state survives wallpaper changes in its own state file, and an explicit
/// change flips the pill's paletteMode so the shell actually listens.
fn scheme(args: &[String]) -> i32 {
    let home = std::env::var("HOME").unwrap_or_default();
    let script = format!("{home}/.config/hypr/scripts/wallcolors.py");
    let run = |flags: Vec<&str>| run_status(Command::new("python3").arg(&script).args(&flags));

    match args.first().map(String::as_str) {
        Some("list") => run(vec!["--list-presets"]),
        Some("get") | None => run(vec!["--state"]),
        Some("preview") => {
            if args.len() < 2 {
                eprintln!("xiu scheme preview: needs a wallpaper path");
                return 2;
            }
            run(vec!["--preview", &args[1]])
        }
        Some("set") => {
            let rest = &args[1..];
            if rest.is_empty() {
                eprintln!("xiu scheme set: needs a preset name, `dynamic`, -v VARIANT, --smart or --no-smart");
                return 2;
            }
            let mut flags: Vec<String> = Vec::new();
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "-v" | "--variant" => {
                        i += 1;
                        match rest.get(i) {
                            Some(v) => {
                                flags.push("--variant".into());
                                flags.push(v.clone());
                            }
                            None => {
                                eprintln!("xiu scheme set: -v needs a variant");
                                return 2;
                            }
                        }
                    }
                    "--smart" | "--no-smart" => flags.push(rest[i].clone()),
                    other if !other.starts_with('-') => {
                        flags.push("--preset".into());
                        flags.push(other.into());
                    }
                    other => {
                        eprintln!("xiu scheme set: unexpected '{other}'");
                        return 2;
                    }
                }
                i += 1;
            }
            let refs: Vec<&str> = flags.iter().map(String::as_str).collect();
            run(refs)
        }
        Some(other) => {
            eprintln!("xiu scheme: unknown action '{other}' (list, get, set, preview)");
            2
        }
    }
}

/// The everyday glyphs plus the composite classics. Pairs of (name, text);
/// a query matches when it appears anywhere in the name.
const EMOJI: &[(&str, &str)] = &[
    ("shrug", "¯\\_(ツ)_/¯"),
    ("tableflip", "(╯°□°)╯︵ ┻━┻"),
    ("unflip", "┬─┬ ノ( ゜-゜ノ)"),
    ("lol", "😂"),
    ("joy", "😂"),
    ("smile", "🙂"),
    ("grin", "😀"),
    ("wink", "😉"),
    ("thinking", "🤔"),
    ("shrugging", "🤷"),
    ("ok", "👌"),
    ("thumbsup", "👍"),
    ("thumbsdown", "👎"),
    ("clap", "👏"),
    ("wave", "👋"),
    ("pray", "🙏"),
    ("muscle", "💪"),
    ("point", "👉"),
    ("eyes", "👀"),
    ("heart", "❤️"),
    ("sparkle", "✨"),
    ("fire", "🔥"),
    ("star", "⭐"),
    ("boom", "💥"),
    ("100", "💯"),
    ("check", "✅"),
    ("cross", "❌"),
    ("question", "❓"),
    ("warning", "⚠️"),
    ("bulb", "💡"),
    ("rocket", "🚀"),
    ("party", "🎉"),
    ("cake", "🎂"),
    ("coffee", "☕"),
    ("pizza", "🍕"),
    ("beer", "🍺"),
    ("moon", "🌙"),
    ("sun", "☀️"),
    ("zap", "⚡"),
    ("snowflake", "❄️"),
    ("bug", "🐛"),
    ("skull", "💀"),
    ("ghost", "👻"),
    ("alien", "👽"),
    ("robot", "🤖"),
    ("cat", "🐱"),
    ("dog", "🐶"),
    ("fox", "🦊"),
    ("panda", "🐼"),
    ("poop", "💩"),
];

fn emoji_matches(query: &str) -> Vec<(&'static str, &'static str)> {
    let q = query.to_lowercase();
    EMOJI.iter().filter(|(name, _)| name.contains(&q)).copied().collect()
}

fn emoji(args: &[String]) -> i32 {
    let mut list = false;
    let mut picker = false;
    let mut query = String::new();
    for a in args {
        match a.as_str() {
            "-l" | "--list" => list = true,
            "-p" | "--pick" => picker = true,
            other => {
                if !query.is_empty() {
                    query.push(' ');
                }
                query.push_str(other);
            }
        }
    }

    if picker && query.is_empty() {
        // Ask for a query with the native dialog tool the rice already ships,
        // then run the same match-and-copy path as a typed query.
        match Command::new("kdialog")
            .args(["--inputbox", "Emoji — type a name (shrug, fire, heart, ...)"])
            .output()
        {
            Ok(out) if out.status.success() => {
                query = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
            Ok(_) => return 1,
            Err(e) => {
                eprintln!("xiu emoji: kdialog unavailable ({e})");
                return 127;
            }
        }
    }

    if list || query.is_empty() {
        let hits: Vec<(&str, &str)> = if query.is_empty() {
            EMOJI.to_vec()
        } else {
            emoji_matches(&query)
        };
        for (name, glyph) in hits {
            println!("{name}\t{glyph}");
        }
        return 0;
    }

    // An exact name always wins; only a fuzzy query is ambiguous.
    if let Some((_, glyph)) = EMOJI.iter().find(|(name, _)| *name == query) {
        return copy_text(glyph);
    }

    let hits = emoji_matches(&query);
    match hits.len() {
        0 => {
            eprintln!("xiu emoji: no match for '{query}' (`xiu emoji -l` lists the set)");
            1
        }
        1 => copy_text(hits[0].1),
        _ => {
            eprintln!("xiu emoji: ambiguous, matches:");
            for (name, glyph) in hits {
                eprintln!("  {name}\t{glyph}");
            }
            2
        }
    }
}

/// Put text on the Wayland clipboard and say so; the glyph lands wherever the
/// user pastes next.
fn copy_text(text: &str) -> i32 {
    match Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            match child.wait() {
                Ok(s) if s.success() => {
                    println!("{text}");
                    0
                }
                _ => 1,
            }
        }
        Err(e) => {
            eprintln!("xiu: wl-copy unavailable ({e})");
            127
        }
    }
}
