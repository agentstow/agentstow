# 01 — Rename the Store to the Commons everywhere

**What to build:** Every place agentstow speaks or is spoken to uses the term **Commons** for the canonical `~/.agents/` directory. A user running any command sees only the new vocabulary; a contributor reading the code finds identifiers that match the glossary. The first mention in help text carries the gloss: "the Commons — the canonical `~/.agents/` directory". Reasoning and evidence: DECISIONS.md 2026-08-16 "The Store is renamed the Commons". This is the one wide, mechanical ticket — it lands as a single atomic change (one crate, compiler-guided) and gates everything behind it so no later ticket writes a "Store" string that immediately needs rewriting.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] All user-facing output of every command says "Commons", never "Store"/"store", for the central directory
- [ ] CLI `--help`/about text uses the Commons gloss at first mention
- [ ] Code identifiers follow the glossary: the Commons type and module; no public Store-named items remain
- [ ] The existing ADRs are untouched (their amendment notes already record the rename)
- [ ] Full test suite green, including updated message assertions
