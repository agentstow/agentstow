//! `sync` — make every Target match the Store.

use std::path::Path;
use std::time::Duration;

use crate::config::Config;
use crate::env::Env;
use crate::instructions;
use crate::link::{self, Item};
use crate::lock;
use crate::report::Reporter;
use crate::store::{Store, FANOUT_FAMILIES};
use crate::target;
use crate::{EXIT_CLEAN, EXIT_ERROR};

pub fn run(env: &Env, config: &Config, r: &mut Reporter, dry_run: bool) -> i32 {
    let store = Store::new(env.store());
    if !store.exists() {
        r.problem(format!(
            "no Store at {} — run `agentstow init` to create one",
            store.root().display()
        ));
        return EXIT_ERROR;
    }

    // A dry run only reads, so it never contends for the lock.
    let _lock = if dry_run {
        None
    } else {
        match lock::acquire(env.config_dir(), lock_timeout(env)) {
            Ok(guard) => Some(guard),
            Err(e) => {
                r.problem(e.to_string());
                return EXIT_ERROR;
            }
        }
    };

    if dry_run {
        r.line("dry run — no changes will be made");
        r.blank();
    }

    let mut changes = 0usize;

    for (family, shape) in FANOUT_FAMILIES {
        let scan = store.scan(family, *shape);
        for issue in &scan.issues {
            r.warn(issue.to_string());
        }

        for target in target::resolve(env, config) {
            let Some(dir) = target.fanout_dir(family) else {
                continue;
            };
            let target_dir = env.in_home(dir);
            let items = link::survey(&target_dir, store.root(), &scan.entries);
            changes += sync_target(&target.name, dir, &items, env.home(), r, dry_run);
        }
    }

    changes += sync_instructions(env, config, &store, r, dry_run);

    if changes == 0 {
        r.line("Everything is up to date.");
    } else {
        r.blank();
        let verb = if dry_run { "would be made" } else { "made" };
        r.line(format!("{changes} changes {verb}."));
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else {
        EXIT_CLEAN
    }
}

/// The instructions family: one Store file, three per-agent mechanisms.
fn sync_instructions(
    env: &Env,
    config: &Config,
    store: &Store,
    r: &mut Reporter,
    dry_run: bool,
) -> usize {
    let store_file = store.root().join(crate::store::INSTRUCTIONS);
    let items = instructions::survey(env, config, &store_file);
    let noteworthy: Vec<&instructions::Item> = items
        .iter()
        .filter(|i| i.state != instructions::State::Linked)
        .filter(|i| i.state != instructions::State::ImportPresent)
        .collect();
    if noteworthy.is_empty() {
        return 0;
    }

    r.line("instructions (AGENTS.md)");
    let mut changed = 0;

    for item in noteworthy {
        if !item.state.needs_change() {
            r.line(format!(
                "  {:<9} {} — {}",
                item.state.label(),
                item.target,
                item.note()
            ));
            continue;
        }
        if dry_run {
            r.line(format!("  {:<9} {}", item.state.label(), item.target));
            changed += 1;
        } else if let Err(e) = instructions::apply(item) {
            r.problem(format!("cannot write {}: {e}", item.path.display()));
        } else {
            r.line(format!("  {:<9} {}", item.state.label(), item.target));
            changed += 1;
        }
    }

    changed
}

/// Report — and unless this is a dry run, apply — one Target's items. Returns
/// how many of them changed the filesystem.
fn sync_target(
    agent: &str,
    dir: &str,
    items: &[Item],
    home: &Path,
    r: &mut Reporter,
    dry_run: bool,
) -> usize {
    let interesting: Vec<&Item> = items
        .iter()
        .filter(|i| i.state != link::State::Linked)
        .collect();
    if interesting.is_empty() {
        return 0;
    }

    r.line(format!("{agent} ({dir})"));
    let mut changed = 0;

    for item in interesting {
        let note = item.state.note();
        if !item.state.needs_change() {
            r.line(format!(
                "  {:<9} {} — {note}",
                item.state.label(),
                item.name
            ));
            continue;
        }

        if dry_run {
            r.line(format!("  {:<9} {}", item.state.label(), item.name));
            changed += 1;
        } else if let Err(e) = link::apply(item) {
            r.problem(format!(
                "cannot fix {}: {e}",
                item.path.strip_prefix(home).unwrap_or(&item.path).display()
            ));
        } else {
            r.line(format!("  {:<9} {}", item.state.label(), item.name));
            changed += 1;
        }
    }

    changed
}

/// How long a mutating command waits for a competing one.
pub fn lock_timeout(env: &Env) -> Duration {
    let ms = env
        .var("AGENTSTOW_LOCK_TIMEOUT_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(lock::DEFAULT_TIMEOUT_MS);
    Duration::from_millis(ms)
}
