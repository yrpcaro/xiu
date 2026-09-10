//! Command implementations, one module per family: shell-facing IPC calls,
//! the control verbs, the health check and the emoji picker.

pub mod check;
pub mod control;
pub mod emoji;
pub mod shell;
