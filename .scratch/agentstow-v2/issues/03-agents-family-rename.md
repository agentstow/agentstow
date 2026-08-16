# 03 — Rename the subagents family to agents

**What to build:** The Commons directory and family formerly called `subagents` is now `agents` — in the directory name, the report labels, and the config key — matching both fan-out destinations and the `.agents` Protocol's name for it. `subagents` survives as an accepted alias for the custom-target config key, and `doctor` hints when a non-empty `subagents/` directory still exists in the Commons (stateless migration — no version marker). The duplicate family-directory-name constants collapse onto the single source of truth. Reasoning: DECISIONS.md 2026-08-16 "The .agents Protocol is adopted with two reservations".

**Blocked by:** 01 — Rename the Store to the Commons everywhere.

**Status:** done

- [ ] The family's Commons dir, report label, and config key are `agents`; sync/status/doctor all say `agents`
- [ ] `subagents` still works as a custom-target config key (alias), with a test
- [ ] `doctor` hints when a non-empty `subagents/` dir exists in the Commons
- [ ] The dir-name constants have one source of truth
- [ ] Fan-out still reaches Claude's and opencode's agents directories; suite green
