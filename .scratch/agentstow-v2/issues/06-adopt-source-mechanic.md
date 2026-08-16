# 06 — The Source mechanic and doctor's Sourced section

**What to build:** Adopting a path with a durable home — a `.git` directory *or file* (worktrees, submodules) found walking up — creates an **absolute** Commons symlink to the path *as given* (symlink inputs are legal and never resolved through): a Sourced entry, fanned out like any other, with the message "Sourced from `<path>`". The family comes from the parent directory's basename (`skills`/`commands`/`agents`); any other parent refuses naming the accepted ones. Collision handling per the shared matrix: an identical real Commons copy is replaced by the link; a divergent one refuses; an existing link resolving to the same place is a clean no-op (idempotent re-runs, compared by resolved target); a link elsewhere refuses naming the fix. Declared custom-target surfaces keep precedence over the repo walk-up — an explicit declaration means fan-out semantics, warned in the custom-target docs. `doctor` gains the Sourced view: a `Sourced entries:` section (`name ← source`, with `(source missing)` markers) and a `(N sourced)` suffix on the skills count. Dangling-Source behavior is unchanged from v1 (warn, skip, prune on next sync) — by decision, not omission.

**Blocked by:** 05 — adopt dispatch, guard, absorb fan-out.

**Status:** ready-for-agent

- [ ] A repo path (including via a `.git` file) adopts as an absolute as-given Commons link and fans out; re-running is a clean no-op
- [ ] Identical real copy → replaced by the link; divergent → refuses; link elsewhere → refuses with the repoint instruction
- [ ] Non-family parent basename refuses, naming the accepted directories
- [ ] Custom-target surface precedence over the walk-up is pinned by a test
- [ ] `doctor` lists every Sourced entry with its source, marks missing sources, and shows the sourced count
- [ ] `adopt --dry-run` names the Source mechanic and its would-actions
