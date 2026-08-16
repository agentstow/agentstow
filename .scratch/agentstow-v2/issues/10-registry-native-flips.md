# 10 — Registry corrections: Native where natively read

**What to build:** Fan-out into an agent that also reads the Commons natively produces duplicate skills, so the registry must catch up with the ecosystem: for each candidate (Codex, Cursor, Cline, Amp, Gemini CLI, Copilot — from the 2026-08 research; opencode is already Native), verify against the agent's own primary documentation whether it reads user-level `~/.agents/skills` **unconditionally**. Flip that agent's skills row to Native where true, recording the doc evidence in the registry comment; anything conditional (opt-in config, project-level only) stays FanOut with the reason noted — ADR-0004's bar: a registry row must be true unconditionally. One consequence needs an explicit call: a flipped agent's directory stops being visited, stranding its existing fan-out links (the same shape as the disabled-agent gap). Decide the handling — one-time prune on the flip's first sync, doctor guidance, or documented manual cleanup — record it in DECISIONS.md, and test the chosen behavior.

**Blocked by:** 01 — Rename the Store to the Commons everywhere.

**Status:** ready-for-agent

- [ ] Every candidate agent verified against primary docs; evidence quoted in the registry comment, dated
- [ ] Unconditional user-level readers flipped to Native; conditional ones stay FanOut with the reason
- [ ] The stranded-links consequence has a recorded decision (DECISIONS.md) and the chosen behavior is tested
- [ ] `doctor`'s capability output reflects the new rows; suite green
