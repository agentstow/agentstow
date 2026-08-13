# MCP: rendered copies with stateless key-merge and secret resolution

MCP servers are the one config agentstow cannot symlink: a secret-bearing key-subtree needing per-agent format translation (JSON `mcpServers` vs Codex TOML `[mcp_servers]` vs opencode's `mcp` with flattened command arrays vs Windsurf's `serverUrl` dialect), usually embedded in files the agent also owns. So MCP is **rendered**: the canonical store is a single standard-shape `~/.agents/mcp.json` (the de-facto `mcpServers` schema, kept pure so agents may one day read it natively), and `sync` copies each server into every targeted agent config via key-merge with secret resolution.

Ownership stays stateless per ADR-0001, by **name-identity**: a server name present in the store ⇒ agentstow owns that entry in every targeted config — sync overwrites exactly those entries (store always wins), preserves all other keys, and never touches foreign server names. Removal is imperative (`agentstow mcp remove <name>` edits store + all targets); a hand-deleted store entry leaves renderings that `status` can only report as foreign — the accepted price of having no state file.

## Consequences

- Secrets: store values may reference `${env:VAR}`, resolved from the environment at sync and redacted in all diff/status output. No vault; resolved cleartext lands in target files exactly as hand-configuration would put it there.
- agentstow-specific concerns never pollute the standard store file: per-server `agents` allowlists and per-agent tweak tables (e.g. `[mcp.node_repl.codex] startup_timeout_sec`) live in `~/.agentstow/agentstow.toml` and merge at render time.
- Writes are direct read-modify-write with atomic rename, touching only owned entries. Racing a live agent session on a shared file (notably `~/.claude.json`) can lose one side's write; documented as "don't sync mid-session" rather than bought off with agent-CLI delegation.
- Command surface is deliberately minimal: `mcp list`, `mcp adopt`, `mcp remove`. No `mcp add` — the store file is a standard, documented format; adding a server is editing `mcp.json`.
- pi is excluded (rejects MCP by design); oh-my-pi is native-via-discovery (it reads other agents' MCP configs, so writing to it would double-configure).
