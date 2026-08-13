# Decisions

Append-only. Calls the spec did not settle, made while implementing v1.

## 2026-08-13 — Conflict is a kind of Foreign, not a Variant

`CONTEXT.md` lists "conflict" under _Avoid_ for **Variant**, but the spec and
ticket 04 both call a Foreign-owned instructions file a conflict. Resolved by
defining **Conflict** in the glossary as its own term — a Foreign *file*
blocking a destination — rather than dropping either word. A Variant shadows a
Store entry on purpose; a Conflict blocks one. Both stay distinct from each
other and from plain Foreign links.

## 2026-08-13 — Conflicts are reported but not "actionable"

`status` exits 2 only for states `sync` can resolve. A Conflict needs a human
decision about another tool's content, so counting it would leave any machine
where claude-mem owns `~/.config/opencode/AGENTS.md` permanently red, training
users to ignore the exit code. Same reasoning as a diverged Variant.

## 2026-08-13 — Instructions mechanisms are per-agent; conflict is a runtime state

The spec's capability matrix lists opencode and Gemini instructions as
"conflict". That describes the state observed on the author's machine, not a
mechanism. The registry therefore gives both `Instructions::Symlink`, and a
Foreign file occupying the path is detected at runtime. Any agent can be in
conflict; none is inherently conflicted.

## 2026-08-13 — `init` implemented ahead of its ticket, scaffold only

Tickets 01–07 all print "run `agentstow init`" when the Store is missing, which
was false advice while ticket 13 was unimplemented. The scaffolding half of
`init` was pulled forward so the message is true. The guided first-run report
(detected agents, adoption candidates, conflicts) remains ticket 13, which is
still blocked by MCP adoption.

## 2026-08-13 — A third environment override, `AGENTSTOW_LOCK_TIMEOUT_MS`

The spec's seam names two overrides. Ticket 02 requires proving that a second
process fails cleanly on a held lock, which is untestable against a 30 second
default. The override is test-facing; the two documented overrides remain the
only ones that redirect paths.

## 2026-08-13 — Writability is asked of the OS, not inferred from mode bits

`doctor` uses `access(2)` with `W_OK` rather than `permissions().readonly()`,
which reports false for a directory owned by another user at 0755 — precisely
the case worth catching. Unix-only, which matches the v1 platform scope.

## 2026-08-13 — Registry rows still needing live verification

The spec flags rows sourced from agentsync's adapters rather than observation.
Verified during implementation, against the installed agents on the author's
machine or the agent's own source: opencode (`agents/`, `commands/`, native
skills, unknown-key preservation), oh-my-pi (native store scan), pi (no MCP by
design), Codex/Cursor/Roo/Windsurf fan-out directories.

Still unverified, and deliberately unused until their tickets: Codex hooks —
the registry says `.codex/hooks.json` (observed on the machine) while agentsync
targets inline `[hooks.*]` in `config.toml`. Both may be true, since trust
metadata lives in `config.toml` regardless. Pin this before ticket 11.

## 2026-08-13 — `sync` never deletes an MCP server

Without state, a name that has vanished from the Store is indistinguishable
from one the user added by hand — both are simply "in the config, not in the
Store". Deleting on that evidence would eventually delete someone's hand-added
server. So `sync` adds and restores but never removes; removal is the explicit
`mcp remove` (ticket 10). The cost is that a server dropped from the Store
lingers until asked for by name, and `status` calls it Foreign.

## 2026-08-13 — Ticket 08 renders only the standard JSON dialect

The registry declares Codex as TOML and four agents as non-standard dialects,
but only the standard JSON shape has a renderer until ticket 09. Rendering was
therefore gated on `Format::Json && McpDialect::Standard`. Without the gate,
`sync` wrote a JSON document into `~/.codex/config.toml` — a file that also
holds the user's model and sandbox settings. Found by adversarial review, not
by the tests, which only ever installed one MCP-capable agent.

## 2026-08-13 — An unparseable agent config is skipped, not fatal

A destination agentstow cannot parse is exactly the file it promised never to
touch. Aborting the whole survey meant one ordinary `~/.codex/config.toml`
denied service to every other agent and made `status` print nothing at all —
including under `--json`. Now the Target is skipped, the fault is reported, and
every healthy Target still syncs. The exit code is still non-zero, because the
user asked for work that did not happen.

## 2026-08-13 — Cross-file atomicity is not attempted

Each config file is replaced atomically, but a run touching several agents is
not a transaction: if the second write fails, the first stands. Everything is
resolved before any file is opened, so the remaining writes are still correct,
and each failure is reported individually. Rolling back a successful write
would mean restoring a file another program may already have changed.

## 2026-08-13 — The Store's `mcp.json` has exactly one shape

Accepting a bare map as well as the standard `{"mcpServers": {...}}` wrapper
read leniently but failed dangerously: a `$schema` key becomes a server named
`$schema`, and a typo in the wrapper silently syncs nothing. One documented
shape, and a clear error otherwise.

## 2026-08-13 — File modes are preserved, exposure is reported

A rendered file inherits the mode it already had; silently tightening a file
another program owns is its own surprise. But resolved secrets landing in a
group- or world-readable file is worth saying out loud, so that combination
warns. New files are created 0600.

## 2026-08-13 — No re-stat before rename

Claude Code rewrites `~/.claude.json` continuously, so a sync during a live
session can lose whichever side finishes second. Detecting this (re-stat
immediately before `rename`, bail on change) was considered and rejected for
v1: the spec chose a documented "don't sync mid-session" caveat, and a spurious
failure is worse than a rare race for a command users run deliberately.
