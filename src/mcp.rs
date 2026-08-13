//! MCP servers — the Rendered family (ADR-0002).
//!
//! One Store file, `~/.agents/mcp.json`, in the de-facto standard `mcpServers`
//! shape and kept pure, is rendered into each agent's native config and merged
//! under **name identity**: a server name present in the Store is Managed and
//! the Store wins; any other name is Foreign and never touched, along with
//! every other key in what is usually a large file the agent owns.
//!
//! Two consequences of having no state file (ADR-0001) are load-bearing here:
//!
//! * `sync` never *deletes* a server. A name that has gone from the Store is
//!   indistinguishable from one a user added by hand, so removal is an explicit
//!   act (`mcp remove`), not something inferred.
//! * Resolved `${env:VAR}` values never reach the [`Reporter`]. Notes name the
//!   *keys* that changed, never the values, so the redaction guarantee is
//!   structural rather than a discipline applied at each call site.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::Config;
use crate::env::Env;
use crate::registry::{Format, Mcp, McpDialect};
use crate::target;

/// The state of one server name in one agent's config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Managed and already exactly what the Store renders.
    Managed,
    /// In the Store, absent from this agent's config.
    Missing,
    /// In both, but the agent's copy differs — the Store wins.
    Drifted,
    /// Only in the agent's config. Not ours; never touched.
    Foreign,
}

impl State {
    pub fn needs_change(&self) -> bool {
        matches!(self, State::Missing | State::Drifted)
    }

    /// A Foreign server is somebody else's, so it is reported but never
    /// counted as work.
    pub fn actionable(&self) -> bool {
        self.needs_change()
    }

    pub fn label(&self) -> &'static str {
        match self {
            State::Managed => "managed",
            State::Missing => "missing",
            State::Drifted => "drifted",
            State::Foreign => "foreign",
        }
    }
}

/// One server name in one agent's config.
///
/// `rendered` holds resolved secrets, so this type deliberately implements
/// neither `Debug` nor `Display`: there is no way to print it by accident.
#[derive(Clone)]
pub struct Item {
    pub target: String,
    pub name: String,
    /// The config file this server belongs in.
    pub path: PathBuf,
    /// Top-level key in that file holding the server map.
    pub root_key: &'static str,
    /// Wire format of that file.
    pub format: Format,
    pub state: State,
    /// What the Store says this entry should be — resolved, never printed.
    rendered: Option<Value>,
    /// Names of the keys that differ. Key names only, never values.
    changed_keys: Vec<String>,
    /// Whether rendering this entry resolved an `${env:VAR}` reference.
    holds_secret: bool,
}

/// Everything one survey learned, including the Targets it had to skip.
#[derive(Default)]
pub struct Survey {
    pub items: Vec<Item>,
    /// Per-Target reasons a config could not be read. Advisory: the Target is
    /// left exactly as found and every other Target still syncs.
    pub skipped: Vec<String>,
}

impl Item {
    /// A human-facing note. Safe by construction: it names keys, not values.
    pub fn note(&self) -> String {
        match self.state {
            State::Managed => String::new(),
            State::Missing => "not in this agent's config yet".into(),
            State::Drifted => {
                if self.changed_keys.is_empty() {
                    "differs from the Store — will be restored".into()
                } else {
                    format!(
                        "differs from the Store ({}) — will be restored",
                        self.changed_keys.join(", ")
                    )
                }
            }
            State::Foreign => "not in the Store — left alone".into(),
        }
    }
}

/// Something that stopped a render before anything was written.
#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Survey every MCP-capable Target against the Store.
///
/// An absent Store file yields nothing: a family the user does not use is not a
/// problem to report. Any failure — unparseable Store file, unparseable agent
/// config, an unset `${env:VAR}` — is returned as an error so that *no* file is
/// written, rather than some agents being updated and others not.
pub fn survey(env: &Env, config: &Config, store_file: &Path) -> Result<Survey, Error> {
    if !store_file.is_file() {
        return Ok(Survey::default());
    }

    let servers = read_store(store_file)?;
    let mut survey = Survey::default();

    for target in target::resolve(env, config) {
        let Some(agent) = target.agent else {
            continue;
        };
        let Mcp::KeyMerge {
            file,
            root_key,
            format,
            dialect,
        } = agent.mcp
        else {
            continue;
        };

        let path = env.in_home(file);
        let existing_servers = match read_existing(&path, root_key, format) {
            Ok(servers) => servers,
            Err(e) => {
                // This is a file agentstow promised never to touch. One
                // unreadable config must not deny service to every other agent,
                // nor blank out a read-only report.
                survey.skipped.push(e.message);
                continue;
            }
        };

        for (name, spec) in &servers {
            let rendered = render(name, spec, dialect, env)?;
            let state = match existing_servers.get(name) {
                None => State::Missing,
                Some(current) if *current == rendered => State::Managed,
                Some(_) => State::Drifted,
            };
            let changed_keys = match state {
                State::Drifted => differing_keys(existing_servers.get(name), &rendered),
                _ => Vec::new(),
            };

            survey.items.push(Item {
                target: target.name.clone(),
                name: name.clone(),
                path: path.clone(),
                root_key,
                format,
                state,
                rendered: Some(rendered),
                changed_keys,
                holds_secret: mentions_env_ref(spec),
            });
        }

        for name in existing_servers.keys() {
            if servers.contains_key(name) {
                continue;
            }
            survey.items.push(Item {
                target: target.name.clone(),
                name: name.clone(),
                path: path.clone(),
                root_key,
                format,
                state: State::Foreign,
                rendered: None,
                changed_keys: Vec::new(),
                holds_secret: false,
            });
        }
    }

    Ok(survey)
}

/// Whether a Store entry references the environment, and so renders to a value
/// worth keeping out of a readable file.
fn mentions_env_ref(spec: &Value) -> bool {
    serde_json::to_string(spec)
        .map(|text| text.contains("${env:"))
        .unwrap_or(false)
}

/// Write every Managed change destined for one config file, in one pass.
///
/// Grouping by file matters: an agent may receive more than one family in the
/// same file, and a write per item would clobber the previous one.
pub fn apply(
    path: &Path,
    root_key: &str,
    format: Format,
    items: &[&Item],
) -> Result<Report, Error> {
    let body = match format {
        Format::Json => render_json_document(path, root_key, items)?,
        Format::Toml => render_toml_document(path, root_key, items)?,
    };

    let written = crate::write::atomic(path, body.as_bytes()).map_err(|e| Error {
        message: format!("cannot write {}: {e}", path.display()),
    })?;

    Ok(Report {
        path: written.path,
        exposed: crate::write::is_exposed(written.mode)
            && items.iter().any(|item| item.holds_secret),
        mode: written.mode,
    })
}

/// Merge the Managed entries into a TOML config, preserving everything else.
fn render_toml_document(path: &Path, root_key: &str, items: &[&Item]) -> Result<String, Error> {
    let text = read_text(path)?;
    let mut entries = Map::new();
    for item in items {
        if let Some(rendered) = &item.rendered {
            entries.insert(item.name.clone(), rendered.clone());
        }
    }

    // toml_edit normalises both of these away; the user's file gets them back.
    let had_bom = text.starts_with('\u{feff}');
    let had_crlf = text.contains("\r\n");
    let body = text.strip_prefix('\u{feff}').unwrap_or(&text);

    let merged =
        crate::mcp_toml::merge_section(body, root_key, &entries).map_err(|detail| Error {
            message: format!("{}: {detail} — left untouched", path.display()),
        })?;

    let merged = if had_crlf {
        merged.replace('\n', "\r\n")
    } else {
        merged
    };
    Ok(if had_bom {
        format!("\u{feff}{merged}")
    } else {
        merged
    })
}

/// Merge the Managed entries into a JSON config, preserving everything else.
fn render_json_document(path: &Path, root_key: &str, items: &[&Item]) -> Result<String, Error> {
    let mut document = read_agent_config(path)?;

    // An existing document that is not an object is somebody else's data in a
    // shape we do not understand. Replacing it with `{}` would destroy it.
    if !document.is_null() && !document.is_object() {
        return Err(Error {
            message: format!(
                "{} does not hold a JSON object — left untouched",
                path.display()
            ),
        });
    }
    if document.is_null() {
        document = Value::Object(Map::new());
    }
    let root = document.as_object_mut().expect("checked above");

    let mut servers = match root.get(root_key) {
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
        let Some(rendered) = &item.rendered else {
            continue;
        };
        // Replace the whole subtree: a key the Store dropped must disappear,
        // which a deep merge would preserve forever.
        servers.insert(item.name.clone(), rendered.clone());
    }

    root.insert(root_key.to_string(), Value::Object(servers));

    serde_json::to_string_pretty(&document).map_err(|e| Error {
        message: format!("cannot serialise {}: {e}", path.display()),
    })
}

/// What one write produced, for the caller to report on.
pub struct Report {
    /// The file that actually received the bytes, after following any symlink.
    pub path: PathBuf,
    /// Resolved secrets landed in a file others can read.
    pub exposed: bool,
    pub mode: u32,
}

/// The Store's servers, keyed by name.
///
/// One shape only, the de-facto standard `{"mcpServers": {...}}`. Falling back
/// to "treat every top-level key as a server" would quietly turn a `$schema`
/// line into a server named `$schema`, and would silently ignore real servers
/// in a file that merely had a typo in the wrapper.
fn read_store(path: &Path) -> Result<BTreeMap<String, Value>, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| Error {
        message: format!("cannot read {}: {e}", path.display()),
    })?;
    let parsed: Value = serde_json::from_str(&text).map_err(|e| Error {
        message: format!("{} is not valid JSON: {e}", path.display()),
    })?;

    let map = parsed
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| Error {
            message: format!(
                "{} must hold an `mcpServers` object at the top level",
                path.display()
            ),
        })?;

    for (name, spec) in map {
        if !spec.is_object() {
            return Err(Error {
                message: format!("{}: server `{name}` must be an object", path.display()),
            });
        }
    }

    Ok(map
        .iter()
        .map(|(name, spec)| (name.clone(), spec.clone()))
        .collect())
}

/// The servers already in an agent's config, whatever format it is written in.
///
/// TOML is normalised to JSON here purely so the survey can compare entries
/// with one set of rules; the JSON form is never written back.
fn read_existing(path: &Path, root_key: &str, format: Format) -> Result<Map<String, Value>, Error> {
    match format {
        Format::Json => match read_agent_config(path)?.get(root_key) {
            None => Ok(Map::new()),
            Some(Value::Object(servers)) => Ok(servers.clone()),
            // Treating this as "no servers" would report every Store server as
            // missing forever, while the write refuses every time.
            Some(_) => Err(Error {
                message: format!(
                    "{}: `{root_key}` is not an object — left untouched",
                    path.display()
                ),
            }),
        },
        Format::Toml => {
            let text = read_text(path)?;
            crate::mcp_toml::read_section(&text, root_key).map_err(|detail| Error {
                message: format!("{}: {detail} — left untouched", path.display()),
            })
        }
    }
}

/// A file's text, treating "not there yet" as empty.
fn read_text(path: &Path) -> Result<String, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(Error {
            message: format!("cannot read {}: {e}", path.display()),
        }),
    }
}

/// Parse an agent's config, treating "not there yet" as an empty document.
fn read_agent_config(path: &Path) -> Result<Value, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(Value::Object(Map::new())),
        Ok(text) => serde_json::from_str(&text).map_err(|e| Error {
            // The message names the file and the position, never the content:
            // this file may hold resolved secrets.
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

/// How a server talks, whether or not the Store said so outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    Stdio,
    Http,
    Sse,
}

/// An absent `type` is inferred the way every agent infers it: a server with a
/// URL and no command is remote.
fn transport(spec: &Map<String, Value>) -> Transport {
    match spec.get("type").and_then(Value::as_str) {
        Some("http") => Transport::Http,
        Some("sse") => Transport::Sse,
        Some("stdio") => Transport::Stdio,
        _ if spec.contains_key("url") && !spec.contains_key("command") => Transport::Http,
        _ => Transport::Stdio,
    }
}

/// Keys every dialect handles itself, and so must not pass through verbatim.
const TRANSLATED: &[&str] = &["type", "command", "args", "env", "url", "headers"];

/// Copy the keys agentstow does not model, so an agent-specific setting the
/// user wrote in the Store still reaches its agent.
fn passthrough(spec: &Map<String, Value>, out: &mut Map<String, Value>) {
    for (key, value) in spec {
        if !TRANSLATED.contains(&key.as_str()) {
            out.insert(key.clone(), value.clone());
        }
    }
}

/// Render one Store server into an agent's native shape, resolving references.
fn render(name: &str, spec: &Value, dialect: McpDialect, env: &Env) -> Result<Value, Error> {
    let resolved = resolve_refs(name, spec, env)?;
    let Some(fields) = resolved.as_object() else {
        return Ok(resolved);
    };
    Ok(Value::Object(match dialect {
        // The Store is already in this shape, so there is nothing to translate.
        McpDialect::Standard => return Ok(resolved),
        McpDialect::Codex => {
            // The TOML merge drops null-valued keys because TOML has no null.
            // Dropping them from the rendered entry too keeps the comparison
            // symmetric with what the file can actually hold — otherwise the
            // entry reads back different every time and never converges.
            let mut out = Value::Object(render_codex(fields));
            strip_nulls(&mut out);
            return Ok(out);
        }
        McpDialect::Opencode => render_opencode(fields),
        McpDialect::Gemini => render_gemini(fields),
        McpDialect::Windsurf => render_windsurf(fields),
    }))
}

/// Codex has one remote transport and no `type` key, and calls headers
/// `http_headers`.
fn render_codex(spec: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    match transport(spec) {
        Transport::Stdio => {
            copy(spec, "command", &mut out);
            copy(spec, "args", &mut out);
            copy(spec, "env", &mut out);
        }
        _ => {
            copy(spec, "url", &mut out);
            if let Some(headers) = spec.get("headers") {
                out.insert("http_headers".into(), headers.clone());
            }
        }
    }
    passthrough(spec, &mut out);
    out
}

/// OpenCode splits servers into local and remote, takes the executable and its
/// arguments as one array, and calls the environment `environment`.
fn render_opencode(spec: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    match transport(spec) {
        Transport::Stdio => {
            out.insert("type".into(), Value::from("local"));
            let mut command = Vec::new();
            if let Some(program) = spec.get("command").and_then(Value::as_str) {
                command.push(Value::from(program));
            }
            if let Some(args) = spec.get("args").and_then(Value::as_array) {
                command.extend(args.iter().cloned());
            }
            if !command.is_empty() {
                out.insert("command".into(), Value::Array(command));
            }
            if let Some(env) = spec.get("env") {
                out.insert("environment".into(), env.clone());
            }
        }
        _ => {
            out.insert("type".into(), Value::from("remote"));
            copy(spec, "url", &mut out);
            copy(spec, "headers", &mut out);
        }
    }
    passthrough(spec, &mut out);
    out
}

/// Gemini encodes the remote transport in the key name: `url` is SSE,
/// `httpUrl` is streamable HTTP.
fn render_gemini(spec: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    match transport(spec) {
        Transport::Stdio => {
            copy(spec, "command", &mut out);
            copy(spec, "args", &mut out);
            copy(spec, "env", &mut out);
        }
        remote => {
            let key = if remote == Transport::Sse {
                "url"
            } else {
                "httpUrl"
            };
            if let Some(url) = spec.get("url") {
                out.insert(key.into(), url.clone());
            }
            copy(spec, "headers", &mut out);
        }
    }
    passthrough(spec, &mut out);
    out
}

/// Windsurf calls the remote URL `serverUrl` and has no `type` key.
fn render_windsurf(spec: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    match transport(spec) {
        Transport::Stdio => {
            copy(spec, "command", &mut out);
            copy(spec, "args", &mut out);
            copy(spec, "env", &mut out);
        }
        _ => {
            if let Some(url) = spec.get("url") {
                out.insert("serverUrl".into(), url.clone());
            }
            copy(spec, "headers", &mut out);
        }
    }
    passthrough(spec, &mut out);
    out
}

/// Remove null-valued keys, recursively.
fn strip_nulls(value: &mut Value) {
    if let Value::Object(fields) = value {
        fields.retain(|_, v| !v.is_null());
        for nested in fields.values_mut() {
            strip_nulls(nested);
        }
    }
}

fn copy(spec: &Map<String, Value>, key: &str, out: &mut Map<String, Value>) {
    if let Some(value) = spec.get(key) {
        out.insert(key.to_string(), value.clone());
    }
}

/// Replace every `${env:VAR}` in every string with its value.
///
/// Fails on the first unset variable, so a partially resolved entry can never
/// reach a file.
fn resolve_refs(server: &str, value: &Value, env: &Env) -> Result<Value, Error> {
    match value {
        Value::String(text) => Ok(Value::String(expand(server, text, env)?)),
        Value::Array(items) => items
            .iter()
            .map(|v| resolve_refs(server, v, env))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(fields) => {
            let mut out = Map::new();
            for (key, v) in fields {
                out.insert(key.clone(), resolve_refs(server, v, env)?);
            }
            Ok(Value::Object(out))
        }
        other => Ok(other.clone()),
    }
}

/// Expand `${env:VAR}` occurrences in one string.
fn expand(server: &str, text: &str, env: &Env) -> Result<String, Error> {
    const OPEN: &str = "${env:";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find(OPEN) {
        let (before, from_marker) = rest.split_at(start);
        out.push_str(before);

        let body = &from_marker[OPEN.len()..];
        let Some(end) = body.find('}') else {
            // An unterminated reference is literal text, not an error.
            out.push_str(from_marker);
            return Ok(out);
        };

        let name = &body[..end];
        let Some(value) = env.var(name).filter(|v| !v.is_empty()) else {
            return Err(Error {
                message: format!(
                    "mcp server `{server}`: ${{env:{name}}} is not set in the environment"
                ),
            });
        };
        out.push_str(value);
        rest = &body[end + 1..];
    }

    out.push_str(rest);
    Ok(out)
}

/// Which top-level keys of an entry differ. Names only — never values.
fn differing_keys(current: Option<&Value>, rendered: &Value) -> Vec<String> {
    let empty = Map::new();
    let current = current.and_then(Value::as_object).unwrap_or(&empty);
    let rendered = rendered.as_object().unwrap_or(&empty);

    let mut keys: Vec<String> = rendered
        .iter()
        .filter(|(key, value)| current.get(*key) != Some(*value))
        .map(|(key, _)| key.clone())
        .chain(
            current
                .keys()
                .filter(|key| !rendered.contains_key(*key))
                .cloned(),
        )
        .collect();
    keys.sort();
    keys.dedup();
    keys
}
