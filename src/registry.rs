//! The Target registry: one row per known agent, declaring where it keeps each
//! config family and by which mechanism agentstow reaches it.
//!
//! Adding an agent is a data change — one [`Agent`] row — never new control
//! flow. Detection is "the agent's config root exists": agentstow creates
//! subdirectories under an existing root, but never a root itself, so an agent
//! that is not installed stays inert.

use std::path::{Path, PathBuf};

/// How an agent gets skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skills {
    /// Symlink fan-out into this home-relative directory.
    FanOut(&'static str),
    /// The agent reads the Store directly; nothing to do.
    Native,
    /// The agent has no skill surface.
    None,
}

/// How an agent gets the shared `AGENTS.md`.
///
/// A Foreign file occupying the destination is a *runtime* state (reported as a
/// conflict), not a separate mechanism — every mechanism below yields to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instructions {
    /// Symlink the Store `AGENTS.md` to this home-relative path.
    Symlink(&'static str),
    /// Ensure an import line inside this home-relative user-owned file.
    ImportLine(&'static str),
    /// Drop an `AGENTS.md` symlink into this home-relative rules directory.
    RulesDirLink(&'static str),
    /// No user-scope instructions surface.
    None,
}

/// Wire format of the file an entry family is merged into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Toml,
}

/// Per-agent shape of one MCP server entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpDialect {
    /// `{type, command, args, env, url, headers}` — the de-facto standard.
    Standard,
    /// Codex: `headers` becomes `http_headers`; http and sse collapse to a URL server.
    Codex,
    /// OpenCode: `type` is local/remote, `command` is one array, `env` is `environment`.
    Opencode,
    /// Gemini: remote transport splits into `url` (SSE) vs `httpUrl` (HTTP).
    Gemini,
    /// Windsurf: the remote URL key is `serverUrl`.
    Windsurf,
}

/// How an agent gets MCP servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mcp {
    /// Key-merge Managed servers into a native config file.
    KeyMerge {
        /// Home-relative config file.
        file: &'static str,
        /// Top-level key holding the server map.
        root_key: &'static str,
        format: Format,
        dialect: McpDialect,
    },
    /// The agent discovers other agents' MCP config; writing would double-configure.
    NativeViaDiscovery,
    /// The agent has no MCP surface.
    None,
}

/// How an agent gets slash commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commands {
    /// Symlink fan-out of Store markdown into this home-relative directory.
    FanOut(&'static str),
    /// Render whole files (carrying the Marker) into this home-relative directory.
    Render { dir: &'static str, format: Format },
    /// The agent discovers other agents' command directories.
    NativeViaDiscovery,
    /// No user-scope command surface.
    None,
}

/// How an agent gets subagents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subagents {
    /// Symlink fan-out into this home-relative directory.
    FanOut(&'static str),
    /// No subagent surface agentstow targets in v1.
    None,
}

/// How an agent gets command-hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hooks {
    /// Key-merge hook elements into a native config file.
    KeyMerge {
        file: &'static str,
        root_key: &'static str,
        format: Format,
    },
    /// No hook surface agentstow targets in v1.
    None,
}

/// One Target: an agent and everything agentstow knows about reaching it.
#[derive(Debug, Clone, Copy)]
pub struct Agent {
    /// Stable identifier, used in reports and in `agentstow.toml`.
    pub name: &'static str,
    /// Home-relative config root. Its existence *is* detection.
    pub root: &'static str,
    pub skills: Skills,
    pub instructions: Instructions,
    pub mcp: Mcp,
    pub commands: Commands,
    pub subagents: Subagents,
    pub hooks: Hooks,
}

impl Agent {
    /// Absolute config root under the given home directory.
    pub fn root_path(&self, home: &Path) -> PathBuf {
        home.join(self.root)
    }

    /// Whether this agent is installed: its config root exists.
    pub fn detected(&self, home: &Path) -> bool {
        self.root_path(home).is_dir()
    }

    /// Human-readable capability summary, in family order.
    pub fn capabilities(&self) -> Vec<(&'static str, String)> {
        vec![
            ("skills", describe_skills(self.skills)),
            ("instructions", describe_instructions(self.instructions)),
            ("mcp", describe_mcp(self.mcp)),
            ("commands", describe_commands(self.commands)),
            ("subagents", describe_subagents(self.subagents)),
            ("hooks", describe_hooks(self.hooks)),
        ]
    }
}

fn describe_skills(c: Skills) -> String {
    match c {
        Skills::FanOut(dir) => format!("fan-out → {dir}"),
        Skills::Native => "native (reads the Store)".into(),
        Skills::None => "none".into(),
    }
}

fn describe_instructions(c: Instructions) -> String {
    match c {
        Instructions::Symlink(p) => format!("symlink → {p}"),
        Instructions::ImportLine(p) => format!("import-line → {p}"),
        Instructions::RulesDirLink(d) => format!("rules-dir link → {d}"),
        Instructions::None => "none".into(),
    }
}

fn describe_mcp(c: Mcp) -> String {
    match c {
        Mcp::KeyMerge { file, .. } => format!("key-merge → {file}"),
        Mcp::NativeViaDiscovery => "native (via discovery)".into(),
        Mcp::None => "none".into(),
    }
}

fn describe_commands(c: Commands) -> String {
    match c {
        Commands::FanOut(dir) => format!("fan-out → {dir}"),
        Commands::Render { dir, .. } => format!("render → {dir}"),
        Commands::NativeViaDiscovery => "native (via discovery)".into(),
        Commands::None => "none".into(),
    }
}

fn describe_subagents(c: Subagents) -> String {
    match c {
        Subagents::FanOut(dir) => format!("fan-out → {dir}"),
        Subagents::None => "none".into(),
    }
}

fn describe_hooks(c: Hooks) -> String {
    match c {
        Hooks::KeyMerge { file, .. } => format!("key-merge → {file}"),
        Hooks::None => "none".into(),
    }
}

/// The built-in registry — the v1 capability matrix as data.
pub const AGENTS: &[Agent] = &[
    Agent {
        name: "claude",
        root: ".claude",
        skills: Skills::FanOut(".claude/skills"),
        // Claude users keep their own content in CLAUDE.md, so agentstow adds
        // only the import line — never a symlink over the whole file.
        instructions: Instructions::ImportLine(".claude/CLAUDE.md"),
        mcp: Mcp::KeyMerge {
            file: ".claude.json",
            root_key: "mcpServers",
            format: Format::Json,
            dialect: McpDialect::Standard,
        },
        commands: Commands::FanOut(".claude/commands"),
        subagents: Subagents::FanOut(".claude/agents"),
        hooks: Hooks::KeyMerge {
            file: ".claude/settings.json",
            root_key: "hooks",
            format: Format::Json,
        },
    },
    Agent {
        name: "codex",
        root: ".codex",
        skills: Skills::FanOut(".codex/skills"),
        instructions: Instructions::Symlink(".codex/AGENTS.md"),
        mcp: Mcp::KeyMerge {
            file: ".codex/config.toml",
            root_key: "mcp_servers",
            format: Format::Toml,
            dialect: McpDialect::Codex,
        },
        commands: Commands::FanOut(".codex/prompts"),
        subagents: Subagents::None,
        hooks: Hooks::KeyMerge {
            file: ".codex/hooks.json",
            root_key: "hooks",
            format: Format::Json,
        },
    },
    Agent {
        name: "opencode",
        root: ".config/opencode",
        // OpenCode scans ~/.agents/skills natively — fan-out would only create
        // duplicate-name warnings.
        skills: Skills::Native,
        instructions: Instructions::Symlink(".config/opencode/AGENTS.md"),
        mcp: Mcp::KeyMerge {
            file: ".config/opencode/opencode.json",
            root_key: "mcp",
            format: Format::Json,
            dialect: McpDialect::Opencode,
        },
        commands: Commands::FanOut(".config/opencode/commands"),
        subagents: Subagents::FanOut(".config/opencode/agents"),
        hooks: Hooks::None,
    },
    Agent {
        name: "pi",
        root: ".pi",
        skills: Skills::FanOut(".pi/agent/skills"),
        instructions: Instructions::Symlink(".pi/agent/AGENTS.md"),
        // pi rejects MCP by design ("build CLI tools with READMEs").
        mcp: Mcp::None,
        commands: Commands::None,
        subagents: Subagents::None,
        hooks: Hooks::None,
    },
    Agent {
        name: "oh-my-pi",
        root: ".omp",
        skills: Skills::Native,
        instructions: Instructions::Symlink(".omp/agent/AGENTS.md"),
        mcp: Mcp::NativeViaDiscovery,
        commands: Commands::NativeViaDiscovery,
        subagents: Subagents::None,
        hooks: Hooks::None,
    },
    Agent {
        name: "gemini",
        root: ".gemini",
        // Gemini has no skill surface — it uses extensions instead.
        skills: Skills::None,
        instructions: Instructions::Symlink(".gemini/GEMINI.md"),
        mcp: Mcp::KeyMerge {
            file: ".gemini/settings.json",
            root_key: "mcpServers",
            format: Format::Json,
            dialect: McpDialect::Gemini,
        },
        commands: Commands::Render {
            dir: ".gemini/commands",
            format: Format::Toml,
        },
        subagents: Subagents::None,
        hooks: Hooks::KeyMerge {
            file: ".gemini/settings.json",
            root_key: "hooks",
            format: Format::Json,
        },
    },
    Agent {
        name: "cursor",
        root: ".cursor",
        skills: Skills::FanOut(".cursor/skills"),
        // Cursor's user-level rules live in app-local storage, out of scope.
        instructions: Instructions::None,
        mcp: Mcp::KeyMerge {
            file: ".cursor/mcp.json",
            root_key: "mcpServers",
            format: Format::Json,
            dialect: McpDialect::Standard,
        },
        commands: Commands::FanOut(".cursor/commands"),
        subagents: Subagents::None,
        hooks: Hooks::None,
    },
    Agent {
        name: "windsurf",
        root: ".codeium/windsurf",
        skills: Skills::None,
        instructions: Instructions::Symlink(".codeium/windsurf/memories/global_rules.md"),
        mcp: Mcp::KeyMerge {
            file: ".codeium/windsurf/mcp_config.json",
            root_key: "mcpServers",
            format: Format::Json,
            dialect: McpDialect::Windsurf,
        },
        commands: Commands::FanOut(".codeium/windsurf/global_workflows"),
        subagents: Subagents::None,
        hooks: Hooks::None,
    },
    Agent {
        name: "roo",
        root: ".roo",
        skills: Skills::None,
        instructions: Instructions::RulesDirLink(".roo/rules"),
        // Roo's user-scope MCP lives in VS Code globalStorage — out of scope.
        mcp: Mcp::None,
        commands: Commands::FanOut(".roo/commands"),
        subagents: Subagents::None,
        hooks: Hooks::None,
    },
    Agent {
        name: "cline",
        root: ".cline",
        skills: Skills::None,
        instructions: Instructions::None,
        mcp: Mcp::KeyMerge {
            file: ".cline/mcp.json",
            root_key: "mcpServers",
            format: Format::Json,
            dialect: McpDialect::Standard,
        },
        commands: Commands::None,
        subagents: Subagents::None,
        hooks: Hooks::None,
    },
];

/// Look up a registry row by name.
pub fn by_name(name: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|a| a.name == name)
}

/// Every agent whose config root exists under `home`.
pub fn detected(home: &Path) -> Vec<&'static Agent> {
    AGENTS.iter().filter(|a| a.detected(home)).collect()
}
