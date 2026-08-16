//! Path resolution. Everything the tool touches is resolved from here, so the
//! two env overrides redirect the whole tool at a throwaway tree during tests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where the Commons lives when `AGENTSTOW_HOME` is unset.
pub const COMMONS_DIR: &str = ".agents";
/// Where tool configuration lives when the home directory is `~`.
pub const CONFIG_DIR: &str = ".agentstow";

/// Resolved locations for one invocation.
#[derive(Debug, Clone)]
pub struct Env {
    /// Home directory for Target resolution — `AGENTSTOW_TARGET_ROOT` or `HOME`.
    home: PathBuf,
    /// The Commons — `AGENTSTOW_HOME`, else `<home>/.agents`.
    commons: PathBuf,
    /// Tool configuration directory — `<home>/.agentstow`.
    config: PathBuf,
    vars: BTreeMap<String, String>,
}

/// Neither `AGENTSTOW_TARGET_ROOT` nor `HOME` was set, so nothing can resolve.
#[derive(Debug)]
pub struct NoHome;

impl std::fmt::Display for NoHome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "cannot determine home directory: set HOME (USERPROFILE on Windows) or AGENTSTOW_TARGET_ROOT",
        )
    }
}

impl Env {
    /// Resolve from environment variables. `AGENTSTOW_TARGET_ROOT` redirects
    /// home-relative Target resolution; `AGENTSTOW_HOME` relocates the Commons
    /// independently, so tests can point them at different trees.
    pub fn resolve(vars: &[(String, String)]) -> Result<Self, NoHome> {
        let vars: BTreeMap<String, String> = vars.iter().cloned().collect();

        let home = vars
            .get("AGENTSTOW_TARGET_ROOT")
            .or_else(|| vars.get("HOME"))
            // cmd and PowerShell set USERPROFILE, not HOME; Git Bash sets both.
            .or_else(|| vars.get("USERPROFILE"))
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .ok_or(NoHome)?;

        let commons = match vars.get("AGENTSTOW_HOME").filter(|v| !v.is_empty()) {
            Some(v) => PathBuf::from(v),
            None => home.join(COMMONS_DIR),
        };

        let config = home.join(CONFIG_DIR);

        Ok(Self {
            home,
            commons,
            config,
            vars,
        })
    }

    /// The home directory Targets resolve against.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The Commons root.
    pub fn commons(&self) -> &Path {
        &self.commons
    }

    /// The tool configuration directory (never inside the Commons).
    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    /// Resolve a home-relative path such as `.claude/skills`.
    pub fn in_home(&self, rel: &str) -> PathBuf {
        self.home.join(rel)
    }

    /// Look up an environment variable, for `${env:VAR}` resolution.
    pub fn var(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }
}
