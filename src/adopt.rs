//! `adopt` — absorb an existing config into the Store and leave a link behind.
//!
//! Three cases, and no `--force`. Refusing to merge a divergence by hand is the
//! point: silently discarding one side of it is exactly the failure mode this
//! tool is defined against.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::link;
use crate::lock;
use crate::registry::Instructions;
use crate::report::Reporter;
use crate::store::{self, Store};
use crate::target;
use crate::{EXIT_CLEAN, EXIT_ERROR};

pub fn run(env: &Env, config: &Config, r: &mut Reporter, raw_path: &str) -> i32 {
    let store = Store::new(env.store());
    if !store.exists() {
        r.problem(store::missing_message(store.root()));
        return EXIT_ERROR;
    }

    let path = absolute(raw_path);

    if !path.exists() && fs::symlink_metadata(&path).is_err() {
        r.problem(format!("{} does not exist", path.display()));
        return EXIT_ERROR;
    }

    if fs::symlink_metadata(&path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        r.problem(format!(
            "{} is already a symlink — there is nothing to adopt",
            path.display()
        ));
        return EXIT_ERROR;
    }

    // Which Target surface is this, and therefore where does it belong?
    let Some(placement) = locate(env, config, &path) else {
        r.problem(format!(
            "{} is not somewhere agentstow manages — \
             run `agentstow doctor` to see the directories it syncs",
            path.display()
        ));
        return EXIT_ERROR;
    };

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let destination = store.root().join(&placement.store_rel);

    // Everything is decided before the lock is taken, so a refusal leaves the
    // machine exactly as it was — not even a lock file.
    if destination.exists() && !link::same_contents(&path, &destination) {
        r.problem(format!(
            "{} differs from the Store copy at {} — this is a Variant. \
             Merge it by hand if you want it in the Store; agentstow will not \
             choose which side to discard",
            path.display(),
            destination.display()
        ));
        return EXIT_ERROR;
    }

    let _lock = match lock::acquire(env.config_dir(), lock::timeout(env)) {
        Ok(guard) => guard,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    if destination.exists() {
        return relink(&path, &destination, &file_name, &placement.target, r);
    }

    move_in(
        &path,
        &destination,
        &file_name,
        &placement.family,
        &placement.target,
        r,
    )
}

/// The Store does not have this name: move it in, leave a link behind.
fn move_in(
    path: &Path,
    destination: &Path,
    file_name: &str,
    family: &str,
    target: &str,
    r: &mut Reporter,
) -> i32 {
    if let Some(parent) = destination.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        r.problem(format!("cannot create {}: {e}", parent.display()));
        return EXIT_ERROR;
    }

    if let Err(e) = move_path(path, destination) {
        r.problem(format!(
            "cannot move {} into the Store: {e}",
            path.display()
        ));
        return EXIT_ERROR;
    }

    match place_link(path, destination) {
        Ok(text) => {
            r.line(format!(
                "adopted {file_name} from {target} into the Store ({family})"
            ));
            r.line(format!("  {} → {}", path.display(), text.display()));
            EXIT_CLEAN
        }
        Err(e) => {
            // Put it back rather than leave the user with neither copy.
            let _ = move_path(destination, path);
            r.problem(format!("cannot link {}: {e}", path.display()));
            EXIT_ERROR
        }
    }
}

/// The Store already has an identical copy: drop the duplicate, link instead.
fn relink(path: &Path, destination: &Path, file_name: &str, target: &str, r: &mut Reporter) -> i32 {
    let removed = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    if let Err(e) = removed {
        r.problem(format!("cannot remove {}: {e}", path.display()));
        return EXIT_ERROR;
    }

    match place_link(path, destination) {
        Ok(text) => {
            r.line(format!(
                "{file_name} in {target} was identical to the Store copy — re-linked"
            ));
            r.line(format!("  {} → {}", path.display(), text.display()));
            EXIT_CLEAN
        }
        Err(e) => {
            r.problem(format!("cannot link {}: {e}", path.display()));
            EXIT_ERROR
        }
    }
}

/// Create the canonical relative link from `path` to `destination`.
fn place_link(path: &Path, destination: &Path) -> std::io::Result<PathBuf> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let text = link::relative_from(parent, destination);
    link::create_symlink(&text, path)?;
    Ok(text)
}

/// Where in the Store an adopted path belongs: which Target it came from, and
/// the Store-relative path it will occupy.
struct Placement {
    target: String,
    /// What to call the family in reports.
    family: String,
    /// Store-relative destination, e.g. `skills/research` or `AGENTS.md`.
    store_rel: PathBuf,
}

/// Match a path against every Target surface agentstow manages: the fan-out
/// directories, and the per-agent instructions destinations.
fn locate(env: &Env, config: &Config, path: &Path) -> Option<Placement> {
    let parent = link::normalize(path.parent()?);
    let file_name = path.file_name()?.to_string_lossy().into_owned();

    for target in target::resolve(env, config) {
        for family in Family::ALL {
            let Some(dir) = target.fanout_dir(*family) else {
                continue;
            };
            if link::normalize(&env.in_home(dir)) == parent {
                return Some(Placement {
                    target: target.name.clone(),
                    family: family.name().to_string(),
                    store_rel: PathBuf::from(family.name()).join(&file_name),
                });
            }
        }

        // Instructions are a single file rather than a directory of entries, so
        // they are matched by full path, not by parent directory.
        if let Some(agent) = target.agent {
            let destination = match agent.instructions {
                Instructions::Symlink(rel) => Some(env.in_home(rel)),
                Instructions::RulesDirLink(dir) => Some(env.in_home(dir).join("AGENTS.md")),
                // Claude's file is the user's own; it is never adopted wholesale.
                Instructions::ImportLine(_) | Instructions::None => None,
            };
            if destination.map(|d| link::normalize(&d)) == Some(link::normalize(path)) {
                return Some(Placement {
                    target: target.name.clone(),
                    family: "instructions".to_string(),
                    store_rel: PathBuf::from(store::INSTRUCTIONS),
                });
            }
        }
    }
    None
}

fn absolute(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        link::normalize(&path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        link::normalize(&cwd.join(path))
    }
}

/// Rename where possible, copy across filesystems where not.
fn move_path(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_tree(from, to)?;
            if from.is_dir() {
                fs::remove_dir_all(from)
            } else {
                fs::remove_file(from)
            }
        }
    }
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(|_| ())
    }
}
