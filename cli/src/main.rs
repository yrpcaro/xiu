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
    use commands::{
        app_install, check, clipboard, control, cursor, default_app, device, display, guard,
        keybinds, layout, media, resizer, session, shell, theme, user, wallcolors, wallpaper, watchdog,
        window, yazi,
    };
    match &cli.command {
        Commands::Shell { kill, target, args } => shell::shell(*kill, target.as_deref(), args),
        Commands::Open { surface } => shell::open(surface),
        Commands::Wallpaper {
            action,
            target,
            print,
            list,
            file,
            extra,
        } => wallpaper::wallpaper(
            action.as_deref(),
            target.as_deref(),
            *print,
            *list,
            file.as_deref(),
            extra.as_deref(),
        ),
        Commands::Mpris { action } => shell::mpris(action),
        Commands::Record { stop } => shell::record(*stop),
        Commands::Screenshot { args } => shell::screenshot(args),
        Commands::Clipboard { action, target } => {
            clipboard::clipboard(action.as_deref(), target.as_deref())
        }
        Commands::Notifs { action } => shell::notifs(action),
        Commands::Gamemode { action, target } => shell::gamemode(action, target.as_deref()),
        Commands::Scheme {
            action,
            value,
            variant,
        } => shell::scheme(action, value.as_deref(), variant.as_deref()),
        Commands::Theme {
            action,
            target,
            preset,
            variant,
        } => theme::theme(
            action,
            target.as_deref(),
            preset.as_deref(),
            variant.as_deref(),
        ),
        Commands::Cursor { theme, size } => {
            cursor::cursor(theme.as_deref(), *size)
        }
        Commands::Wallcolors { args } => wallcolors::wallcolors(args),
        Commands::Layout { action, target } => layout::layout(action, target.as_deref()),
        Commands::Keybinds { action, combo } => keybinds::keybinds(action, combo.as_deref()),
        Commands::Browser => shell::browser(),
        Commands::Check => check::check(),
        Commands::Restart { target } => control::restart(target.as_deref()),
        Commands::Start { target } => control::start(target.as_deref()),
        Commands::Stop { target } => control::stop(target.as_deref()),
        Commands::Log { target, args } => control::log(target.as_deref(), args),
        Commands::Update {
            action,
            sha,
            commit,
            remote,
            config_root,
            take,
            install_deps,
            json,
        } => control::update(
            action.as_deref(),
            sha.as_deref().or(commit.as_deref()),
            remote.as_deref(),
            config_root.as_deref(),
            take.as_deref(),
            install_deps.as_deref(),
            *json,
        ),
        Commands::Status => control::status(),
        Commands::Uninstall => control::uninstall(),
        Commands::Session { action } => control::session(action),
        Commands::DefaultApp {
            probe,
            category,
            desktop_id,
        } => default_app::default_app(*probe, category.as_deref(), desktop_id.as_deref()),
        Commands::YaziChooser {
            multiple,
            directory,
            save,
            path,
            out,
            debug,
            extra,
        } => yazi::yazi_chooser(
            multiple.as_deref(),
            directory.as_deref(),
            save.as_deref(),
            path.as_deref(),
            out.as_deref(),
            debug.as_deref(),
            &extra,
        ),
        Commands::Special { name } => window::special(name.as_deref()),
        Commands::Minimize => window::minimize(),
        Commands::Lock { idle } => session::lock_cmd(*idle),
        Commands::Suspend { idle } => session::suspend_cmd(*idle),
        Commands::PasteLatest => clipboard::paste_latest(),
        Commands::MountPhone { action } => device::mount_phone(action),
        Commands::Display {
            verb,
            out,
            mode,
            position,
            scale,
        } => display::display(
            verb,
            out.as_deref(),
            mode.as_deref(),
            position.as_deref(),
            scale.as_deref(),
        ),
        Commands::LaunchGuard {
            name,
            icon,
            wd,
            cmd,
        } => guard::launch_guard(name, icon.as_deref(), wd.as_deref(), cmd),
        Commands::RecThumbs { recdir } => media::rec_thumbs(recdir.as_deref()),
        Commands::Watchdog { surface } => watchdog::watchdog(surface),
        Commands::AppInstall {
            action,
            target,
            extra,
        } => app_install::app_install(action, target.as_deref(), extra.as_deref()),
        Commands::Resizer { args } => resizer::resizer(args),
        Commands::User {
            action,
            target,
            value,
        } => user::user(action, target.as_deref(), value.as_deref()),
    }
}
