# 04 — Instructions fan-out: three mechanisms + conflicts

**What to build:** One `~/.agents/AGENTS.md` reaches every agent through its registry mechanism: **symlink** where the target file is absent or already ours (Codex, pi, oh-my-pi, Windsurf's global rules file), **import-line** for Claude (ensure the `@~/.agents/AGENTS.md` line exists in `CLAUDE.md`, additive and idempotent, touching nothing else), **rules-dir link** for Roo (drop an `AGENTS.md` symlink into its rules directory). Where a Foreign tool owns the file (claude-mem's opencode/Gemini boilerplate), nothing is written; `status` reports the conflict with a one-line remediation hint.

**Blocked by:** 02 — Skills fan-out `sync`; 03 — `status` for link families.

**Status:** ready-for-agent

- [ ] Absent instructions file → symlink created per registry mechanism, per target
- [ ] `CLAUDE.md` without the import line → line added, rest of the file byte-identical; with the line → no-op
- [ ] Roo's rules directory receives the `AGENTS.md` link
- [ ] Foreign-content instructions file → no write occurs; `status` shows a conflict with a remediation hint naming the occupying tool where known
- [ ] Verify-at-build rows (oh-my-pi's probe path, Windsurf's global rules file) confirmed against the installed agent or corrected in the registry
