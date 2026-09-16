//! Command implementations, one module per family: shell-facing IPC calls,
//! the control verbs, and the health check.

pub mod check;
pub mod control;
pub mod shell;
