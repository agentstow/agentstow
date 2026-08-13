//! The single testing seam.
//!
//! Every test builds a throwaway tree, points `AGENTSTOW_TARGET_ROOT` (home) and
//! `AGENTSTOW_HOME` (Store) at it, and drives the CLI through `agentstow::run`.
//! Nothing here touches the real home directory, and no test reaches inside the
//! implementation — assertions are on the resulting filesystem, stdout, stderr
//! and exit code only.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// What one CLI invocation produced.
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Outcome {
    /// Assert a clean exit, showing stderr when it isn't.
    pub fn assert_clean(&self) -> &Self {
        assert_eq!(
            self.code, 0,
            "expected exit 0\nstdout:\n{}\nstderr:\n{}",
            self.stdout, self.stderr
        );
        self
    }

    pub fn assert_code(&self, want: i32) -> &Self {
        assert_eq!(
            self.code, want,
            "expected exit {want}\nstdout:\n{}\nstderr:\n{}",
            self.stdout, self.stderr
        );
        self
    }

    pub fn assert_stdout_has(&self, needle: &str) -> &Self {
        assert!(
            self.stdout.contains(needle),
            "expected stdout to contain {needle:?}\nstdout:\n{}",
            self.stdout
        );
        self
    }

    pub fn assert_stdout_lacks(&self, needle: &str) -> &Self {
        assert!(
            !self.stdout.contains(needle),
            "expected stdout NOT to contain {needle:?}\nstdout:\n{}",
            self.stdout
        );
        self
    }

    pub fn assert_stderr_has(&self, needle: &str) -> &Self {
        assert!(
            self.stderr.contains(needle),
            "expected stderr to contain {needle:?}\nstderr:\n{}",
            self.stderr
        );
        self
    }

    pub fn assert_stderr_lacks(&self, needle: &str) -> &Self {
        assert!(
            !self.stderr.contains(needle),
            "expected stderr NOT to contain {needle:?}\nstderr:\n{}",
            self.stderr
        );
        self
    }

    /// Neither stream may contain this text — used for secret canaries.
    pub fn assert_no_output_contains(&self, needle: &str) -> &Self {
        self.assert_stdout_lacks(needle).assert_stderr_lacks(needle)
    }

    pub fn assert_stderr_empty(&self) -> &Self {
        assert!(
            self.stderr.is_empty(),
            "expected empty stderr, got:\n{}",
            self.stderr
        );
        self
    }
}

/// A throwaway machine: a home directory, and a Store beside it.
pub struct Fixture {
    _dir: TempDir,
    home: PathBuf,
    store: PathBuf,
}

impl Fixture {
    /// A machine with no Store and no agents installed.
    pub fn bare() -> Self {
        let dir = tempfile::tempdir().expect("create tempdir");
        let home = dir.path().join("home");
        fs::create_dir_all(&home).expect("create home");
        let store = home.join(".agents");
        Self {
            _dir: dir,
            home,
            store,
        }
    }

    /// A machine with an initialised, empty Store.
    pub fn new() -> Self {
        let f = Self::bare();
        fs::create_dir_all(f.store.join("skills")).expect("create store");
        f
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn store(&self) -> &Path {
        &self.store
    }

    /// The temporary directory containing the home — somewhere to put a second
    /// tree when a test needs one (an alternate Store, a decoy home).
    pub fn root(&self) -> &Path {
        self._dir.path()
    }

    /// Home-relative path.
    pub fn path(&self, rel: &str) -> PathBuf {
        self.home.join(rel)
    }

    /// Install an agent by creating its config root.
    pub fn agent(&self, root_rel: &str) -> &Self {
        fs::create_dir_all(self.home.join(root_rel)).expect("create agent root");
        self
    }

    /// Create a home-relative directory.
    pub fn dir(&self, rel: &str) -> &Self {
        fs::create_dir_all(self.home.join(rel)).expect("create dir");
        self
    }

    /// Write a home-relative file, creating parents.
    pub fn file(&self, rel: &str, body: &str) -> &Self {
        let p = self.home.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("create parents");
        }
        fs::write(&p, body).expect("write file");
        self
    }

    /// Add a skill to the Store.
    pub fn store_skill(&self, name: &str) -> &Self {
        let dir = self.store.join("skills").join(name);
        fs::create_dir_all(&dir).expect("create store skill");
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n\nbody of {name}\n"),
        )
        .expect("write SKILL.md");
        self
    }

    /// Create a Store-relative symlink pointing at `target` (verbatim, so a
    /// test can create a deliberately dangling one).
    pub fn store_symlink(&self, rel: &str, target: &str) -> &Self {
        let p = self.store.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("create parents");
        }
        std::os::unix::fs::symlink(target, &p).expect("create symlink");
        self
    }

    /// Write a Store-relative file, creating parents.
    pub fn store_file(&self, rel: &str, body: &str) -> &Self {
        let p = self.store.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("create parents");
        }
        fs::write(&p, body).expect("write store file");
        self
    }

    /// Run the CLI with both overrides pointed at this fixture.
    pub fn run(&self, args: &[&str]) -> Outcome {
        self.run_with_vars(
            args,
            &[
                ("AGENTSTOW_TARGET_ROOT", self.home.display().to_string()),
                ("AGENTSTOW_HOME", self.store.display().to_string()),
            ],
        )
    }

    /// Run the CLI with an explicit environment — for testing resolution itself.
    pub fn run_with_vars(&self, args: &[&str], vars: &[(&str, String)]) -> Outcome {
        let mut argv: Vec<String> = vec!["agentstow".to_string()];
        argv.extend(args.iter().map(|a| a.to_string()));
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();

        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let code = agentstow::run(&argv, &vars, &mut out, &mut err);

        Outcome {
            code,
            stdout: String::from_utf8_lossy(&out).into_owned(),
            stderr: String::from_utf8_lossy(&err).into_owned(),
        }
    }

    /// The verbatim text of a home-relative symlink.
    pub fn link_text(&self, rel: &str) -> String {
        fs::read_link(self.home.join(rel))
            .unwrap_or_else(|e| panic!("{rel} is not a symlink: {e}"))
            .display()
            .to_string()
    }

    /// Whether a home-relative path is a symlink (without following it).
    pub fn is_symlink(&self, rel: &str) -> bool {
        fs::symlink_metadata(self.home.join(rel))
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    }

    /// Whether a home-relative path exists as a real directory (not a link).
    pub fn is_real_dir(&self, rel: &str) -> bool {
        fs::symlink_metadata(self.home.join(rel))
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Whether a home-relative path exists at all (following links).
    pub fn exists(&self, rel: &str) -> bool {
        self.home.join(rel).exists()
    }

    /// Whether a home-relative path exists as a link or file, even if broken.
    pub fn present(&self, rel: &str) -> bool {
        fs::symlink_metadata(self.home.join(rel)).is_ok()
    }

    /// Where a home-relative symlink actually lands, following it.
    pub fn resolves_to(&self, rel: &str) -> PathBuf {
        fs::canonicalize(self.home.join(rel))
            .unwrap_or_else(|e| panic!("{rel} does not resolve: {e}"))
    }

    /// Create a home-relative symlink with verbatim link text.
    pub fn symlink(&self, rel: &str, target: &str) -> &Self {
        let p = self.home.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("create parents");
        }
        std::os::unix::fs::symlink(target, &p).expect("create symlink");
        self
    }

    /// Raw text of a Store-relative file.
    pub fn contents_of_store(&self, rel: &str) -> String {
        fs::read_to_string(self.store.join(rel))
            .unwrap_or_else(|e| panic!("cannot read store/{rel}: {e}"))
    }

    /// Parse a home-relative JSON file.
    pub fn json(&self, rel: &str) -> serde_json::Value {
        let body = fs::read_to_string(self.home.join(rel))
            .unwrap_or_else(|e| panic!("cannot read {rel}: {e}"));
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("{rel} is not JSON: {e}"))
    }

    /// Raw bytes of a home-relative file, as text.
    pub fn contents(&self, rel: &str) -> String {
        fs::read_to_string(self.home.join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
    }

    /// Unix permission bits of a home-relative file.
    pub fn mode(&self, rel: &str) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(self.home.join(rel))
            .unwrap_or_else(|e| panic!("cannot stat {rel}: {e}"))
            .permissions()
            .mode()
            & 0o777
    }

    /// Write the Store's canonical MCP file.
    pub fn store_mcp(&self, body: &str) -> &Self {
        self.store_file("mcp.json", body)
    }

    /// Run with the standard overrides plus extra environment variables — the
    /// only way a test can supply a `${env:VAR}` value, since the tool never
    /// reads the real process environment.
    pub fn run_with_env(&self, args: &[&str], extra: &[(&str, &str)]) -> Outcome {
        let mut vars = vec![
            ("AGENTSTOW_TARGET_ROOT", self.home.display().to_string()),
            ("AGENTSTOW_HOME", self.store.display().to_string()),
        ];
        for (k, v) in extra {
            vars.push((k, v.to_string()));
        }
        self.run_with_vars(args, &vars)
    }

    /// Snapshot of the whole home tree, for "nothing was created" assertions.
    /// Each entry is `kind:relative/path`, where kind is dir, file or link.
    pub fn tree(&self) -> BTreeSet<String> {
        let mut acc = BTreeSet::new();
        walk(&self.home, &self.home, &mut acc);
        acc
    }
}

/// Hold the global lock the way a second agentstow process would, so a test can
/// prove a concurrent run refuses rather than racing.
pub struct HeldLock {
    _file: fs::File,
}

pub fn hold_lock(path: PathBuf) -> HeldLock {
    use std::os::unix::io::AsRawFd;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create lock dir");
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .expect("open lock file");
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    assert_eq!(rc, 0, "test could not take the lock");
    HeldLock { _file: file }
}

fn walk(root: &Path, dir: &Path, acc: &mut BTreeSet<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        // symlink_metadata: a link is recorded as a link, never followed.
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            let target = fs::read_link(&path).unwrap_or_default();
            acc.insert(format!("link:{rel} -> {}", target.display()));
        } else if meta.is_dir() {
            acc.insert(format!("dir:{rel}"));
            walk(root, &path, acc);
        } else {
            acc.insert(format!("file:{rel}"));
        }
    }
}
