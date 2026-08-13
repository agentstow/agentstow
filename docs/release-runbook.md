# Release runbook

Everything here is manual and outward-facing. CI builds and verifies; it never
publishes. `scripts/verify-packaging.sh` runs the same checks locally.

## Prerequisites — both settled 2026-08-13

Frank claimed the organisation and set the version to `1.0.0`. Both checks below
now pass; they are kept because they are what to re-run if a release ever fails
at the publish step.

### 1. The `@agentstow` npm organisation — **claimed**

The four platform packages are named `@agentstow/darwin-arm64`,
`@agentstow/darwin-x64`, `@agentstow/linux-arm64`, `@agentstow/linux-x64`. A
scoped name requires the scope to exist and the publishing account to belong to
it. Verified on 2026-08-13:

```
$ npm view @agentstow/darwin-arm64 version
npm error 404 Not Found

$ npm org ls agentstow
npm error 403 Forbidden — You may not perform that action with these credentials.
```

Those were the readings *before* the org existed. Note that `npm org ls` still
returns 403 from this machine's credentials: the CLI token can publish but is
not authorised to read organisation membership, so a 403 there is not evidence
either way. The publish itself is what settles it. To claim the scope:

1. Sign in to npmjs.com as `soulmachine` (the account that owns `agentstow`).
2. Create an organisation named exactly `agentstow`
   (<https://www.npmjs.com/org/create>). The **free** tier is enough — public
   packages only, which is what these are.
3. Confirm it worked: `npm org ls agentstow` should list `soulmachine` as an
   owner rather than returning 403.

**A passing dry run never proved this.** `npm publish --dry-run` packs and
validates locally; it does not check that the scope exists or that you may
publish into it. The four platform packages reported `ok` while the scope was
still unclaimed, so that signal is worth nothing here — which is why this
section exists rather than trusting the script.

If the name turns out to be taken, the alternative is unscoped names
(`agentstow-darwin-arm64` and so on). That changes `optionalDependencies` in
`npm/agentstow/package.json` and the naming in `scripts/build-npm.sh`, and
nothing else — the launcher resolves whatever those names say.

### 2. The published version — **set to `1.0.0`**

`agentstow@0.0.1` is the name-claim placeholder, published 2026-08-13 and owned
by `soulmachine`. A release cannot reuse it:

```
$ npm publish --dry-run       # in npm/agentstow
npm error You cannot publish over the previously published versions: 0.0.1.
```

`Cargo.toml` now says `1.0.0`, and `scripts/verify-packaging.sh` confirms all
five packages dry-run cleanly at that version. `scripts/build-npm.sh` reads the
version from `Cargo.toml` and stamps every package, so the crate and all five
npm packages stay in lockstep by construction — a release is a one-line bump.

## Releasing

1. Bump `version` in `Cargo.toml`. Nothing else carries a version. The crate
   requires Rust 1.97 (edition 2024), so a release machine needs a current
   toolchain.
2. `./scripts/verify-packaging.sh` — must end with *Local packaging checks
   passed* and no blocked packages.
3. Commit, tag `vX.Y.Z`, push the tag. The `release` workflow cross-builds all
   four targets, assembles the packages, installs them offline and dry-run
   publishes. Download the `npm-packages` artifact.
4. `cargo publish`.
5. Publish the **platform packages first**, then the launcher:
   ```sh
   for target in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
     (cd "dist/$target" && npm publish --access public)
   done
   (cd dist/agentstow && npm publish --access public)
   ```
   Order matters. The launcher declares the platform packages as optional
   dependencies; publishing it first leaves a window where installing it
   resolves nothing and the binary is missing.
   `--access public` is required: scoped packages default to restricted.
6. Verify from a clean directory:
   ```sh
   npm install --no-save agentstow && ./node_modules/.bin/agentstow --version
   ```

## Notes

- **2FA.** The account enforces 2FA for publishing. `npm publish` prompts for a
  one-time code; `--otp=<code>` skips the browser round trip. Granular tokens
  that bypass 2FA are being restricted from January 2027, so plan on the OTP.
- **No install hooks, ever.** The packages carry no `preinstall`, `install` or
  `postinstall` script. That is what makes an install work offline and inside a
  sandboxed CI, and it is asserted by both the local script and the workflow.
  Anything that would need a postinstall fetch belongs in a platform package
  instead.
- **Unsupported platforms.** A machine with no matching platform package gets a
  message naming the package it looked for and pointing at `cargo install`,
  rather than a missing-file crash. Windows is not built: v1 is macOS and Linux.
