# 06 — Commands + subagents families

**What to build:** The link engine generalizes from "skills" to a family abstraction, then two new families ride it: `~/.agents/commands/<name>.md` (Claude-dialect markdown) fans out to every markdown-taking command surface (Claude, opencode, Codex prompts, Cursor, Roo, Windsurf workflows — the last three verify-at-build), and `~/.agents/subagents/<name>.md` fans out to Claude and opencode only. pi and Cline get nothing; oh-my-pi is native-via-discovery and untouched. All sync/status/adopt semantics (canonical links, prune, Variants, Foreign) apply to both families automatically.

**Blocked by:** 02 — Skills fan-out `sync`.

**Status:** ready-for-agent

- [ ] Store commands appear as canonical links in every markdown command target; excluded agents receive nothing
- [ ] Store subagents reach Claude and opencode only
- [ ] Variants, Foreign entries, pruning, `--dry-run`, and `status` vocabulary all work identically for the new families
- [ ] Verify-at-build command rows (Cursor, Roo, Windsurf) confirmed against current agent releases or corrected
- [ ] Gemini commands are explicitly *not* linked (that's ticket 12's rendered path)
