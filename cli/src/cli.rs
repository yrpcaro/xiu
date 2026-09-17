//! The CLI surface: one clap-derive enum whose variants carry the same names,
//! flags and defaults the hand-rolled parser served, so scripts and keybinds
//! calling `xiu ...` keep working across the refactor. Doc comments are the
//! help text.

use clap::{Parser, Subcommand};


#[derive(Parser)]
#[command(
    name = "xiu",
    version,
    about = "the xiu shell's command line",
    long_about = "xiu — the shell's command line",
    arg_required_else_help(true),
    subcommand_negates_reqs(true)
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Raw passthrough to `qs -c pill ipc call` (bare: ipc show, -k: kill the shell)
    Shell {
        /// Kill the running pill instead of calling through
        #[arg(short, long)]
        kill: bool,
        /// The ipc target function
        target: Option<String>,
        /// Arguments passed to the ipc function
        args: Vec<String>,
    },
    /// Open a pill surface (launcher, power, link, mixer, wallpaper, clipboard, gameMode, ...)
    Open {
        /// The surface to open
        surface: String,
    },
    /// Wallpaper operations: init, set, next, prev, query, list (or flags: -p, -l, -f)
    Wallpaper {
        /// init, set, next, prev, query, list (default: random/next)
        action: Option<String>,
        /// Target image or argument
        target: Option<String>,
        /// Print the current wallpaper
        #[arg(short, long)]
        print: bool,
        /// List the wallpaper collection
        #[arg(short, long)]
        list: bool,
        /// Set this image as the wallpaper
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Active player (default), play, next, prev, stop, list
    Mpris {
        /// The mpris action
        #[arg(default_value = "active")]
        action: String,
    },
    /// Quick-record on the focused monitor (-s: stop)
    Record {
        /// Stop the running recording instead of starting one
        #[arg(short, long)]
        stop: bool,
    },
    /// rishot passthrough
    Screenshot {
        /// Arguments for rishot
        args: Vec<String>,
    },
    /// Clipboard manager: watch, get, thumbs, wipe (default: open clipboard surface)
    Clipboard {
        /// watch, get, thumbs, wipe
        action: Option<String>,
        /// Target entry ID for get
        target: Option<String>,
    },
    /// Clear (default) or mark notifications seen
    Notifs {
        /// clear or seen
        #[arg(default_value = "clear")]
        action: String,
    },
    /// status (default), on, off, toggle
    Gamemode {
        /// The gamemode action
        #[arg(default_value = "status")]
        action: String,
    },
    /// list, get, set <preset|dynamic> [-v VARIANT], preview <wallpaper> (engine: wallcolors.py)
    Scheme {
        /// The scheme action
        #[arg(default_value = "get")]
        action: String,
        /// The preset to set, or the wallpaper to preview
        value: Option<String>,
        /// The matugen variant for `set`
        #[arg(short, long)]
        variant: Option<String>,
    },
    /// Theme engine: generate, apply, live
    Theme {
        /// generate, apply, live
        #[arg(default_value = "apply")]
        action: String,
        /// Wallpaper path or preset name
        target: Option<String>,
        /// Preset name override
        #[arg(short, long)]
        preset: Option<String>,
        /// Matugen variant
        #[arg(short, long)]
        variant: Option<String>,
    },
    /// Keyboard layout: switch, get, format
    Layout {
        /// switch, get, format
        #[arg(default_value = "get")]
        action: String,
        /// Target layout or descriptor (for switch or format)
        target: Option<String>,
    },
    /// Keybinds inspector: list, format, export
    Keybinds {
        /// list, format, export
        #[arg(default_value = "list")]
        action: String,
        /// Combo to format
        combo: Option<String>,
    },
    /// Apply the palette policy to Brave/Chromium
    Browser,
    /// Drift and health report for the install
    Check,
    /// Cycle a surface (default: pill)
    Restart {
        /// pill, lock or all
        target: Option<String>,
    },
    /// Start the watchdog (default: all)
    Start {
        /// pill, lock or all
        target: Option<String>,
    },
    /// Stop watchdog and surface (default: all)
    Stop {
        /// pill, lock or all
        target: Option<String>,
    },
    /// Follow the quickshell log (default: pill)
    Log {
        /// pill or lock
        target: Option<String>,
        /// Arguments for the log viewer
        args: Vec<String>,
    },
    /// Check, show changelog, apply, restart
    Update {
        /// check, apply, baseline
        action: Option<String>,
        /// Commit SHA for baseline
        #[arg(short, long)]
        sha: Option<String>,
    },
    /// What's running and the installed version
    Status,
    /// Remove the configs, restore backups
    Uninstall,
    /// Session control: logout, lock, stats
    Session {
        /// logout, lock, stats
        #[arg(default_value = "logout")]
        action: String,
    },
}

// Flat aliases over the enum's struct variants, so the command modules
// spell their arguments as `&cli::Shell` instead of the nested enum form.
