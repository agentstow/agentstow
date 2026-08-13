# 11 — Hooks family

**What to build:** Command-hooks declared once in `~/.agents/hooks/<event>.toml` (matcher + command) reach Claude and Gemini (their settings-file hook sections) and Codex (its hooks file — pin the current native format at build time) under **element identity**: agentstow owns exactly the array elements whose command matches a Store hook, and Foreign elements in the same event array are preserved verbatim. Trust metadata (Codex's hash entries) is never written — the agent re-prompts the user to trust changed hooks. Hook scripts are not managed; Store commands must be agent-agnostic paths.

**Blocked by:** 08 — MCP tracer bullet: Claude.

**Status:** done

- [x] A Store hook renders into all three targets' native hook sections; other settings keys survive intact
- [x] A Foreign hook element in the same event array (e.g. another tool's SessionStart hook) survives every sync byte-for-byte
- [x] Codex trust/hash entries are provably never created or modified by any fixture run
- [x] A hook removed from the Store is not deleted from targets; `status` reports the leftover as Foreign (the accepted statelessness price)
- [x] Only command-type hooks are representable; anything else in the Store hook file is a clear config error
