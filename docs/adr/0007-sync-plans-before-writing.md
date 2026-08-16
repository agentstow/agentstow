# sync plans everything before writing anything

v1's sync was incremental per family: the symlink families wrote as they scanned, then MCP, rendered files, and hooks each surveyed-then-applied. A survey failure in a later family (an unparseable Commons file, an unset `${env:VAR}`) aborted the remaining families *after* the symlink writes had already landed — and a comment claimed "nothing has been written," which was false for those writes. GNU Stow settled the same question in 2.0: scan for every conflict first, display them all, and terminate without touching the filesystem if any exist. v2 adopts that shape wholesale.

Phase 1 surveys every family read-only and collects **every** problem — not just the first — before anything is written. Any problem: report all of them, exit 1, zero writes; "no changes were made" is now unconditionally true. Phase 2 executes the plan. Write failures during execution report and continue with the remaining operations: they are filesystem races and permission surprises no survey can preclude, a half-executed plan plus an idempotent re-run heals, and aborting mid-plan would just leave a different half. The plan preserves per-file grouping — every operation targeting one config file (Gemini's settings file takes both MCP and hook merges) lands as one coherent write — and the family-ordering constraints v1 encoded (hooks after MCP) hold inside the plan.

`--dry-run` becomes phase 1 plus the printed plan. One code path serves simulation and execution, so the preview cannot drift from reality — the property that makes dry-run trustworthy enough to be adopt's safety net too.

## Consequences

- **Exit semantics are unchanged**: 0 clean, 1 error, 2 actionable (status only). `sync --dry-run` still exits 0 with pending work — `status` remains the CI drift gate, documented as such; dry-run answers "what exactly would you do," not "is anything pending."
- **A config error can no longer leave a half-synced machine.** The failure mode the old ordering permitted — symlinks updated, MCP abandoned — is structurally gone, and the misleading comment goes with it.
- **adopt's fan-out rides the same machinery**: the per-entry plan-and-execute path is what adopt calls after any mechanic, which is how "sync after adopt is a no-op" is kept true by construction rather than by discipline.
- Stow's other half — displaying *all* conflicts rather than the first — is part of the contract: a user fixes one list, not one error per run.
