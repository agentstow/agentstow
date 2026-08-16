//! `sync` — make every Target match the Commons.

use std::path::{Path, PathBuf};

use crate::commons::{self, Commons};
use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::hooks;
use crate::instructions;
use crate::link::{self, Item};
use crate::lock;
use crate::mcp;
use crate::render;
use crate::report::Reporter;
use crate::target;
use crate::{EXIT_CLEAN, EXIT_ERROR};

pub fn run(env: &Env, config: &Config, r: &mut Reporter, dry_run: bool) -> i32 {
    let commons = Commons::new(env.commons());
    if !commons.exists() {
        r.problem(commons::missing_message(commons.root()));
        return EXIT_ERROR;
    }

    // A dry run only reads, so it never contends for the lock.
    let _lock = if dry_run {
        None
    } else {
        match lock::acquire(env.config_dir(), lock::timeout(env)) {
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
    let targets = target::resolve(env, config);

    for family in Family::ALL {
        let scan = commons.scan(*family);
        for issue in &scan.issues {
            r.warn(issue.to_string());
        }

        for target in &targets {
            let Some(dir) = target.fanout_dir(*family) else {
                continue;
            };
            let target_dir = env.in_home(dir);
            let items = link::survey(&target_dir, commons.root(), &scan.entries);
            changes += sync_target(&target.name, dir, &items, env.home(), r, dry_run);
        }
    }

    changes += sync_instructions(env, config, &commons, r, dry_run);

    match sync_mcp(env, config, &commons, r, dry_run) {
        Ok(n) => changes += n,
        Err(e) => {
            // Nothing has been written: an MCP failure stops the whole family
            // rather than leaving some agents updated and others not.
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    }

    match sync_rendered(env, config, &commons, r, dry_run) {
        Ok(n) => changes += n,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    }

    // Hooks run after MCP because Gemini keeps both in one settings file, and
    // each family re-reads it before merging.
    match sync_hooks(env, config, &commons, r, dry_run) {
        Ok(n) => changes += n,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    }

    if changes == 0 && r.problem_count() == 0 {
        r.line("Everything is up to date.");
    } else if changes == 0 {
        r.blank();
        r.line("No changes were made — see the errors above.");
    } else {
        r.blank();
        let noun = if changes == 1 { "change" } else { "changes" };
        let verb = if dry_run { "would be made" } else { "made" };
        r.line(format!("{changes} {noun} {verb}."));
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else {
        EXIT_CLEAN
    }
}

/// The rendered family: whole files generated for agents that cannot take a
/// symlink, each carrying the Marker that makes it ours to replace.
fn sync_rendered(
    env: &Env,
    config: &Config,
    commons: &Commons,
    r: &mut Reporter,
    dry_run: bool,
) -> Result<usize, render::Error> {
    let items = render::survey(env, config, commons)?;
    let noteworthy: Vec<&render::Item> = items
        .iter()
        .filter(|i| i.state != render::State::Managed)
        .collect();
    if noteworthy.is_empty() {
        return Ok(0);
    }

    r.line("rendered commands");
    for item in &noteworthy {
        r.line(format!(
            "  {:<10} {} → {} — {}",
            item.state.label(),
            item.name,
            item.target,
            item.state.note()
        ));
    }

    let changing: Vec<&&render::Item> = noteworthy
        .iter()
        .filter(|i| i.state.needs_change())
        .collect();
    if changing.is_empty() || dry_run {
        return Ok(changing.len());
    }

    let mut written = 0usize;
    for item in &changing {
        match render::apply(item) {
            Ok(()) => written += 1,
            Err(e) => r.problem(e.to_string()),
        }
    }
    Ok(written)
}

/// The hooks family: command-hooks merged into each agent's hook arrays.
fn sync_hooks(
    env: &Env,
    config: &Config,
    commons: &Commons,
    r: &mut Reporter,
    dry_run: bool,
) -> Result<usize, hooks::Error> {
    let commons_dir = commons.family_dir(commons::HOOKS);
    let survey = hooks::survey(env, config, &commons_dir)?;
    for skipped in &survey.skipped {
        r.problem(skipped.clone());
    }

    let noteworthy: Vec<&hooks::Item> = survey
        .items
        .iter()
        .filter(|i| i.state != hooks::State::Managed)
        .collect();
    if noteworthy.is_empty() {
        return Ok(0);
    }

    r.line("hooks");
    for item in &noteworthy {
        r.line(format!(
            "  {:<12} {} {} → {} — {}",
            item.state.label(),
            item.event,
            short(&item.label),
            item.target,
            item.note()
        ));
    }

    let changing: Vec<&hooks::Item> = noteworthy
        .into_iter()
        .filter(|i| i.state.needs_change())
        .collect();
    if changing.is_empty() || dry_run {
        return Ok(changing.len());
    }

    let mut by_file: Vec<(PathBuf, Vec<&hooks::Item>)> = Vec::new();
    for item in changing.iter().copied() {
        match by_file.iter_mut().find(|(path, _)| path == &item.path) {
            Some((_, group)) => group.push(item),
            None => by_file.push((item.path.clone(), vec![item])),
        }
    }

    let mut written = 0usize;
    for (path, group) in &by_file {
        match hooks::apply(path, group[0].root_key, group) {
            Ok(report) => {
                written += group.len();
                if report.exposed {
                    r.warn(format!(
                        "{} has mode {:o} — it now holds resolved secrets and can be read by others",
                        report.path.display(),
                        report.mode
                    ));
                }
            }
            Err(e) => r.problem(e.to_string()),
        }
    }

    Ok(written)
}

/// A command, shortened for a report line. Never resolved.
fn short(command: &str) -> String {
    const MAX: usize = 40;
    if command.chars().count() <= MAX {
        return command.to_string();
    }
    let head: String = command.chars().take(MAX - 1).collect();
    format!("{head}…")
}

/// The MCP family: one Commons file, rendered and key-merged per agent.
fn sync_mcp(
    env: &Env,
    config: &Config,
    commons: &Commons,
    r: &mut Reporter,
    dry_run: bool,
) -> Result<usize, mcp::Error> {
    let commons_file = commons.root().join(commons::MCP);
    let survey = mcp::survey(env, config, &commons_file)?;
    // A config agentstow cannot parse is a real fault worth an exit code, but
    // it must not stop the Targets that are healthy.
    for skipped in &survey.skipped {
        r.problem(skipped.clone());
    }
    let noteworthy: Vec<&mcp::Item> = survey
        .items
        .iter()
        .filter(|i| i.state != mcp::State::Managed)
        .collect();
    if noteworthy.is_empty() {
        return Ok(0);
    }

    r.line("mcp (mcp.json)");
    for item in &noteworthy {
        r.line(format!(
            "  {:<9} {} → {} — {}",
            item.state.label(),
            item.name,
            item.target,
            item.note()
        ));
    }

    let changing: Vec<&mcp::Item> = noteworthy
        .into_iter()
        .filter(|i| i.state.needs_change())
        .collect();
    if changing.is_empty() || dry_run {
        return Ok(changing.len());
    }

    // One write per config file, not per server: an agent may take more than
    // one family in the same file.
    let mut by_file: Vec<(PathBuf, Vec<&mcp::Item>)> = Vec::new();
    for item in changing.iter().copied() {
        match by_file.iter_mut().find(|(path, _)| path == &item.path) {
            Some((_, group)) => group.push(item),
            None => by_file.push((item.path.clone(), vec![item])),
        }
    }

    // One agent's write failing must not cancel the others: everything was
    // already resolved before any file was opened, so the remaining writes are
    // still correct. Each failure is reported and the run ends non-clean.
    let mut written = 0usize;
    for (path, group) in &by_file {
        let root_key = group[0].root_key;
        match mcp::apply(path, root_key, group[0].format, group) {
            Ok(report) => {
                written += group.len();
                if report.exposed {
                    r.warn(format!(
                        "{} has mode {:o} — it now holds resolved secrets and can be read by others",
                        report.path.display(),
                        report.mode
                    ));
                }
            }
            Err(e) => r.problem(e.to_string()),
        }
    }

    Ok(written)
}

/// The instructions family: one Commons file, three per-agent mechanisms.
fn sync_instructions(
    env: &Env,
    config: &Config,
    commons: &Commons,
    r: &mut Reporter,
    dry_run: bool,
) -> usize {
    let commons_file = commons.root().join(commons::INSTRUCTIONS);
    let items = instructions::survey(env, config, &commons_file);
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
