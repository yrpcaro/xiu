//! Command implementations, one module per family: shell-facing IPC calls,
//! the control verbs, wallpaper lifecycle, theme engine, keyboard layout,
//! clipboard supervision, keybinds inspector, and health check.

pub mod check;
pub mod clipboard;
pub mod control;
pub mod keybinds;
pub mod layout;
pub mod shell;
pub mod theme;
pub mod wallpaper;
