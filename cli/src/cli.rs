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
    /// Wallpaper operations: init, set, next, prev, query, list, search, download, thumbs, resolve
    Wallpaper {
        /// init, set, next, prev, query, list, search, download, thumbs, resolve
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
        /// Extra argument (e.g. output or kind)
        extra: Option<String>,
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
    /// Screenshot and annotation overlay (rishot)
    #[command(alias = "rishot")]
    Screenshot {
        /// Arguments for rishot
        args: Vec<String>,
    },
    /// Clipboard manager: watch, get, thumbs, wipe, paste-latest (default: open clipboard surface)
    Clipboard {
        /// watch, get, thumbs, wipe, paste-latest
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
    /// status (default), on, off, toggle, strip
    Gamemode {
        /// The gamemode action
        #[arg(default_value = "status")]
        action: String,
        /// Optional target (e.g. on/off for strip)
        target: Option<String>,
    },
    /// list, get, set <preset|dynamic> [-v VARIANT], preview <wallpaper>, dark, light, toggle (engine: wallcolors)
    Scheme {
        /// The scheme action: list, get, set, preview, dark, light, toggle
        #[arg(default_value = "get")]
        action: String,
        /// The preset to set, or the wallpaper to preview
        value: Option<String>,
        /// The matugen variant for `set`
        #[arg(short, long)]
        variant: Option<String>,
    },
    /// Theme engine: generate, apply, live, dark, light, toggle
    Theme {
        /// generate, apply, live, dark, light, toggle
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
    /// Cursor theme and size management and system-wide synchronization (XWayland, GTK, Qt, CEF)
    Cursor {
        /// Cursor theme name (e.g. Bibata-Modern-Ice) or "sync"
        theme: Option<String>,
        /// Cursor size in px (e.g. 24)
        size: Option<u32>,
    },
    /// Wallcolors palette generation and fan-out engine
    Wallcolors {
        /// Arguments for wallcolors engine
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
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
        sha: Option<String>,
        /// Commit SHA flag for baseline
        #[arg(short, long)]
        commit: Option<String>,
        /// Remote git URL override
        #[arg(long)]
        remote: Option<String>,
        /// Config root directory override
        #[arg(long)]
        config_root: Option<String>,
        /// Comma-separated list of protected files to take upstream for
        #[arg(long)]
        take: Option<String>,
        /// Comma-separated list of package IDs to install
        #[arg(long)]
        install_deps: Option<String>,
        /// Output single raw JSON object
        #[arg(long)]
        json: bool,
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
    /// Default application query and selection (probe or category + desktop_id)
    #[command(alias = "set-default-app", alias = "default-apps")]
    DefaultApp {
        /// Probe installed applications
        #[arg(short, long)]
        probe: bool,
        /// Application category (e.g. x-scheme-handler/http)
        category: Option<String>,
        /// Target desktop file ID (e.g. firefox.desktop)
        desktop_id: Option<String>,
    },
    /// Yazi terminal file chooser wrapper
    YaziChooser {
        /// Multiple selection allowed (1/0)
        multiple: Option<String>,
        /// Directory select mode (1/0)
        directory: Option<String>,
        /// Save mode (1/0)
        save: Option<String>,
        /// Recommended path
        path: Option<String>,
        /// Output file path
        out: Option<String>,
        /// Debug or modal flag from xdg-desktop-portal-termfilechooser
        debug: Option<String>,
        /// Trailing arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Toggle window in/out of special workspace or scratchpad
    #[command(alias = "special-toggle", alias = "scratchpad")]
    Special {
        /// Target special workspace name (e.g. stash, private)
        name: Option<String>,
    },
    /// Toggle focused window between normal and minimized special workspace
    #[command(alias = "minimize-toggle")]
    Minimize,
    /// Lock the session (grabs monitors and wakes lock surface)
    #[command(alias = "idle")]
    Lock,
    /// Paste and type latest clipboard entry into active window
    #[command(alias = "paste")]
    PasteLatest,
    /// Mount or unmount Android phone over MTP under ~/mnt/phone
    #[command(alias = "phone")]
    MountPhone {
        /// up, down, or toggle (default: toggle)
        #[arg(default_value = "toggle")]
        action: String,
    },
    /// Display monitor mode apply, keep, and revert watchdog
    #[command(alias = "display-apply")]
    Display {
        /// apply, keep, or revert
        verb: String,
        /// Target output name (e.g. DP-1)
        out: Option<String>,
        /// Mode string (e.g. 1920x1080@60)
        mode: Option<String>,
        /// Position string (e.g. 0x0)
        position: Option<String>,
        /// Scale factor (e.g. 1)
        scale: Option<String>,
    },
    /// Supervise app launch and toast on crash
    #[command(alias = "guard")]
    LaunchGuard {
        /// App display name
        name: String,
        /// Icon name
        icon: Option<String>,
        /// Working directory
        wd: Option<String>,
        /// Command and arguments to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Generate thumbnails for screen recordings
    RecThumbs {
        /// Directory containing recording_*.mp4 files
        recdir: Option<String>,
    },
    /// Quickshell surface watchdog
    Watchdog {
        /// Surface name (pill or lock)
        surface: String,
    },
    /// AppImage, package, archive, and font installer
    #[command(alias = "appimage-install", alias = "app")]
    AppInstall {
        /// install, remove, or rename
        action: String,
        /// File path or slug
        target: Option<String>,
        /// New name for rename
        extra: Option<String>,
    },
    /// Window resizer and PiP daemon
    #[command(
        long_about = "xiu resizer — window resizer / PiP daemon.\n\n\
Usage:\n  \
xiu resizer [-d|--daemon]                    watch windowtitle/openwindow events and apply rules\n  \
xiu resizer pip                              quick PiP on the active floating window\n  \
xiu resizer PATTERN MATCH_TYPE WIDTH HEIGHT ACTIONS\n                                               \
one-shot: apply to every matching window (\"active\" targets focused window)"
    )]
    Resizer {
        /// Arguments for xiu-resizer
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// User profile and account management (avatar, passwd, shell, gecos, info)
    #[command(alias = "account")]
    User {
        /// avatar, passwd, shell, gecos, or info
        #[arg(default_value = "info")]
        action: String,
        /// Sub-action (e.g. set, remove, status) or parameter
        target: Option<String>,
        /// Path or additional parameter
        value: Option<String>,
    },
}
