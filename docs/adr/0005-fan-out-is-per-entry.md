# Fan-out is per-entry, not per-directory

ADR-0001 chose symlinks over copy-and-render but never said at what granularity. Both readings are consistent with it: one symlink per Store entry inside a real target directory (`~/.claude/skills/research -> ../../.agents/skills/research`), or a single symlink replacing the whole directory (`~/.claude/skills -> ../.agents/skills`). agentstow does the former, and this records why that is a decision rather than an accident.

Per-directory has one genuine advantage, and it is not small: a new skill in the Store reaches every agent instantly, with no `sync` and no instruction to remember. Since ADR-0001 also forecloses a watcher or daemon, it is the only way to get that property at all. It is nonetheless rejected, because a skills directory is not exclusively a fan-out target:

- **It destroys Variants.** A per-agent version of a shared skill needs a slot to occupy. With one directory symlink there is no slot: `plannotator-annotate` cannot exist as a Claude-specific real directory shadowing the Store copy. Renaming it in the Store is not a substitute — the rename would fan the Claude-only variant out to every *other* agent, which is precisely what shadowing avoids.
- **It swallows content the agent owns.** Codex ships `~/.codex/skills/.system/` (its own bundled skills) and Hermes keeps `~/.hermes/skills/.hub/` (skill bookkeeping — audit log, lock, quarantine). Under per-directory, those writes land in the Store, fan out to every other agent, and enter the git tree the user syncs across machines. Nobody decided that; the mechanism did.
- **It removes `adopt`'s destination.** `adopt` moves a real directory out of a target into the Store and leaves a link behind. With no per-skill slot there is nothing to leave behind, and the sanctioned way to share something deliberately disappears.

Deliberate sharing keeps a home: `adopt` makes an agent's own skill a Store entry as a named, per-item act. What is rejected is that happening silently, for everything, as a side effect of the link mechanism.

## Consequences

- **One mechanism, no per-agent `link-mode`.** An opt-in directory mode for agents that happen to have no variants today would double every state in `link.rs`, force `adopt` to refuse on some agents, and leave a migration no command performs the first time such an agent gains a variant. Being variant-free is a property of a machine at a moment, not of an agent.
- **`sync` stays a step users run**, and the instruction to run it after installing a skill stays in the documentation. Agents that read the Store natively (opencode, oh-my-pi, and Hermes once `skills.external_dirs` is set) get instant propagation for free; the rest trade it for the three properties above.
- **A target directory may hold three kinds of thing** — our links, deliberate Variants, and content that is simply the agent's own. `link::survey` classifies only the first two; a real object at a name the Store does not have is not a Variant of anything and is left unreported, which is why `.system/` and `.hub/` are invisible rather than flagged.
