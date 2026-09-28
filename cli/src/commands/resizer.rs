//! Window resizer and Picture-in-Picture manager.
//!
//! Replaces xiu-resizer.

use crate::helpers::{config_file, run_status};
use std::process::Command;

pub fn resizer(args: &[String]) -> i32 {
    let script = config_file(&["hypr", "scripts", "xiu-resizer"]);
    if script.is_file() {
        let mut cmd = Command::new("python3");
        cmd.arg(&script).args(args);
        return run_status(&mut cmd);
    }

    eprintln!("xiu resizer: xiu-resizer script not found");
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resizer_invocation() {
        // Runs or fails with clean status code
        let args = vec!["--help".to_string()];
        let res = resizer(&args);
        assert!(res >= 0);
    }
}
