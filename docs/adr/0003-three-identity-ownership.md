# Stateless ownership via three identities: link, name, marker

Everything agentstow writes must be recognizable as agentstow's on a later run with no state file (ADR-0001). Three identity mechanisms cover every sync mechanism, chosen per family:

- **Link identity** — symlink fan-out (skills, instructions, commands, subagents): a symlink resolving into the store is ours; we canonicalize, prune when dangling, and never touch links resolving elsewhere.
- **Name identity** — key-merged entries in shared config files (MCP servers; hooks): an entry whose name appears in the store is ours and the store wins; entries with unknown names are foreign and untouched. Hooks live in JSON *arrays*, so their "name" is the command string — agentstow owns exactly the array elements whose command matches a store hook, preserving foreign elements in the same event array (element identity).
- **Marker identity** — rendered whole files (e.g. Gemini `commands/*.toml`): every file agentstow generates carries a one-line marker comment; marked files are overwritten/pruned freely, unmarked files are foreign. This does not contradict the no-markers rule for MCP entries: that rule bars injecting fields into *data* agents parse as config values; a comment in a wholly generated file is invisible to the agent.

## Consequences

- Hooks are in scope only as declarations: agentstow writes hook definitions but **never trust metadata** (Codex's `[hooks.state]` sha256 entries) — the agent re-prompts the user to trust changed hooks, keeping execution approval human. Hook scripts themselves are not managed; store commands must use agent-agnostic paths.
- No mechanism exists to detect an orphan whose identity was destroyed (a store MCP entry hand-deleted, a marker comment hand-stripped). `status` reports such leftovers as foreign; that is the accepted price of statelessness.
