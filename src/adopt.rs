//! `adopt` — absorb an existing config into the Store and leave a link behind.
//!
//! Three cases, and no `--force`. Refusing to merge a divergence by hand is the
//! point: silently discarding one side of it is exactly the failure mode this
//! tool is defined against.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::env::Env;
use crate::link;
use crate::lock;
use crate::report::Reporter;
use crate::store::{Store, FANOUT_FAMILIES};
use crate::target;
use crate::{EXIT_CLEAN, EXIT_ERROR};

pub fn run(env: &Env, config: &Config, r: &mut Reporter, raw_path: &str) -> i32 {
    let store = Store::new(env.store());
    if !store.exists() {
        r.problem(format!(
            "no Store at {} — run `agentstow init` to create one",
            store.root().display()
        ));
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

    // Which Target directory is this in, and therefore which family?
    let Some((target_name, family)) = locate(env, config, &path) else {
        r.problem(format!(
            "{} is not inside a fan-out directory agentstow manages — \
             run `agentstow doctor` to see which directories those are",
            path.display()
        ));
        return EXIT_ERROR;
    };

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let destination = store.family_dir(family).join(&file_name);

    // Everything is decided before the lock is taken, so a refusal leaves the
    // machine exactly as it was — not even a lock file.
    if destination.exists() && !same_contents(&path, &destination) {
        r.problem(format!(
            "{} differs from the Store copy at {} — this is a Variant. \
             Merge it by hand if you want it in the Store; agentstow will not \
             choose which side to discard",
            path.display(),
            destination.display()
        ));
        return EXIT_ERROR;
    }

    let _lock = match lock::acquire(env.config_dir(), crate::sync::lock_timeout(env)) {
        Ok(guard) => guard,
        Err(e) => {
            r.problem(e.to_string());
            return EXIT_ERROR;
        }
    };

    if destination.exists() {
        return relink(&path, &destination, &file_name, target_name, r);
    }

    move_in(&path, &destination, &file_name, family, target_name, r)
}

/// The Store does not have this name: move it in, leave a link behind.
fn move_in(
    path: &Path,
    destination: &Path,
    file_name: &str,
    family: &str,
    target: String,
    r: &mut Reporter,
) -> i32 {
    if let Some(parent) = destination.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            r.problem(format!("cannot create {}: {e}", parent.display()));
            return EXIT_ERROR;
        }
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
fn relink(
    path: &Path,
    destination: &Path,
    file_name: &str,
    target: String,
    r: &mut Reporter,
) -> i32 {
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
    std::os::unix::fs::symlink(&text, path)?;
    Ok(text)
}

/// Which Target and family owns the directory this path sits in.
fn locate(env: &Env, config: &Config, path: &Path) -> Option<(String, &'static str)> {
    let parent = link::normalize(path.parent()?);
    for target in target::resolve(env, config) {
        for (family, _) in FANOUT_FAMILIES {
            let Some(dir) = target.fanout_dir(family) else {
                continue;
            };
            if link::normalize(&env.in_home(dir)) == parent {
                return Some((target.name.clone(), family));
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

fn same_contents(a: &Path, b: &Path) -> bool {
    match (a.is_dir(), b.is_dir()) {
        (true, true) => tree_bytes(a) == tree_bytes(b),
        (false, false) => match (fs::read(a), fs::read(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        },
        _ => false,
    }
}

fn tree_bytes(root: &Path) -> Option<Vec<(String, Vec<u8>)>> {
    let mut acc: Vec<(String, Vec<u8>)> = Vec::new();
    fn walk(root: &Path, dir: &Path, acc: &mut Vec<(String, Vec<u8>)>) -> Option<()> {
        for entry in fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).ok()?.display().to_string();
            if path.is_dir() {
                walk(root, &path, acc)?;
            } else {
                acc.push((rel, fs::read(&path).ok()?));
            }
        }
        Some(())
    }
    walk(root, root, &mut acc)?;
    acc.sort_by(|a, b| a.0.cmp(&b.0));
    Some(acc)
}
