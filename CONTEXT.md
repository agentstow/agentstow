# agentstow

Syncs AI coding agent configs from one canonical store to every installed agent — symlink fan-out for identical-bytes configs (skills, instructions, commands, subagents), rendered key-merge for entries in shared files (MCP, hooks), rendered whole files where formats differ. GNU Stow's philosophy applied to agent configs.

## Language

**Store**:
The canonical directory `~/.agents/` holding the single real copy of everything agentstow syncs. A shared commons, not agentstow's private directory: the path is an interop contract other agents hardcode (ADR-0004), and other tools keep their own files in it. Distinct from `~/.agentstow/`, the private config dir — the Store is meant to be committed and synced across machines, and machine-local state must stay out of it.
_Avoid_: source, central repo, canonical source, agentstow's directory

**Target**:
One agent's config surface that receives fan-out (e.g. `~/.claude/skills/`).
_Avoid_: destination, agent dir

**Fan-out**:
The mechanism: one store entry materialized into every target as a symlink. Always **per entry**, never a single symlink standing in for the whole target directory (ADR-0005) — a target holds one link per store entry, so Variants have a slot and content the agent owns is left alone.
_Avoid_: distribution, propagation

**Native (agent)**:
An agent that reads the store directly and therefore needs no fan-out (e.g. opencode scans `~/.agents/skills/` itself). A capability the registry claims only when it holds unconditionally: an agent that *could* read the store after the user edits its config is not Native, because agentstow does not write that config.
_Avoid_: zero-config agent

**Co-tenant**:
An entry at the Store root that is not one of agentstow's families — another tool's file in a directory agentstow shares (e.g. the `skills` CLI's `.skill-lock.json`). Named by `doctor`, never counted as tools (filenames carry no authorship), never reported by `status`, never touched. Not a Foreign, which is about a *target*; not an issue.
_Avoid_: foreign, stray, unmanaged, junk

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
