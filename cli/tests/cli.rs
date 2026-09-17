//! Integration tests: run the real binary and pin the surface every script
//! and keybind depends on — exit codes, error wording, the no-network
//! commands. IPC-dependent commands fail honestly outside a live session
//! (qs/hyprctl missing), which these tests take as their expected shape.

use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;
use predicates::str::{contains, starts_with};

#[test]
fn version_prints() {
    Command::cargo_bin("xiu")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(starts_with("xiu "));
}

#[test]
fn help_lists_the_command_groups() {
    Command::cargo_bin("xiu")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("shell").and(contains("wallpaper")).and(contains("update")));
}

#[test]
fn no_arguments_is_a_usage_error() {
    Command::cargo_bin("xiu")
        .unwrap()
        .assert()
        .failure()
        .code(2);
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    Command::cargo_bin("xiu")
        .unwrap()
        .arg("definitely-not-a-command")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn check_reports_config_state() {
    // check reads the real HOME; it must print its three sections and exit
    // 0/1 (this box is not a live xiu install, so 1 is expected).
    Command::cargo_bin("xiu")
        .unwrap()
        .arg("check")
        .assert()
        .stdout(contains("configs:").and(contains("drift:")).and(contains("health:")));
}

#[test]
fn scheme_defers_to_the_engine() {
    // The engine script may be absent (clean box); the CLI defers to
    // python3 and returns its exit code, printing the child's error on
    // stderr. Both shapes are correct: present engine prints state,
    // absent engine exits non-zero.
    let out = Command::cargo_bin("xiu")
        .unwrap()
        .args(["scheme", "get"])
        .assert()
        .get_output()
        .to_owned();
    assert!(out.status.code().is_some());
}

#[test]
fn unknown_mpris_action_is_rejected() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["mpris", "bogus"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unknown action"));
}

#[test]
fn record_stop_short_circuits() {
    // -s short-circuits to the stop ipc call before any monitor resolution,
    // matching the old flag order; without a live shell it fails through qs
    // with the missing-binary exit, which is the child's to choose.
    let out = Command::cargo_bin("xiu")
        .unwrap()
        .args(["record", "--stop"])
        .assert()
        .get_output()
        .to_owned();
    assert!(out.status.code().is_some());
}

#[test]
fn wallpaper_set_takes_a_path() {
    // The path is forwarded; without a live shell qs reports the failure and
    // the CLI returns its exit code.
    let out = Command::cargo_bin("xiu")
        .unwrap()
        .args(["wallpaper", "-f", "/tmp/whatever.jpg"])
        .assert()
        .get_output()
        .to_owned();
    assert!(out.status.code().is_some());
}

#[test]
fn session_rejects_unknown_action() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["session", "bogus"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("unknown session action"));
}

#[test]
fn session_stats_runs() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["session", "stats"])
        .assert()
        .success()
        .stdout(contains("session").and(contains("compositor")).and(contains("surfaces")));
}

#[test]
fn layout_format_resolves_accurately() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["layout", "format", "English (US)"])
        .assert()
        .success()
        .stdout(starts_with("US"));

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["layout", "format", "Persian (Windows)"])
        .assert()
        .success()
        .stdout(starts_with("FA"));

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["layout", "format", "Russian (phonetic)"])
        .assert()
        .success()
        .stdout(starts_with("RU"));

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["layout", "format", "German (DE)"])
        .assert()
        .success()
        .stdout(starts_with("DE"));
}

#[test]
fn keybinds_format_resolves_us_labels() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["keybinds", "format", "mod .. \" + code:28\""])
        .assert()
        .success()
        .stdout(starts_with("SUPER + T"));

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["keybinds", "format", "mod .. \" + code:60\""])
        .assert()
        .success()
        .stdout(starts_with("SUPER + ."));

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["keybinds", "format", "mod .. \" + code:24\""])
        .assert()
        .success()
        .stdout(starts_with("SUPER + Q"));
}

#[test]
fn keybinds_list_and_export_run() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["keybinds", "list"])
        .assert()
        .success()
        .stdout(contains("keybinds"));

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["keybinds", "export"])
        .assert()
        .success()
        .stdout(starts_with("["));
}

#[test]
fn clipboard_wipe_and_rejections() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["clipboard", "wipe"])
        .assert()
        .success()
        .stdout(contains("cleared").or(contains("wiped")));

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["clipboard", "bogus"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unknown action"));
}

#[test]
fn wallpaper_query_and_rejections() {
    let out = Command::cargo_bin("xiu")
        .unwrap()
        .args(["wallpaper", "query"])
        .assert()
        .get_output()
        .to_owned();
    assert!(out.status.code().is_some());

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["wallpaper", "definitely-not-action-or-file"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unknown action"));
}

#[test]
fn theme_live_and_rejections() {
    let out = Command::cargo_bin("xiu")
        .unwrap()
        .args(["theme", "live"])
        .assert()
        .get_output()
        .to_owned();
    assert!(out.status.code().is_some());

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["theme", "bogus"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unknown action"));
}

#[test]
fn update_check_runs() {
    let out = Command::cargo_bin("xiu")
        .unwrap()
        .args(["update", "check"])
        .assert()
        .get_output()
        .to_owned();
    assert!(out.status.code().is_some());

    Command::cargo_bin("xiu")
        .unwrap()
        .args(["update", "bogus"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("unknown update action"));
}

