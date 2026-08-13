# 12 — Rendered whole files + Marker (Gemini commands)

**What to build:** The third ownership identity. Store commands render for Gemini as native TOML command files, each carrying the one-line Marker comment. Marked files are agentstow's: overwritten when the Store changes, pruned when the Store entry vanishes. An unmarked file at the same path is Foreign — never touched, reported as a conflict.

**Blocked by:** 06 — Commands + subagents families; 08 — MCP tracer bullet: Claude.

**Status:** ready-for-agent

- [ ] A Store command produces a Gemini TOML file with the Marker and faithful name/description/prompt mapping
- [ ] Editing the Store command re-renders the file; deleting the Store command prunes the Marked file
- [ ] A hand-authored (unmarked) file at the colliding path is untouched and reported
- [ ] A Marked file hand-edited out-of-band is restored to the render (overwrite courtesy: change shown)
- [ ] Sync remains a no-op when Store and renders already agree
