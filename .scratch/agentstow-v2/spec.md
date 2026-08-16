# agentstow v2 — spec

Status: settled 2026-08-15/16 across eight grilling sessions; not yet implemented.

This is the record of what v2 is. The design was interrogated question by question
(every call below was an explicit user decision); the reasoning lives in
`DECISIONS.md` (2026-08-16 entries) and `docs/adr/0006`–`0007`. Vocabulary is
`CONTEXT.md`'s — Commons, Sourced, Revert, Protocol surface and the rest are used
here without re-definition. v1's spec is `.scratch/agentstow-v1/spec.md`; v2 changes
it in the ways below and nothing else.

## Version

v2 ships as **2.0.0**. The breaking change is the retirement of `~/.agentstow/`
(config silently ignored at the old location is a behavior change, not an addition).
Everything else is additive or textual.

## The Commons rename

The term **Store** becomes **Commons** everywhere agentstow speaks or is spoken to:
glossary (done — `CONTEXT.md`), every user-facing message (~38 lines), code
identifiers (`Store` struct → `Commons`, `store.rs` → `commons.rs`, `store::` →
`commons::`), README, CLI help. The five existing ADRs keep their historical wording
and carry a one-line amendment note. Evidence and reasoning: DECISIONS.md
2026-08-16. The README's first mention glosses it: *"the Commons — the canonical
`~/.agents/` directory"*. The output-vocabulary test (below) enforces the rename:
"store" joins the avoid-list.

## Private files move to XDG (reverses ADR-0004's third settled question)

- Config: `$XDG_CONFIG_HOME/agentstow/agentstow.toml` (default `~/.config/agentstow/`).
- Lock: `$XDG_STATE_HOME/agentstow/` (default `~/.local/state/agentstow/`). The lock
  is a runtime file by XDG purism, but `XDG_RUNTIME_DIR` is routinely unset on macOS;
  the state dir is the deliberate simplification.
- Resolution, all platforms including Windows: explicit `XDG_*` var if set (absolute),
  else derived from the overridable home (`HOME` → `USERPROFILE` → `AGENTSTOW_TARGET_ROOT`
  precedence unchanged). No new dependencies, no new env vars.
- **Hard cut**: the legacy `~/.agentstow/` is ignored entirely. `doctor` names a stray
  `~/.agentstow/` and states the move. No fallback read, no auto-migration.
- The Commons itself never moves — ADR-0004's "no XDG for the commons" reasoning stands.

## `.agents` Protocol alignment (adopted with two reservations)

- **`subagents/` → `agents/`**: Commons dir and `Family` name renamed.
  `Family::from_name` accepts `subagents` as a config-key alias; `doctor` hints when
  a non-empty `subagents/` dir exists (stateless migration). The duplicate dir-name
  constants in `store.rs` collapse onto `Family::name()`.
- **Casing reservation**: `AGENTS.md` and `SKILL.md` stay uppercase — the conventions
  with shipping consumers win over the draft protocol's lowercase.
- **Secrets reservation**: `models.json` (provider keys) is never managed; keys do
  not belong in a committed commons (ADR-0002 posture).
- **Protocol surfaces**: `doctor` recognizes `tasks/`, `memories/`, `models.json`,
  `system-prompt.md` by name as protocol surfaces — attributed, never touched, never
  counted as issues. App-specific artifacts remain ordinary Co-tenants. A surface
  becomes a family only when a real consumer exists to fan out to.
- `mcp.json` already conforms (same `mcpServers` schema). Nothing else changes.

## adopt: one verb, three mechanics (ADR-0006)

`agentstow adopt <path>` — one positional path; `--dry-run` is the only flag
(read-only, lockless, same classification, actions printed in "would" tense).
No content validation, ever. Dispatch on the existing, lexically-normalized path,
in order:

0. **Commons guard**: a path inside the Commons refuses — "already in the Commons —
   nothing to adopt." Ordered before the git walk-up because a committed Commons has
   a `.git` above every entry and would otherwise self-link (a test pins exactly the
   committed-Commons scenario).
1. **Absorb** — parent is a Target fan-out dir (registry or custom) or a per-agent
   instructions path: move the real file/dir into the Commons, leave a relative
   symlink behind. A symlink there refuses ("already a symlink — there is nothing to
   adopt"). Custom targets keep surface precedence even inside a git repo — an
   explicit declaration means what it says; the custom-target docs carry the warning.
2. **Source** — a `.git` (dir or file — worktrees, submodules) found walking up:
   create an **absolute** Commons symlink to the path *as given* (symlink inputs
   legal, never resolved through). The result is a Sourced entry.
3. **Copy** — everything else: copy in; the original is untouched, nothing points
   back, and the message says the original is no longer consulted and will diverge.
   A symlink input is copied through. After adoption, absorbed and copied entries are
   indistinguishable, ordinary Commons entries; only Sourced entries are recognizable
   on disk.

External paths (2, 3) take their family from the parent dir basename —
`skills`/`commands`/`agents`; any other parent refuses naming the accepted ones.

**One collision matrix, all mechanics**, compared by resolved targets and contents:
Commons copy identical → absorb relinks / source replaces the copy with the link /
copy no-ops; divergent → refuse ("this is a Variant — merge it by hand"), no
`--force`; existing link resolving to the same place → clean no-op, exit 0; link
elsewhere → refuse ("remove the Commons link first if you mean to repoint it").

**Every mechanic ends with per-entry fan-out to every installed agent** through
sync's own code path (Variants kept, Native agents skipped), so `sync` immediately
afterward is a true no-op. All refusals precede the lock.

Message shapes (final texts in ADR-0006 / the implementation):
`adopted skills/xyz from claude — moved into the Commons, link left behind`;
`adopted skills/xyz — Sourced from /abs/path`; `adopted skills/xyz — copied into the
Commons` + `the original at <path> is no longer consulted; edits there will diverge
from the Commons copy`; plus a `fanned out: …` line.

**Sourced entry lifecycle**: dangling Source keeps v1 behavior — stderr warning,
entry skipped from the scan, next sync prunes the agent links; sync never creates
links for a skipped entry. `doctor` gains a `Sourced entries:` section
(`name ← source`, `(source missing)` markers) and the `(N sourced)` count suffix;
`status` stays quiet when healthy.

## sync: two-phase (ADR-0007)

Phase 1 surveys every family read-only, collects **every** problem (unparseable
Commons file, unset `${env:VAR}`, conflicts), reports them all, and on any problem
exits 1 having written nothing — "no changes were made" becomes unconditionally
true. Phase 2 executes the plan; write failures report and continue (idempotent
re-run heals). `--dry-run` is phase 1 plus the printed plan — one code path. The
plan keeps per-file grouping (Gemini's shared settings file gets one coherent
write). Family ordering constraints (hooks after MCP) are preserved inside the plan.

## revert <agent>

Removes everything agentstow put into one target — links, merged MCP and hook keys,
Markers. **Refuses unless the target is already disabled**, printing the exact
`targets.<name> = false` line to add, so a later sync cannot silently redo what
revert undoes. Works on disabled-and-still-detected agents (the gap it exists to
close: disabling orphans fanned-out links forever with no state remembering them).

## mcp enable | disable

`"disabled": true` on the server entry in the Commons `mcp.json` — the file remains
the interface; the verbs are sugar that edit the key. Disable acts immediately under
the lock like `mcp remove` (removes the managed entry from every agent config, keeps
the definition and its rules); enable restores immediately. A disabled server is
skipped by render, shown by `mcp list` as `(disabled)`, and counts as clean.

## Registry corrections

Verify from primary docs, per agent, whether user-level `~/.agents/skills` is read
natively and unconditionally (candidates from the ecosystem research: Codex, Cursor,
Cline, Amp, Gemini CLI, Copilot; opencode already Native). Flip `Skills::FanOut` →
`Skills::Native` where true — fan-out into an agent that also reads the Commons
natively produces duplicate skills. Anything conditional stays FanOut (ADR-0004:
a registry row must be true unconditionally).

## Output-vocabulary test

A test scans user-facing output strings against `CONTEXT.md`'s avoid-lists (and the
retired term "store"). Messages cannot drift from the language the docs promise.

## Docs

README: Commons gloss at first mention; `agents/` in the tree; interop paragraph
(the conventions the Commons speaks: `.agents/skills/<name>/SKILL.md`, `AGENTS.md`,
`mcpServers`-shape `mcp.json`; protocol surfaces recognized); env-var section
promoted to documented interface (`AGENTSTOW_TARGET_ROOT`, `AGENTSTOW_HOME`,
`AGENTSTOW_LOCK_TIMEOUT_MS`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`); "gate CI with
`status`, not `sync --dry-run`" line; the no-undo philosophy sentence (refusals
before writes, two-phase abort-clean, dry-run everywhere, idempotent re-runs, a
Commons that lives in git). GNU Stow's `--adopt`-then-`git diff` review framing goes
in the adopt docs.

## Deliberately not done (recorded in DECISIONS.md)

No `remove <entry>` verb (`rm` + `sync` is the interface) · no target
enable/disable verb (the config file is the interface; revert's refusal prints the
line) · no bulk/`--all` path adopt (init's report + a shell loop; refusal semantics
stay atomic) · no color, ever (plain text is the interface; trivially NO_COLOR
compliant) · no verbosity levels · no `--ignore/--defer/--override` pattern flags
(Variants, disabling, and Co-tenancy are the structural answers) · no CLI flags for
the env vars · `sync --dry-run` keeps exit 0 (status is the drift gate) · no
`explain` command (doctor + status + the printed plan) · no LSP family (no consumer
in the registry) · no undo/backups (state — ADR-0001; git is the history) · project
scope (`./.agents/`) is a reasoned non-goal: agents already read project-level
`.agents/skills` natively, so a project agentstow would manage a directory its
consumers handle themselves · full `.agents` Protocol adoption declined (one-app
draft; casing conflicts with the installed base; provider keys in a committed
commons).

## Implementation order

1. Commons rename (glossary→code→messages→docs; amendment notes on ADR-0001..0005).
2. XDG move, hard cut, doctor naming the stray dir.
3. `subagents/` → `agents/` (+ config alias, doctor hint, constants collapse).
4. Two-phase sync + ADR-0007.
5. adopt redesign + ADR-0006 + DECISIONS entries.
6. `revert`.
7. `mcp enable|disable`.
8. doctor additions (Sourced section, protocol surfaces) + registry verification/flips.
9. Output-vocabulary test + README/docs sweep.

Gate: green `cargo test`, `cargo check --target x86_64-pc-windows-msvc`,
`verify-packaging` — before any commit.

## Testing

Same single seam as v1 (CLI against a throwaway tree via `AGENTSTOW_TARGET_ROOT`,
now plus `XDG_*` derivation). New coverage: the committed-Commons self-link guard;
each adopt mechanic × the collision matrix; dry-run parity with the executed plan
(two-phase makes this literal); revert refusal and teardown; disabled-server render
skip; `subagents` alias; XDG resolution and the doctor hints; vocabulary scan.
