//! `agentstow mcp list | adopt | remove`.
//!
//! `remove` is the one operation in the whole tool that deletes something from
//! a Target. That is deliberate: without state, a name that has vanished from
//! the Commons cannot be told apart from one a user added by hand, so removal has
//! to be asked for by name rather than inferred (ADR-0002).

use serde_json::json;
use std::collections::BTreeMap;

use crate::commons::{self, Commons};
use crate::config::Config;
use crate::config_edit;
use crate::env::Env;
use crate::mcp::{self, State};
use crate::registry::Mcp;
use crate::report::Reporter;
use crate::target::{self, Target};
use crate::{EXIT_ACTIONABLE, EXIT_CLEAN, EXIT_ERROR};

/// Where a server lives in one agent, and in what state.
struct Row {
    agent: String,
    state: State,
}

pub fn list(env: &Env, config: &Config, r: &mut Reporter, as_json: bool) -> i32 {
    let commons = Commons::new(env.commons());
    let commons_file = commons.root().join(commons::MCP);

    let survey = mcp::survey(env, config, &commons_file);
    if !survey.problems.is_empty() {
        // A Commons fault means there is no true answer to list; every fault
        // is named — not just the first — and the command refuses.
        for problem in &survey.problems {
            r.problem(problem);
        }
        return EXIT_ERROR;
    }
    for skipped in &survey.skipped {
        r.problem(skipped.clone());
    }

    let commons_names: Vec<String> = match mcp::commons_servers(&commons_file) {
        Ok(servers) => servers.keys().cloned().collect(),
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    // Group by server name, Commons servers first, then anything Foreign.
    let mut names = commons_names.clone();
    for item in &survey.items {
        if !names.contains(&item.name) {
            names.push(item.name.clone());
        }
    }

    if names.is_empty() {
        if as_json {
            r.json(&json!({"servers": []}));
        } else {
            r.line("no MCP servers in the Commons, and none found in any agent's config");
        }
        return EXIT_CLEAN;
    }

    let mut actionable = 0usize;
    let mut payload = Vec::new();

    for name in &names {
        let rows: Vec<Row> = survey
            .items
            .iter()
            .filter(|i| &i.name == name)
            .map(|i| Row {
                agent: i.target.clone(),
                state: i.state,
            })
            .collect();
        actionable += rows.iter().filter(|row| row.state.actionable()).count();
        let managed = commons_names.contains(name);

        if as_json {
            payload.push(json!({
                "name": name,
                "managed": managed,
                "targets": rows.iter().map(|row| json!({
                    "agent": row.agent,
                    "state": row.state.label(),
                    "actionable": row.state.actionable(),
                })).collect::<Vec<_>>(),
            }));
        } else {
            let origin = if managed {
                "Commons"
            } else {
                "not in the Commons"
            };
            r.line(format!("{name}  ({origin})"));
            for row in &rows {
                r.line(format!("  {:<10} {}", row.state.label(), row.agent));
            }
        }
    }

    if as_json {
        r.json(&json!({"servers": payload}));
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else if actionable > 0 {
        EXIT_ACTIONABLE
    } else {
        EXIT_CLEAN
    }
}

pub fn remove(env: &Env, config: &Config, r: &mut Reporter, name: &str) -> i32 {
    let commons = Commons::new(env.commons());
    let commons_file = commons.root().join(commons::MCP);

    let mut servers = match mcp::commons_servers(&commons_file) {
        Ok(servers) => servers,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    if !servers.contains_key(name) {
        r.problem(format!(
            "`{name}` is not in the Commons — agentstow only removes servers it manages"
        ));
        return EXIT_ERROR;
    }

    let _lock = match crate::lock::acquire(env.state_dir(), crate::lock::timeout(env)) {
        Ok(guard) => guard,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    // Targets first, then the config, then the Commons: at every point in between,
    // what remains is still coherent.
    for target in target::resolve(env, config) {
        let Some((path, root_key, format)) = mcp_destination(env, &target) else {
            continue;
        };
        match mcp::strip(&path, root_key, format, name) {
            Ok(Some(_)) => r.line(format!("removed {name} from {}", target.name)),
            Ok(None) => {}
            Err(e) => r.problem(e.to_string()),
        }
    }

    if let Err(e) = config_edit::remove_mcp_rule(env.config_dir(), name) {
        r.problem(e.to_string());
    }

    servers.shift_remove(name);
    if let Err(e) = mcp::commons_write(&commons_file, &servers) {
        r.problem(e.to_string());
        return EXIT_ERROR;
    }
    r.line(format!("removed {name} from the Commons"));

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else {
        EXIT_CLEAN
    }
}

pub fn adopt(env: &Env, config: &Config, r: &mut Reporter, spec: Option<&str>, all: bool) -> i32 {
    let targets = target::resolve(env, config);

    // Which (agent, server) pairs to take.
    let wanted: Vec<(Target, String)> = if all {
        let mut found = Vec::new();
        for target in &targets {
            let Some((path, root_key, format)) = mcp_destination(env, target) else {
                continue;
            };
            match mcp::native_servers(&path, root_key, format) {
                Ok(servers) => {
                    for name in servers.keys() {
                        found.push((target.clone(), name.clone()));
                    }
                }
                Err(e) => r.problem(e.to_string()),
            }
        }
        found
    } else {
        let Some(spec) = spec else {
            r.problem("give a server to adopt as agent/server, or --all");
            return EXIT_ERROR;
        };
        let Some((agent, name)) = spec.split_once('/') else {
            r.problem(format!(
                "`{spec}` is not an agent/server pair — try `agentstow mcp adopt claude/serena`"
            ));
            return EXIT_ERROR;
        };
        let Some(target) = targets.iter().find(|t| t.name == agent) else {
            r.problem(format!(
                "`{agent}` is not an agent agentstow knows about, or it is not installed"
            ));
            return EXIT_ERROR;
        };
        vec![(target.clone(), name.to_string())]
    };

    if wanted.is_empty() {
        r.line("no servers found to adopt");
        return EXIT_CLEAN;
    }

    let commons = Commons::new(env.commons());
    let commons_file = commons.root().join(commons::MCP);
    let mut servers = match mcp::commons_servers(&commons_file) {
        Ok(servers) => servers,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    let _lock = match crate::lock::acquire(env.state_dir(), crate::lock::timeout(env)) {
        Ok(guard) => guard,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    let capable: Vec<String> = targets
        .iter()
        .filter(|t| mcp_destination(env, t).is_some())
        .map(|t| t.name.clone())
        .collect();
    let mut adopted = 0usize;
    // Agents whose entry was genuinely absorbed, per server. The allowlist is
    // built from this and never from what was merely asked for: naming an agent
    // whose entry was refused would let the next sync overwrite it.
    let mut taken: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (target, name) in &wanted {
        let Some((path, root_key, format)) = mcp_destination(env, target) else {
            r.problem(format!("{} has no MCP surface", target.name));
            continue;
        };
        let dialect = match target.agent.map(|a| a.mcp) {
            Some(Mcp::KeyMerge { dialect, .. }) => dialect,
            _ => continue,
        };

        let native = match mcp::native_servers(&path, root_key, format) {
            Ok(servers) => match servers.get(name) {
                Some(entry) => entry.clone(),
                None => {
                    r.problem(format!("`{name}` is not in {}'s config", target.name));
                    continue;
                }
            },
            Err(e) => {
                r.problem(e.to_string());
                continue;
            }
        };

        let absorbed = match mcp::absorb(name, &native, dialect) {
            Ok(absorbed) => absorbed,
            Err(e) => {
                r.problem(e.to_string());
                continue;
            }
        };

        let known = servers.get(name).cloned();
        if let Some(existing) = &known
            && existing != &absorbed.canonical
        {
            r.problem(format!(
                "`{name}` differs from the Commons copy — merge it by hand; agentstow will not \
                 choose which side to discard"
            ));
            continue;
        }

        // Prove the claim before making it: an adoption that would not survive
        // the next sync is not an adoption, it is a pending overwrite.
        let tweaks = (!absorbed.tweaks.is_empty()).then(|| absorbed.tweaks.clone());
        match mcp::verify(
            name,
            &absorbed.canonical,
            dialect,
            &native,
            env,
            tweaks.as_ref(),
        ) {
            Ok(keys) if keys.is_empty() => {}
            Ok(keys) => {
                r.problem(format!(
                    "`{name}` in {}: {} cannot be represented faithfully — not adopted",
                    target.name,
                    keys.join(", ")
                ));
                continue;
            }
            Err(e) => {
                r.problem(e.to_string());
                continue;
            }
        }

        // Scope it to the agents it actually came from, so adopting records
        // what is rather than deciding to spread it.
        let sources = taken.entry(name.clone()).or_default();
        if !sources.contains(&target.name) {
            sources.push(target.name.clone());
        }
        let allowlist = (sources.len() < capable.len()).then(|| sources.clone());

        if let Err(e) = config_edit::set_mcp_rule(
            env.config_dir(),
            name,
            allowlist.as_deref(),
            &absorbed.tweaks,
            &target.name,
        ) {
            r.problem(e.to_string());
            // The agent never made it into the config, so it must not stay in
            // the allowlist either.
            if let Some(sources) = taken.get_mut(name) {
                sources.retain(|a| a != &target.name);
            }
            continue;
        }

        if known.is_some() {
            r.line(format!("{name} is already in the Commons"));
        } else {
            servers.insert(name.clone(), absorbed.canonical.clone());
            adopted += 1;
            r.line(format!("adopted {name} from {}", target.name));
        }
        for note in &absorbed.notes {
            r.line(format!("  note: {note}"));
        }
        if let Some(list) = &allowlist {
            r.line(format!(
                "  scoped to {} — remove the allowlist in agentstow.toml to share it",
                list.join(", ")
            ));
        }
        warn_about_literal_secrets(name, &absorbed.canonical, r);
    }

    if adopted > 0
        && let Err(e) = mcp::commons_write(&commons_file, &servers)
    {
        r.problem(e.to_string());
        return EXIT_ERROR;
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else {
        EXIT_CLEAN
    }
}

/// Where one Target keeps its MCP servers, if it takes them at all.
fn mcp_destination(
    env: &Env,
    target: &Target,
) -> Option<(std::path::PathBuf, &'static str, crate::registry::Format)> {
    let agent = target.agent?;
    match agent.mcp {
        Mcp::KeyMerge {
            file,
            root_key,
            format,
            ..
        } => Some((env.in_home(file), root_key, format)),
        _ => None,
    }
}

/// Adoption copies values verbatim out of an agent's config, so a credential
/// somebody typed there lands in the Commons — the file this design encourages
/// committing. Say so, by key name only.
fn warn_about_literal_secrets(name: &str, canonical: &serde_json::Value, r: &mut Reporter) {
    const SUSPICIOUS: &[&str] = &["token", "secret", "password", "passwd", "key", "auth"];

    let mut found: Vec<String> = Vec::new();
    for section in ["env", "headers"] {
        let Some(fields) = canonical.get(section).and_then(|v| v.as_object()) else {
            continue;
        };
        for (key, value) in fields {
            let literal = value
                .as_str()
                .map(|v| !v.contains("${env:"))
                .unwrap_or(false);
            let looks_secret = SUSPICIOUS
                .iter()
                .any(|needle| key.to_ascii_lowercase().contains(needle));
            if literal && looks_secret {
                found.push(format!("{section}.{key}"));
            }
        }
    }

    if !found.is_empty() {
        found.sort();
        r.warn(format!(
            "`{name}`: {} look like credentials and were copied into the Commons as written — \
             replace them with ${{env:VAR}} before committing it",
            found.join(", ")
        ));
    }
}
