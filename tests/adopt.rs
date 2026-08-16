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
fn a_skill_the_commons_does_not_have_moves_in_and_leaves_a_link() {
    let f = machine();
    f.file(".claude/skills/local/SKILL.md", "a hand-written skill\n");

    let out = f.run(&[
        "adopt",
        &f.path(".claude/skills/local").display().to_string(),
    ]);

    out.assert_clean().assert_stdout_has("local");
    assert_eq!(
        std::fs::read_to_string(f.commons().join("skills/local/SKILL.md")).unwrap(),
        "a hand-written skill\n",
        "the content moves into the Commons intact"
    );
    assert_eq!(
        f.link_text(".claude/skills/local"),
        "../../.agents/skills/local",
        "a canonical link is left behind"
    );
}

#[test]
fn an_adopted_skill_reaches_every_other_agent_at_once() {
    let f = machine();
    f.file(".claude/skills/local/SKILL.md", "mine\n");

    let out = f.run(&[
        "adopt",
        &f.path(".claude/skills/local").display().to_string(),
    ]);

    out.assert_clean()
        .assert_stdout_has("linked into codex (.codex/skills)");
    assert_eq!(
        f.link_text(".codex/skills/local"),
        "../../.agents/skills/local",
        "the fan-out must leave sync's canonical link form"
    );
}

#[test]
fn sync_right_after_an_adopt_has_nothing_to_do() {
    let f = machine();
    f.file(".claude/skills/local/SKILL.md", "mine\n");
    f.run(&[
        "adopt",
        &f.path(".claude/skills/local").display().to_string(),
    ])
    .assert_clean();

    f.run(&["sync"])
        .assert_clean()
        .assert_stdout_has("Everything is up to date.");
}

#[test]
fn the_fan_out_keeps_variants_and_skips_native_agents() {
    let f = machine();
    f.agent(".config/opencode"); // reads the Commons natively — no skills fan-out
    f.file(".claude/skills/dup/SKILL.md", "the adopted copy\n");
    f.file(".codex/skills/dup/SKILL.md", "the codex variant\n");

    let out = f.run(&["adopt", &f.path(".claude/skills/dup").display().to_string()]);

    out.assert_clean()
        .assert_stdout_has("codex (.codex/skills): variant — left alone");
    assert!(f.is_real_dir(".codex/skills/dup"), "a Variant is kept");
    assert_eq!(
        f.contents(".codex/skills/dup/SKILL.md"),
        "the codex variant\n"
    );
    assert!(
        !f.present(".config/opencode/skills/dup"),
        "a Native agent receives nothing"
    );
}

#[test]
fn an_identical_copy_is_collapsed_into_a_link() {
    let f = machine();
    f.commons_skill("research");
    let body = std::fs::read_to_string(f.commons().join("skills/research/SKILL.md")).unwrap();
    f.file(".claude/skills/research/SKILL.md", &body);

    let out = f.run(&[
        "adopt",
        &f.path(".claude/skills/research").display().to_string(),
    ]);

    out.assert_clean().assert_stdout_has("identical");
    assert!(f.is_symlink(".claude/skills/research"));
    assert_eq!(
        std::fs::read_to_string(f.commons().join("skills/research/SKILL.md")).unwrap(),
        body,
        "the Commons copy is untouched"
    );
}

#[test]
fn a_diverged_copy_is_refused_and_nothing_moves() {
    let f = machine();
    f.commons_skill("plannotator");
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
    assert!(f.commons().join("commands/ship.md").exists());
    assert!(f.is_symlink(".claude/commands/ship.md"));
}

#[test]
fn something_already_linked_has_nothing_to_adopt() {
    let f = machine();
    f.commons_skill("research");
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

    out.assert_clean()
        .assert_stdout_has("instructions")
        .assert_stdout_has("added the import line to claude (.claude/CLAUDE.md)");
    assert_eq!(
        std::fs::read_to_string(f.commons().join("AGENTS.md")).unwrap(),
        "# my shared instructions\n"
    );
    assert!(f.is_symlink(".codex/AGENTS.md"));

    // The fan-out already happened, exactly as instructions sync applies it.
    assert!(
        f.contents(".claude/CLAUDE.md")
            .contains("@~/.agents/AGENTS.md"),
        "the import line lands during adopt, not on the next sync"
    );
    f.run(&["sync"])
        .assert_clean()
        .assert_stdout_has("Everything is up to date.");
}

#[test]
fn claudes_own_file_is_never_adopted_wholesale() {
    let f = machine();
    f.file(".claude/CLAUDE.md", "# claude's own notes\n");

    f.run(&["adopt", &f.path(".claude/CLAUDE.md").display().to_string()])
        .assert_code(1)
        .assert_stderr_has("not somewhere agentstow manages");
}

// ---- the Source mechanic ----

#[test]
fn a_repo_path_becomes_an_absolute_commons_link_and_fans_out() {
    let f = machine();
    f.file("github/skillrepo/.git/HEAD", "ref: refs/heads/main\n");
    f.file(
        "github/skillrepo/skills/research/SKILL.md",
        "kept in the repo\n",
    );
    let path = f.path("github/skillrepo/skills/research");

    let out = f.run(&["adopt", &path.display().to_string()]);

    out.assert_clean()
        .assert_stdout_has(&format!(
            "adopted research — Sourced from {} (skills)",
            path.display()
        ))
        .assert_stdout_has("linked into claude (.claude/skills)")
        .assert_stdout_has("linked into codex (.codex/skills)");
    assert_eq!(
        f.link_text(".agents/skills/research"),
        path.display().to_string(),
        "the Commons link is absolute, the path exactly as given"
    );
    assert_eq!(
        f.link_text(".claude/skills/research"),
        "../../.agents/skills/research",
        "agents link to the Commons entry, which holds the pointer"
    );
    assert_eq!(
        f.contents(".claude/skills/research/SKILL.md"),
        "kept in the repo\n",
        "the content reaches agents through the chain of links"
    );
}

#[test]
fn sync_right_after_a_source_adopt_has_nothing_to_do() {
    let f = machine();
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/skills/research/SKILL.md", "mine\n");
    f.run(&[
        "adopt",
        &f.path("repo/skills/research").display().to_string(),
    ])
    .assert_clean();

    f.run(&["sync"])
        .assert_clean()
        .assert_stdout_has("Everything is up to date.");
}

#[test]
fn a_git_file_marks_a_durable_home_too() {
    let f = machine();
    // A worktree or submodule carries `.git` as a *file* pointing home.
    f.file(
        "worktrees/wt/.git",
        "gitdir: /somewhere/main/.git/worktrees/wt\n",
    );
    f.file("worktrees/wt/commands/ship.md", "ship from the worktree\n");
    let path = f.path("worktrees/wt/commands/ship.md");

    f.run(&["adopt", &path.display().to_string()])
        .assert_clean()
        .assert_stdout_has("Sourced from")
        .assert_stdout_has("(commands)");
    assert_eq!(
        f.link_text(".agents/commands/ship.md"),
        path.display().to_string()
    );
}

#[test]
fn re_running_a_source_adopt_is_a_clean_no_op() {
    let f = machine();
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/skills/research/SKILL.md", "mine\n");
    let path = f.path("repo/skills/research").display().to_string();
    f.run(&["adopt", &path]).assert_clean();
    let before = f.tree();

    let out = f.run(&["adopt", &path]);

    out.assert_clean().assert_stdout_has(&format!(
        "skills/research is already Sourced from {path} — nothing to do"
    ));
    assert_eq!(before, f.tree(), "a no-op must change nothing");
}

#[test]
fn a_symlink_input_is_linked_as_given_never_resolved_through() {
    let f = machine();
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/lib/research/SKILL.md", "the real copy\n");
    f.symlink("repo/skills/research", "../lib/research");
    let path = f.path("repo/skills/research");

    f.run(&["adopt", &path.display().to_string()])
        .assert_clean();

    assert_eq!(
        f.link_text(".agents/skills/research"),
        path.display().to_string(),
        "the Commons links to the symlink, not through it"
    );
    assert_eq!(
        f.contents(".agents/skills/research/SKILL.md"),
        "the real copy\n",
        "the content still reads through the chain"
    );
}

#[test]
fn an_identical_commons_copy_is_replaced_by_the_link() {
    let f = machine();
    f.commons_skill("research");
    let body = f.contents_of_commons("skills/research/SKILL.md");
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/skills/research/SKILL.md", &body);
    let path = f.path("repo/skills/research");

    let out = f.run(&["adopt", &path.display().to_string()]);

    out.assert_clean()
        .assert_stdout_has("replaced the identical Commons copy with the link");
    assert!(
        f.is_symlink(".agents/skills/research"),
        "the Source holds the single real copy now"
    );
    assert_eq!(
        f.contents(".agents/skills/research/SKILL.md"),
        body,
        "the content still reads through the link"
    );
}

#[test]
fn a_diverged_commons_copy_refuses_the_source() {
    let f = machine();
    f.commons_skill("research");
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/skills/research/SKILL.md", "a different body\n");
    let before = f.tree();

    f.run(&[
        "adopt",
        &f.path("repo/skills/research").display().to_string(),
    ])
    .assert_code(1)
    .assert_stderr_has("Variant")
    .assert_stderr_has("Merge it by hand");
    assert_eq!(before, f.tree(), "a refusal must change nothing");
}

#[test]
fn a_commons_link_pointing_elsewhere_refuses_to_repoint() {
    let f = machine();
    f.file(
        "elsewhere/skills/research/SKILL.md",
        "the other repo's copy\n",
    );
    let other = f.path("elsewhere/skills/research");
    f.commons_symlink("skills/research", &other.display().to_string());
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/skills/research/SKILL.md", "mine\n");

    f.run(&[
        "adopt",
        &f.path("repo/skills/research").display().to_string(),
    ])
    .assert_code(1)
    .assert_stderr_has(&format!(
        "skills/research is already Sourced from {} — \
         remove the Commons link first if you mean to repoint it",
        other.display()
    ));
    assert_eq!(
        f.link_text(".agents/skills/research"),
        other.display().to_string(),
        "the existing link is untouched"
    );
}

#[test]
fn a_repo_path_under_a_non_family_parent_is_refused() {
    let f = machine();
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/prompts/thing.md", "no family claims this\n");

    f.run(&[
        "adopt",
        &f.path("repo/prompts/thing.md").display().to_string(),
    ])
    .assert_code(1)
    .assert_stderr_has("parent directory \"prompts\" is not a family")
    .assert_stderr_has("skills/, commands/, or agents/");
}

#[test]
fn a_family_parent_without_a_repo_still_refuses() {
    let f = machine();
    // The parent basename alone never triggers the Source mechanic — only a
    // durable home does. Without `.git` above, the existing refusal stands.
    f.file("loose/skills/thing/SKILL.md", "no durable home\n");

    f.run(&["adopt", &f.path("loose/skills/thing").display().to_string()])
        .assert_code(1)
        .assert_stderr_has("not somewhere agentstow manages");
}

#[test]
fn a_declared_custom_surface_absorbs_even_inside_a_repo() {
    let f = machine();
    // Declaring a custom target is an explicit statement that the dir has
    // fan-out semantics — the surface wins over the repo walk-up (ADR-0006).
    f.file(
        ".config/agentstow/agentstow.toml",
        "[custom.myagent]\nroot = \"repo/.myagent\"\nskills = \"repo/.myagent/skills\"\n",
    );
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/.myagent/skills/local/SKILL.md", "mine\n");

    let out = f.run(&[
        "adopt",
        &f.path("repo/.myagent/skills/local").display().to_string(),
    ]);

    out.assert_clean()
        .assert_stdout_has("adopted local from myagent into the Commons (skills)")
        .assert_stdout_lacks("Sourced");
    assert!(
        !f.is_symlink(".agents/skills/local"),
        "absorb moves the content in — the Commons entry is real"
    );
    assert!(
        f.is_symlink("repo/.myagent/skills/local"),
        "the surface keeps a link behind, exactly as any absorb"
    );
}

#[test]
fn dry_run_of_a_source_changes_nothing_and_previews_the_real_run() {
    let f = machine();
    f.file("repo/.git/HEAD", "ref: refs/heads/main\n");
    f.file("repo/skills/research/SKILL.md", "mine\n");
    let path = f.path("repo/skills/research").display().to_string();
    let commons_entry = f.commons().join("skills/research");
    let before = f.tree();

    let dry = f.run(&["adopt", &path, "--dry-run"]);

    dry.assert_clean()
        .assert_stdout_has("dry run — no changes will be made")
        .assert_stdout_has(&format!("source: research — Sourced from {path} (skills)"))
        .assert_stdout_has(&format!("would link {} → {path}", commons_entry.display()))
        .assert_stdout_has("would link into claude (.claude/skills)")
        .assert_stdout_has("would link into codex (.codex/skills)");
    assert_eq!(before, f.tree(), "--dry-run must not touch the filesystem");

    // Every fan-out the dry run promised, the real run delivers verbatim.
    let real = f.run(&["adopt", &path]);
    real.assert_clean()
        .assert_stdout_has(&format!("adopted research — Sourced from {path} (skills)"));
    for line in dry.stdout.lines() {
        if let Some(rest) = line.strip_prefix("  would link into ") {
            assert!(
                real.stdout.contains(&format!("  linked into {rest}")),
                "the dry run promised a link into {rest:?}\nreal run:\n{}",
                real.stdout
            );
        }
    }
}

// ---- the Commons guard ----

#[test]
fn an_entry_of_a_committed_commons_is_refused_and_left_intact() {
    let f = machine();
    // The Commons is meant to be committed. A `.git` at its root must never
    // make adopt classify its own entries as repo-backed and self-link them —
    // the guard answers before any repo detection can.
    std::fs::create_dir_all(f.commons().join(".git")).unwrap();
    f.commons_skill("research");
    let before = f.tree();

    let entry = f.commons().join("skills/research");
    let out = f.run(&["adopt", &entry.display().to_string()]);

    out.assert_code(1)
        .assert_stderr_has("already in the Commons — nothing to adopt");
    assert_eq!(before, f.tree(), "the refusal must change nothing");
    assert!(
        std::fs::read_to_string(entry.join("SKILL.md"))
            .unwrap()
            .contains("body of research"),
        "the entry must survive intact"
    );
}

#[test]
fn the_commons_guard_answers_before_the_symlink_refusal() {
    let f = machine();
    f.commons_skill("research");
    // A symlink inside the Commons must hit the guard, not the symlink
    // refusal: the guard is first in dispatch.
    f.commons_symlink("skills/linked", "research");

    let entry = f.commons().join("skills/linked");
    f.run(&["adopt", &entry.display().to_string()])
        .assert_code(1)
        .assert_stderr_has("already in the Commons")
        .assert_stderr_lacks("already a symlink");
}

// ---- --dry-run ----

#[test]
fn dry_run_changes_nothing_and_previews_the_real_run() {
    let f = machine();
    f.file(".claude/skills/local/SKILL.md", "mine\n");
    let path = f.path(".claude/skills/local").display().to_string();
    let before = f.tree();

    let dry = f.run(&["adopt", &path, "--dry-run"]);

    dry.assert_clean()
        .assert_stdout_has("dry run — no changes will be made")
        .assert_stdout_has("absorb: local from claude into the Commons (skills)")
        .assert_stdout_has(&format!("would move {path} into the Commons"))
        .assert_stdout_has(&format!("would leave a link at {path} → "))
        .assert_stdout_has("would link into codex (.codex/skills)");
    assert_eq!(before, f.tree(), "--dry-run must not touch the filesystem");

    // Every fan-out the dry run promised, the real run delivers verbatim.
    let real = f.run(&["adopt", &path]);
    real.assert_clean()
        .assert_stdout_has("adopted local from claude into the Commons (skills)");
    for line in dry.stdout.lines() {
        if let Some(rest) = line.strip_prefix("  would link into ") {
            assert!(
                real.stdout.contains(&format!("  linked into {rest}")),
                "the dry run promised a link into {rest:?}\nreal run:\n{}",
                real.stdout
            );
        }
    }
}

#[test]
fn dry_run_previews_the_instructions_fan_out_too() {
    let f = machine();
    f.file(".codex/AGENTS.md", "# my shared instructions\n");
    let before = f.tree();

    f.run(&[
        "adopt",
        &f.path(".codex/AGENTS.md").display().to_string(),
        "--dry-run",
    ])
    .assert_clean()
    .assert_stdout_has("absorb: AGENTS.md from codex into the Commons (instructions)")
    .assert_stdout_has("would add the import line to claude (.claude/CLAUDE.md)");
    assert_eq!(before, f.tree(), "--dry-run must not touch the filesystem");
}

#[test]
fn dry_run_refuses_in_the_same_words_as_a_real_run() {
    let f = machine();
    f.commons_skill("plannotator");
    f.file(
        ".claude/skills/plannotator/SKILL.md",
        "the claude variant\n",
    );
    let path = f.path(".claude/skills/plannotator").display().to_string();
    let before = f.tree();

    let dry = f.run(&["adopt", &path, "--dry-run"]);
    dry.assert_code(1).assert_stderr_has("Variant");
    assert_eq!(before, f.tree(), "a dry-run refusal must change nothing");

    let real = f.run(&["adopt", &path]);
    real.assert_code(1);
    assert_eq!(dry.stderr, real.stderr, "same refusal, same words");
}

#[test]
fn dry_run_never_contends_for_the_lock() {
    let f = machine();
    f.file(".claude/skills/local/SKILL.md", "mine\n");
    let _held = common::hold_lock(f.path(".local/state/agentstow/lock"));

    f.run_with_vars(
        &[
            "adopt",
            &f.path(".claude/skills/local").display().to_string(),
            "--dry-run",
        ],
        &[
            ("AGENTSTOW_TARGET_ROOT", f.home().display().to_string()),
            ("AGENTSTOW_HOME", f.commons().display().to_string()),
            ("AGENTSTOW_LOCK_TIMEOUT_MS", "150".to_string()),
        ],
    )
    .assert_clean();
}
