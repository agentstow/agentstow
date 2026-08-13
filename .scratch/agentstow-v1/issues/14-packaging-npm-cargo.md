# 14 — Packaging: npm platform packages + cargo

**What to build:** Installation channels. `cargo install agentstow` builds a working binary. The npm channel follows the esbuild/Biome pattern: `@agentstow/<platform>` prebuilt-binary sub-packages under the `agentstow` launcher's `optionalDependencies` — install never runs a postinstall network fetch. Release CI cross-builds darwin-arm64, darwin-x64, linux-x64, linux-arm64 and assembles the packages. (Publishing the real release gates on the rest of v1; this ticket delivers the verified pipeline.)

**Blocked by:** 01 — Crate scaffold, test seam, target registry, `doctor`.

**Status:** ready-for-agent

- [ ] `cargo install --path .` yields a working `agentstow` binary
- [ ] The launcher package resolves and executes the correct platform sub-package binary on macOS and Linux
- [ ] No postinstall network access: a fully offline `npm install` from a local registry fixture succeeds
- [ ] CI produces all four platform artifacts and a dry-run npm publish of the package set passes
- [ ] The npm org scope for platform packages is claimed and documented in the release runbook
