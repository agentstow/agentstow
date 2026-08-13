# 05 — `adopt`: the three-case rule

**What to build:** `agentstow adopt <path>` absorbs an existing real config from a target into the Store: name absent from the Store → move it in and leave a canonical link; name present with byte-identical content → replace the target copy with a link (the cure for accidental Variants); name present with divergent content → refuse with a Variant explanation and change nothing. No force flag exists — divergence is never silently discarded.

**Blocked by:** 02 — Skills fan-out `sync`.

**Status:** done

- [x] Absent-from-Store case: object moved into the Store, canonical relative link left at the original path, content byte-identical after the move
- [x] Identical case: target copy replaced by a link; Store copy untouched
- [x] Divergent case: refusal message explains it's a Variant and suggests manual merge; exit 1; filesystem untouched
- [x] Works for skill directories and single-file instructions targets alike
