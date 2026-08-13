//! agentstow — sync AI coding agent configs from one canonical Store.
//!
//! The single testing seam is [`run`]: it takes argv, the process environment,
//! and writers for stdout/stderr, and returns the process exit code. Every
//! integration test drives the tool through this function against a throwaway
//! directory tree, so no test touches the real home directory.

use std::io::Write;

use clap::Parser;

pub mod cli;
pub mod doctor;
pub mod env;
pub mod registry;
pub mod report;
pub mod store;

/// Everything is in the state the Store describes.
pub const EXIT_CLEAN: i32 = 0;
/// The command could not complete.
pub const EXIT_ERROR: i32 = 1;
/// Reserved: actionable state exists (something to sync or resolve).
pub const EXIT_ACTIONABLE: i32 = 2;

/// The single seam. `args` is argv including the program name; `vars` is the
/// process environment as key/value pairs.
pub fn run(
    args: &[String],
    vars: &[(String, String)],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let parsed = match cli::Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind;
            // --help and --version are results, not diagnostics.
            let informational = matches!(
                e.kind(),
                ErrorKind::DisplayHelp
                    | ErrorKind::DisplayVersion
                    | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            );
            if informational {
                let _ = write!(out, "{e}");
                return EXIT_CLEAN;
            }
            let _ = write!(err, "{e}");
            return EXIT_ERROR;
        }
    };

    let env = match env::Env::resolve(vars) {
        Ok(env) => env,
        Err(e) => {
            let _ = writeln!(err, "error: {e}");
            return EXIT_ERROR;
        }
    };

    let mut reporter = report::Reporter::new(out, err);

    match parsed.command {
        cli::Command::Doctor => doctor::run(&env, &mut reporter),
    }
}
