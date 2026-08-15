//! `doctor` — is this machine ready to sync?
//!
//! Reports what is installed and what would be silently skipped. It is strictly
//! read-only: doctor never creates a directory, least of all an agent root,
//! because a root's existence is what detection means.

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::link;
use crate::registry;
use crate::registry::Skills;
use crate::report::Reporter;
use crate::store::{self, Store};
use crate::target;

pub fn run(env: &Env, config: &Config, r: &mut Reporter) -> i32 {
    let store = Store::new(env.store());

    r.line(format!("Store   {}", store.root().display()));
    r.line(format!("Home    {}", env.home().display()));
    r.line(format!("Config  {}", env.config_dir().display()));
    r.blank();

    if store.exists() {
        report_store(&store, r);
    } else {
        r.problem(store::missing_message(store.root()));
    }

    report_store_override(env, config, r);
    report_agents(env, config, r);

    r.verdict()
}

fn report_store(store: &Store, r: &mut Reporter) {
    let scans: Vec<(Family, store::Scan)> = Family::ALL
        .iter()
        .map(|family| (*family, store.scan(*family)))
        .collect();

    let hooks = store.family_dir(store::HOOKS);
    let hook_files: Vec<String> = std::fs::read_dir(&hooks)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();

    let instructions = if store.root().join(store::INSTRUCTIONS).exists() {
        "present"
    } else {
        "absent"
    };
    let mcp = if store.root().join(store::MCP).exists() {
        "present"
    } else {
        "absent"
    };

    r.line("Store contents:");
    for (family, scan) in &scans {
        r.line(format!("  {:<13} {}", family.name(), scan.entries.len()));
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

    let others = co_tenants(store);
    if !others.is_empty() {
        r.line("Other tools in the Store:");
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
            "store hooks/{name}: not a `<Event>.toml` file, skipped"
        ));
    }
}

/// Names at the Store root that are not agentstow's own families.
///
/// The Store is a shared commons, not agentstow's private directory (ADR-0004):
/// opencode, oh-my-pi and hermes read `~/.agents/` themselves, and the `skills`
/// CLI keeps its lock file there. An unrecognised name is a neighbour, not a
/// fault — so these are named and never counted, never called an issue, and
/// never touched. agentstow can read filenames but not authorship, so it must
/// not claim how many *tools* are present, only which entries are not its own.
fn co_tenants(store: &Store) -> Vec<String> {
    const OURS: &[&str] = &[
        store::SKILLS,
        store::COMMANDS,
        store::SUBAGENTS,
        store::HOOKS,
        store::INSTRUCTIONS,
        store::MCP,
    ];

    let Ok(read) = std::fs::read_dir(store.root()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| !OURS.contains(&name.as_str()))
        .collect();
    names.sort();
    names
}

/// Warn when a relocated Store is invisible to the agents that read the
/// canonical path themselves.
///
/// `AGENTSTOW_HOME` moves agentstow's Store, but it cannot move the path a
/// Native agent hardcodes. Those agents keep reading `~/.agents/` and silently
/// diverge from every agent that gets fan-out — so name them.
fn report_store_override(env: &Env, config: &Config, r: &mut Reporter) {
    if env
        .var("AGENTSTOW_HOME")
        .filter(|v| !v.is_empty())
        .is_none()
    {
        return;
    }

    let canonical = env.home().join(crate::env::STORE_DIR);
    if link::normalize(env.store()) == link::normalize(&canonical) {
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
        "AGENTSTOW_HOME points the Store at {}, but {} read {} directly \
         and will not see it",
        env.store().display(),
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
