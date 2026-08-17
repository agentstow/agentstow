#!/usr/bin/env python3
"""Assemble the PyPI wheels from built binaries.

    scripts/build-wheels.py <out-dir> [target ...]

Each target is the same "<platform>-<arch>" name build-npm.sh uses, and the
binary for a target is read from target/<rust-triple>/release/agentstow, so a
release packs the *identical* binary that the tarball and the npm package ship.
Nothing is compiled here.

There is no pyproject.toml and no maturin on purpose: maturin would recompile
the crate in a second six-target matrix rather than reuse the artifacts the
build job already produced. A wheel is a zip with a prescribed layout, so it is
cheaper to write that layout than to build everything twice.

The binary goes in <name>-<version>.data/scripts/, which pip installs into the
environment's bin/ (Scripts/ on Windows) and marks executable — that is what
makes `pip install agentstow` and `uvx agentstow` put a working `agentstow` on
PATH without any Python entry-point shim in between.
"""

import base64
import hashlib
import os
import re
import stat
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Rust triple and the wheel platform tag for each target. The tag is the part
# pip matches against; get it wrong and pip reports "no matching distribution"
# rather than anything that points at the cause. The two-tag Linux values are
# the compressed tag sets auditwheel emits, so older pips that only know
# manylinux2014 still match.
TARGETS = {
    "darwin-arm64": ("aarch64-apple-darwin", "macosx_11_0_arm64"),
    "darwin-x64": ("x86_64-apple-darwin", "macosx_10_12_x86_64"),
    "linux-arm64": ("aarch64-unknown-linux-gnu", "manylinux_2_17_aarch64.manylinux2014_aarch64"),
    "linux-x64": ("x86_64-unknown-linux-gnu", "manylinux_2_17_x86_64.manylinux2014_x86_64"),
    "win32-arm64": ("aarch64-pc-windows-msvc", "win_arm64"),
    "win32-x64": ("x86_64-pc-windows-msvc", "win_amd64"),
}

SUMMARY = "One canonical .agents/ folder, fanned out to all your AI coding agents"

CLASSIFIERS = [
    "Development Status :: 5 - Production/Stable",
    "Environment :: Console",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Operating System :: MacOS",
    "Operating System :: Microsoft :: Windows",
    "Operating System :: POSIX :: Linux",
    "Programming Language :: Rust",
    "Topic :: Software Development :: Build Tools",
    "Topic :: Utilities",
]


def cargo_version():
    with open(os.path.join(ROOT, "Cargo.toml"), encoding="utf-8") as fh:
        for line in fh:
            m = re.match(r'^version = "(.*)"', line)
            if m:
                return m.group(1)
    sys.exit("no version in Cargo.toml")


def host_target():
    """This machine's target name, or None if it is not one we ship."""
    import platform

    arch = {
        "arm64": "arm64",
        "aarch64": "arm64",
        "x86_64": "x64",
        "amd64": "x64",
    }.get(platform.machine().lower())
    if sys.platform == "darwin":
        plat = "darwin"
    elif sys.platform.startswith("linux"):
        plat = "linux"
    elif sys.platform in ("win32", "cygwin"):
        plat = "win32"
    else:
        return None
    return f"{plat}-{arch}" if arch else None


def binary_for(target):
    """The built binary for a target, or None when it was not built."""
    triple, _ = TARGETS[target]
    exe = ".exe" if target.startswith("win32-") else ""
    path = os.path.join(ROOT, "target", triple, "release", "agentstow" + exe)
    if os.path.exists(path):
        return path
    # A plain `cargo build --release` writes the host binary untripled. Accept
    # it so a single-target local run works without cross-compiling — but only
    # for the host's own target. Without that guard a local build would happily
    # seal a macOS binary into a linux wheel, and a wheel published to PyPI
    # cannot be replaced, only yanked.
    if target != host_target():
        return None
    host = os.path.join(ROOT, "target", "release", "agentstow" + exe)
    return host if os.path.exists(host) else None


def metadata(version):
    with open(os.path.join(ROOT, "README.md"), encoding="utf-8") as fh:
        readme = fh.read()
    lines = [
        "Metadata-Version: 2.1",
        "Name: agentstow",
        f"Version: {version}",
        f"Summary: {SUMMARY}",
        "License: MIT",
        "Requires-Python: >=3.8",
        "Project-URL: Homepage, https://github.com/agentstow/agentstow",
        "Project-URL: Repository, https://github.com/agentstow/agentstow",
        "Description-Content-Type: text/markdown",
    ]
    lines += [f"Classifier: {c}" for c in CLASSIFIERS]
    return "\n".join(lines) + "\n\n" + readme


def record_line(path, data):
    digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
    return f"{path},sha256={digest},{len(data)}"


def build(target, version, outdir):
    binary = binary_for(target)
    if binary is None:
        return None
    _, platform_tag = TARGETS[target]
    exe = ".exe" if target.startswith("win32-") else ""
    distinfo = f"agentstow-{version}.dist-info"
    scripts = f"agentstow-{version}.data/scripts"

    with open(binary, "rb") as fh:
        binary_bytes = fh.read()

    # One Tag: line per tag in the compressed set, which is what the compound
    # platform tag in the filename expands to.
    tags = "\n".join(f"Tag: py3-none-{t}" for t in platform_tag.split("."))
    files = [
        (f"{scripts}/agentstow{exe}", binary_bytes, True),
        (f"{distinfo}/METADATA", metadata(version).encode("utf-8"), False),
        (
            f"{distinfo}/WHEEL",
            (
                "Wheel-Version: 1.0\n"
                "Generator: agentstow build-wheels.py\n"
                "Root-Is-Purelib: false\n" + tags + "\n"
            ).encode("utf-8"),
            False,
        ),
    ]

    record = [record_line(name, data) for name, data, _ in files]
    record.append(f"{distinfo}/RECORD,,")
    files.append((f"{distinfo}/RECORD", ("\n".join(record) + "\n").encode("utf-8"), False))

    name = f"agentstow-{version}-py3-none-{platform_tag}.whl"
    path = os.path.join(outdir, name)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as zf:
        for entry, data, executable in files:
            # A fixed timestamp keeps the wheel byte-identical across rebuilds
            # of the same binary; zip has no notion of "no timestamp".
            info = zipfile.ZipInfo(entry, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            # S_IFREG is not decoration. pip decides whether to install a file
            # executable with `stat.S_ISREG(mode) and mode & 0o111`, so a bare
            # 0o755 here fails S_ISREG and pip writes a non-executable binary
            # to the venv's bin/ — an `agentstow` on PATH that cannot run.
            mode = stat.S_IFREG | (0o755 if executable else 0o644)
            info.external_attr = mode << 16
            zf.writestr(info, data)
    return name


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: scripts/build-wheels.py <out-dir> [target ...]")
    outdir = sys.argv[1]
    requested = sys.argv[2:] or list(TARGETS)
    for target in requested:
        if target not in TARGETS:
            sys.exit(f"unknown target {target}; known: {' '.join(TARGETS)}")

    os.makedirs(outdir, exist_ok=True)
    version = cargo_version()
    built = []
    for target in requested:
        name = build(target, version, outdir)
        if name is None:
            print(f"  {target}: no binary, skipped")
        else:
            built.append(name)
            print(f"  {target}: {name}")
    if not built:
        sys.exit("no wheels built — is anything in target/<triple>/release?")
    print(f"{len(built)} wheel(s) in {outdir}")


if __name__ == "__main__":
    main()
