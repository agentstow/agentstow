# 08 — revert <agent>: deliberate offboarding

**What to build:** `revert <agent>` removes everything agentstow put into one target — fan-out links, merged MCP and hook entries, Marker files — and nothing else: Foreign content and Variants are untouched. It **refuses while the target is still enabled**, printing the exact `targets.<name> = false` config line to add, so a later sync cannot silently redo what revert undid and revert never edits the user's config as a side effect. It works on disabled-but-still-detected agents — the gap it exists to close, since disabling removes an agent from every scan and orphans its links forever. Reasoning: DECISIONS.md 2026-08-16 "Two verbs join the surface".

**Blocked by:** 01 — Rename the Store to the Commons everywhere.

**Status:** done

- [ ] Revert on an enabled agent refuses and prints the exact disable line for that agent
- [ ] Revert on a disabled agent removes our links, our merged MCP/hook entries, and Marker files from that agent only
- [ ] Foreign files/links/entries and Variants in that agent survive untouched
- [ ] After revert, `sync` recreates nothing for that agent and `status`/`doctor` report it clean/absent
- [ ] Runs under the lock; refusal precedes it
