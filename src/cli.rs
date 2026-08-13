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
    /// Check machine readiness: installed agents, Store usability, hygiene.
    Doctor,
}
