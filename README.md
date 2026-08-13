# agentstow

**GNU Stow for your AI coding agents.**

One Store at `~/.agents/` holds the single real copy of every config you share — skills,
instructions, MCP servers, slash commands, subagents and hooks. `agentstow sync` fans it
out to every agent you actually have installed.

Configs that can be byte-identical everywhere are **symlinked**, so there is one file seen
from ten places and drift is impossible by construction. Configs that cannot be — MCP
servers and hooks, which live inside files the agent also owns, in formats no two agents
share — are **rendered and key-merged**, with your other keys preserved.

There is no state file. There never will be. The filesystem is the state.

```
~/.agents/
├── skills/<name>/       fanned out as symlinks
├── commands/<name>.md
├── subagents/<name>.md
├── AGENTS.md            symlink, import-line, or rules-dir link per agent
├── mcp.json             rendered into each agent's native dialect
└── hooks/<Event>.toml   merged by command string
```

## Install

```sh
npm install -g agentstow      # prebuilt binary, macOS and Linux, no toolchain
cargo install agentstow       # from source, needs Rust 1.97+
```

## Use

```sh
agentstow init      # create the Store and report what this machine already has
agentstow adopt ~/.claude/skills/my-skill   # take an existing config into the Store
agentstow sync      # fan it out; --dry-run previews every action
agentstow status    # what is linked, what is not, and what is not ours
agentstow doctor    # installed agents, Store usability, hygiene
agentstow mcp list | adopt | remove
```

## Agents

Claude Code, Codex, opencode, pi, oh-my-pi, OpenClaw, Hermes, Gemini CLI, Cursor, Windsurf,
Roo and Cline. Detection is simply whether the agent's config directory exists — agentstow
never creates one. opencode and oh-my-pi read `~/.agents/` natively, so nothing is written
for them.

## What it will not touch

Anything it does not own. A symlink pointing outside the Store, a hand-written file, an
MCP server whose name isn't in the Store — all Foreign, all reported by `status`, none
ever modified. A real directory shadowing a Store entry is a **Variant**: deliberate,
preserved, and flagged only when its contents are identical so you can dedupe on purpose.

The Store is a commons, not agentstow's alone — opencode, oh-my-pi and Hermes read
`~/.agents/` themselves, and other tools keep their own files there. `doctor` names Store
entries that aren't agentstow's and leaves them be; `status` stays target-only, since a
neighbour's file has no fan-out to report.

## What it does not do

No cross-machine sync — version the Store with git or chezmoi. **No memory sync**: agent
memory is not a defined artifact and agentstow will not pretend otherwise. No Windows. No
GUI, daemon or file watcher. It does not install skills; it fans out whatever is in the
Store, whoever put it there.

[Documentation](https://agentstow.dev/docs) · [agentstow.dev](https://agentstow.dev) · MIT
