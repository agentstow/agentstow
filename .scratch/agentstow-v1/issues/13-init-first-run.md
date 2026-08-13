# 13 — `init` + guided first run

**What to build:** The two-minute on-ramp. `agentstow init` scaffolds the Store skeleton (never agent roots) and prints a guided report of this machine: detected agents and their capabilities, adoption candidates across every family ("4 unmanaged MCP servers found — `agentstow mcp adopt --all`"), and conflicts with remediation hints. Running `sync` on an un-inited machine suggests `init` instead of failing cryptically. Re-running `init` is a safe no-op that reprints the report.

**Blocked by:** 04 — Instructions fan-out; 05 — `adopt`; 10 — `mcp list / adopt / remove`.

**Status:** ready-for-agent (scaffold half landed early — see DECISIONS.md)

- [x] `init` on an empty fixture creates the Store skeleton and nothing else
- [ ] The report on a populated fixture names detected agents, per-family adoption candidates, and conflicts with hints — all in the fixed vocabulary
- [x] `sync` without a Store suggests `init` and exits with an error, touching nothing
- [x] Second `init` changes no files and still prints the current report
