//! The seam itself: how the CLI resolves the Store and home from the
//! environment, and how it separates results from diagnostics.

mod common;

use common::Fixture;
use std::fs;

#[test]
fn target_root_redirects_home_without_touching_the_real_one() {
    let f = Fixture::new();
    f.agent(".claude");

    // Only AGENTSTOW_TARGET_ROOT is set: the Store defaults to <home>/.agents.
    let out = f.run_with_vars(
        &["doctor"],
        &[("AGENTSTOW_TARGET_ROOT", f.home().display().to_string())],
    );

    out.assert_clean()
        .assert_stdout_has(&f.home().join(".agents").display().to_string())
        .assert_stdout_has("claude");
}

#[test]
fn store_location_is_independent_of_home() {
    let f = Fixture::new();
    f.agent(".claude");
    let elsewhere = f.root().join("store-elsewhere");
    fs::create_dir_all(elsewhere.join("skills")).unwrap();
    fs::create_dir_all(elsewhere.join("skills").join("moved")).unwrap();

    let out = f.run_with_vars(
        &["doctor"],
        &[
            ("AGENTSTOW_TARGET_ROOT", f.home().display().to_string()),
            ("AGENTSTOW_HOME", elsewhere.display().to_string()),
        ],
    );

    out.assert_clean()
        .assert_stdout_has(&elsewhere.display().to_string())
        .assert_stdout_has("skills        1");
}

#[test]
fn falls_back_to_home_when_target_root_is_unset() {
    let f = Fixture::new();
    f.agent(".codex");

    let out = f.run_with_vars(&["doctor"], &[("HOME", f.home().display().to_string())]);

    out.assert_clean().assert_stdout_has("codex");
}

#[test]
fn target_root_wins_over_home() {
    let f = Fixture::new();
    f.agent(".codex");
    let decoy = f.root().join("decoy-home");
    fs::create_dir_all(decoy.join(".claude")).unwrap();

    let out = f.run_with_vars(
        &["doctor"],
        &[
            ("HOME", decoy.display().to_string()),
            ("AGENTSTOW_TARGET_ROOT", f.home().display().to_string()),
        ],
    );

    out.assert_clean()
        .assert_stdout_has("codex")
        .assert_stdout_lacks("claude");
}

#[test]
fn without_any_home_it_refuses_to_guess() {
    let f = Fixture::new();

    let out = f.run_with_vars(&["doctor"], &[]);

    out.assert_code(1).assert_stderr_has("home directory");
}

#[test]
fn config_directory_is_never_inside_the_store() {
    let f = Fixture::new();

    let out = f.run(&["doctor"]);

    out.assert_clean()
        .assert_stdout_has(&f.home().join(".agentstow").display().to_string());
    assert!(
        !f.store().join(".agentstow").exists(),
        "tool config must not live in the Store"
    );
}

#[test]
fn help_is_a_result_not_a_diagnostic() {
    let f = Fixture::new();

    let out = f.run(&["--help"]);

    out.assert_clean()
        .assert_stderr_empty()
        .assert_stdout_has("agentstow");
}

#[test]
fn an_unknown_command_is_a_diagnostic() {
    let f = Fixture::new();

    let out = f.run(&["frobnicate"]);

    out.assert_code(1);
    assert!(!out.stderr.is_empty(), "usage errors belong on stderr");
    assert!(out.stdout.is_empty(), "a failed parse produces no results");
}
