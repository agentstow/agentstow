//! `sync` — make every Target match the Store.

use std::path::Path;
use std::time::Duration;

use crate::env::Env;
use crate::link::{self, Act};
use crate::lock;
use crate::registry::{self, Agent, Skills};
use crate::report::Reporter;
use crate::store::{self, Store};
use crate::{EXIT_CLEAN, EXIT_ERROR};

pub fn run(env: &Env, r: &mut Reporter, dry_run: bool) -> i32 {
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

    let skills = store.scan_dirs(store::SKILLS);
    for issue in &skills.issues {
        r.warn(issue.to_string());
    }

    if dry_run {
        r.line("dry run — no changes will be made");
        r.blank();
    }

    let mut changes = 0usize;

    for agent in registry::detected(env.home()) {
        let Skills::FanOut(dir) = agent.skills else {
            continue;
        };
        let target_dir = env.in_home(dir);
        let acts = link::plan(&target_dir, store.root(), &skills.entries);
        changes += report_target(agent, &target_dir, &acts, env.home(), r, dry_run);
    }

    if changes == 0 {
        r.line("Everything is up to date.");
    } else if dry_run {
        r.blank();
        r.line(format!("{changes} changes would be made."));
    } else {
        r.blank();
        r.line(format!("{changes} changes made."));
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else {
        EXIT_CLEAN
    }
}

/// Report (and unless this is a dry run, apply) one Target's plan. Returns the
/// number of mutating acts.
fn report_target(
    agent: &Agent,
    target_dir: &Path,
    acts: &[Act],
    home: &Path,
    r: &mut Reporter,
    dry_run: bool,
) -> usize {
    if acts.is_empty() {
        return 0;
    }

    let mut mutations = 0;
    let mut header_written = false;

    for act in acts {
        let rel = act
            .path()
            .strip_prefix(home)
            .unwrap_or(act.path())
            .display()
            .to_string();

        if !header_written {
            r.line(format!(
                "{} ({})",
                agent.name,
                display_dir(target_dir, home)
            ));
            header_written = true;
        }

        match act {
            Act::Variant { identical, .. } => {
                if *identical {
                    r.line(format!(
                        "  variant  {rel} — identical to the Store, could be re-linked"
                    ));
                } else {
                    r.line(format!("  variant  {rel} — left alone"));
                }
            }
            Act::Foreign { .. } => {
                r.line(format!("  foreign  {rel} — left alone"));
            }
            _ => {
                mutations += 1;
                if dry_run {
                    r.line(format!("  {:<8} {rel}", act.verb()));
                } else if let Err(e) = link::apply(act) {
                    r.problem(format!("cannot {} {rel}: {e}", act.verb()));
                } else {
                    r.line(format!("  {:<8} {rel}", act.verb()));
                }
            }
        }
    }

    mutations
}

fn display_dir(dir: &Path, home: &Path) -> String {
    dir.strip_prefix(home).unwrap_or(dir).display().to_string()
}

fn lock_timeout(env: &Env) -> Duration {
    let ms = env
        .var("AGENTSTOW_LOCK_TIMEOUT_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(lock::DEFAULT_TIMEOUT_MS);
    Duration::from_millis(ms)
}
