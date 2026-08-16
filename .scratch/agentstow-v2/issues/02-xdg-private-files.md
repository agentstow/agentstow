# 02 — Private files move to XDG; retire ~/.agentstow (hard cut)

**What to build:** agentstow's config is read from `$XDG_CONFIG_HOME/agentstow/agentstow.toml` (default `~/.config/agentstow/`) and its lock lives under `$XDG_STATE_HOME/agentstow/` (default `~/.local/state/agentstow/`), on every platform including Windows: an explicit `XDG_*` variable (absolute path) wins, otherwise the default derives from the overridable home. The legacy `~/.agentstow/` directory is ignored entirely — no fallback read, no auto-migration — and `doctor` names a stray one with the move instruction. The Commons itself does not move. Reasoning (including that this reverses ADR-0004's third settled question): DECISIONS.md 2026-08-16.

**Blocked by:** 01 — Rename the Store to the Commons everywhere.

**Status:** ready-for-agent

- [ ] `agentstow.toml` is read only from the XDG config path; explicit `XDG_CONFIG_HOME` and the derived default both work
- [ ] The lock is created only under the XDG state path; nothing is ever created under `~/.agentstow/`
- [ ] A present legacy `~/.agentstow/` is ignored by every command, and `doctor` names it and states the move
- [ ] Windows resolves the same way from the home chain; no new dependencies, no new env vars
- [ ] `doctor`'s header reports the new config location
- [ ] The test seam stays hermetic: derived XDG defaults follow the overridable home
