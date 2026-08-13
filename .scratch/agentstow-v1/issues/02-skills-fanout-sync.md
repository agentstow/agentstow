# 02 — Skills fan-out `sync` (+ global lock)

**What to build:** `agentstow sync` makes every Store skill reachable from every detected fan-out target: one canonical **relative** symlink per skill, missing target subdirectories created (never roots), our mis-formed links (absolute or odd paths resolving into the Store) rewritten to canonical form, dangling Store-pointing links pruned. Variants (any real object at a Store-colliding path) and Foreign links are never touched. `--dry-run` previews everything. Mutating commands serialize on a global lock.

**Blocked by:** 01 — Crate scaffold, test seam, target registry, `doctor`.

**Status:** done

- [x] Fresh fixture: one canonical relative link per Store skill in each fan-out target; native targets (opencode, oh-my-pi) receive nothing
- [x] An absolute link resolving into the Store is rewritten to canonical relative form; a link resolving elsewhere is untouched
- [x] Dangling Store-pointing links are pruned; dangling Foreign links are left in place
- [x] A real directory colliding with a Store skill name is preserved unchanged (Variant)
- [x] A second `sync` is a byte-identical no-op and says so
- [x] `--dry-run` prints every planned action and modifies nothing
- [x] Two concurrent mutating invocations serialize on the lock; the loser waits or fails cleanly
