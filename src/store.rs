//! The Store: the one place a synced config lives.
//!
//! Scanning is deliberately strict about what it *cannot* see. The hand-rolled
//! script this tool replaces globbed `*/`, so dot-prefixed and non-directory
//! entries were skipped in silence; here every skipped entry becomes a warning.

use std::fs;
use std::path::{Path, PathBuf};

/// Store-relative directory for each fan-out family.
pub const SKILLS: &str = "skills";
pub const COMMANDS: &str = "commands";
pub const SUBAGENTS: &str = "subagents";
pub const HOOKS: &str = "hooks";
/// Store-relative path of the shared instructions file.
pub const INSTRUCTIONS: &str = "AGENTS.md";
/// Store-relative path of the canonical MCP server file.
pub const MCP: &str = "mcp.json";

/// One usable entry in a Store family directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Name as agents see it — directory name, or file stem for file families.
    pub name: String,
    pub path: PathBuf,
}

/// Something in the Store that agentstow will not sync, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    /// A dot-prefixed name: invisible to most agents' scanners, so never synced.
    DotPrefixed { family: &'static str, name: String },
    /// Wrong shape for the family (a file where a directory is required).
    WrongShape {
        family: &'static str,
        name: String,
        want: &'static str,
    },
    /// A symlink in the Store whose own target is missing.
    DanglingLink { family: &'static str, name: String },
    /// The family directory could not be read.
    Unreadable { path: PathBuf, error: String },
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Issue::DotPrefixed { family, name } => write!(
                f,
                "store {family}/{name}: dot-prefixed names are invisible to agents and are never synced"
            ),
            Issue::WrongShape { family, name, want } => {
                write!(f, "store {family}/{name}: not a {want}, skipped")
            }
            Issue::DanglingLink { family, name } => write!(
                f,
                "store {family}/{name}: symlink target is missing, skipped"
            ),
            Issue::Unreadable { path, error } => {
                write!(f, "cannot read {}: {error}", path.display())
            }
        }
    }
}

/// What one family directory holds, plus everything skipped.
#[derive(Debug, Default)]
pub struct Scan {
    pub entries: Vec<Entry>,
    pub issues: Vec<Issue>,
}

/// The canonical Store.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn exists(&self) -> bool {
        self.root.is_dir()
    }

    /// Absolute path of a family directory.
    pub fn family_dir(&self, family: &str) -> PathBuf {
        self.root.join(family)
    }

    /// Scan a directory-shaped family (skills): every entry must be a directory.
    pub fn scan_dirs(&self, family: &'static str) -> Scan {
        self.scan(family, Shape::Directory)
    }

    /// Scan a markdown-file family (commands, subagents).
    pub fn scan_markdown(&self, family: &'static str) -> Scan {
        self.scan(family, Shape::Markdown)
    }

    /// Scan one family directory according to its shape.
    pub fn scan(&self, family: &'static str, shape: Shape) -> Scan {
        let dir = self.family_dir(family);
        let mut scan = Scan::default();

        let read = match fs::read_dir(&dir) {
            Ok(r) => r,
            // An absent family directory is simply an empty family, not a fault.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return scan,
            Err(e) => {
                scan.issues.push(Issue::Unreadable {
                    path: dir,
                    error: e.to_string(),
                });
                return scan;
            }
        };

        for entry in read.flatten() {
            let path = entry.path();
            let raw = entry.file_name().to_string_lossy().into_owned();

            if raw.starts_with('.') {
                scan.issues.push(Issue::DotPrefixed { family, name: raw });
                continue;
            }

            // symlink_metadata does not follow; metadata does. A Store entry may
            // legitimately be a symlink into a git repo, so judge the target.
            let link = fs::symlink_metadata(&path)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            if link && fs::metadata(&path).is_err() {
                scan.issues.push(Issue::DanglingLink { family, name: raw });
                continue;
            }

            match shape {
                Shape::Directory => {
                    if path.is_dir() {
                        scan.entries.push(Entry { name: raw, path });
                    } else {
                        scan.issues.push(Issue::WrongShape {
                            family,
                            name: raw,
                            want: "directory",
                        });
                    }
                }
                Shape::Markdown => {
                    if path.is_dir() {
                        scan.issues.push(Issue::WrongShape {
                            family,
                            name: raw,
                            want: "markdown file",
                        });
                    } else if raw.ends_with(".md") {
                        let name = raw.trim_end_matches(".md").to_string();
                        scan.entries.push(Entry { name, path });
                    } else {
                        scan.issues.push(Issue::WrongShape {
                            family,
                            name: raw,
                            want: "markdown file",
                        });
                    }
                }
            }
        }

        scan.entries.sort_by(|a, b| a.name.cmp(&b.name));
        scan.issues.sort_by_key(|i| format!("{i}"));
        scan
    }
}

/// What a family's Store entries look like on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// One directory per entry (skills).
    Directory,
    /// One markdown file per entry (commands, subagents).
    Markdown,
}

/// Every fan-out family, in report order. Adding a family is a row here plus a
/// registry column — the sync and status loops are already generic over it.
pub const FANOUT_FAMILIES: &[(&str, Shape)] = &[(SKILLS, Shape::Directory)];
