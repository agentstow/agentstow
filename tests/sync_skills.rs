//! `sync` for the skills family — the link engine every fan-out family rides.

mod common;

use common::Fixture;

/// A machine with every skills fan-out agent plus one native agent.
fn machine() -> Fixture {
    let f = Fixture::new();
    f.agent(".claude");
    f.agent(".codex");
    f.agent(".pi");
    f.agent(".cursor");
    f.agent(".openclaw");
    f.agent(".hermes");
    f.agent(".config/opencode");
    f
}

#[test]
fn links_every_commons_skill_into_every_fan_out_target() {
    let f = machine();
    f.commons_skill("research");
    f.commons_skill("tdd");

    f.run(&["sync"]).assert_clean();

    for target in [
        ".claude/skills",
        ".codex/skills",
        ".pi/agent/skills",
        ".cursor/skills",
        ".openclaw/skills",
        ".hermes/skills",
    ] {
        for skill in ["research", "tdd"] {
            let rel = format!("{target}/{skill}");
            assert!(f.is_symlink(&rel), "{rel} should be a symlink");
            assert_eq!(
                f.resolves_to(&rel),
                std::fs::canonicalize(f.commons().join("skills").join(skill)).unwrap(),
                "{rel} should resolve into the Commons"
            );
        }
    }
}

#[test]
fn native_agents_receive_no_links() {
    let f = machine();
    f.commons_skill("research");

    f.run(&["sync"]).assert_clean();

    // OpenCode scans the Commons itself; a fan-out would only duplicate names.
    assert!(
        !f.exists(".config/opencode/skills"),
        "native agents must not get a skills directory"
    );
}

#[test]
fn agents_without_a_skill_surface_receive_nothing() {
    let f = Fixture::new();
    f.agent(".gemini");
    f.agent(".roo");
    f.commons_skill("research");

    f.run(&["sync"]).assert_clean();

    assert!(!f.exists(".gemini/skills"));
    assert!(!f.exists(".roo/skills"));
}

#[test]
fn links_are_relative_so_the_home_directory_can_move() {
    let f = machine();
    f.commons_skill("research");

    f.run(&["sync"]).assert_clean();

    assert_eq!(
        f.link_text(".claude/skills/research"),
        "../../.agents/skills/research"
    );
    // Depth is computed, not hand-encoded: pi sits one level deeper.
    assert_eq!(
        f.link_text(".pi/agent/skills/research"),
        "../../../.agents/skills/research"
    );
}

#[test]
fn rewrites_our_own_link_when_it_is_not_in_canonical_form() {
    let f = machine();
    f.commons_skill("research");
    let absolute = f.commons().join("skills").join("research");
    f.symlink(".claude/skills/research", &absolute.display().to_string());

    f.run(&["sync"]).assert_clean();

    assert_eq!(
        f.link_text(".claude/skills/research"),
        "../../.agents/skills/research",
        "a link resolving into the Commons is ours to canonicalise"
    );
}

#[test]
fn leaves_a_link_that_resolves_outside_the_commons() {
    let f = machine();
    f.commons_skill("research");
    let elsewhere = f.root().join("someone-elses-skills").join("research");
    std::fs::create_dir_all(&elsewhere).unwrap();
    f.symlink(".claude/skills/other", &elsewhere.display().to_string());

    f.run(&["sync"]).assert_clean();

    assert_eq!(
        f.link_text(".claude/skills/other"),
        elsewhere.display().to_string(),
        "Foreign links are never touched"
    );
}

#[test]
fn prunes_a_dangling_link_that_pointed_into_the_commons() {
    let f = machine();
    f.commons_skill("research");
    // What a skill deleted from the Commons leaves behind.
    f.symlink(".claude/skills/deleted", "../../.agents/skills/deleted");

    f.run(&["sync"]).assert_clean();

    assert!(
        !f.present(".claude/skills/deleted"),
        "a dangling Commons-pointing link should be pruned"
    );
    assert!(f.is_symlink(".claude/skills/research"));
}

#[test]
fn leaves_a_dangling_link_that_pointed_somewhere_else() {
    let f = machine();
    f.symlink(".claude/skills/orphan", "../../../elsewhere/orphan");

    f.run(&["sync"]).assert_clean();

    assert!(
        f.present(".claude/skills/orphan"),
        "Foreign garbage is still Foreign — never ours to delete"
    );
}

#[test]
fn preserves_a_variant_directory() {
    let f = machine();
    f.commons_skill("plannotator");
    f.file(
        ".claude/skills/plannotator/SKILL.md",
        "claude-specific variant\n",
    );

    f.run(&["sync"]).assert_clean();

    assert!(
        f.is_real_dir(".claude/skills/plannotator"),
        "a Variant must survive sync"
    );
    assert_eq!(
        std::fs::read_to_string(f.path(".claude/skills/plannotator/SKILL.md")).unwrap(),
        "claude-specific variant\n"
    );
    // Other agents still get the Commons copy.
    assert!(f.is_symlink(".codex/skills/plannotator"));
}

#[test]
fn reports_what_it_did() {
    let f = machine();
    f.commons_skill("research");

    f.run(&["sync"])
        .assert_clean()
        .assert_stdout_has("research");
}

#[test]
fn a_second_sync_changes_nothing_and_says_so() {
    let f = machine();
    f.commons_skill("research");
    f.commons_skill("tdd");
    f.run(&["sync"]).assert_clean();
    let after_first = f.tree();

    let out = f.run(&["sync"]);

    out.assert_clean().assert_stdout_has("up to date");
    assert_eq!(after_first, f.tree(), "a second sync must be a no-op");
}

#[test]
fn dry_run_reports_the_plan_and_changes_nothing() {
    let f = machine();
    f.commons_skill("research");
    let before = f.tree();

    let out = f.run(&["sync", "--dry-run"]);

    out.assert_clean().assert_stdout_has("research");
    assert_eq!(before, f.tree(), "--dry-run must not touch the filesystem");
}

#[test]
fn creates_missing_target_directories_but_never_agent_roots() {
    let f = Fixture::new();
    f.agent(".claude"); // root exists, skills/ does not
    f.commons_skill("research");

    f.run(&["sync"]).assert_clean();

    assert!(
        f.is_real_dir(".claude/skills"),
        "subdirectories are created"
    );
    for absent in [".codex", ".pi", ".cursor", ".gemini"] {
        assert!(
            !f.exists(absent),
            "{absent} is not installed and must not be created"
        );
    }
}

#[test]
fn missing_commons_is_an_error_that_names_the_fix() {
    let f = Fixture::bare();
    f.agent(".claude");

    f.run(&["sync"])
        .assert_code(1)
        .assert_stderr_has("agentstow init");
}

#[test]
fn a_second_process_holding_the_lock_fails_cleanly() {
    let f = machine();
    f.commons_skill("research");
    let _held = common::hold_lock(f.path(".agentstow/lock"));

    let out = f.run_with_vars(
        &["sync"],
        &[
            ("AGENTSTOW_TARGET_ROOT", f.home().display().to_string()),
            ("AGENTSTOW_HOME", f.commons().display().to_string()),
            ("AGENTSTOW_LOCK_TIMEOUT_MS", "150".to_string()),
        ],
    );

    out.assert_code(1).assert_stderr_has("another agentstow");
    assert!(
        !f.exists(".claude/skills/research"),
        "the blocked run must not have written anything"
    );
}
