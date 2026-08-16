# 12 — Output-vocabulary test, docs verification, 2.0.0

**What to build:** The glossary becomes executable: a test scans every user-facing output string against `CONTEXT.md`'s avoid-lists (including the retired term "store" for the central directory) and fails on any violation, so messages can never drift from the documented language. With every behavior ticket landed, the README and CLI help are verified against actual shipped behavior (the docs were written ahead of the code), and the release is cut as **2.0.0** — the XDG hard cut is the breaking change (DECISIONS.md 2026-08-16) — following the release runbook.

**Blocked by:** 02, 03, 04, 05, 06, 07, 08, 09, 10, 11 — every ticket that writes or changes user-facing strings.

**Status:** done

- [ ] The vocabulary test exists, covers all user-facing strings, enforces the avoid-lists plus "store", and is green
- [ ] README and `--help` text verified sentence-by-sentence against shipped behavior; discrepancies fixed
- [ ] Version 2.0.0 across the crate and all npm packages; release notes drafted per the runbook
- [ ] Full gate green: `cargo test`, the Windows target check, and packaging verification
