//! Command implementations, one module per family: shell-facing IPC calls,
//! the control verbs, wallpaper lifecycle, theme engine, keyboard layout,
//! clipboard supervision, keybinds inspector, window management, default apps,
//! devices, display, watchdog, and health check.

pub mod app_install;
pub mod check;
pub mod clipboard;
pub mod control;
pub mod default_app;
pub mod device;
pub mod display;
pub mod guard;
pub mod keybinds;
pub mod layout;
pub mod media;
pub mod resizer;
pub mod session;
pub mod shell;
pub mod theme;
pub mod update;
pub mod wallpaper;
pub mod user;
pub mod watchdog;
pub mod window;
pub mod yazi;
