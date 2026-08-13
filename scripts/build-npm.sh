#!/usr/bin/env bash
# Assemble the npm packages from built binaries.
#
#   scripts/build-npm.sh <out-dir> [target ...]
#
# Each target is a node-style "<platform>-<arch>" name. The binary for a target
# is expected at target/<rust-triple>/release/agentstow, except for the host
# target in a plain `cargo build --release`, which is also accepted.
#
# Produces <out-dir>/agentstow (the launcher) and <out-dir>/<target> packages.
set -euo pipefail

OUT="${1:?usage: build-npm.sh <out-dir> [target ...]}"
shift
TARGETS=("$@")
if [ ${#TARGETS[@]} -eq 0 ]; then
  TARGETS=(darwin-arm64 darwin-x64 linux-arm64 linux-x64)
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"

rust_triple() {
  case "$1" in
    darwin-arm64) echo aarch64-apple-darwin ;;
    darwin-x64)   echo x86_64-apple-darwin ;;
    linux-arm64)  echo aarch64-unknown-linux-gnu ;;
    linux-x64)    echo x86_64-unknown-linux-gnu ;;
    *) echo "unknown target: $1" >&2; return 1 ;;
  esac
}

node_os()  { case "$1" in darwin-*) echo darwin ;; linux-*) echo linux ;; esac; }
node_cpu() { case "$1" in *-arm64) echo arm64 ;; *-x64) echo x64 ;; esac; }

mkdir -p "$OUT"

# The launcher, with its version and optional dependencies pinned to VERSION.
cp -R "$ROOT/npm/agentstow" "$OUT/agentstow"
node - "$OUT/agentstow/package.json" "$VERSION" <<'JS'
const fs = require("node:fs");
const [file, version] = process.argv.slice(2);
const pkg = JSON.parse(fs.readFileSync(file, "utf8"));
pkg.version = version;
for (const name of Object.keys(pkg.optionalDependencies ?? {})) {
  pkg.optionalDependencies[name] = version;
}
fs.writeFileSync(file, JSON.stringify(pkg, null, 2) + "\n");
JS
cp "$ROOT/README.md" "$OUT/agentstow/README.md" 2>/dev/null || true

for target in "${TARGETS[@]}"; do
  triple="$(rust_triple "$target")"
  binary="$ROOT/target/$triple/release/agentstow"
  # A host-target build without --target lands in target/release.
  [ -f "$binary" ] || binary="$ROOT/target/release/agentstow"
  if [ ! -f "$binary" ]; then
    echo "missing binary for $target (looked for target/$triple/release/agentstow)" >&2
    exit 1
  fi

  dir="$OUT/$target"
  mkdir -p "$dir/bin"
  cp "$binary" "$dir/bin/agentstow"
  chmod +x "$dir/bin/agentstow"

  node - "$dir/package.json" "$target" "$VERSION" "$(node_os "$target")" "$(node_cpu "$target")" <<'JS'
const fs = require("node:fs");
const [file, target, version, os, cpu] = process.argv.slice(2);
fs.writeFileSync(
  file,
  JSON.stringify(
    {
      name: `@agentstow/${target}`,
      version,
      description: `agentstow binary for ${target}`,
      license: "MIT",
      repository: { type: "git", url: "git+https://github.com/agentstow/agentstow.git" },
      // npm installs an optional dependency only when os and cpu match, which
      // is what keeps three of these four off any given machine.
      os: [os],
      cpu: [cpu],
      files: ["bin/agentstow"],
      preferUnplugged: true,
    },
    null,
    2
  ) + "\n"
);
JS
done

echo "assembled ${#TARGETS[@]} platform packages plus the launcher in $OUT"
