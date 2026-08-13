//! Output discipline: results to stdout, diagnostics to stderr, and an exit
//! code derived from what was found.
//!
//! Exit 0 is clean, 1 is an error, 2 is reserved for actionable state (drift)
//! so automation can tell "needs a sync" from "the command broke".

use std::io::Write;

use crate::{EXIT_CLEAN, EXIT_ERROR};

/// Writers plus the running verdict for one invocation.
pub struct Reporter<'a> {
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    problems: usize,
    warnings: usize,
}

impl<'a> Reporter<'a> {
    pub fn new(out: &'a mut dyn Write, err: &'a mut dyn Write) -> Self {
        Self {
            out,
            err,
            problems: 0,
            warnings: 0,
        }
    }

    /// A result line — stdout.
    pub fn line(&mut self, text: impl AsRef<str>) {
        let _ = writeln!(self.out, "{}", text.as_ref());
    }

    /// A machine-readable result — stdout, pretty-printed, newline-terminated.
    pub fn json(&mut self, value: &serde_json::Value) {
        let _ = writeln!(
            self.out,
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
        );
    }

    /// A blank result line.
    pub fn blank(&mut self) {
        let _ = writeln!(self.out);
    }

    /// Advisory diagnostic — stderr, does not affect the exit code.
    pub fn warn(&mut self, text: impl AsRef<str>) {
        self.warnings += 1;
        let _ = writeln!(self.err, "warning: {}", text.as_ref());
    }

    /// A blocking problem — stderr, forces a non-clean exit.
    pub fn problem(&mut self, text: impl AsRef<str>) {
        self.problems += 1;
        let _ = writeln!(self.err, "error: {}", text.as_ref());
    }

    pub fn problem_count(&self) -> usize {
        self.problems
    }

    /// `EXIT_ERROR` if any problem was reported, else `EXIT_CLEAN`.
    /// Warnings are advisory and never change the code.
    pub fn verdict(&self) -> i32 {
        if self.problems > 0 {
            EXIT_ERROR
        } else {
            EXIT_CLEAN
        }
    }
}
