//! Runtime Targets: the built-in registry rows plus anything the user's config
//! adds, filtered down to what is actually installed and enabled.
//!
//! The registry stays pure static data; this is where it meets the machine and
//! the config, so every command iterates the same resolved list.

use std::path::Path;

use crate::config::Config;
use crate::env::Env;
use crate::registry::{self, Agent};

/// One Target this invocation will act on.
#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub root: String,
    /// Home-relative fan-out directory per family.
    fanout: Vec<(String, String)>,
    /// The registry row, when this is a built-in agent. Config-defined targets
    /// have none, and therefore take fan-out families only.
    pub agent: Option<&'static Agent>,
}

impl Target {
    fn from_agent(agent: &'static Agent) -> Self {
        let fanout = ["skills", "commands", "subagents"]
            .iter()
            .filter_map(|family| {
                agent
                    .fanout_dir(family)
                    .map(|dir| (family.to_string(), dir.to_string()))
            })
            .collect();

        Self {
            name: agent.name.to_string(),
            root: agent.root.to_string(),
            fanout,
            agent: Some(agent),
        }
    }

    /// The home-relative fan-out directory for a family, if this Target takes one.
    pub fn fanout_dir(&self, family: &str) -> Option<&str> {
        self.fanout
            .iter()
            .find(|(f, _)| f == family)
            .map(|(_, dir)| dir.as_str())
    }

    pub fn detected(&self, home: &Path) -> bool {
        home.join(&self.root).is_dir()
    }

    /// Capability summary for reports.
    pub fn capabilities(&self) -> Vec<(&'static str, String)> {
        match self.agent {
            Some(agent) => agent.capabilities(),
            None => self
                .fanout
                .iter()
                .map(|(family, dir)| {
                    let family: &'static str = match family.as_str() {
                        "skills" => "skills",
                        "commands" => "commands",
                        _ => "subagents",
                    };
                    (family, format!("fan-out → {dir}"))
                })
                .collect(),
        }
    }
}

/// Every Target that is installed and not switched off, built-ins first.
pub fn resolve(env: &Env, config: &Config) -> Vec<Target> {
    let mut targets: Vec<Target> = registry::AGENTS
        .iter()
        .filter(|agent| !config.is_disabled(agent.name))
        .map(Target::from_agent)
        .collect();

    for custom in config.custom() {
        targets.push(Target {
            name: custom.name.clone(),
            root: custom.root.clone(),
            fanout: custom
                .fanout
                .iter()
                .map(|(f, d)| (f.clone(), d.clone()))
                .collect(),
            agent: None,
        });
    }

    targets.retain(|t| t.detected(env.home()));
    targets
}
