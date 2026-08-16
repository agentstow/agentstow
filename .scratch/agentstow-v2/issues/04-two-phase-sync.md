# 04 — Two-phase sync: plan everything before writing anything

**What to build:** `sync` surveys every family read-only first, collecting **every** problem (unparseable Commons file, unset `${env:VAR}` reference, conflicts) before anything is written. Any problem: all of them are reported, the run exits 1, and nothing has been written — which makes that summary line unconditionally true and retires the currently-false code comment. Phase 2 executes the plan; a write failure is reported and the remaining operations continue (an idempotent re-run heals). `--dry-run` becomes phase 1 plus the printed plan — one code path, so the preview cannot drift from reality. The plan preserves per-file grouping (a shared config file taking both MCP and hook merges gets one coherent write) and the existing family-ordering constraints. Design: ADR-0007.

**Blocked by:** 01 — Rename the Store to the Commons everywhere.

**Status:** done

- [ ] A sync with any survey problem exits 1, lists every problem (not just the first), and writes nothing
- [ ] `--dry-run` output is the same plan the real run executes, from the same code path
- [ ] One coherent write per shared config file; hooks-after-MCP ordering preserved inside the plan
- [ ] A phase-2 write failure is reported, remaining operations continue, exit 1; a re-run converges
- [ ] "No changes were made" is printed only when literally true
- [ ] Suite green, including a test pinning the survey-abort-writes-nothing property
