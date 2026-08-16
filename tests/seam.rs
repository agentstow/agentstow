//! The seam itself: how the CLI resolves the Commons and home from the
//! environment, and how it separates results from diagnostics.

mod common;

use common::Fixture;
use std::fs;

#[test]
fn target_root_redirects_home_without_touching_the_real_one() {
    let f = Fixture::new();
    f.agent(".claude");

    // Only AGENTSTOW_TARGET_ROOT is set: the Commons defaults to <home>/.agents.
    let out = f.run_with_vars(
        &["doctor"],
        &[("AGENTSTOW_TARGET_ROOT", f.home().display().to_string())],
    );

    out.assert_clean()
        .assert_stdout_has(&f.home().join(".agents").display().to_string())
        .assert_stdout_has("claude");
}

#[test]
fn commons_location_is_independent_of_home() {
    let f = Fixture::new();
    f.agent(".claude");
    let elsewhere = f.root().join("commons-elsewhere");
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
fn config_directory_is_never_inside_the_commons() {
    let f = Fixture::new();

    let out = f.run(&["doctor"]);

    out.assert_clean()
        .assert_stdout_has(&f.home().join(".config/agentstow").display().to_string())
        .assert_stdout_has(
            &f.home()
                .join(".local/state/agentstow")
                .display()
                .to_string(),
        );
    assert!(
        !f.commons().join(".config").exists(),
        "tool config must not live in the Commons"
    );
}

#[test]
fn an_absolute_xdg_config_home_wins_over_the_derived_default() {
    let f = Fixture::new();
    let elsewhere = f.home().join("xdg-elsewhere").display().to_string();

    let out = f.run_with_env(&["doctor"], &[("XDG_CONFIG_HOME", &elsewhere)]);

    out.assert_clean()
        .assert_stdout_has(&format!("{elsewhere}/agentstow"));
}

#[test]
fn a_relative_xdg_config_home_is_ignored() {
    let f = Fixture::new();

    let out = f.run_with_env(&["doctor"], &[("XDG_CONFIG_HOME", "relative/path")]);

    out.assert_clean()
        .assert_stdout_has(&f.home().join(".config/agentstow").display().to_string());
}

#[test]
fn the_lock_lives_in_the_state_dir_and_the_legacy_dir_stays_absent() {
    let f = Fixture::new();

    f.run(&["sync"]).assert_clean();

    assert!(
        f.present(".local/state/agentstow/lock"),
        "the lock must land in the XDG state dir"
    );
    assert!(
        !f.present(".agentstow"),
        "nothing may be created at the legacy config dir"
    );
}

#[test]
fn doctor_names_a_leftover_legacy_config_dir() {
    let f = Fixture::new();
    f.dir(".agentstow");

    let out = f.run(&["doctor"]);

    out.assert_clean()
        .assert_stderr_has("no longer read")
        .assert_stderr_has(".config/agentstow");
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

#[test]
fn the_binary_reports_the_crate_version() {
    // scripts/build-npm.sh reads this version out of Cargo.toml and stamps it
    // onto all five npm packages. If the binary disagreed, a release would ship
    // packages whose name promises one version and whose contents are another.
    let f = Fixture::new();

    let out = f.run(&["--version"]);

    out.assert_clean()
        .assert_stderr_empty()
        .assert_stdout_has(env!("CARGO_PKG_VERSION"));
}
