//! xiu — the shell's command line.
//!
//! One binary wrapping the surfaces that already exist: the pill's Quickshell
//! IPC targets, the hypr scripts and the small tools the rice ships. No
//! dependency on anything outside stdlib so `cargo build` works offline; the
//! installer puts the binary on PATH and the keybinds call it.
//!
//! `xiu shell` is a raw passthrough to `qs -c pill ipc call`, every other
//! subcommand is a thin, typed convenience over the same socket.

use std::io::Write;
use std::process::{Command, Stdio};

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

COMMANDS:
    shell [TARGET FN [ARGS...]]   raw passthrough to `qs -c pill ipc call`
                                  (bare: ipc show, -k: kill the shell)
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
    version                       print the version"
    );
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
