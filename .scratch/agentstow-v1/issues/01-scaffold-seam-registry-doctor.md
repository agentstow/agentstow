# 01 — Crate scaffold, test seam, target registry, `doctor`

**What to build:** A Rust CLI skeleton a user can run as `agentstow doctor` and get a truthful report of their machine: which of the ten registry agents are detected (config root exists — roots are never created), whether the Store exists, whether targets are writable, and Store-hygiene warnings (non-directory entries, dot-prefixed names). All path resolution honors `AGENTSTOW_HOME` (Store) and `AGENTSTOW_TARGET_ROOT` (home redirection), and the single-seam integration harness — build a fixture tree, run the CLI, assert filesystem/stdout/stderr/exit — is established here for every later ticket.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] `doctor` on a fixture tree lists exactly the agents whose config roots exist; no directory is ever created
- [x] Store hygiene warnings fire for non-directory Store entries and dot-prefixed names
- [x] All tests run hermetically against temp roots via the two env overrides; no test touches the real home
- [x] Results go to stdout, diagnostics to stderr; exit 0 = clean, 1 = error
- [x] Built-in registry is a data-driven table (one row per agent, per-family capability) so adding an agent is a small data change
