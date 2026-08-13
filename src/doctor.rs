//! `doctor` — is this machine ready to sync?
//!
//! Reports what is installed and what would be silently skipped. It is strictly
//! read-only: doctor never creates a directory, least of all an agent root,
//! because a root's existence is what detection means.

use std::fs;
use std::path::Path;

use crate::env::Env;
use crate::registry;
use crate::report::Reporter;
use crate::store::{self, Store};

pub fn run(env: &Env, r: &mut Reporter) -> i32 {
    let store = Store::new(env.store());

    r.line(format!("Store   {}", store.root().display()));
    r.line(format!("Home    {}", env.home().display()));
    r.line(format!("Config  {}", env.config_dir().display()));
    r.blank();

    if store.exists() {
        report_store(&store, r);
    } else {
        r.problem(format!(
            "no Store at {} — run `agentstow init` to create one",
            store.root().display()
        ));
    }

    report_agents(env.home(), r);

    r.verdict()
}

fn report_store(store: &Store, r: &mut Reporter) {
    let skills = store.scan_dirs(store::SKILLS);
    let commands = store.scan_markdown(store::COMMANDS);
    let subagents = store.scan_markdown(store::SUBAGENTS);

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
    r.line(format!("  skills        {}", skills.entries.len()));
    r.line(format!("  commands      {}", commands.entries.len()));
    r.line(format!("  subagents     {}", subagents.entries.len()));
    r.line(format!("  AGENTS.md     {instructions}"));
    r.line(format!("  mcp.json      {mcp}"));
    r.blank();

    for issue in skills
        .issues
        .iter()
        .chain(&commands.issues)
        .chain(&subagents.issues)
    {
        r.warn(issue.to_string());
    }
}

fn report_agents(home: &Path, r: &mut Reporter) {
    let detected = registry::detected(home);
    let total = registry::AGENTS.len();

    r.line(format!("Detected agents ({} of {total}):", detected.len()));
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

        let root = agent.root_path(home);
        if is_readonly(&root) {
            r.problem(format!(
                "{} config root is not writable: {}",
                agent.name,
                root.display()
            ));
        }
    }

    let missing = total - detected.len();
    if missing > 0 {
        r.blank();
        r.line(format!(
            "{missing} known agents are not installed and were skipped."
        ));
    }
}

fn is_readonly(path: &Path) -> bool {
    fs::metadata(path)
        .map(|m| m.permissions().readonly())
        .unwrap_or(false)
}
