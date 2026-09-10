//! xiu — the shell's command line.
//!
//! One binary wrapping the surfaces that already exist: the pill's Quickshell
//! IPC targets, the hypr scripts and the small tools the rice ships. The
//! command surface is clap-derived (`cli.rs`), implementations live in
//! `commands/`, shared plumbing in `helpers.rs`, the output skin in `ui.rs`,
//! and the JSON parser stays the stdlib-only one in `json.rs` (tested; a
//! dependency would buy nothing this size).
//!
//! `xiu shell` is a raw passthrough to `qs -c pill ipc call`, every other
//! subcommand is a thin, typed convenience over the same socket. The control
//! verbs (restart/start/stop/log/status/update/uninstall) are the old `ricelin`
//! shell wrapper reborn as subcommands, so one command owns the whole rice.

mod cli;
mod commands;
mod helpers;
mod json;
mod ui;

use clap::Parser;

fn main() {
    let parsed = cli::Cli::parse();
    std::process::exit(dispatch(&parsed));
}

fn dispatch(cli: &cli::Cli) -> i32 {
    use cli::Commands;
    use commands::{check, control, emoji, shell};
    match &cli.command {
        Commands::Shell { kill, target, args } => shell::shell(*kill, target.as_deref(), args),
        Commands::Open { surface } => shell::open(surface),
        Commands::Wallpaper { print, list, file } => shell::wallpaper(*print, *list, file.as_deref()),
        Commands::Mpris { action } => shell::mpris(action),
        Commands::Record { stop } => shell::record(*stop),
        Commands::Screenshot { args } => shell::screenshot(args),
        Commands::Clipboard => shell::clipboard(),
        Commands::Notifs { action } => shell::notifs(action),
        Commands::Gamemode { action } => shell::gamemode(action),
        Commands::Scheme { action, value, variant } => shell::scheme(action, value.as_deref(), variant.as_deref()),
        Commands::Browser => shell::browser(),
        Commands::Emoji { pick, list, query } => emoji::emoji(*pick, *list, query),
        Commands::Check => check::check(),
        Commands::Restart { target } => control::restart(target.as_deref()),
        Commands::Start { target } => control::start(target.as_deref()),
        Commands::Stop { target } => control::stop(target.as_deref()),
        Commands::Log { target, args } => control::log(target.as_deref(), args),
        Commands::Update => control::update(),
        Commands::Status => control::status(),
        Commands::Uninstall => control::uninstall(),
    }
}
