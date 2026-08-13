# 03 — `status` for link families

**What to build:** `agentstow status` answers "is everything synced?" in the project's fixed vocabulary: per target, each entry is linked / missing / dangling / Variant / Foreign. Identical Variants are flagged "could be re-linked"; divergent Variants are listed neutrally (divergence is the point). `--json` emits a machine-readable report on a clean stdout. Exit code 2 if and only if anything actionable exists, 0 when clean, 1 on errors — so cron and CI can gate on it.

**Blocked by:** 02 — Skills fan-out `sync`.

**Status:** done

- [x] Reports use CONTEXT.md vocabulary exactly (Store, Target, Variant, Foreign, dangling)
- [x] Identical Variant flagged re-linkable (content comparison); divergent Variant listed without warning tone
- [x] `--json` output parses and carries the same facts as the human report; diagnostics stay on stderr
- [x] Exit 2 iff at least one actionable state exists; 0 clean; 1 errors — verified through fixtures for all three
