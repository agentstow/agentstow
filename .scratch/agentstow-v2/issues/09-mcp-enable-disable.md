# 09 — mcp enable | disable

**What to build:** A Managed server can be parked without retyping its definition (JSON has no comments). `mcp disable <name>` sets `"disabled": true` on the entry in the Commons `mcp.json` — the file remains the interface; the verb is sugar — and acts immediately under the lock, like `mcp remove`: the managed entry is removed from every targeted agent config, while the definition and its rules are kept. `mcp enable <name>` clears the key and restores everywhere immediately. A Disabled server renders nowhere, is shown by `mcp list` as `(disabled)`, and counts as clean — not drift — in `status`. Hand-editing the key has identical effect on the next sync. Reasoning: DECISIONS.md 2026-08-16 "Two verbs join the surface".

**Blocked by:** 01 — Rename the Store to the Commons everywhere.

**Status:** ready-for-agent

- [ ] `mcp disable <name>` removes the managed entry from every targeted agent config immediately and keeps the definition and rules in the Commons file
- [ ] `mcp enable <name>` restores the entry everywhere immediately
- [ ] `mcp list` shows `(disabled)`; `status` counts a Disabled server as clean
- [ ] `sync` skips Disabled servers in render; a hand-set `"disabled": true` behaves identically
- [ ] Unknown names refuse; both verbs run under the lock
