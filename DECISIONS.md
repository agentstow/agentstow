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
