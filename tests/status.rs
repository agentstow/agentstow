//! `status` — "is everything synced?", in the project's fixed vocabulary, with
//! an exit code automation can gate on.

mod common;

use common::Fixture;

fn machine() -> Fixture {
    let f = Fixture::new();
    f.agent(".claude");
    f.agent(".codex");
    f
}

#[test]
fn a_synced_machine_is_clean() {
    let f = machine();
    f.store_skill("research");
    f.run(&["sync"]).assert_clean();

    f.run(&["status"])
        .assert_code(0)
        .assert_stdout_has("in sync")
        .assert_stderr_empty();
}

#[test]
fn unsynced_work_is_actionable() {
    let f = machine();
    f.store_skill("research");

    // Exit 2 is reserved for "there is something to do", distinct from errors.
    f.run(&["status"])
        .assert_code(2)
        .assert_stdout_has("missing")
        .assert_stdout_has("research");
}

#[test]
fn reports_a_dangling_link_of_ours_as_actionable() {
    let f = machine();
    f.symlink(".claude/skills/deleted", "../../.agents/skills/deleted");

    f.run(&["status"])
        .assert_code(2)
        .assert_stdout_has("dangling")
        .assert_stdout_has("deleted");
}

#[test]
fn reports_a_link_that_is_not_in_canonical_form() {
    let f = machine();
    f.store_skill("research");
    let absolute = f.store().join("skills").join("research");
    f.symlink(".claude/skills/research", &absolute.display().to_string());

    f.run(&["status"]).assert_code(2).assert_stdout_has("stale");
}

#[test]
fn an_identical_variant_is_flagged_as_re_linkable() {
    let f = machine();
    f.store_skill("research");
    // Same bytes as the Store copy: a duplicate nobody meant to keep.
    let body = std::fs::read_to_string(f.store().join("skills/research/SKILL.md")).unwrap();
    f.file(".claude/skills/research/SKILL.md", &body);

    f.run(&["status"])
        .assert_code(2)
        .assert_stdout_has("variant-identical")
        .assert_stdout_has("could be re-linked");
}

#[test]
fn a_diverged_variant_is_reported_without_demanding_action() {
    let f = Fixture::new();
    f.agent(".claude");
    f.store_skill("plannotator");
    f.file(".claude/skills/plannotator/SKILL.md", "a real variant\n");
    // Everything else is already in place.
    f.run(&["sync"]).assert_clean();

    let out = f.run(&["status"]);

    // Divergence is the point of a Variant, so it is listed, not flagged.
    out.assert_code(0)
        .assert_stdout_has("variant")
        .assert_stdout_lacks("could be re-linked");
}

#[test]
fn a_foreign_link_is_reported_without_demanding_action() {
    let f = Fixture::new();
    f.agent(".claude");
    let elsewhere = f.root().join("other").join("thing");
    std::fs::create_dir_all(&elsewhere).unwrap();
    f.symlink(".claude/skills/thing", &elsewhere.display().to_string());

    f.run(&["status"])
        .assert_code(0)
        .assert_stdout_has("foreign");
}

#[test]
fn json_carries_the_same_facts_on_a_clean_stdout() {
    let f = machine();
    f.store_skill("research");
    f.store_skill("tdd");
    f.run(&["sync"]).assert_clean();
    f.symlink(".claude/skills/deleted", "../../.agents/skills/deleted");

    let out = f.run(&["status", "--json"]);
    out.assert_code(2);

    let parsed: serde_json::Value =
        serde_json::from_str(&out.stdout).expect("status --json must emit parseable JSON");

    assert_eq!(parsed["clean"], serde_json::Value::Bool(false));
    assert!(parsed["actionable"].as_u64().unwrap() >= 1);

    let targets = parsed["targets"].as_array().unwrap();
    let claude = targets
        .iter()
        .find(|t| t["agent"] == "claude" && t["family"] == "skills")
        .expect("claude skills target");
    let entries = claude["entries"].as_array().unwrap();

    let state_of = |name: &str| {
        entries
            .iter()
            .find(|e| e["name"] == name)
            .map(|e| e["state"].as_str().unwrap().to_string())
    };
    assert_eq!(state_of("research").as_deref(), Some("linked"));
    assert_eq!(state_of("deleted").as_deref(), Some("dangling"));
}

#[test]
fn json_stays_parseable_when_there_are_warnings() {
    let f = machine();
    // A Store hygiene warning must go to stderr, never into the JSON stream.
    f.store_file("skills/README.md", "not a skill\n");

    let out = f.run(&["status", "--json"]);

    serde_json::from_str::<serde_json::Value>(&out.stdout)
        .expect("warnings must not corrupt the JSON on stdout");
    out.assert_stderr_has("README.md");
}

#[test]
fn status_changes_nothing() {
    let f = machine();
    f.store_skill("research");
    let before = f.tree();

    f.run(&["status"]).assert_code(2);

    assert_eq!(before, f.tree(), "status is read-only");
}

#[test]
fn a_missing_store_is_an_error_not_a_drift_report() {
    let f = Fixture::bare();

    f.run(&["status"])
        .assert_code(1)
        .assert_stderr_has("agentstow init");
}
