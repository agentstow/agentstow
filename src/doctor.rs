//! `doctor` — is this machine ready to sync?
//!
//! Reports what is installed and what would be silently skipped. It is strictly
//! read-only: doctor never creates a directory, least of all an agent root,
//! because a root's existence is what detection means.

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::commons::{self, Commons};
use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::link;
use crate::registry;
use crate::registry::Skills;
use crate::report::Reporter;
use crate::target;

pub fn run(env: &Env, config: &Config, r: &mut Reporter) -> i32 {
    let commons = Commons::new(env.commons());

    r.line(format!("Commons {}", commons.root().display()));
    r.line(format!("Home    {}", env.home().display()));
    r.line(format!("Config  {}", env.config_dir().display()));
    r.line(format!("State   {}", env.state_dir().display()));
    r.blank();

    if env.legacy_config_dir().is_dir() {
        r.warn(format!(
            "{} is no longer read — move agentstow.toml to {} and delete the directory",
            env.legacy_config_dir().display(),
            env.config_dir().display()
        ));
    }

    if commons.exists() {
        report_commons(&commons, r);
    } else {
        r.problem(commons::missing_message(commons.root()));
    }

    // The agents family was `subagents/` before v2; a non-empty leftover is
    // silently invisible to sync, so name it.
    let pre_v2_agents = commons.root().join("subagents");
    if std::fs::read_dir(&pre_v2_agents).is_ok_and(|mut d| d.next().is_some()) {
        r.warn(format!(
            "{} is no longer a family — the agents family reads agents/; rename the directory",
            pre_v2_agents.display()
        ));
    }

    report_commons_override(env, config, r);
    report_agents(env, config, r);

    r.verdict()
}

fn report_commons(commons: &Commons, r: &mut Reporter) {
    let scans: Vec<(Family, commons::Scan)> = Family::ALL
        .iter()
        .map(|family| (*family, commons.scan(*family)))
        .collect();

    let hooks = commons.family_dir(commons::HOOKS);
    let hook_files: Vec<String> = std::fs::read_dir(&hooks)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();

    let instructions = if commons.root().join(commons::INSTRUCTIONS).exists() {
        "present"
    } else {
        "absent"
    };
    let mcp = if commons.root().join(commons::MCP).exists() {
        "present"
    } else {
        "absent"
    };

    let sourced = sourced_entries(commons);

    r.line("Commons contents:");
    for (family, scan) in &scans {
        let n = sourced.iter().filter(|s| s.family == *family).count();
        if n > 0 {
            r.line(format!(
                "  {:<13} {} ({n} sourced)",
                family.name(),
                scan.entries.len()
            ));
        } else {
            r.line(format!("  {:<13} {}", family.name(), scan.entries.len()));
        }
    }
    if hooks.is_dir() {
        let count = hook_files.iter().filter(|n| n.ends_with(".toml")).count();
        r.line(format!("  hooks         {count}"));
    } else {
        r.line("  hooks         absent");
    }
    r.line(format!("  AGENTS.md     {instructions}"));
    r.line(format!("  mcp.json      {mcp}"));
    r.blank();

    // The machine-bootstrap question "what must I clone?" (ADR-0006): every
    // Sourced entry with its source, missing sources marked.
    if !sourced.is_empty() {
        r.line("Sourced entries:");
        for s in &sourced {
            let marker = if s.missing { " (source missing)" } else { "" };
            r.line(format!(
                "  {}/{} ← {}{marker}",
                s.family.name(),
                s.name,
                s.source.display()
            ));
        }
        r.blank();
    }

    let others = co_tenants(commons);
    if !others.is_empty() {
        r.line("Other tools in the Commons:");
        for name in &others {
            r.line(format!("  {name}"));
        }
        r.blank();
    }

    for (_, scan) in &scans {
        for issue in &scan.issues {
            r.warn(issue.to_string());
        }
    }

    // The hooks family reads `<Event>.toml` only, so anything else in there
    // would be skipped in silence — the failure this tool exists to end.
    for name in hook_files.iter().filter(|n| !n.ends_with(".toml")) {
        r.warn(format!(
            "Commons hooks/{name}: not a `<Event>.toml` file, skipped"
        ));
    }
}

/// One Sourced entry: a Commons symlink out to its Source.
struct Sourced {
    family: Family,
    name: String,
    /// Where the link points, resolved lexically — live or not.
    source: PathBuf,
    /// The source does not resolve: the repo is not cloned here (yet).
    missing: bool,
}

/// Every Sourced entry, gathered by reading the family directories directly:
/// the scan skips a dangling link (`Issue::DanglingLink`), and a Sourced entry
/// whose repo is not cloned yet is exactly what this view exists to name.
fn sourced_entries(commons: &Commons) -> Vec<Sourced> {
    let mut acc = Vec::new();
    for family in Family::ALL {
        let dir = commons.family_dir(family.name());
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Dot-prefixed names are never synced; the scan already warns.
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            let is_link = std::fs::symlink_metadata(&path)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            if !is_link {
                continue;
            }
            let Ok(text) = std::fs::read_link(&path) else {
                continue;
            };
            acc.push(Sourced {
                family: *family,
                name,
                source: link::resolve_link(&path, &text),
                missing: std::fs::metadata(&path).is_err(),
            });
        }
    }
    acc.sort_by(|a, b| a.family.cmp(&b.family).then_with(|| a.name.cmp(&b.name)));
    acc
}

/// Names at the Commons root that are not agentstow's own families.
///
/// The Commons is a shared commons, not agentstow's private directory (ADR-0004):
/// opencode, oh-my-pi and hermes read `~/.agents/` themselves, and the `skills`
/// CLI keeps its lock file there. An unrecognised name is a neighbour, not a
/// fault — so these are named and never counted, never called an issue, and
/// never touched. agentstow can read filenames but not authorship, so it must
/// not claim how many *tools* are present, only which entries are not its own.
fn co_tenants(commons: &Commons) -> Vec<String> {
    let ours: Vec<&str> = Family::ALL
        .iter()
        .map(|f| f.name())
        .chain([commons::HOOKS, commons::INSTRUCTIONS, commons::MCP])
        .collect();

    let Ok(read) = std::fs::read_dir(commons.root()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| !ours.contains(&name.as_str()))
        .collect();
    names.sort();
    names
}

/// Warn when a relocated Commons is invisible to the agents that read the
/// canonical path themselves.
///
/// `AGENTSTOW_HOME` moves agentstow's Commons, but it cannot move the path a
/// Native agent hardcodes. Those agents keep reading `~/.agents/` and silently
/// diverge from every agent that gets fan-out — so name them.
fn report_commons_override(env: &Env, config: &Config, r: &mut Reporter) {
    if env
        .var("AGENTSTOW_HOME")
        .filter(|v| !v.is_empty())
        .is_none()
    {
        return;
    }

    let canonical = env.home().join(crate::env::COMMONS_DIR);
    if link::normalize(env.commons()) == link::normalize(&canonical) {
        return;
    }

    let native: Vec<&str> = target::resolve(env, config)
        .iter()
        .filter_map(|t| t.agent)
        .filter(|a| a.skills == Skills::Native)
        .map(|a| a.name)
        .collect();
    if native.is_empty() {
        return;
    }

    r.warn(format!(
        "AGENTSTOW_HOME points the Commons at {}, but {} read {} directly \
         and will not see it",
        env.commons().display(),
        native.join(" and "),
        canonical.display()
    ));
}

fn report_agents(env: &Env, config: &Config, r: &mut Reporter) {
    let home = env.home();
    let detected = target::resolve(env, config);
    // Disabled agents are not "known but missing" — the user switched them off,
    // so they leave the reckoning entirely.
    let known = registry::AGENTS
        .iter()
        .filter(|a| !config.is_disabled(a.name))
        .count()
        + config.custom().len();

    r.line(format!("Detected agents ({} of {known}):", detected.len()));
    if detected.is_empty() {
        r.line("  none — no agent config root found");
    }

    for agent in &detected {
        let families: Vec<String> = agent
            .capabilities()
            .into_iter()
            .filter(|(_, how)| how != "none")
            .map(|(family, how)| format!("{family} {how}"))
            .collect();
        r.line(format!("  {:<10} {}", agent.name, agent.root));
        for family in families {
            r.line(format!("               {family}"));
        }

        let root = home.join(&agent.root);
        if !is_writable(&root) {
            r.problem(format!(
                "{} config root is not writable: {}",
                agent.name,
                root.display()
            ));
        }
    }

    let missing = known.saturating_sub(detected.len());
    if missing > 0 {
        r.blank();
        r.line(format!(
            "{missing} known agents are not installed and were skipped."
        ));
    }
}

/// Ask the operating system, not the permission bits: a directory owned by
/// another user at 0755 is not read-only, yet we still cannot write in it.
#[cfg(unix)]
fn is_writable(path: &Path) -> bool {
    let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: c_path is a valid NUL-terminated string for the duration of the call.
    unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
}

/// Windows has no `access(2)` that answers ACLs honestly, so prove it by
/// doing: create and remove a scratch file. The one deliberate exception to
/// doctor being read-only — the probe never survives the call.
#[cfg(windows)]
fn is_writable(path: &Path) -> bool {
    let probe = path.join(format!(".agentstow-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(file) => {
            drop(file);
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}
