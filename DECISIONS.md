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

## 2026-08-13 — Which dialect translations are lossy, and why that is accepted

Rendering is one-way for now, but the losses matter for the eventual `mcp adopt`
of a rendered entry:

- **Codex** has one remote transport, so `http` and `sse` render identically.
  Reading one back cannot tell which it was.
- **OpenCode** has `local`/`remote` only, the same collapse, and flattens
  `command` + `args` into a single array, so the boundary between the executable
  and its first argument is gone.
- **Windsurf** emits `serverUrl` with no type key — again, transport identity is
  not recoverable.
- **Gemini** is the exception: `url` means SSE and `httpUrl` means streamable
  HTTP, so its transport round-trips.

Nothing is dropped that the canonical entry declared; only the *distinction*
between two transports is, and only where the agent itself has no way to express
it. Keys agentstow does not model pass through verbatim to every dialect.

## 2026-08-13 — TOML parse failures are named by position, never quoted

`toml_edit::TomlError`'s `Display` includes the offending source line. These
files hold resolved secrets, so formatting one leaked a credential to stderr in
`status`, `status --json`, `sync` and `sync --dry-run` alike. Errors now carry
line and column derived from the error's span and nothing else. Found by
adversarial review; the same discipline the JSON path already followed.

## 2026-08-13 — Codex renders drop null-valued keys

TOML has no null, so the merge cannot write one. Leaving nulls in the rendered
entry meant the written file never matched what was rendered, so Codex reported
Drifted on every run and `status` never returned 0. Nulls are now stripped from
the Codex render itself, which keeps the comparison symmetric with what the file
can hold. The five JSON dialects still pass nulls through harmlessly.

## 2026-08-13 — An integer TOML cannot hold is refused, not approximated

TOML integers are signed 64-bit. A Store value above `i64::MAX` was being
written as a float, which both corrupted it and left the entry permanently
drifted. It is now an error naming the key, and nothing is written.

## 2026-08-13 — The user's TOML formatting is theirs

Three things the merge preserves that it did not at first: a comment written
above a Managed server (it lives in the table's decor, not the key's), a bare
`[mcp_servers]` header the user typed by hand, and the file's own line endings
and byte-order mark. None of these change what the file means, which is exactly
why silently rewriting them would be an unrequested edit.

## 2026-08-13 — The tool config schema for MCP: `agents` and `tweaks`, nothing else

`[mcp.<name>] agents = [...]` scopes a server; `[mcp.<name>.tweaks.<agent>]`
holds native knobs for one agent. The reserved `tweaks` segment avoids the
ambiguity in a flatter `[mcp.<name>.<agent>]`, where adding an agent to the
registry would silently change what an unedited config file means. Inside
`[mcp.<name>]` only those two keys are accepted; anything else is an error, so
a typo is loud rather than ignored forever.

## 2026-08-13 — Adoption records what is, rather than deciding to spread it

`mcp adopt` writes an allowlist naming only the agents it actually absorbed
from, so the following `sync` is a genuine no-op. Adopting a server that lives
in one agent does not silently push it to the other five; the message says how
to widen it. The allowlist is built from what was absorbed, never from what was
asked for — naming an agent whose entry was refused would let the next sync
overwrite that agent's server with another agent's version.

## 2026-08-13 — An adoption that would not survive a sync is refused

`mcp adopt` renders its own result back through the dialect and compares it
against the entry it read. Anything that differs means the next sync would
change that file, so the adoption is refused rather than reported as success.
The comparison deliberately skips `${env:VAR}` resolution: a literal reference
in an agent's own config must compare equal to itself, or every such server
would read as unfaithful.

## 2026-08-13 — Ambiguous transports are recorded by omission

Codex, OpenCode and Windsurf cannot express the difference between http and
sse, so adopting a remote server from them records no `type` at all. Omission
re-renders identically, which is what makes the no-op guarantee hold; a guess
would not. Gemini encodes the transport in its URL key, so adoption from it is
exact. Every omission is reported.

## 2026-08-13 — Adoption warns when it copies something that looks like a secret

Values are taken verbatim out of an agent's config, so a credential typed there
by hand lands in the Store — the file this design encourages committing. When a
key under `env` or `headers` looks like a credential and holds a literal value,
adoption says so by key name, never by value, and suggests `${env:VAR}`.

## 2026-08-13 — `mcp remove` is the only deletion, and it is thorough

It clears the server from every Target, from the tool config (allowlist and
Tweaks alike) and from the Store, in that order, so no intermediate state is
incoherent. It refuses a name the Store does not have: a server agentstow never
managed is Foreign, and deleting Foreign entries is exactly what the rest of the
tool promises never to do. JSON removal uses `shift_remove`, because with
`preserve_order` a plain `remove` swaps the last entry into the hole and
scrambles the user's file.

## 2026-08-13 — Codex hook rows verified against the live agent

The earlier entry flagged these as unverified. Confirmed on this machine:
hook *definitions* live in `~/.codex/hooks.json` (JSON, top-level `hooks` key,
the same nested shape Claude uses), and `[features] hooks = true` enables them.
The registry rows were already correct. Claude's own hooks live in
`~/.claude/settings.json` and Gemini's in `~/.gemini/settings.json`, both under
a `hooks` key with the same shape.

## 2026-08-13 — Trust safety is structural, not behavioural

Codex records a `trusted_hash` per hook in `~/.codex/config.toml` — a
*different* file from the `hooks.json` that holds the definitions. agentstow
writes only the latter, so "never write trust metadata" holds by construction
rather than by care. A test asserts `config.toml` is byte-identical after a
sync, and the same was confirmed against the real file.

The hash formula itself was not reverse-engineered: eight candidate
serialisations (the hook object, the group, the whole file, the command, the
referenced script) all failed to reproduce it. That does not weaken the
guarantee, but it does mean agentstow cannot *predict* whether Codex will
re-prompt — only that it never grants trust itself.

## 2026-08-13 — Claude and Codex share a hook vocabulary; Gemini does not

Codex's snake_case trust keys (`pre_tool_use`, `user_prompt_submit`) are
lowercased forms of the PascalCase names in `hooks.json`, which match Claude's.
So the canonical event name is Claude's, matching the choice already made for
commands and subagents. Gemini uses its own words for some of the same moments
(`BeforeTool`, `AfterTool`, `AfterAgent`, `PreCompress`) and has no equivalent
for others; an event an agent does not have is skipped and reported as
`unsupported`, never invented.

## 2026-08-13 — The hooks family is "in use" when its directory exists

An empty `~/.agents/hooks/` still means the user adopted the family, so
leftovers are reported. No directory at all means the family is unused, and no
agent's own hooks are listed. Without that distinction, either a hook deleted
from the Store would go unreported (the ticket requires reporting it), or every
user who never touched hooks would see every tool's hooks listed as Foreign.

## 2026-08-13 — A Foreign hook is named by its program, not its command line

Reports name our own hooks by their Store command, unresolved. A Foreign hook
is named by its program only: `curl …` rather than the full line. Another
tool's command line can carry a credential, and agentstow echoing it into a
terminal, a CI log or `--json` would be exposure the user never asked for —
found by probing a hook with an `Authorization:` header on it.

## 2026-08-13 — Hook commands are never expanded, because the command is the identity

`${env:VAR}` is written through verbatim rather than resolved. Element identity
matches hooks by command string, so a resolved command stops matching the
reference that produced it: the hook was reported `missing` and its own
rendering `foreign`, on every run, with `status` stuck at exit 2 and `sync`
claiming a change forever. Verified by reproduction, then by the fix converging
across three different values of the same variable.

The consequence is better than the bug it replaces: a hook that needs a secret
reads it from the environment when it runs, so the value never lands in a config
file at all. This is the opposite of the MCP rule, and deliberately so — there
the value is data an agent consumes, here it is a name agentstow matches on.

## 2026-08-13 — Gemini hook timeout units are unresolved

Claude and Codex hook timeouts are seconds. Gemini's may be milliseconds: on
this machine claude-mem writes `10` to Claude and `10000` to Gemini for what is
plainly the same ten seconds, which only makes sense as a unit conversion. But
plannotator writes `345600` to both Codex and Gemini, which is four days as
seconds and under six minutes as milliseconds — so the evidence conflicts, and
gemini-cli is not installed here to settle it.

agentstow therefore writes the timeout **verbatim** and does not guess. Applying
a 1000x multiplier on this evidence risks a hook that is killed instantly; not
applying one risks a hook that hangs. Passing through what the user wrote is the
only choice that is wrong in a way they can see and correct. Resolve this by
reading gemini-cli's source when it is available.

## 2026-08-13 — Two Gemini event mappings left deliberately unmapped

A design review proposed mapping `UserPromptSubmit` to Gemini's `BeforeAgent`.
Gemini's own config uses `BeforeAgent` for claude-mem's *session-init*, which
reads more like session start than prompt submission, so the mapping is not
clearly right. An unsupported event is reported out loud; a wrong mapping is
silent. The conservative choice stands until Gemini's event semantics can be
confirmed. The same applies to the claim that Codex has no `Notification` event.

## 2026-08-13 — The matcher is part of what "managed" means for a hook

Drift was decided by comparing the rendered leaf — type, command, timeout —
but a hook's matcher lives on the enclosing *group*. So changing a Store hook's
matcher from `Bash` to `Write` reported "Everything is up to date" and left the
hook firing on `Bash`: a narrowing safety constraint the user had deliberately
changed, silently ignored. The comparison now includes the group's matcher
(absent and empty treated alike), and applying a change moves the hook into a
group with the right matcher rather than editing in place.

Moving removes only leaves whose command matches ours, and drops a group only
when emptying it left it with nothing — so a foreign hook sharing our old group
stays exactly where its owner put it.

## 2026-08-13 — `init` deliberately does not create `hooks/`

The other families' directories are created by `init` because an empty one is
inert. An empty `hooks/` is not: the family treats directory presence as "the
user adopted this", which is what makes a hook deleted from the Store report as
a leftover. Creating it for everyone would show every user every other tool's
hooks as Foreign. `doctor` reports the family as `absent` instead, and
`hooks/<Event>.toml` is created by the user when they want it.

`doctor` now also warns about non-`.toml` files in `hooks/`, which the family
would otherwise skip in silence — the exact failure the Store hygiene scan was
introduced to end.

## 2026-08-13 — The Marker is what makes pruning safe

Rendered files are the only family agentstow deletes without being asked by
name. That is sound precisely because the Marker proves authorship: a file
carrying it was generated here, so removing it when its Store entry goes is
returning the machine to a state agentstow created. A file without one was
written by somebody else and is never touched — verified against the two real
`plannotator-*.toml` commands on this machine, which survive a sync untouched.

This is not in tension with the MCP rule against markers (ADR-0003). There a
marker would have had to live inside data the agent parses as configuration;
here it is a comment in a file agentstow generates wholesale, invisible to the
agent reading it.

## 2026-08-13 — Rendered prompts use escaped strings, not triple quotes

Gemini's own command files use `"""..."""` for multi-line prompts. agentstow
emits an escaped basic string instead, which is valid TOML carrying identical
content — verified by round-tripping a body containing `"""`, backslashes and
non-ASCII. Matching Gemini's cosmetic style would mean hand-rolling the escaping
of triple quotes inside triple quotes, which is exactly the kind of thing that
produces an unparseable file for one unlucky user. The Marker already tells a
reader to edit the Store rather than the file.
