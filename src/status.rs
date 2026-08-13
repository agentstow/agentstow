//! `status` — what is in sync, what is not, and what is deliberately not ours.
//!
//! Read-only. The exit code is the contract automation depends on: 0 clean,
//! 2 something actionable, 1 the command could not answer.

use serde_json::{json, Value};

use crate::config::Config;
use crate::env::Env;
use crate::link::{self, Item, State};
use crate::report::Reporter;
use crate::store::{Store, FANOUT_FAMILIES};
use crate::target;
use crate::{EXIT_ACTIONABLE, EXIT_CLEAN, EXIT_ERROR};

/// One Target directory's worth of survey results.
struct TargetReport {
    agent: String,
    family: &'static str,
    dir: String,
    items: Vec<Item>,
}

pub fn run(env: &Env, config: &Config, r: &mut Reporter, as_json: bool) -> i32 {
    let store = Store::new(env.store());
    if !store.exists() {
        r.problem(format!(
            "no Store at {} — run `agentstow init` to create one",
            store.root().display()
        ));
        return EXIT_ERROR;
    }

    let mut targets: Vec<TargetReport> = Vec::new();

    for (family, shape) in FANOUT_FAMILIES {
        let scan = store.scan(family, *shape);
        for issue in &scan.issues {
            r.warn(issue.to_string());
        }

        for target in target::resolve(env, config) {
            let Some(dir) = target.fanout_dir(family) else {
                continue;
            };
            let items = link::survey(&env.in_home(dir), store.root(), &scan.entries);
            targets.push(TargetReport {
                agent: target.name.clone(),
                family,
                dir: dir.to_string(),
                items,
            });
        }
    }

    let actionable: usize = targets
        .iter()
        .flat_map(|t| &t.items)
        .filter(|i| i.state.actionable())
        .count();

    if as_json {
        r.json(&as_value(&store, &targets, actionable));
    } else {
        human(&targets, actionable, r);
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else if actionable > 0 {
        EXIT_ACTIONABLE
    } else {
        EXIT_CLEAN
    }
}

fn human(targets: &[TargetReport], actionable: usize, r: &mut Reporter) {
    for target in targets {
        let linked = target
            .items
            .iter()
            .filter(|i| i.state == State::Linked)
            .count();
        let noteworthy: Vec<&Item> = target
            .items
            .iter()
            .filter(|i| i.state != State::Linked)
            .collect();

        if linked == 0 && noteworthy.is_empty() {
            continue;
        }

        r.line(format!(
            "{} ({})  {linked} linked",
            target.agent, target.dir
        ));
        for item in noteworthy {
            r.line(format!(
                "  {:<17} {} — {}",
                item.state.label(),
                item.name,
                item.state.note()
            ));
        }
    }

    r.blank();
    if actionable == 0 {
        r.line("Everything is in sync.");
    } else {
        r.line(format!(
            "{actionable} items need attention — run `agentstow sync`."
        ));
    }
}

fn as_value(store: &Store, targets: &[TargetReport], actionable: usize) -> Value {
    let targets: Vec<Value> = targets
        .iter()
        .map(|t| {
            let entries: Vec<Value> = t
                .items
                .iter()
                .map(|i| {
                    json!({
                        "name": i.name,
                        "state": i.state.label(),
                        "actionable": i.state.actionable(),
                    })
                })
                .collect();
            json!({
                "agent": t.agent,
                "family": t.family,
                "dir": t.dir,
                "entries": entries,
            })
        })
        .collect();

    json!({
        "store": store.root().display().to_string(),
        "clean": actionable == 0,
        "actionable": actionable,
        "targets": targets,
    })
}
