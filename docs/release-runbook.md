# Release runbook

Since 1.1.2 a release is published by CI: pushing a `vX.Y.Z` tag makes the
`release` workflow build, verify, and — only on the tag push — publish to
crates.io and npm. Both registries are authenticated with **OIDC trusted
publishing**: the workflow mints short-lived credentials per run, so there is
no long-lived token to store, rotate, or leak, and npm's 2FA enforcement is
satisfied without an OTP. `scripts/verify-packaging.sh` runs the same
verification locally.

## One-time setup — trusted publishing on both registries

Done once per registry from the owning account (`soulmachine`); a publish from
CI fails with an auth error until this exists.

- **crates.io** — <https://crates.io/crates/agentstow/settings> → *Trusted
  Publishing* → *Add*: repository owner `agentstow`, repository name
  `agentstow`, workflow filename `release.yml`, environment left blank.
- **npm** — for **each of the five packages** (`agentstow`,
  `@agentstow/darwin-arm64`, `@agentstow/darwin-x64`,
  `@agentstow/linux-arm64`, `@agentstow/linux-x64`): package page → *Settings*
  → *Trusted Publisher* → *GitHub Actions*: organization `agentstow`,
  repository `agentstow`, workflow filename `release.yml`, environment left
  blank. Trusted publishing also generates provenance attestations; the
  `repository` field every package already carries must keep matching the
  GitHub repo or the publish is rejected.

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
3. Commit, tag `vX.Y.Z`, push the tag. The tag must match `Cargo.toml` — a
   `guard` job fails the run otherwise. The `release` workflow cross-builds
   all four targets, assembles the packages, installs them offline, dry-run
   publishes, then publishes the crate and all five npm packages. Publish
   jobs run only on the tag push — never for `workflow_dispatch` or pull
   requests.
4. Verify as described below once the workflow is green.

## Manual fallback

If CI publishing is unavailable, publish by hand from the workflow's
`npm-packages` artifact (or a local `scripts/build-npm.sh dist`):

1. `cargo publish`.
2. Publish the **platform packages first**, then the launcher:
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
   CI publishes in this same order.

## Verifying

1. **Wait for propagation before verifying.** A package name that is new to the
   registry is not readable the instant `npm publish` returns, even though the
   upload succeeded. On the 1.0.0 release all four `@agentstow/*` packages
   returned `PUT 200` and then 404 on `GET` for several minutes, appearing one
   at a time; the `agentstow` launcher was visible immediately only because that
   name already existed. A 404 straight after publishing is not a failed
   publish — check the npm debug log for `PUT 200` before assuming anything is
   wrong, and re-check the registry rather than republishing.
   ```sh
   until curl -sf -o /dev/null https://registry.npmjs.org/@agentstow%2Fdarwin-arm64; do sleep 15; done
   ```
2. Verify from a clean directory, with the cache cleared so a stale packument
   cannot mask the result:
   ```sh
   npm cache clean --force
   npm install --no-save agentstow && ./node_modules/.bin/agentstow --version
   ```

## Notes

- **2FA.** The account enforces 2FA for publishing. CI is untouched by this:
  trusted publishing mints per-run credentials that satisfy the enforcement
  without an OTP. The **manual fallback** still prompts — `--otp=<code>` skips
  the browser round trip. Granular tokens that bypass 2FA are being restricted
  from January 2027, which is exactly why CI uses trusted publishing and not a
  stored token.
- **Artifacts strip permissions.** `actions/download-artifact` does not
  preserve file modes, so any job consuming the `npm-packages` artifact must
  re-`chmod +x` the binaries before publishing — the publish job does, and
  proves it by executing the linux-x64 binary. 1.1.2 shipped a non-executable
  binary because this step was missing.
- **No install hooks, ever.** The packages carry no `preinstall`, `install` or
  `postinstall` script. That is what makes an install work offline and inside a
  sandboxed CI, and it is asserted by both the local script and the workflow.
  Anything that would need a postinstall fetch belongs in a platform package
  instead.
- **Unsupported platforms.** A machine with no matching platform package gets a
  message naming the package it looked for and pointing at `cargo install`,
  rather than a missing-file crash. Windows is not built: v1 is macOS and Linux.
