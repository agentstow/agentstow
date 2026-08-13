# 07 — `agentstow.toml` configuration

**What to build:** The optional tool config at `~/.agentstow/agentstow.toml` lets a user disable a detected target (`[targets] <name> = false`) or define a custom target (root + per-family paths) that then participates in sync/status/doctor exactly like a built-in row. An absent or empty file means pure defaults; a malformed file is a clear error naming the problem.

**Blocked by:** 02 — Skills fan-out `sync`.

**Status:** ready-for-agent

- [ ] A disabled target disappears from `sync`, `status`, and `doctor` output entirely
- [ ] A custom target with a root and skills path receives fan-out and appears in reports like any registry agent
- [ ] Absent/empty config behaves identically to no config; malformed TOML → clear diagnostic on stderr, exit 1
- [ ] The Store itself never gains a config file — tool configuration lives only under `~/.agentstow/`
