//! Argument parsing. Kept separate from [`crate::run`] so the seam stays a
//! plain function over argv.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "agentstow",
    version,
    about = "GNU Stow for your AI coding agents",
    long_about = "Sync skills, instructions, MCP servers, commands, subagents and hooks \
                  from one canonical Store (~/.agents) to every installed agent."
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
    /// Report what is in sync, what is not, and what is not ours.
    Status {
        /// Emit a machine-readable report on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Check machine readiness: installed agents, Store usability, hygiene.
    Doctor,
}
