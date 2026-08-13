# 10 — `mcp list / adopt / remove` + targeting and Tweaks

**What to build:** The MCP management surface. `mcp list` shows Store servers and their per-target Managed/Foreign state. `[mcp.<name>] agents = [...]` allowlists in `agentstow.toml` scope a server to specific agents (default: all capable); per-agent Tweak tables merge native-only knobs (timeouts, enabled flags) into that agent's rendering alone. `mcp remove <name>` deletes a server from the Store and every target in one imperative action. `mcp adopt` reverse-translates an existing native entry into the Store — lossy per-agent fields become Tweaks — after which sync is a no-op.

**Blocked by:** 07 — `agentstow.toml` configuration; 09 — MCP dialects.

**Status:** ready-for-agent

- [ ] `mcp list` reports Store servers plus per-target state in the fixed vocabulary
- [ ] An allowlisted server reaches only its listed agents; unlisted capable agents get nothing
- [ ] A Tweak table's keys appear only in that one agent's rendering
- [ ] `mcp remove` leaves neither the Store entry nor any target rendering behind
- [ ] `mcp adopt` from a Claude (JSON) entry and a Codex (TOML) entry produces a Store server + Tweaks such that the next sync changes nothing
- [ ] Adoption reports any field it could not represent rather than dropping it silently
