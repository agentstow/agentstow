# 08 — MCP tracer bullet: Claude (JSON key-merge)

**What to build:** The first Rendered family, end to end for one target. A server declared in `~/.agents/mcp.json` (standard `mcpServers` shape, kept pure) appears in Claude's user config under **name identity**: Store names are Managed (the Store wins on every sync), unknown names are Foreign and untouched, every other key in the shared file is preserved. `${env:VAR}` values resolve from the environment at sync and are redacted in all output. Writes are atomic (0600 temp, fsync, parent fsync) and resolve through a symlinked destination so dotfiles wiring survives. When sync changes an existing Managed entry, it prints a per-key redacted diff — visible, never blocking.

**Blocked by:** 03 — `status` for link families.

**Status:** ready-for-agent

- [ ] A Store server appears in the target's `mcpServers`; all unrelated keys in the file survive structurally intact
- [ ] Foreign server names are never modified and appear in `status`
- [ ] A hand-edited Managed entry is restored to the Store's render, with a per-key diff printed and secrets redacted
- [ ] `${env:VAR}` resolves at sync; an unset variable is a clear error naming the variable; resolved values never appear in any diff, status, or dry-run output
- [ ] A symlinked destination file keeps its symlink; the resolved file receives the atomic write
- [ ] A second sync is a no-op; `status` gains Managed/Foreign server reporting with the standard exit-code contract
