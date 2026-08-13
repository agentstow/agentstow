//! `doctor` — is this machine ready to sync?
//!
//! Reports what is installed and what would be silently skipped. It is strictly
//! read-only: doctor never creates a directory, least of all an agent root,
//! because a root's existence is what detection means.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::registry;
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
fn is_writable(path: &Path) -> bool {
    let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: c_path is a valid NUL-terminated string for the duration of the call.
    unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
}
