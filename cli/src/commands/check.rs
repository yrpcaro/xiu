//! `xiu check` — drift and health report for the install. Three sections:
//! which of the deploy set's main config dirs are in place and ours, how far
//! the xiu clone drifts from its remotes (as of the last fetch — checking
//! never touches the network), and whether the binaries the keybinds and the
//! palette pipeline shell out to are on PATH. Exits 1 when a core tool is
//! missing or a tracked config is absent, so scripts can gate on it.

use crate::helpers::{git_out, on_path};

pub fn check() -> i32 {
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
