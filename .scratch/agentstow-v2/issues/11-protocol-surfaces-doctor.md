# 11 — doctor recognizes Protocol surfaces

**What to build:** `.agents` Protocol paths at the Commons root — `tasks/`, `memories/`, `models.json`, `system-prompt.md` — are recognized by name: `doctor` attributes them as Protocol surfaces instead of listing them anonymously under other tools' files. They are never touched, never counted as issues, and never reported by `status` — a named kind of Co-tenant, per the glossary. Anything else unknown at the root remains an ordinary Co-tenant. A surface is promoted to a managed family only when a real consumer exists to fan out to — not in this ticket. Reasoning: DECISIONS.md 2026-08-16 "The .agents Protocol is adopted with two reservations".

**Blocked by:** 03 — agents family rename (same doctor root-entry classification is touched there).

**Status:** done

- [ ] `doctor` lists present protocol surfaces under their own attributed grouping, distinct from other co-tenants
- [ ] Absent surfaces are not mentioned; nothing is ever created, counted as an issue, or touched
- [ ] `status` stays silent about them; other unknown root entries still list as ordinary co-tenants
