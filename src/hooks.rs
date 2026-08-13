//! The hooks family — command-hooks, declared once, merged into each agent's
//! native hook arrays.
//!
//! Ownership here is **element identity** (ADR-0003): a hook object whose
//! command string matches a Store hook is agentstow's to update, and every
//! other element in the same event array belongs to whoever put it there. That
//! matters more than usual, because these arrays are where several tools
//! (claude-mem, plugins, the user) all keep their own hooks.
//!
//! Two deliberate limits:
//!
//! * **Trust metadata is never written.** Codex records a `trusted_hash` per
//!   hook in `config.toml`, a different file from the `hooks.json` this writes,
//!   so approving code execution stays the user's decision by construction.
//! * **Scripts are not managed.** A Store command must be a path that means the
//!   same thing to every agent; agentstow syncs the declaration, not the file
//!   it runs.
//! * **`${env:VAR}` is not expanded.** The command string *is* the identity, so
//!   it must not vary with the ambient environment — a resolved command would
//!   stop matching the reference that produced it, leaving the hook reported as
//!   missing and its own rendering as foreign, on every run, forever. A hook
//!   that needs a secret should read it from the environment itself; then the
//!   value never reaches the config file at all.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::Config;
use crate::env::Env;
use crate::registry::Hooks;
use crate::target;

/// Canonical event names. Claude and Codex share this vocabulary; Gemini has
/// its own word for some of the same moments, and no word at all for others.
const EVENTS: &[Event] = &[
    Event {
        canonical: "SessionStart",
        gemini: Some("SessionStart"),
    },
    Event {
        canonical: "Stop",
        gemini: Some("AfterAgent"),
    },
    Event {
        canonical: "PreToolUse",
        gemini: Some("BeforeTool"),
    },
    Event {
        canonical: "PostToolUse",
        gemini: Some("AfterTool"),
    },
    Event {
        canonical: "Notification",
        gemini: Some("Notification"),
    },
    // Claude and Codex record the prompt going in; Gemini has no equivalent.
    Event {
        canonical: "UserPromptSubmit",
        gemini: None,
    },
    Event {
        canonical: "SubagentStop",
        gemini: None,
    },
    Event {
        canonical: "PreCompact",
        gemini: Some("PreCompress"),
    },
];

struct Event {
    canonical: &'static str,
    gemini: Option<&'static str>,
}

/// One hook as the Store declares it.
#[derive(Debug, Clone)]
pub struct Hook {
    pub event: String,
    pub matcher: Option<String>,
    pub command: String,
    pub timeout: Option<i64>,
}

/// What agentstow found for one hook in one agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Present and exactly what the Store renders.
    Managed,
    /// In the Store, absent from this agent.
    Missing,
    /// Present but different — the Store wins.
    Drifted,
    /// This agent has no name for this moment, so nothing is written.
    Unsupported,
    /// In the agent's config, not in the Store. Never touched.
    Foreign,
}

impl State {
    pub fn needs_change(&self) -> bool {
        matches!(self, State::Missing | State::Drifted)
    }

    /// Neither an unsupported event nor somebody else's hook is work.
    pub fn actionable(&self) -> bool {
        self.needs_change()
    }

    pub fn label(&self) -> &'static str {
        match self {
            State::Managed => "managed",
            State::Missing => "missing",
            State::Drifted => "drifted",
            State::Unsupported => "unsupported",
            State::Foreign => "foreign",
        }
    }
}

/// One hook's standing in one agent.
///
/// `rendered` can hold a resolved `${env:VAR}`, so this type implements neither
/// `Debug` nor `Display` — the same rule the MCP family follows.
#[derive(Clone)]
pub struct Item {
    pub target: String,
    pub event: String,
    /// What to call this hook in a report. For our own hooks that is the Store
    /// command verbatim, unresolved. For a Foreign hook it is the program only:
    /// somebody else's command line can carry a credential, and echoing it into
    /// a terminal or a CI log is exposure nobody asked for.
    pub label: String,
    pub path: PathBuf,
    pub root_key: &'static str,
    pub state: State,
    rendered: Option<Value>,
    native_event: Option<&'static str>,
    matcher: Option<String>,
}

impl Item {
    pub fn note(&self) -> String {
        match self.state {
            State::Managed => String::new(),
            State::Missing => "not in this agent's hooks yet".into(),
            State::Drifted => "differs from the Store — will be restored".into(),
            State::Unsupported => format!("{} has no {} hook", self.target, self.event),
            State::Foreign => "not in the Store — left alone".into(),
        }
    }
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

#[derive(Default)]
pub struct Survey {
    pub items: Vec<Item>,
    pub skipped: Vec<String>,
}

/// Survey every hook-capable Target against the Store's hook files.
pub fn survey(env: &Env, config: &Config, store_dir: &Path) -> Result<Survey, Error> {
    let mut survey = Survey::default();

    // The presence of the directory, not of any file in it, is what says the
    // user uses this family. That distinction is what lets a hook deleted from
    // the Store still be reported as the leftover it is, without listing every
    // agent's own hooks for someone who never adopted the family at all.
    if !store_dir.is_dir() {
        return Ok(survey);
    }
    let declared = read_store(store_dir)?;

    for target in target::resolve(env, config) {
        let Some(agent) = target.agent else {
            continue;
        };
        let Hooks::KeyMerge { file, root_key } = agent.hooks else {
            continue;
        };

        let path = env.in_home(file);
        let existing = match read_agent(&path, root_key) {
            Ok(existing) => existing,
            Err(e) => {
                survey.skipped.push(e.message);
                continue;
            }
        };

        for hook in &declared {
            let native_event = native_event(&target.name, &hook.event);
            let Some(native_event) = native_event else {
                survey.items.push(Item {
                    target: target.name.clone(),
                    event: hook.event.clone(),
                    label: hook.command.clone(),
                    path: path.clone(),
                    root_key,
                    state: State::Unsupported,
                    rendered: None,
                    native_event: None,
                    matcher: None,
                });
                continue;
            };

            let rendered = render(hook);
            let current = find_by_command(&existing, native_event, &hook.command);
            let state = match current {
                None => State::Missing,
                Some(found) if found == rendered => State::Managed,
                Some(_) => State::Drifted,
            };

            survey.items.push(Item {
                target: target.name.clone(),
                event: hook.event.clone(),
                label: hook.command.clone(),
                path: path.clone(),
                root_key,
                state,
                rendered: Some(rendered),
                native_event: Some(native_event),
                matcher: hook.matcher.clone(),
            });
        }

        // Anything in this agent's hooks that the Store does not declare.
        for (event, groups) in &existing {
            for object in flatten(groups) {
                let Some(command) = object.get("command").and_then(Value::as_str) else {
                    continue;
                };
                let ours = declared.iter().any(|hook| {
                    native_event(&target.name, &hook.event) == Some(event.as_str())
                        && hook.command == command
                });
                if ours {
                    continue;
                }
                survey.items.push(Item {
                    target: target.name.clone(),
                    event: event.clone(),
                    label: program_only(command),
                    path: path.clone(),
                    root_key,
                    state: State::Foreign,
                    rendered: None,
                    native_event: None,
                    matcher: None,
                });
            }
        }
    }

    Ok(survey)
}

/// Merge the Managed hooks into one agent's config file.
pub fn apply(path: &Path, root_key: &str, items: &[&Item]) -> Result<crate::mcp::Report, Error> {
    let mut document = read_document(path)?;
    let root = match document.as_object_mut() {
        Some(root) => root,
        None => {
            return Err(Error {
                message: format!(
                    "{} does not hold a JSON object — left untouched",
                    path.display()
                ),
            })
        }
    };

    let mut events = match root.get(root_key) {
        None => Map::new(),
        Some(Value::Object(existing)) => existing.clone(),
        Some(_) => {
            return Err(Error {
                message: format!(
                    "{}: `{root_key}` is not an object — left untouched",
                    path.display()
                ),
            })
        }
    };

    for item in items {
        let (Some(rendered), Some(event)) = (&item.rendered, item.native_event) else {
            continue;
        };
        let groups = events
            .entry(event.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        let Some(groups) = groups.as_array_mut() else {
            return Err(Error {
                message: format!("{}: `{root_key}.{event}` is not a list", path.display()),
            });
        };

        if !replace_by_command(groups, rendered) {
            // Ours goes in a group of its own rather than into somebody else's,
            // so their matcher and grouping stay exactly as they wrote them.
            let mut group = Map::new();
            if let Some(matcher) = &item.matcher {
                group.insert("matcher".into(), Value::from(matcher.as_str()));
            }
            group.insert("hooks".into(), Value::Array(vec![rendered.clone()]));
            groups.push(Value::Object(group));
        }
    }

    root.insert(root_key.to_string(), Value::Object(events));

    let body = serde_json::to_string_pretty(&document).map_err(|e| Error {
        message: format!("cannot serialise {}: {e}", path.display()),
    })?;
    let written = crate::write::atomic(path, body.as_bytes()).map_err(|e| Error {
        message: format!("cannot write {}: {e}", path.display()),
    })?;

    Ok(crate::mcp::Report {
        path: written.path,
        // Hook commands are written verbatim, so a sync never puts a resolved
        // secret anywhere it was not already.
        exposed: false,
        mode: written.mode,
    })
}

/// Replace the hook object with this command, wherever it sits. Returns whether
/// one was found.
fn replace_by_command(groups: &mut [Value], rendered: &Value) -> bool {
    let Some(command) = rendered.get("command") else {
        return false;
    };
    for group in groups.iter_mut() {
        let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
            continue;
        };
        for object in hooks.iter_mut() {
            if object.get("command") == Some(command) {
                *object = rendered.clone();
                return true;
            }
        }
    }
    false
}

/// The hook object with this command under this event, if any.
fn find_by_command(existing: &Map<String, Value>, event: &str, command: &str) -> Option<Value> {
    let groups = existing.get(event)?;
    flatten(groups)
        .into_iter()
        .find(|object| object.get("command").and_then(Value::as_str) == Some(command))
}

/// Every hook object under one event, across all its groups.
fn flatten(groups: &Value) -> Vec<Value> {
    groups
        .as_array()
        .map(|groups| {
            groups
                .iter()
                .filter_map(|group| group.get("hooks").and_then(Value::as_array))
                .flatten()
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// The program a command runs, without its arguments.
///
/// Used to name Foreign hooks in reports: enough to recognise whose hook it is,
/// without printing arguments that may hold a secret.
fn program_only(command: &str) -> String {
    let trimmed = command.trim_start();
    // A quoted program may contain spaces, so take the quoted run as one token.
    let (program, quoted) = match trimmed.chars().next() {
        Some(quote @ ('"' | '\'')) => {
            let rest = &trimmed[1..];
            match rest.find(quote) {
                Some(end) => (&rest[..end], true),
                None => (trimmed, false),
            }
        }
        _ => (trimmed.split_whitespace().next().unwrap_or(trimmed), false),
    };

    let had_more = quoted || trimmed.split_whitespace().count() > 1;
    if had_more {
        format!("{program} …")
    } else {
        program.to_string()
    }
}

/// This agent's name for a canonical event, if it has one.
fn native_event(agent: &str, canonical: &str) -> Option<&'static str> {
    let event = EVENTS.iter().find(|e| e.canonical == canonical)?;
    match agent {
        "gemini" => event.gemini,
        _ => Some(event.canonical),
    }
}

/// One hook as an agent's config wants it.
///
/// The map is built from scratch out of a closed set of keys, which is the
/// structural reason no code path here can emit trust metadata.
fn render(hook: &Hook) -> Value {
    let mut object = Map::new();
    object.insert("type".into(), Value::from("command"));
    object.insert("command".into(), Value::from(hook.command.as_str()));
    if let Some(timeout) = hook.timeout {
        object.insert("timeout".into(), Value::from(timeout));
    }
    Value::Object(object)
}

/// Every hook the Store declares, from `hooks/<event>.toml`.
fn read_store(dir: &Path) -> Result<Vec<Hook>, Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(Error {
                message: format!("cannot read {}: {e}", dir.display()),
            })
        }
    };

    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "toml").unwrap_or(false))
        .collect();
    files.sort();

    let mut hooks = Vec::new();
    for path in files {
        let event = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !EVENTS.iter().any(|e| e.canonical == event) {
            let known: Vec<&str> = EVENTS.iter().map(|e| e.canonical).collect();
            return Err(Error {
                message: format!(
                    "{}: `{event}` is not a hook event — expected one of {}",
                    path.display(),
                    known.join(", ")
                ),
            });
        }
        hooks.extend(read_hook_file(&path, &event)?);
    }
    Ok(hooks)
}

fn read_hook_file(path: &Path, event: &str) -> Result<Vec<Hook>, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| Error {
        message: format!("cannot read {}: {e}", path.display()),
    })?;
    let doc: toml_edit::DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| Error {
        // Never format the error: it quotes the offending line, and a hook
        // command can carry a secret.
        message: format!(
            "{} is not valid TOML ({})",
            path.display(),
            crate::mcp_toml::position_only(&text, &e)
        ),
    })?;

    let Some(array) = doc.get("hook").and_then(|item| item.as_array_of_tables()) else {
        return Err(Error {
            message: format!(
                "{}: expected one or more [[hook]] entries with a `command`",
                path.display()
            ),
        });
    };

    let mut hooks = Vec::new();
    for table in array.iter() {
        if let Some(kind) = table.get("type").and_then(|v| v.as_str()) {
            if kind != "command" {
                return Err(Error {
                    message: format!(
                        "{}: `type = \"{kind}\"` — agentstow syncs command hooks only",
                        path.display()
                    ),
                });
            }
        }
        let command = table
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error {
                message: format!("{}: every [[hook]] needs a `command`", path.display()),
            })?
            .to_string();

        hooks.push(Hook {
            event: event.to_string(),
            matcher: table
                .get("matcher")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            command,
            timeout: table.get("timeout").and_then(|v| v.as_integer()),
        });
    }
    Ok(hooks)
}

fn read_agent(path: &Path, root_key: &str) -> Result<Map<String, Value>, Error> {
    match read_document(path)?.get(root_key) {
        None => Ok(Map::new()),
        Some(Value::Object(events)) => Ok(events.clone()),
        Some(_) => Err(Error {
            message: format!(
                "{}: `{root_key}` is not an object — left untouched",
                path.display()
            ),
        }),
    }
}

fn read_document(path: &Path) -> Result<Value, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(Value::Object(Map::new())),
        Ok(text) => serde_json::from_str(&text).map_err(|e| Error {
            message: format!(
                "{} is not valid JSON (line {}, column {}) — left untouched",
                path.display(),
                e.line(),
                e.column()
            ),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::Object(Map::new())),
        Err(e) => Err(Error {
            message: format!("cannot read {}: {e}", path.display()),
        }),
    }
}
