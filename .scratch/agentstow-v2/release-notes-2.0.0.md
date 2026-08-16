# agentstow 2.0.0

## Breaking

- **Private files move to XDG; `~/.agentstow/` is retired.** Config now lives in
  `$XDG_CONFIG_HOME/agentstow/` (default `~/.config/agentstow/`), the lock in
  `$XDG_STATE_HOME/agentstow/`. The old directory is no longer read — migrate with
  `mkdir -p ~/.config/agentstow && mv ~/.agentstow/agentstow.toml ~/.config/agentstow/ && rm -r ~/.agentstow`.
  `doctor` names a leftover `~/.agentstow/` and states the move.
- **The `subagents/` family is now `agents/`.** Rename the Commons directory;
  `doctor` warns while a non-empty `subagents/` remains. The `subagents` config
  key still parses as an alias, so existing `agentstow.toml` files keep working.
- **`status --json` renames the `store` key to `commons`.**

## New

- **The Store is now the Commons.** `~/.agents/` is a shared commons other tools
  also live in — agentstow is a tenant, never the landlord — and every command
  now says so.
- **`adopt` is one verb with three mechanics**, picked by where the path lives:
  a config in an agent's directory is absorbed into the Commons with a link left
  behind; a path inside a git repo becomes a Sourced entry — the Commons links
  out, the repo keeps the truth; anything else is copied in. `--dry-run` names
  the mechanic.
- **Two-phase sync.** Every family is surveyed before the first write; any
  Commons fault reports the complete list and writes nothing, so a config error
  can never leave a half-synced machine.
- **`revert <agent>`** removes everything agentstow put into one target — links,
  merged MCP and hook keys, rendered files. It refuses until the target is
  disabled, so a later sync cannot redo what revert undid.
- **`mcp enable | disable`** park a server without retyping it: the definition
  and its rules stay in the Commons, renders are removed everywhere, and enable
  restores them.
- **Codex, Cursor and Gemini CLI read `~/.agents/skills` natively** (verified
  against vendor docs), so they get no skill links — and where an agent still
  reads its old fan-out directory (Codex, Cursor), `sync` prunes agentstow's
  now-duplicate links from it automatically.
- **`doctor` shows Sourced entries** — each with its Source, missing clones
  marked, answering "what must I clone on this machine?" — and attributes
  `.agents` Protocol surfaces (`tasks/`, `memories/`, `models.json`,
  `system-prompt.md`) by name, touching none of them.

## Deliberately not in 2.0.0

No `remove <entry>` verb, no target enable/disable verb, no bulk path adopt, no
color, no verbosity levels, no ignore/defer/override patterns, no CLI flags for
the env vars, no `explain`, no undo or backups — each rejection recorded with
its grounds in DECISIONS.md (2026-08-16).
