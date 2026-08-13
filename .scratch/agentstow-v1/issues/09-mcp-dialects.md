# 09 — MCP dialects: Codex, opencode, Gemini, Cursor, Windsurf, Cline

**What to build:** The remaining MCP-capable targets receive the same Managed servers in their native dialects: Codex TOML `[mcp_servers.<name>]` tables (canonical `headers` → `http_headers`, http/sse collapsed to a URL server) merged without disturbing model settings or other tables; opencode's `mcp` key (`local`/`remote`, command+args flattened to one array, `env` → `environment`); Gemini's `url` (SSE) vs `httpUrl` (HTTP) distinction; Windsurf's `serverUrl` dialect; Cursor and Cline's standard-shape JSON files. pi is never written (no MCP by design); oh-my-pi is never written (native-via-discovery).

**Blocked by:** 08 — MCP tracer bullet: Claude.

**Status:** done

- [x] Codex: rendered TOML tables coexist with unrelated config content byte-preserved outside the merged keys; header rename and transport collapse verified
- [x] opencode: command-array flattening and `environment` rename verified; the file survives a simulated plugin read-modify-write round-trip fixture
- [x] Gemini: transport maps to `url` vs `httpUrl` correctly per server type
- [x] Windsurf: remote servers carry `serverUrl`
- [x] Cursor and Cline: standard `mcpServers` files created/merged
- [x] pi and oh-my-pi configs are provably untouched by an MCP sync over all fixtures
