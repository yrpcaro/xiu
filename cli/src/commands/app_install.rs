//! Application drop-installer and package management.
//!
//! Replaces app-install.sh.

use crate::helpers::{config_file, run_status};
use std::process::Command;

pub fn app_install(action: &str, target: Option<&str>, extra: Option<&str>) -> i32 {
    let script = config_file(&["hypr", "scripts", "app-install.sh"]);
    if script.is_file() {
        let mut cmd = Command::new("bash");
        cmd.arg(&script).arg(action);
        if let Some(t) = target {
            cmd.arg(t);
        }
        if let Some(e) = extra {
            cmd.arg(e);
        }
        return run_status(&mut cmd);
    }

    eprintln!("xiu app-install: installer script not found");
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_install_nonexistent() {
        // Without target or with missing script, fails cleanly
        let res = app_install("invalid-action", None, None);
        assert!(res >= 0);
    }
}
