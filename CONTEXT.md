# agentstow

Syncs AI coding agent configs from one canonical store to every installed agent — symlink fan-out for identical-bytes configs (skills, instructions, commands, subagents), rendered key-merge for entries in shared files (MCP, hooks), rendered whole files where formats differ. GNU Stow's philosophy applied to agent configs.

## Language

**Store**:
The canonical directory `~/.agents/` holding the single real copy of everything agentstow syncs.
_Avoid_: source, central repo, canonical source

**Target**:
One agent's config surface that receives fan-out (e.g. `~/.claude/skills/`).
_Avoid_: destination, agent dir

**Fan-out**:
The mechanism: one store entry materialized into every target as a symlink.
_Avoid_: distribution, propagation

**Native (agent)**:
An agent that reads the store directly and therefore needs no fan-out (e.g. opencode scans `~/.agents/skills/` itself).
_Avoid_: zero-config agent

**Variant**:
A real directory (or file) in a target that intentionally shadows the store copy for that agent alone, while the store copy serves every other agent. First-class and preserved, never clobbered.
_Avoid_: override, fork, conflict, drift

**Adopt**:
Absorbing an existing real file/dir from a target into the store, leaving a link behind (GNU Stow's `--adopt`).
_Avoid_: import, migrate

**Foreign**:
A symlink, file, or MCP server entry in a target that agentstow does not own (doesn't resolve into the store / name not in the store). Never touched, listed by `status`.
_Avoid_: unmanaged, external, orphan

**Conflict**:
A Foreign *file* occupying a destination agentstow would otherwise write — reported with the remedy named, never overwritten. A kind of Foreign, and never a Variant (a Variant shadows a Store entry deliberately; a Conflict blocks one).
_Avoid_: clash, collision, blocked

**Render**:
Translating a store MCP server into one agent's native format and key-merging it into that agent's config file. The non-symlink sync path, used only where identical bytes are impossible.
_Avoid_: compile, generate, apply

**Managed (server)**:
An MCP server entry whose name appears in the store — agentstow owns that entry in every targeted config, and the store wins on sync.
_Avoid_: owned, tracked

**Tweak**:
A per-agent addition merged into one agent's rendering of a server (e.g. Codex's `startup_timeout_sec`), declared in `agentstow.toml`, never in the store file.
_Avoid_: override, extra, extension

**Import-line**:
The instructions mechanism for agents whose file must stay user-owned: ensure one additive, idempotent import line exists (Claude's `@~/.agents/AGENTS.md` in `CLAUDE.md`). The sole sanctioned edit to a user file outside MCP rendering.

**Rules-dir link**:
The instructions mechanism for agents that glob a rules directory: drop a symlink to the store `AGENTS.md` into it (Roo's `~/.roo/rules/`).

**Marker**:
The one-line comment agentstow places in every rendered whole file, making the file recognizable as agentstow's without a state file. Marked = ours; unmarked = foreign.
_Avoid_: banner, watermark
