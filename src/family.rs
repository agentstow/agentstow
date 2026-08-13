//! The fan-out families, as a type rather than a string.
//!
//! Every family question — which Store directory, what shape its entries are,
//! which registry column to read — is answered here, so adding a family is one
//! variant and the compiler finds the rest.

use std::fmt;

/// What a family's Store entries look like on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// One directory per entry.
    Directory,
    /// One markdown file per entry.
    Markdown,
}

/// A family of configs that reaches Targets by symlink fan-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    Skills,
    Commands,
    Subagents,
}

impl Family {
    /// Every fan-out family, in report order.
    pub const ALL: &'static [Family] = &[Family::Skills, Family::Commands, Family::Subagents];

    /// The Store directory, and the name used in config and reports.
    pub fn name(&self) -> &'static str {
        match self {
            Family::Skills => "skills",
            Family::Commands => "commands",
            Family::Subagents => "subagents",
        }
    }

    pub fn shape(&self) -> Shape {
        match self {
            Family::Skills => Shape::Directory,
            Family::Commands | Family::Subagents => Shape::Markdown,
        }
    }

    /// Parse a family name from configuration. Unknown names are rejected
    /// rather than guessed at.
    pub fn from_name(name: &str) -> Option<Family> {
        Family::ALL.iter().copied().find(|f| f.name() == name)
    }
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
