//! Tool configuration: `~/.agentstow/agentstow.toml`.
//!
//! Deliberately tiny, and deliberately *not* in the Store — the Store holds
//! ecosystem content that other tools may one day read, so nothing
//! agentstow-specific squats there. An absent file means pure defaults.
//!
//! ```toml
//! [targets]
//! cursor = false            # a detected agent agentstow should leave alone
//!
//! [custom.myagent]          # an agent the built-in registry does not know
//! root = ".myagent"
//! skills = ".myagent/skills"
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use toml_edit::{DocumentMut, Item};

/// Where the config file lives inside the config directory.
pub const FILE: &str = "agentstow.toml";

/// A target defined entirely by the user's config.
#[derive(Debug, Clone)]
pub struct CustomTarget {
    pub name: String,
    pub root: String,
    /// Home-relative fan-out directory per family.
    pub fanout: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Registry agents the user switched off, by name.
    disabled: Vec<String>,
    custom: Vec<CustomTarget>,
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Config {
    /// Read the config from a config directory. An absent file is not an error.
    pub fn load(config_dir: &Path) -> Result<Self, Error> {
        let path = config_dir.join(FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(Error {
                    message: format!("cannot read {}: {e}", path.display()),
                })
            }
        };

        let doc: DocumentMut = text.parse().map_err(|e| Error {
            message: format!("{} is not valid TOML: {e}", path.display()),
        })?;

        let mut config = Self::default();

        if let Some(targets) = doc.get("targets").and_then(Item::as_table) {
            for (name, value) in targets.iter() {
                match value.as_bool() {
                    Some(false) => config.disabled.push(name.to_string()),
                    Some(true) => {}
                    None => {
                        return Err(Error {
                            message: format!(
                                "{}: targets.{name} must be true or false",
                                path.display()
                            ),
                        })
                    }
                }
            }
        }

        if let Some(custom) = doc.get("custom").and_then(Item::as_table) {
            for (name, value) in custom.iter() {
                let table = value.as_table().ok_or_else(|| Error {
                    message: format!("{}: custom.{name} must be a table", path.display()),
                })?;
                let root = table
                    .get("root")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error {
                        message: format!("{}: custom.{name} needs a `root`", path.display()),
                    })?
                    .to_string();

                let mut fanout = BTreeMap::new();
                for family in ["skills", "commands", "subagents"] {
                    if let Some(dir) = table.get(family).and_then(|v| v.as_str()) {
                        fanout.insert(family.to_string(), dir.to_string());
                    }
                }

                config.custom.push(CustomTarget {
                    name: name.to_string(),
                    root,
                    fanout,
                });
            }
        }

        Ok(config)
    }

    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled.iter().any(|d| d == name)
    }

    pub fn custom(&self) -> &[CustomTarget] {
        &self.custom
    }
}
