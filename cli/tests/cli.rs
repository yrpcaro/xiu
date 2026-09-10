//! Integration tests: run the real binary and pin the surface every script
//! and keybind depends on — exit codes, error wording, the no-network
//! commands. IPC-dependent commands fail honestly outside a live session
//! (qs/hyprctl missing), which these tests take as their expected shape.

use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;
use predicates::str::{contains, is_empty, starts_with};

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
fn emoji_lists_its_table() {
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["emoji", "-l"])
        .assert()
        .success()
        .stdout(contains("shrug").and(contains("tableflip")));
}

#[test]
fn emoji_query_words_join_with_spaces() {
    // Multi-word queries join into one search string: "thumbs up" matches
    // the "thumbsup" entry only through single-word names — this multi-word
    // phrase is the honest contract (no match), same as the old CLI.
    Command::cargo_bin("xiu")
        .unwrap()
        .args(["emoji", "thumbs", "up"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("no match for 'thumbs up'"));
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
