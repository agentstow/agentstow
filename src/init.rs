//! `init` — create the Store skeleton.
//!
//! Scaffolding only: it creates the Store and its family directories and
//! nothing else. It never creates an agent root, because a root's existence is
//! what detection means, and it never invents an `AGENTS.md` — an empty one
//! would fan out to every agent as empty instructions.

use std::fs;

use crate::env::Env;
use crate::family::Family;
use crate::report::Reporter;
use crate::store::Store;
use crate::{EXIT_CLEAN, EXIT_ERROR};

pub fn run(env: &Env, r: &mut Reporter) -> i32 {
    let store = Store::new(env.store());
    let existed = store.exists();

    for dir in std::iter::once(store.root().to_path_buf())
        .chain(Family::ALL.iter().map(|f| store.family_dir(f.name())))
    {
        if let Err(e) = fs::create_dir_all(&dir) {
            r.problem(format!("cannot create {}: {e}", dir.display()));
            return EXIT_ERROR;
        }
    }

    if existed {
        r.line(format!(
            "Store already present at {}",
            store.root().display()
        ));
    } else {
        r.line(format!("Created the Store at {}", store.root().display()));
    }
    for family in Family::ALL {
        r.line(format!("  {}/", family.name()));
    }

    r.blank();
    r.line("Put a skill in skills/, instructions in AGENTS.md, then run `agentstow sync`.");
    r.line("`agentstow adopt <path>` takes an existing config into the Store for you.");

    EXIT_CLEAN
}
