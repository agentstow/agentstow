//! The instructions family: one `AGENTS.md` in the Store, reaching every agent
//! by whichever mechanism that agent supports.
//!
//! Three mechanisms, because agents differ in kind and not just in path:
//! a plain symlink where the whole file can be ours; an **import-line** where
//! the file is the user's and we may only add one additive, idempotent line;
//! and a **rules-dir link** where the agent globs a directory.
//!
//! A Foreign file already occupying the destination is a conflict: agentstow
//! reports it with a remediation hint and writes nothing. Resolving it is a
//! decision about someone else's content, which is not agentstow's to make.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::env::Env;
use crate::link;
use crate::registry::Instructions;
use crate::target;

/// What agentstow found at one agent's instructions destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Our symlink is in place and canonical.
    Linked,
    /// Nothing there yet — a link will be created.
    Missing,
    /// The user's file already imports the Store instructions.
    ImportPresent,
    /// The user's file exists (or does not) and needs the import line added.
    ImportMissing,
    /// A file we did not write occupies the destination.
    Conflict,
    /// A symlink pointing somewhere other than the Store.
    Foreign,
}

impl State {
    /// Whether `sync` will change anything for this state.
    pub fn needs_change(&self) -> bool {
        matches!(self, State::Missing | State::ImportMissing)
    }

    /// A conflict is a decision for the user about another tool's file, so it
    /// is reported but never counted as something `sync` can resolve.
    pub fn actionable(&self) -> bool {
        self.needs_change()
    }

    pub fn label(&self) -> &'static str {
        match self {
            State::Linked => "linked",
            State::Missing => "missing",
            State::ImportPresent => "imported",
            State::ImportMissing => "import",
            State::Conflict => "conflict",
            State::Foreign => "foreign",
        }
    }
}

/// One agent's instructions destination and what to do about it.
#[derive(Debug, Clone)]
pub struct Item {
    pub target: String,
    pub path: PathBuf,
    pub state: State,
    /// Canonical link text, for the symlink mechanisms.
    link_text: Option<PathBuf>,
    /// The line to ensure, for the import-line mechanism.
    import_line: Option<String>,
}

impl Item {
    /// A human-facing explanation, including the fix where there is one.
    pub fn note(&self) -> String {
        match self.state {
            State::Linked => String::new(),
            State::Missing => "will be linked to the Store".into(),
            State::ImportPresent => "imports the Store instructions".into(),
            State::ImportMissing => "the import line will be added".into(),
            State::Conflict => format!(
                "{} is owned by something else — move or merge it, then sync again",
                self.path.display()
            ),
            State::Foreign => "points outside the Store — left alone".into(),
        }
    }
}

/// Survey every detected Target's instructions destination.
///
/// Returns nothing at all when the Store has no `AGENTS.md`: an absent family
/// is not a problem to report, it is simply a family the user does not use.
pub fn survey(env: &Env, config: &Config, store_file: &Path) -> Vec<Item> {
    if !store_file.is_file() {
        return Vec::new();
    }

    let mut items = Vec::new();

    for target in target::resolve(env, config) {
        let Some(agent) = target.agent else {
            // Config-defined targets take fan-out families only.
            continue;
        };

        let item = match agent.instructions {
            Instructions::Symlink(rel) => link_item(&target.name, env.in_home(rel), store_file),
            Instructions::RulesDirLink(dir) => {
                let path = env.in_home(dir).join("AGENTS.md");
                link_item(&target.name, path, store_file)
            }
            Instructions::ImportLine(rel) => import_item(&target.name, env.in_home(rel), env),
            Instructions::None => continue,
        };
        items.push(item);
    }

    items
}

fn link_item(target: &str, path: PathBuf, store_file: &Path) -> Item {
    let parent = path.parent().unwrap_or(Path::new("."));
    let text = link::relative_from(parent, store_file);

    let state = match fs::symlink_metadata(&path) {
        Err(_) => State::Missing,
        Ok(meta) if meta.file_type().is_symlink() => {
            let actual = fs::read_link(&path).unwrap_or_default();
            let resolved = link::resolve_link(&path, &actual);
            if resolved == link::normalize(store_file) {
                State::Linked
            } else {
                State::Foreign
            }
        }
        Ok(_) => State::Conflict,
    };

    Item {
        target: target.to_string(),
        path,
        state,
        link_text: Some(text),
        import_line: None,
    }
}

/// Claude keeps its own content in `CLAUDE.md`, so the whole file can never be
/// ours. The one sanctioned edit is adding an import line if it is not there.
fn import_item(target: &str, path: PathBuf, env: &Env) -> Item {
    let reference = import_reference(env);
    let line = format!("@{reference}");

    let state = match fs::read_to_string(&path) {
        Ok(body) => {
            if body
                .lines()
                .any(|l| l.trim() == line || l.contains(&reference))
            {
                State::ImportPresent
            } else {
                State::ImportMissing
            }
        }
        // No file yet: one containing just the import is a fine starting point.
        Err(_) => State::ImportMissing,
    };

    Item {
        target: target.to_string(),
        path,
        state,
        link_text: None,
        import_line: Some(line),
    }
}

/// How the Store instructions should be referred to inside a user's file:
/// `~`-relative when the Store is under the home directory, absolute otherwise.
fn import_reference(env: &Env) -> String {
    let file = env.store().join(crate::store::INSTRUCTIONS);
    match file.strip_prefix(env.home()) {
        Ok(rel) => format!("~/{}", rel.display()),
        Err(_) => file.display().to_string(),
    }
}

/// Bring one item to its intended state. Conflicts and Foreign links are no-ops.
pub fn apply(item: &Item) -> std::io::Result<()> {
    match item.state {
        State::Missing => {
            let text = item.link_text.as_ref().expect("a link mechanism");
            if let Some(parent) = item.path.parent() {
                fs::create_dir_all(parent)?;
            }
            std::os::unix::fs::symlink(text, &item.path)
        }
        State::ImportMissing => {
            let line = item.import_line.as_ref().expect("an import mechanism");
            if let Some(parent) = item.path.parent() {
                fs::create_dir_all(parent)?;
            }
            let existing = fs::read_to_string(&item.path).unwrap_or_default();
            let mut body = existing.clone();
            if !body.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(line);
            body.push('\n');
            fs::write(&item.path, body)
        }
        _ => Ok(()),
    }
}
