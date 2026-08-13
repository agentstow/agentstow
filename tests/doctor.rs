//! `doctor` — machine readiness: which agents are installed, is the Store
//! usable, and is anything in it going to be silently skipped.

mod common;

use common::Fixture;

#[test]
fn reports_only_agents_whose_config_root_exists() {
    let f = Fixture::new();
    f.agent(".claude");
    f.agent(".codex");

    let out = f.run(&["doctor"]);

    out.assert_clean()
        .assert_stdout_has("claude")
        .assert_stdout_has("codex")
        .assert_stdout_lacks("windsurf")
        .assert_stdout_lacks("cline");
}

#[test]
fn every_registry_agent_is_detected_by_creating_its_root() {
    // The registry is data: a row's `root` is the whole detection rule. If this
    // holds for every row, adding an agent really is a data-only change.
    for agent in agentstow::registry::AGENTS {
        let f = Fixture::new();
        f.agent(agent.root);

        f.run(&["doctor"])
            .assert_clean()
            .assert_stdout_has(agent.name);
    }
}

#[test]
fn reports_the_capabilities_of_a_detected_agent() {
    let f = Fixture::new();
    f.agent(".claude");

    f.run(&["doctor"])
        .assert_clean()
        .assert_stdout_has("import-line")
        .assert_stdout_has(".claude/skills");
}

#[test]
fn creates_nothing() {
    let f = Fixture::new();
    f.agent(".claude");
    f.agent(".gemini");
    f.store_skill("research");
    let before = f.tree();

    f.run(&["doctor"]).assert_clean();

    assert_eq!(before, f.tree(), "doctor must be strictly read-only");
}

#[test]
fn does_not_create_roots_for_agents_that_are_not_installed() {
    let f = Fixture::new();

    f.run(&["doctor"]).assert_clean();

    for agent in agentstow::registry::AGENTS {
        assert!(
            !f.path(agent.root).exists(),
            "doctor created a config root for {}",
            agent.name
        );
    }
}

#[test]
fn warns_about_store_entries_that_agents_would_never_see() {
    let f = Fixture::new();
    f.store_skill("research");
    // Dot-prefixed: invisible to the agents' own scanners.
    f.store_file("skills/.hidden/SKILL.md", "hidden\n");
    // A loose file where a skill directory belongs.
    f.store_file("skills/README.md", "not a skill\n");

    let out = f.run(&["doctor"]);

    out.assert_clean()
        .assert_stdout_has("skills        1")
        .assert_stderr_has(".hidden")
        .assert_stderr_has("README.md");
}

#[test]
fn warns_about_a_dangling_symlink_in_the_store() {
    let f = Fixture::new();
    f.store_symlink("skills/gone", "../../nowhere");

    let out = f.run(&["doctor"]);

    out.assert_clean()
        .assert_stdout_has("skills        0")
        .assert_stderr_has("gone");
}

#[test]
fn warnings_do_not_make_the_run_fail() {
    let f = Fixture::new();
    f.store_file("skills/README.md", "not a skill\n");

    // A warning is advisory: it is reported, but the machine is still usable.
    f.run(&["doctor"]).assert_code(0);
}

#[test]
fn missing_store_is_a_problem_that_names_the_fix() {
    let f = Fixture::bare();

    f.run(&["doctor"])
        .assert_code(1)
        .assert_stderr_has("agentstow init");
}

#[test]
fn a_clean_machine_reports_nothing_on_stderr() {
    let f = Fixture::new();
    f.agent(".claude");
    f.store_skill("research");

    let out = f.run(&["doctor"]);

    out.assert_clean().assert_stderr_empty();
    assert!(!out.stdout.is_empty(), "results belong on stdout");
}

#[test]
fn counts_the_agents_it_knows_about_but_did_not_find() {
    let f = Fixture::new();
    f.agent(".claude");

    let total = agentstow::registry::AGENTS.len();

    f.run(&["doctor"])
        .assert_clean()
        .assert_stdout_has(&format!("1 of {total}"))
        .assert_stdout_has(&format!("{} known agents are not installed", total - 1));
}
