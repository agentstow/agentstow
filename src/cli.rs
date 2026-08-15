//! Argument parsing. Kept separate from [`crate::run`] so the seam stays a
//! plain function over argv.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "agentstow",
    version,
    about = "Canonical configs, fanned out to all your AI coding agents",
    long_about = "Canonical configs, fanned out to all your AI coding agents.\n\n\
                  Sync skills, instructions, MCP servers, commands, subagents and hooks \
                  from one canonical Store (~/.agents) to every installed agent.",
    after_help = "Home: https://github.com/agentstow/agentstow"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Fan the Store out to every installed agent.
    Sync {
        /// Report the plan without touching the filesystem.
        #[arg(long)]
        dry_run: bool,
    },
    /// Create the Store skeleton.
    Init,
    /// Absorb an existing config into the Store, leaving a link behind.
    Adopt {
        /// The real file or directory to take into the Store.
        path: String,
    },
    /// Report what is in sync, what is not, and what is not ours.
    Status {
        /// Emit a machine-readable report on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Manage MCP servers.
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Check machine readiness: installed agents, Store usability, hygiene.
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum McpCommand {
    /// Show Store servers and their state in every agent.
    List {
        /// Emit a machine-readable report on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Take an agent's existing server into the Store.
    Adopt {
        /// Which server, as agent/server — for example claude/serena.
        spec: Option<String>,
        /// Adopt every server found in every agent's config.
        #[arg(long, conflicts_with = "spec")]
        all: bool,
    },
    /// Delete a server from the Store and from every agent.
    Remove {
        /// The server name.
        name: String,
    },
}
