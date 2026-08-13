//! Link identity: the fan-out engine shared by every symlink family.
//!
//! Ownership is established by where a link *points*, not by any bookkeeping
//! (ADR-0001): a symlink resolving into the Store is agentstow's to canonicalise
//! or prune, and anything else — a link elsewhere, a real file or directory — is
//! left exactly as found.
//!
//! Resolution here is deliberately lexical. A dangling link still has to be
//! classified, and following it is impossible; comparing link text against the
//! Store path textually gives the same answer for live and broken links alike.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::store::Entry;

/// Resolve `.` and `..` textually, without touching the filesystem.
pub fn normalize(path: &Path) -> PathBuf {
    let mut root: Option<OsString> = None;
    let mut parts: Vec<OsString> = Vec::new();

    for component in path.components() {
        match component {
            Component::RootDir | Component::Prefix(_) => {
                root = Some(component.as_os_str().to_owned())
            }
            Component::CurDir => {}
            Component::ParentDir => match parts.last() {
                Some(last) if last != ".." => {
                    parts.pop();
                }
                // Above an absolute root there is nothing to pop.
                _ if root.is_some() => {}
                _ => parts.push("..".into()),
            },
            Component::Normal(part) => parts.push(part.to_owned()),
        }
    }

    let mut out = PathBuf::new();
    if let Some(root) = root {
        out.push(root);
    }
    for part in parts {
        out.push(part);
    }
    out
}

/// Where a symlink at `link` with contents `text` points, lexically.
pub fn resolve_link(link: &Path, text: &Path) -> PathBuf {
    if text.is_absolute() {
        normalize(text)
    } else {
        let base = link.parent().unwrap_or(Path::new(""));
        normalize(&base.join(text))
    }
}

/// The relative path from directory `base` to `target`.
///
/// Relative links are the canonical form: they survive the whole tree being
/// renamed or restored somewhere else, which absolute links do not.
pub fn relative_from(base: &Path, target: &Path) -> PathBuf {
    let base = normalize(base);
    let target = normalize(target);

    if base.is_absolute() != target.is_absolute() {
        return target;
    }

    let base_parts: Vec<Component> = base.components().collect();
    let target_parts: Vec<Component> = target.components().collect();
    let shared = base_parts
        .iter()
        .zip(target_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut out = PathBuf::new();
    for _ in shared..base_parts.len() {
        out.push("..");
    }
    for part in &target_parts[shared..] {
        out.push(part.as_os_str());
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}

/// What agentstow found at one destination path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// Nothing there.
    Absent,
    /// A symlink pointing into the Store — ours.
    Ours {
        text: PathBuf,
        resolved: PathBuf,
        dangling: bool,
    },
    /// A symlink pointing anywhere else — Foreign.
    Foreign { text: PathBuf },
    /// A real file or directory — a Variant, preserved unconditionally.
    Variant,
}

/// Classify a destination path against the Store.
pub fn classify(path: &Path, store: &Path) -> Found {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return Found::Absent,
    };

    if !meta.file_type().is_symlink() {
        return Found::Variant;
    }

    let text = match fs::read_link(path) {
        Ok(t) => t,
        Err(_) => return Found::Variant,
    };
    let resolved = resolve_link(path, &text);

    if resolved.starts_with(normalize(store)) {
        Found::Ours {
            text,
            resolved,
            dangling: !path.exists(),
        }
    } else {
        Found::Foreign { text }
    }
}

/// One change (or one thing deliberately left alone) at a destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    /// Create a canonical link.
    Link { path: PathBuf, text: PathBuf },
    /// Replace one of our links that is not in canonical form.
    Relink {
        path: PathBuf,
        text: PathBuf,
        was: PathBuf,
    },
    /// Remove one of our links whose Store entry is gone.
    Prune { path: PathBuf },
    /// A Variant shadowing a Store entry — reported, never touched.
    Variant { path: PathBuf, identical: bool },
    /// A Foreign entry in our way — reported, never touched.
    Foreign { path: PathBuf },
}

impl Act {
    /// Whether applying this act would change the filesystem.
    pub fn mutates(&self) -> bool {
        matches!(
            self,
            Act::Link { .. } | Act::Relink { .. } | Act::Prune { .. }
        )
    }

    pub fn path(&self) -> &Path {
        match self {
            Act::Link { path, .. }
            | Act::Relink { path, .. }
            | Act::Prune { path }
            | Act::Variant { path, .. }
            | Act::Foreign { path } => path,
        }
    }

    /// Short verb for reports.
    pub fn verb(&self) -> &'static str {
        match self {
            Act::Link { .. } => "link",
            Act::Relink { .. } => "relink",
            Act::Prune { .. } => "prune",
            Act::Variant { .. } => "variant",
            Act::Foreign { .. } => "foreign",
        }
    }
}

/// Plan the fan-out of `entries` into `target_dir`.
///
/// Pruning is limited to *dangling* links into the Store: a live link to a Store
/// entry that simply is not being synced right now stays, because deleting it
/// would be guessing.
pub fn plan(target_dir: &Path, store: &Path, entries: &[Entry]) -> Vec<Act> {
    let mut acts = Vec::new();
    let wanted: BTreeSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    for entry in entries {
        let dest = target_dir.join(&entry.name);
        let canonical = relative_from(target_dir, &entry.path);

        match classify(&dest, store) {
            Found::Absent => acts.push(Act::Link {
                path: dest,
                text: canonical,
            }),
            Found::Ours { text, resolved, .. } => {
                let on_target = resolved == normalize(&entry.path);
                if on_target && text == canonical {
                    continue;
                }
                acts.push(Act::Relink {
                    path: dest,
                    text: canonical,
                    was: text,
                });
            }
            Found::Foreign { .. } => acts.push(Act::Foreign { path: dest }),
            Found::Variant => {
                let identical = same_contents(&dest, &entry.path);
                acts.push(Act::Variant {
                    path: dest,
                    identical,
                })
            }
        }
    }

    // Anything else in the directory: ours and dangling gets pruned, the rest
    // is somebody else's business.
    if let Ok(read) = fs::read_dir(target_dir) {
        for found in read.flatten() {
            let name = found.file_name().to_string_lossy().into_owned();
            if wanted.contains(name.as_str()) {
                continue;
            }
            let path = found.path();
            if let Found::Ours { dangling: true, .. } = classify(&path, store) {
                acts.push(Act::Prune { path });
            }
        }
    }

    acts.sort_by(|a, b| a.path().cmp(b.path()));
    acts
}

/// Apply one act. Creating a link creates the directories above it, but the
/// caller is responsible for never handing us a path under an absent agent root.
pub fn apply(act: &Act) -> io::Result<()> {
    match act {
        Act::Link { path, text } => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            std::os::unix::fs::symlink(text, path)
        }
        Act::Relink { path, text, .. } => {
            fs::remove_file(path)?;
            std::os::unix::fs::symlink(text, path)
        }
        Act::Prune { path } => fs::remove_file(path),
        Act::Variant { .. } | Act::Foreign { .. } => Ok(()),
    }
}

/// Whether a Variant is byte-identical to the Store entry it shadows — an
/// accident of history rather than a deliberate divergence.
fn same_contents(a: &Path, b: &Path) -> bool {
    match (a.is_dir(), b.is_dir()) {
        (true, true) => dir_digest(a) == dir_digest(b),
        (false, false) => match (fs::read(a), fs::read(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        },
        _ => false,
    }
}

/// Sorted (relative path, bytes) listing of a directory tree.
fn dir_digest(root: &Path) -> Option<Vec<(String, Vec<u8>)>> {
    let mut acc = Vec::new();
    collect(root, root, &mut acc)?;
    acc.sort_by(|a, b| a.0.cmp(&b.0));
    Some(acc)
}

fn collect(root: &Path, dir: &Path, acc: &mut Vec<(String, Vec<u8>)>) -> Option<()> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let rel = path.strip_prefix(root).ok()?.display().to_string();
        if path.is_dir() {
            collect(root, &path, acc)?;
        } else {
            acc.push((rel, fs::read(&path).ok()?));
        }
    }
    Some(())
}
