# agentstow v1 — spec

Status: ready-for-agent

## Problem Statement

People who run several AI coding agents (Claude Code, Codex, opencode, pi, oh-my-pi, Gemini CLI, Cursor, Windsurf, Roo, Cline) accumulate near-identical configuration in each agent's private directory: skills, global instructions, slash commands, subagents, MCP servers, and hooks. Keeping these in sync by hand fails in practice — an audit of a real machine found instructions reaching only one of four agents, seven skill directories full of stale and dangling links, MCP servers with zero overlap between any two agents, and the same logical hooks declared three different ways. Hand-rolled sync scripts rot (hardcoded targets, hand-computed relative paths, silent skips), and the existing tool in this space (agentsync) requires a copy+state+reconcile model whose sharpest edge is silently destroying hand-edits.

## Solution

agentstow is "GNU Stow for your AI coding agents": one canonical Store at `~/.agents/` holds the single real copy of every synced config, and `agentstow sync` fans it out to every detected agent. Configs that can be byte-identical everywhere (skills, instructions, commands, subagents) are symlinked, so drift is impossible by construction. Configs that require per-agent format translation (MCP servers, hooks) are rendered and key-merged into each agent's native file with stateless ownership — the tool holds no state file, ever; the filesystem is the state. Existing setups are absorbed with `adopt`, inspected with `status`, and diagnosed with `doctor`. Anything agentstow does not own — foreign links, foreign files, foreign MCP entries, per-agent Variants — is never touched.

## User Stories

1. As a multi-agent developer, I want to keep every skill in one Store directory, so that adding a skill once makes it available to every agent I use.
2. As a multi-agent developer, I want `agentstow sync` to create a symlink per Store skill in every detected agent's skills directory, so that all agents see the same skills without copies.
3. As a multi-agent developer, I want sync to prune dangling links that point into the Store, so that deleting a skill from the Store cleans it out of every agent.
4. As a multi-agent developer, I want sync to leave dangling Foreign links alone and merely report them, so that agentstow never deletes something it doesn't own.
5. As a multi-agent developer, I want my single `~/.agents/AGENTS.md` to reach every agent through its native mechanism (symlink, import-line, or rules-dir link), so that all agents share one set of global instructions.
6. As a Claude Code user, I want agentstow to ensure the `@~/.agents/AGENTS.md` import line exists in my `CLAUDE.md` without touching anything else in that file, so that my Claude-specific content survives.
7. As a Roo user, I want agentstow to drop an `AGENTS.md` symlink into my `~/.roo/rules/` directory, so that Roo's rules-glob picks up the shared instructions.
8. As an opencode or oh-my-pi user, I want agentstow to recognize my agent reads the Store natively and skip fan-out for it, so that I get no duplicate or pointless links.
9. As a user whose instructions file is occupied by another tool (claude-mem), I want `status` to report the conflict with a one-line remediation hint instead of overwriting, so that I decide how to resolve it.
10. As a user with an intentional per-agent skill variant (a real directory shadowing the Store copy), I want sync to preserve it forever and `status` to list it as a Variant, so that deliberate divergence is a first-class state, not an error.
11. As a user with an accidental Variant, I want `status` to flag Variants whose content is identical to the Store copy as "could be re-linked", so that I can dedupe safely.
12. As a user migrating from a hand-rolled setup, I want `agentstow adopt <path>` to move a real config into the Store and leave a link behind, so that onboarding takes minutes, not a migration doc.
13. As a user adopting a path whose name already exists in the Store with identical content, I want adopt to simply re-link it, so that duplicates collapse.
14. As a user adopting a path that diverges from the Store copy, I want adopt to refuse and explain it's a Variant, so that no side of a divergence is ever silently discarded.
15. As a multi-agent developer, I want one `~/.agents/mcp.json` in the standard `mcpServers` shape to define my MCP servers, so that a server declared once reaches every MCP-capable agent in its native format.
16. As a Codex user, I want my Managed MCP servers rendered as `[mcp_servers.<name>]` TOML tables merged into `config.toml` without disturbing my model settings or other keys, so that shared files stay shared.
17. As a user with a hand-added MCP server in an agent config, I want agentstow to treat unknown server names as Foreign and never touch them, so that experimentation is safe.
18. As a user removing an MCP server, I want `agentstow mcp remove <name>` to delete it from the Store and every target in one action, so that stateless sync doesn't strand renderings.
19. As a user with existing per-agent MCP servers, I want `agentstow mcp adopt` to reverse-translate them into the Store (lossy per-agent fields landing in Tweak tables), so that my current setup becomes Managed.
20. As a security-conscious user, I want to write `${env:VAR}` in `mcp.json` and have agentstow resolve it from the environment only at sync time, so that my Store stays committable to a dotfiles repo.
21. As a user, I want every diff and status output to redact resolved secret values, so that terminal scrollback and CI logs never leak credentials.
22. As a user who scopes servers per agent, I want `[mcp.<name>] agents = [...]` allowlists in `agentstow.toml`, so that heavyweight servers only reach the agents that need them.
23. As a user with agent-specific MCP settings (timeouts, enabled=false, cwd), I want per-agent Tweak tables merged into that agent's rendering, so that native knobs survive without polluting the standard Store file.
24. As a user who hand-edited a Managed MCP entry, I want sync to show me the per-key change it's applying (redacted) as it restores the Store's version, so that overwrites are visible, never silent.
25. As a multi-agent developer, I want slash commands in `~/.agents/commands/*.md` fanned out to every agent that takes markdown commands, so that my prompts work everywhere.
26. As a Gemini user, I want markdown commands rendered to Gemini's TOML command format with a Marker comment, so that even format-mismatched agents get my commands.
27. As a multi-agent developer, I want subagents in `~/.agents/subagents/*.md` fanned out to Claude and opencode, so that my subagent definitions live once.
28. As a multi-agent developer, I want command-hooks declared once in `~/.agents/hooks/<event>.toml` and key-merged into Claude, Codex, and Gemini's native hook config, so that the same logical hook isn't declared three ways.
29. As a hook user, I want agentstow to own only the hook array elements whose command matches a Store hook, so that other tools' hooks in the same event array survive.
30. As a Codex user, I want agentstow to never write hook trust hashes, so that approving hook execution remains my decision in the agent.
31. As a new user, I want `agentstow init` to scaffold the Store and print a guided report (detected agents, adoption candidates, conflicts with hints), so that the first two minutes teach me the whole model.
32. As a returning user, I want `agentstow status` to show per-target state — linked, missing, Variant, Foreign, conflict — so that one command answers "is everything synced?".
33. As a CI/cron user, I want `status --exit-code` semantics (exit 2 = actionable, 0 = clean, 1 = error) and `status --json`, so that automation can gate on drift.
34. As a cautious user, I want `sync --dry-run` to print every action without touching the filesystem, so that I can preview before trusting.
35. As a user with a broken environment, I want `agentstow doctor` to check store existence, target writability, and store hygiene (non-directory entries, dot-prefixed names), so that silent-skip bugs of hand-rolled scripts can't recur.
36. As a user with an unsupported or unwanted agent, I want `[targets] <name> = false` and custom target definitions in `~/.agentstow/agentstow.toml`, so that detection is overridable without code.
37. As a dotfiles user whose agent config file is itself a symlink, I want agentstow to resolve it and write at the final target, so that my dotfiles wiring survives agentstow's atomic writes.
38. As a dotfiles user, I want the README to document the cross-machine recipe (version the Store with git/chezmoi), so that multi-machine works without agentstow inventing sync.
39. As a user running sync from cron and by hand, I want a global lock to serialize mutating commands, so that concurrent runs can't corrupt a target.
40. As a Node-ecosystem user, I want `npm install -g agentstow` to install a prebuilt native binary for my platform, so that installation needs no toolchain.
41. As a Rust user, I want `cargo install agentstow` to work, so that I can build from source.
42. As a contributor running an agent agentstow doesn't know, I want the target registry to be a small data-driven table, so that adding my agent is a five-line pull request.
43. As a user of an agent that isn't installed, I want its registry row to stay inert (detection = config root exists; roots are never created), so that agentstow never scaffolds ghost agent directories.
44. As a user reading my own setup six months later, I want `status` output to use the project's fixed vocabulary (Store, Target, Variant, Foreign, Managed, Marker), so that every report is unambiguous.
45. As a user, I want results on stdout and diagnostics on stderr, so that piping and `--json` output are never corrupted.

## Implementation Decisions

- **Language & shape**: Rust, single binary, subcommand CLI: `sync`, `status`, `adopt`, `doctor`, `init`, `mcp list|adopt|remove`. macOS + Linux only in v1.
- **Distribution**: npm org `agentstow` with `@agentstow/<platform>` prebuilt-binary sub-packages under `optionalDependencies` plus the existing `agentstow` launcher package (Biome/esbuild pattern; no postinstall downloads); `cargo install` from day one; Homebrew deferred.
- **Store**: fixed at `~/.agents/` (env override `AGENTSTOW_HOME` for tests/unusual setups). Layout: `skills/<name>/`, `AGENTS.md`, `mcp.json`, `commands/<name>.md`, `subagents/<name>.md`, `hooks/<event>.toml`. The Store contains only ecosystem-standard or agentstow-canonical content — never tool configuration.
- **Tool config**: optional `~/.agentstow/agentstow.toml` — target enable/disable, custom target definitions, MCP `agents` allowlists, per-agent Tweak tables. Absent file = all defaults. The `~/.agentstow/` directory also holds the global lock.
- **Statelessness (ADR-0001)**: no state file, permanently. Ownership is established by three identities (ADR-0003): link identity (symlinks resolving into the Store), name identity (key-merged entries whose name/command appears in the Store), marker identity (rendered whole files carrying a one-line Marker comment). Anything unowned is Foreign and untouched.
- **Target registry**: built-in data-driven table, one row per agent declaring detection root and per-family capability (skills: fan-out | native | none; instructions: symlink | import-line | rules-dir link | conflict-prone | none; MCP: dialect descriptor | native-via-discovery | none; commands/subagents/hooks likewise). Detection = the agent's config root exists; subdirectories are created as needed, roots never. v1 rows: Claude Code, Codex, opencode, pi, oh-my-pi, Gemini, Cursor, Windsurf, Roo, Cline (capability matrix in Further Notes).
- **Sync semantics**: ensure one canonical **relative** symlink per Store entry per fan-out target; rewrite our links (absolute or odd-path links resolving into the Store) to canonical form; prune dangling links pointing into the Store; never touch Foreign links or files. Any real filesystem object at a Store-colliding path is a Variant: preserved unconditionally, reported by `status`, flagged "could be re-linked" only when content-identical to the Store copy.
- **Instructions mechanisms**: symlink where the target file is absent or ours; import-line for Claude (`@~/.agents/AGENTS.md` ensured in `CLAUDE.md` — additive, idempotent, the sole sanctioned edit to a user-owned file outside rendering); rules-dir link for Roo; report-only conflict with remediation hint where a Foreign tool owns the file (opencode, Gemini via claude-mem).
- **MCP (ADR-0002)**: canonical `~/.agents/mcp.json` in the de-facto standard `mcpServers` shape, kept pure. Per-agent rendering (JSON `mcpServers`, Codex TOML `[mcp_servers]` with `http_headers`, opencode `mcp` with flattened command arrays and `environment`, Windsurf `serverUrl`, Gemini `url`/`httpUrl`) key-merged into native files under name identity: Store name present ⇒ agentstow owns the entry, Store wins; unknown names are Foreign. Removal is imperative (`mcp remove` edits Store + all targets). `${env:VAR}` resolved at sync, redacted in output; no vault. pi is excluded (no MCP by design); oh-my-pi is native-via-discovery (zero writes).
- **Hooks**: command-hooks only, canonical per-event TOML with matcher + command. Element identity by command string inside native hook arrays; Foreign elements preserved. Trust metadata (Codex `[hooks.state]` hashes) never written; hook scripts not managed — Store commands must be agent-agnostic paths.
- **Commands/subagents**: canonical Claude-dialect markdown; symlink fan-out to markdown-taking agents; rendered whole files with Marker for format-mismatched targets (Gemini TOML commands). Subagents v1 = Claude + opencode only.
- **Write discipline**: direct read-modify-write with atomic rename (0600 temp file, fsync, parent-dir fsync); symlinked destinations are resolved and written at the final target so dotfiles wiring survives; a documented "don't sync mid-session" caveat covers races with live agents; changes to existing Managed entries print a per-key redacted diff, never prompt, never block.
- **CLI conventions**: results→stdout, diagnostics→stderr; exit 0 clean, 1 error, 2 actionable drift; `--dry-run` on sync; `--json` on status; `init` prints a guided first-run report and first `sync` on an un-inited machine suggests `init`.
- **Orthogonality**: agentstow never installs skills (skills-CLI interop preserved: it fans out whatever exists in the Store, regardless of provenance).

## Testing Decisions

- **One seam**: invoke the CLI's top-level entry against a throwaway directory tree, with `AGENTSTOW_HOME` pointing at a fixture Store and `AGENTSTOW_TARGET_ROOT` redirecting all home-relative target resolution. Assert on externally observable behavior only: the resulting filesystem (links, link targets, file contents, permissions), stdout, stderr, and exit codes. No test reaches into internals.
- **Fixture-driven coverage**: each behavior is a fixture tree variation run through the same seam — fresh machine, populated machine (Variants, Foreign links, claude-mem-occupied instruction files, hand-added MCP servers, hooks arrays with Foreign elements), dialect edge cases (TOML rendering, command-array flattening, header renames, `${env:VAR}` resolution and redaction), idempotence (a second `sync` is a no-op), and destructive-safety cases (adopt refusal on divergence, Foreign preservation, prune scope).
- **Prior art**: none in-repo (greenfield). Follow standard Rust integration-test conventions (`tests/` directory, temp-dir helpers); agentsync's container-based hermetic suite is the design reference for the redirect-the-root approach, not a dependency.
- **Good tests here** assert what a user would observe (`status` says Variant; the link at the target resolves into the Store; exit code is 2), never how the code got there.

## Out of Scope

- Cross-machine sync (documented dotfiles recipe instead), project/repo scope (v2 fog), memory sync (undefined artifact), Windows (symlink privileges; materializer kept swappable), plugin/marketplace enablement, skill installation (skills CLI's job), hook script management and trust metadata, secrets vault, per-skill/per-target filtering, LSP config, subagents beyond Claude + opencode, Codex TOML subagent rendering, the non-registry residue dirs (`~/.qwen`, `~/.kilocode`, `~/.factory`, `~/.continue`, `~/.augment` — one-time manual cleanup), and any GUI/daemon/watcher.

## Further Notes

- Vocabulary is fixed in `CONTEXT.md` (13 terms); architecture decisions in `docs/adr/0001` (symlink fan-out, zero state), `0002` (MCP rendered key-merge), `0003` (three-identity ownership). Specs and reports must use this vocabulary.
- v1 capability matrix (user scope):

| Agent | Skills | Instructions | MCP | Commands | Subagents | Hooks |
|---|---|---|---|---|---|---|
| Claude Code | fan-out | import-line | key-merge (JSON) | fan-out | fan-out | key-merge |
| Codex | fan-out | symlink | key-merge (TOML) | fan-out (prompts) | none | key-merge (verify format at build) |
| opencode | native | conflict | key-merge (JSON, verified safe) | fan-out | fan-out | none |
| pi | fan-out | symlink | none (by design) | none | none | none |
| oh-my-pi | native | symlink (verify) | native-via-discovery | native-via-discovery | none | none |
| Gemini | none | conflict | key-merge (JSON) | render+Marker (TOML) | none | key-merge |
| Cursor | fan-out | none | key-merge (JSON) | fan-out (verify) | none | none |
| Windsurf | none | symlink | key-merge (`serverUrl`) | fan-out (workflows, verify) | none | none |
| Roo | none | rules-dir link | none | fan-out (verify) | none | none |
| Cline | none | none | key-merge (JSON) | none | none | none |

- "Verify at build" flags mark rows sourced from agentsync's adapters or binary inspection rather than live observation; confirm each against the agent's current release during implementation.
- Reference implementation for per-agent dialect details: agentsync's adapter sources (locally cloned) — steal knowledge, not architecture.
