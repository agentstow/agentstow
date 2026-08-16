# 05 — adopt dispatch, the Commons guard, and absorb that finishes fanned out

**What to build:** `adopt <path>` gains its v2 dispatch skeleton (ADR-0006). First, the guard: a path inside the Commons refuses — "already in the Commons — nothing to adopt" — ordered before any repo detection, because a committed Commons has a `.git` above every entry and would otherwise self-link and destroy the entry; a test pins exactly the committed-Commons scenario. Second, the absorb mechanic (a real file/dir in a target surface) now **finishes the job**: after moving in and leaving the link behind, the entry is fanned out to every installed agent through sync's own per-entry path (Variants kept, Native agents skipped), so a sync immediately after is a no-op. Third, `adopt --dry-run`: read-only, lockless, names the chosen mechanic and prints every action in "would" tense. External paths keep refusing in this ticket (Source and Copy land in 06/07).

**Blocked by:** 03 — agents family rename; 04 — two-phase sync (the fan-out rides its per-entry plan machinery).

**Status:** done

- [ ] Any path inside the Commons refuses with the "already in the Commons" message; the committed-Commons (`.git` present) self-link scenario is pinned by a test
- [ ] Absorbing from an agent surface moves the content in, leaves the relative link, and fans the entry out to every installed agent; `sync` immediately after reports nothing to do
- [ ] Variants in other agents are left alone by the fan-out; Native agents receive nothing
- [ ] `adopt --dry-run` is read-only and lockless, names the mechanic, prints "would" actions matching what a real run does
- [ ] A symlink in an agent surface still refuses; a divergent Commons copy still refuses as a Variant; all refusals precede the lock
