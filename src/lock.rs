//! A global lock so a cron sync and a hand-run sync cannot interleave writes.
//!
//! The lock lives beside the tool's own configuration, never in the Store.
//! It is advisory (flock) and released when the process exits, so a crashed run
//! leaves nothing to clean up.

use std::fs::{self, File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

/// How long a mutating command waits for a competing one to finish.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Held for as long as the guard lives; released on drop.
pub struct Lock {
    _file: File,
}

#[derive(Debug)]
pub enum Error {
    /// Another agentstow process held the lock for longer than we waited.
    Busy,
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Busy => write!(
                f,
                "another agentstow process is running — try again when it finishes"
            ),
            Error::Io(e) => write!(f, "cannot take the agentstow lock: {e}"),
        }
    }
}

/// Take the exclusive lock, waiting up to `timeout`.
pub fn acquire(config_dir: &Path, timeout: Duration) -> Result<Lock, Error> {
    fs::create_dir_all(config_dir).map_err(Error::Io)?;
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(config_dir.join("lock"))
        .map_err(Error::Io)?;

    let start = Instant::now();
    loop {
        // SAFETY: a live fd from the File above; flock only inspects it.
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc == 0 {
            return Ok(Lock { _file: file });
        }

        let err = std::io::Error::last_os_error();
        let would_block = matches!(
            err.raw_os_error(),
            Some(code) if code == libc::EWOULDBLOCK || code == libc::EINTR
        );
        if !would_block {
            return Err(Error::Io(err));
        }
        if start.elapsed() >= timeout {
            return Err(Error::Busy);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
