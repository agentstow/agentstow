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

## 2026-08-13 — The first-run report answers "what of mine could this take over"

On an empty machine the interesting question is what agentstow does; on a used
one it is what of the user's existing config the Store could hold. So `init`
reports agents found, then per-family adoption candidates with the command that
would take them, then conflicts with their remedy.

A candidate is a *real* object in a Target directory whose name the Store does
not already hold. A symlink is never a candidate: it is either already ours or
Foreign, and `adopt` refuses both. On this machine that correctly surfaces the
three hand-made `plannotator-*` skill directories out of the 160 entries in
`~/.claude/skills`, the rest of which are links.

Long lists are capped at eight entries with a count of the remainder — a first
run on a populated machine should be readable, and the full picture is what
`status` is for.

## 2026-08-13 — Packaging is verified by a script, not at the CLI seam

Every other family is tested by driving `agentstow::run` against a fixture tree.
Packaging cannot be: the thing under test is what npm and cargo do with the
built artefact, which lives outside the process. `scripts/verify-packaging.sh`
is therefore its test — it builds, assembles, packs, installs offline from local
tarballs, runs the installed launcher, checks exit-code propagation and the
no-platform-package message, and dry-runs every publish.

One packaging fact *is* testable at the seam and now is: `--version` must report
the crate version, because `build-npm.sh` reads that same version out of
`Cargo.toml` and stamps it onto all five npm packages.

## 2026-08-13 — A dry-run publish does not prove the scope is claimable

`npm publish --dry-run` on `@agentstow/darwin-arm64` reports `ok` today, even
though `npm view @agentstow/darwin-arm64` returns 404 and `npm org ls agentstow`
returns 403. The dry run packs and validates locally; it never asks whether the
scope exists or whether you may publish into it. Treating a green dry run as
release-readiness would fail at the worst moment, so the runbook says outright
which checks actually settle it.

The verification script classifies registry outcomes rather than conflating
them: "version already published" and "scope not claimed" are expected states
before the first release and are reported with a pointer to the runbook, while
any other failure is still a hard error.

## 2026-08-13 — The published version is left at 0.0.1 deliberately

`agentstow@0.0.1` is the name-claim placeholder and cannot be republished, so a
real release must bump it. Which number that is — 0.0.2, 0.1.0, 1.0.0 — says
something about the project's maturity that is Frank's to say, not a detail to
settle in passing. `build-npm.sh` reads the version from `Cargo.toml` and stamps
it everywhere, so bumping one line is the whole change.

## 2026-08-13 — Platform packages are published before the launcher

The launcher declares the platform packages as optional dependencies. Publishing
it first leaves a window in which `npm install agentstow` resolves the launcher,
finds no platform package, and installs a command that cannot run. The runbook
fixes the order and says why.

## 2026-08-13 — Rust edition 2024, MSRV 1.97, version 1.0.0

Frank claimed the `@agentstow` npm organisation and set the version to `1.0.0`,
which unblocked the last of ticket 14. The crate then moved to edition 2024 with
`rust-version = "1.97"`, the current stable at the time.

The MSRV is deliberately the latest rather than the oldest that compiles. Almost
everyone gets agentstow as a prebuilt binary through npm, where the toolchain is
irrelevant; only `cargo install` users are affected, and they are the audience
most likely to be current. The release workflow now installs and selects stable
explicitly, because a runner's preinstalled toolchain could otherwise be older
than the declared edition and fail with a confusing message.

Edition 2024 turned three nested `if let` blocks into clippy warnings, which
let-chains — stabilised in the same edition — resolve directly. The suite is
unchanged at 238 tests and clippy is clean again.

## 2026-08-13 — `npm org ls` returning 403 is not evidence

After the organisation was created, `npm org ls agentstow` still returned 403
from this machine. The publishing token can publish but is not authorised to
read organisation membership, so a 403 there says nothing either way. The
runbook records this so a future release is not held up by a check that cannot
answer the question — the publish itself is what settles it.

## 2026-08-13 — Both macOS targets build on Apple silicon runners

The first tagged run exposed a real hole: `build darwin-x64` asked for
`macos-13`, an image GitHub has retired, so the job sat queued with no runner
assigned while the other four finished in under a minute. It would have hung
until timeout, and the packaging job — which needs all four binaries — would
never have started. A workflow that has never run is not a verified workflow,
which is the lesson worth keeping.

Both macOS targets now build on `macos-latest`. rustc cross-compiles
`x86_64-apple-darwin` from Apple silicon using the SDK already on the runner,
so nothing else is needed, and a single label cannot be retired out from under
one architecture while leaving the other working.

## 2026-08-13 — The deploy token is scoped to one zone, not all three

`CF_DEPLOY_TOKEN` carries Workers Scripts: Edit on the account, plus Workers
Routes: Edit and Zone: Read on `agentstow.dev` alone. It cannot see or change
`agentstow.com` and `agentstow.org`, which is a deliberate narrowing rather than
an oversight: those two zones hold only the canonical-host redirect rulesets,
those rules are static, and nothing in the deploy path rewrites them. Granting
CI the ruleset scope on all three zones would widen a secret that any workflow
on the repo can read, in exchange for an ability the workflow never exercises.

The `max-permissions-cli` token in `~/.zshenv` was not reused for the same
reason — full account access should not be reachable from CI.

The scopes are confirmed sufficient by run `31746869454` deploying green, not by
reading the table. An under-scoped token would still pass the workflow's
`refuse to deploy without a token` guard, which only checks the secret is
non-empty; it would fail later inside `wrangler deploy`. The guard proves
presence, the run proves permission.

## 2026-08-13 — The Store is a commons agentstow rents, not one it owns

`~/.agents/` was chosen as a neutral name; it has since become an interop
contract. opencode and oh-my-pi scan it natively, Hermes Agent ships
`~/.agents/skills` as the documented example for its `skills.external_dirs`
setting, and the `skills` CLI keeps `.skill-lock.json` in the root. Renaming it
to `~/.agentstow/`, or moving it under XDG, would break tools that hardcode it —
so the path is now fixed by other people's source, not by preference. ADR-0004
records this along with the split from `~/.agentstow/`, whose real justification
is the git boundary (the Store is committed and synced; the lock file and
per-machine tweaks must not be), not the "purity" argument ADR-0002 gave.

The consequence is that agentstow must admit tenancy. `doctor` now names
Store-root entries that are not its own families, and deliberately names
*entries* rather than counting *tools*: filenames carry no authorship, so one
neighbour may own three files and agentstow cannot tell. `status` stays silent —
a co-tenant has no fan-out relationship to report — and `init` was left alone.

## 2026-08-13 — Hermes does follow symlinks; the premise was three months stale

The plan to give Hermes real copies rested on the belief that it cannot follow
symlinked skills. That was true and is documented — NousResearch/hermes-agent
issue #8293, where `rglob("SKILL.md")` and `os.walk()` skip directory symlinks
on Python 3.11+ — but it was fixed on main in April 2026, and the installed
v0.20.0 walks with `followlinks=True`. The 140 symlinks already in
`~/.hermes/skills`, the oldest from May, had been working the whole time.

So no `Skills::Copy` mechanism was built. That matters beyond Hermes: a copy
family would have needed marker identity (ADR-0003) applied to a *directory*,
and the marker is a first-line comment in a file — there is nowhere to put one
in a skill directory that the agent will not also read as content. ADR-0001's
"drift is impossible by construction" survives intact.

Hermes is registered as `Skills::FanOut` rather than `Skills::Native` even
though it *can* read the Store directly, because that requires the user to set
`skills.external_dirs` and agentstow does not write that file. A registry row
states what is true unconditionally; a `Native` claim would be false on an
unconfigured machine, and `doctor` would report a capability the agent does not
have.
## 2026-08-13 — claude-mem's instruction files were moved aside, not merged

The first real `sync` on the author's machine hit both conflict paths at once:
`~/.config/opencode/AGENTS.md` and `~/.gemini/GEMINI.md` carried the
`<claude-mem-context>` marker, so agentstow refused them and asked for a move
or a merge. Merge was rejected because the content was not instructions — both
files held only claude-mem's first-run placeholder ("no memory yet"), never
updated since June and July despite daily use, and folding a per-agent memory
block into the Store's `AGENTS.md` would fan it out to every agent. The files
were renamed to `*.claude-mem.bak` in place and sync created the symlinks.

The residual risk is recorded rather than solved: claude-mem's opencode and
Antigravity installers write these paths with a plain `writeFileSync`, which
follows symlinks. Both writers run only on explicit install commands and are
guarded by the marker check, but a re-install would append the placeholder
*through* the link into `~/.agents/AGENTS.md` — and the Store is not currently
a git repo, so nothing would flag it. Two follow-ups fall out: the Store ought
to be under git (its own pitch says "committable"), and a conflict message that
names the owning tool could also warn that the owner may write through the
future symlink.

## 2026-08-13 — variant-identical counts against sync, but sync will not fix it

After the full sync, `status` still exited 2 for exactly one item: openclaw's
`agent-reach`, a real directory byte-identical to the Store — a stray copy from
a direct install, not a variant. `sync` deliberately leaves variant-identical
entries alone (conservatism is right: collapsing a directory is a delete), yet
`status` counts the item and its footer says "run `agentstow sync`", which is
advice that cannot succeed. The copy was verified identical with `diff -r`,
removed by hand, and re-linked by sync; exit code went to 0. The UX gap stands
as product feedback: either variant-identical should not count toward the exit
code, or the footer should name the manual collapse instead of pointing at
sync.

## 2026-08-14 — The Chinese pages say zh-Hans, except where Facebook is asking

The Simplified Chinese mirror targets a script, not a country — readers in
Singapore and Malaysia are not `zh-CN`. So `<html lang>`, hreflang, the
sitemap alternates and `Content-Language` all say `zh-Hans`, and CSS scopes
with `:lang(zh)`, which matches it by prefix. The one exception is
`og:locale`, which takes Facebook's enumeration and gets `zh_CN`.

## 2026-08-14 — /zh is public/zh.html, not public/zh/index.html

Wrangler's default `html_handling = "auto-trailing-slash"` would serve
`zh/index.html` canonically at `/zh/` and answer `/zh` with a redirect hop.
A flat `zh.html` serves at exactly `/zh` — matching how `docs.html` already
serves at `/docs` — so the hreflang URLs carry no trailing-slash asymmetry.
The Chinese docs and 404 live under `public/zh/` because they need the path
prefix: `zh/404.html` is what makes a miss under `/zh/*` serve the Chinese
404 via nearest-404 walk-up, which CI now proves after every deploy.

## 2026-08-14 — The zh pages share the English og.png

The OG artwork is the mark, the wordmark, and English-by-design tokens; a
translated card would mean a second screenshot artifact to regenerate on
every palette change, for a marginal gain. The zh pages get translated
`og:title`/`og:description` and `og:locale zh_CN` over the shared image.
Revisit if the zh pages grow their own social traffic.

## 2026-08-14 — What stays English on the Chinese pages

CLI commands, verbatim CLI output, status tokens (`linked` … `conflict`) and
the capability-matrix mechanism tokens stay English everywhere, because the
tables must match what the tool actually prints. Authored commentary is
translated even inside `<pre>`: `<span class="c">` comment lines and the
annotations in illustrative trees ("your instructions, once"), which no real
`ls` ever printed. Vocabulary terms render as 中文（English）— the English
token is the thing `status` emits, so it stays visible. Section ids are
byte-identical across languages so `#fragment` deep links work in both.

## 2026-08-14 — The site is no longer zero-JavaScript, by exactly one file

The user asked for automatic browser-language detection, and the config
deliberately has no Worker (every request a free static-asset hit), so the
detection is client-side: `public/lang.js`, ~30 dependency-free lines. CSP
moves from `script-src 'none'` to `'self'` — still no inline and no
third-party script, and the site works with JS disabled. The script
negotiates over the full `navigator.languages` list, redirects only
Simplified-script tags (`zh`, `zh-Hans*`, `zh-CN`, `zh-SG`, `zh-MY`) from
`/` and `/docs` to their twins, stores an explicit switcher choice in
localStorage as the override, and never redirects away from a `/zh` URL
someone opened on purpose. Traditional-script readers keep English rather
than being force-fed 简体. The rejected alternative — a Worker with
`run_worker_first` on the hot paths — would have contradicted wrangler.toml's
documented no-Worker stance and put the two busiest routes on paid
invocations.

## 2026-08-15 — CI publishes releases, authenticated by OIDC, not tokens

The runbook's founding rule was "CI builds and verifies; it never publishes."
Frank reversed that for 1.1.2: a `vX.Y.Z` tag push now publishes to crates.io
and npm from the release workflow. The unsettled call was the credential. A
stored npm token was rejected on the runbook's own evidence — the account
enforces 2FA, classic automation tokens are gone, and granular tokens that
bypass 2FA are restricted from January 2027 — so both registries authenticate
by OIDC trusted publishing: per-run credentials, nothing stored in repo
secrets, provenance attestations for free. Costs accepted: a one-time
per-package trusted-publisher configuration on each registry from the owning
account, and a hard dependency on npm ≥ 11.5.1 in the publish job (asserted,
not assumed). Publish jobs run only on tag pushes, a guard job refuses a tag
that disagrees with `Cargo.toml`, and both publishers skip versions the
registry already has, so a re-run after a partial release converges instead
of tripping on publish-over-existing errors. The manual OTP path survives in
the runbook as the fallback.
