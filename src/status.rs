//! `status` — what is in sync, what is not, and what is deliberately not ours.
//!
//! Read-only. The exit code is the contract automation depends on: 0 clean,
//! 2 something actionable, 1 the command could not answer.

use serde_json::{json, Value};

use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::hooks;
use crate::instructions;
use crate::link::{self, Item, State};
use crate::mcp;
use crate::render;
use crate::report::Reporter;
use crate::store::{self, Store};
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
        r.problem(store::missing_message(store.root()));
        return EXIT_ERROR;
    }

    let mut targets: Vec<TargetReport> = Vec::new();

    let resolved = target::resolve(env, config);

    for family in Family::ALL {
        let scan = store.scan(*family);
        for issue in &scan.issues {
            r.warn(issue.to_string());
        }

        for target in &resolved {
            let Some(dir) = target.fanout_dir(*family) else {
                continue;
            };
            let items = link::survey(&env.in_home(dir), store.root(), &scan.entries);
            targets.push(TargetReport {
                agent: target.name.clone(),
                family: family.name(),
                dir: dir.to_string(),
                items,
            });
        }
    }

    let store_file = store.root().join(store::INSTRUCTIONS);
    let instruction_items = instructions::survey(env, config, &store_file);

    let mcp_survey = match mcp::survey(env, config, &store.root().join(store::MCP)) {
        Ok(survey) => survey,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };
    // Reported and counted, but the rest of the report still prints: one
    // unreadable third-party file must never blank out a read-only answer.
    for skipped in &mcp_survey.skipped {
        r.problem(skipped.clone());
    }
    let mcp_items = mcp_survey.items;

    let hook_survey = match hooks::survey(env, config, &store.family_dir(store::HOOKS)) {
        Ok(survey) => survey,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };
    for skipped in &hook_survey.skipped {
        r.problem(skipped.clone());
    }
    let hook_items = hook_survey.items;

    let rendered_items = match render::survey(env, config, &store) {
        Ok(items) => items,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    let actionable: usize = targets
        .iter()
        .flat_map(|t| &t.items)
        .filter(|i| i.state.actionable())
        .count()
        + instruction_items
            .iter()
            .filter(|i| i.state.actionable())
            .count()
        + mcp_items.iter().filter(|i| i.state.actionable()).count()
        + hook_items.iter().filter(|i| i.state.actionable()).count()
        + rendered_items
            .iter()
            .filter(|i| i.state.actionable())
            .count();

    if as_json {
        r.json(&as_value(
            &store,
            &targets,
            &instruction_items,
            &mcp_items,
            &hook_items,
            &rendered_items,
            actionable,
        ));
    } else {
        human(
            &targets,
            &instruction_items,
            &mcp_items,
            &hook_items,
            &rendered_items,
            actionable,
            r,
        );
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else if actionable > 0 {
        EXIT_ACTIONABLE
    } else {
        EXIT_CLEAN
    }
}

fn human(
    targets: &[TargetReport],
    instruction_items: &[instructions::Item],
    mcp_items: &[mcp::Item],
    hook_items: &[hooks::Item],
    rendered_items: &[render::Item],
    actionable: usize,
    r: &mut Reporter,
) {
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

    if !instruction_items.is_empty() {
        r.line("instructions (AGENTS.md)");
        for item in instruction_items {
            let note = item.note();
            if note.is_empty() {
                r.line(format!("  {:<17} {}", item.state.label(), item.target));
            } else {
                r.line(format!(
                    "  {:<17} {} — {note}",
                    item.state.label(),
                    item.target
                ));
            }
        }
    }

    if !mcp_items.is_empty() {
        r.line("mcp (mcp.json)");
        for item in mcp_items {
            let note = item.note();
            let name = format!("{} → {}", item.name, item.target);
            if note.is_empty() {
                r.line(format!("  {:<17} {name}", item.state.label()));
            } else {
                r.line(format!("  {:<17} {name} — {note}", item.state.label()));
            }
        }
    }

    if !hook_items.is_empty() {
        r.line("hooks");
        for item in hook_items {
            let note = item.note();
            let name = format!("{} {} → {}", item.event, item.label, item.target);
            if note.is_empty() {
                r.line(format!("  {:<17} {name}", item.state.label()));
            } else {
                r.line(format!("  {:<17} {name} — {note}", item.state.label()));
            }
        }
    }

    if !rendered_items.is_empty() {
        r.line("rendered commands");
        for item in rendered_items {
            let note = item.state.note();
            let name = format!("{} → {}", item.name, item.target);
            if note.is_empty() {
                r.line(format!("  {:<17} {name}", item.state.label()));
            } else {
                r.line(format!("  {:<17} {name} — {note}", item.state.label()));
            }
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

fn as_value(
    store: &Store,
    targets: &[TargetReport],
    instruction_items: &[instructions::Item],
    mcp_items: &[mcp::Item],
    hook_items: &[hooks::Item],
    rendered_items: &[render::Item],
    actionable: usize,
) -> Value {
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

    let instructions: Vec<Value> = instruction_items
        .iter()
        .map(|i| {
            json!({
                "agent": i.target,
                "state": i.state.label(),
                "actionable": i.state.actionable(),
            })
        })
        .collect();

    let mcp: Vec<Value> = mcp_items
        .iter()
        .map(|i| {
            json!({
                "agent": i.target,
                "name": i.name,
                "state": i.state.label(),
                "actionable": i.state.actionable(),
            })
        })
        .collect();

    let hooks: Vec<Value> = hook_items
        .iter()
        .map(|i| {
            json!({
                "agent": i.target,
                "event": i.event,
                "hook": i.label,
                "state": i.state.label(),
                "actionable": i.state.actionable(),
            })
        })
        .collect();

    let rendered: Vec<Value> = rendered_items
        .iter()
        .map(|i| {
            json!({
                "agent": i.target,
                "name": i.name,
                "state": i.state.label(),
                "actionable": i.state.actionable(),
            })
        })
        .collect();

    json!({
        "store": store.root().display().to_string(),
        "instructions": instructions,
        "mcp": mcp,
        "hooks": hooks,
        "rendered": rendered,
        "clean": actionable == 0,
        "actionable": actionable,
        "targets": targets,
    })
}
