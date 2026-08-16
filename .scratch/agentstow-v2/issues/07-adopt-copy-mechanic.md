# 07 — The Copy mechanic and the unified collision matrix

**What to build:** Adopting an external path with no durable home (no repo above it) copies the content into the Commons. The original is left untouched and nothing points back — a deliberate divorce the message states out loud: the original is no longer consulted and edits there will diverge from the Commons copy. A symlink input is copied through (content, not the link). The result is an ordinary Commons entry — indistinguishable from an absorbed one — fanned out like any other. This ticket also proves the collision matrix is genuinely uniform across all three mechanics: identical → no-op (copy) / replace-with-link (absorb, source); divergent → refuse as a Variant; same-resolved link → no-op; link elsewhere → refuse with the fix.

**Blocked by:** 06 — the Source mechanic (shared dispatch and matrix land there first).

**Status:** done

- [ ] A non-repo external path with a family-named parent copies in, fans out, and `sync` after is a no-op
- [ ] The message states the original's path, that it is no longer consulted, and that it will diverge
- [ ] A symlink input copies the content through
- [ ] The collision matrix behaves identically across absorb/source/copy — covered by one test sweep over all four cases × three mechanics
- [ ] `adopt --dry-run` names the Copy mechanic and its would-actions
