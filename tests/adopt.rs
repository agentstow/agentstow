//! `adopt` — the on-ramp from a hand-rolled setup, and the cure for an
//! accidental Variant. Three cases, no force flag.

mod common;

use common::Fixture;

fn machine() -> Fixture {
    let f = Fixture::new();
    f.agent(".claude");
    f.agent(".codex");
    f
}

#[test]
fn a_skill_the_store_does_not_have_moves_in_and_leaves_a_link() {
    let f = machine();
    f.file(".claude/skills/local/SKILL.md", "a hand-written skill\n");

    let out = f.run(&[
        "adopt",
        &f.path(".claude/skills/local").display().to_string(),
    ]);

    out.assert_clean().assert_stdout_has("local");
    assert_eq!(
        std::fs::read_to_string(f.store().join("skills/local/SKILL.md")).unwrap(),
        "a hand-written skill\n",
        "the content moves into the Store intact"
    );
    assert_eq!(
        f.link_text(".claude/skills/local"),
        "../../.agents/skills/local",
        "a canonical link is left behind"
    );
}

#[test]
fn an_adopted_skill_reaches_every_other_agent_on_the_next_sync() {
    let f = machine();
    f.file(".claude/skills/local/SKILL.md", "mine\n");
    f.run(&[
        "adopt",
        &f.path(".claude/skills/local").display().to_string(),
    ])
    .assert_clean();

    f.run(&["sync"]).assert_clean();

    assert!(f.is_symlink(".codex/skills/local"));
}

#[test]
fn an_identical_copy_is_collapsed_into_a_link() {
    let f = machine();
    f.store_skill("research");
    let body = std::fs::read_to_string(f.store().join("skills/research/SKILL.md")).unwrap();
    f.file(".claude/skills/research/SKILL.md", &body);

    let out = f.run(&[
        "adopt",
        &f.path(".claude/skills/research").display().to_string(),
    ]);

    out.assert_clean().assert_stdout_has("identical");
    assert!(f.is_symlink(".claude/skills/research"));
    assert_eq!(
        std::fs::read_to_string(f.store().join("skills/research/SKILL.md")).unwrap(),
        body,
        "the Store copy is untouched"
    );
}

#[test]
fn a_diverged_copy_is_refused_and_nothing_moves() {
    let f = machine();
    f.store_skill("plannotator");
    f.file(
        ".claude/skills/plannotator/SKILL.md",
        "the claude variant\n",
    );
    let before = f.tree();

    let out = f.run(&[
        "adopt",
        &f.path(".claude/skills/plannotator").display().to_string(),
    ]);

    out.assert_code(1).assert_stderr_has("Variant");
    assert_eq!(before, f.tree(), "a refusal must change nothing");
}

#[test]
fn a_command_is_adopted_into_the_commands_family() {
    let f = machine();
    f.file(".claude/commands/ship.md", "ship it\n");

    let out = f.run(&[
        "adopt",
        &f.path(".claude/commands/ship.md").display().to_string(),
    ]);

    out.assert_clean().assert_stdout_has("commands");
    assert!(f.store().join("commands/ship.md").exists());
    assert!(f.is_symlink(".claude/commands/ship.md"));
}

#[test]
fn something_already_linked_has_nothing_to_adopt() {
    let f = machine();
    f.store_skill("research");
    f.run(&["sync"]).assert_clean();

    f.run(&[
        "adopt",
        &f.path(".claude/skills/research").display().to_string(),
    ])
    .assert_code(1)
    .assert_stderr_has("already a symlink");
}

#[test]
fn a_path_outside_every_target_is_refused() {
    let f = machine();
    f.file("somewhere/else/thing.md", "not in a target\n");

    f.run(&[
        "adopt",
        &f.path("somewhere/else/thing.md").display().to_string(),
    ])
    .assert_code(1)
    .assert_stderr_has("not somewhere agentstow manages");
}

#[test]
fn a_path_that_does_not_exist_is_refused() {
    let f = machine();

    f.run(&[
        "adopt",
        &f.path(".claude/skills/ghost").display().to_string(),
    ])
    .assert_code(1)
    .assert_stderr_has("does not exist");
}

#[test]
fn an_instructions_file_can_be_adopted_too() {
    let f = machine();
    f.file(".codex/AGENTS.md", "# my shared instructions\n");

    let out = f.run(&["adopt", &f.path(".codex/AGENTS.md").display().to_string()]);

    out.assert_clean().assert_stdout_has("instructions");
    assert_eq!(
        std::fs::read_to_string(f.store().join("AGENTS.md")).unwrap(),
        "# my shared instructions\n"
    );
    assert!(f.is_symlink(".codex/AGENTS.md"));

    // And from there it reaches every other agent.
    f.run(&["sync"]).assert_clean();
    assert!(f.path(".claude/CLAUDE.md").exists());
}

#[test]
fn claudes_own_file_is_never_adopted_wholesale() {
    let f = machine();
    f.file(".claude/CLAUDE.md", "# claude's own notes\n");

    f.run(&["adopt", &f.path(".claude/CLAUDE.md").display().to_string()])
        .assert_code(1)
        .assert_stderr_has("not somewhere agentstow manages");
}
