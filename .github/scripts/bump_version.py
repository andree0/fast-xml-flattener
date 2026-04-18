#!/usr/bin/env python3
"""Bump version in Cargo.toml and pyproject.toml.

Usage: python bump_version.py <patch|minor|major>

Reads the current version from Cargo.toml (single source of truth),
increments the requested component, and writes the new version to
both Cargo.toml and pyproject.toml.  Prints the new version to stdout.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent.parent


def current_version() -> str:
    text = (ROOT / "Cargo.toml").read_text()
    m = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if not m:
        raise ValueError("version not found in Cargo.toml")
    return m.group(1)


def bump(version: str, kind: str) -> str:
    major, minor, patch = map(int, version.split("."))
    if kind == "major":
        return f"{major + 1}.0.0"
    if kind == "minor":
        return f"{major}.{minor + 1}.0"
    if kind == "patch":
        return f"{major}.{minor}.{patch + 1}"
    raise ValueError(f"unknown bump kind: {kind!r}")


def replace_version(path: Path, old: str, new: str) -> None:
    text = path.read_text()
    updated = text.replace(f'version = "{old}"', f'version = "{new}"', 1)
    if updated == text:
        raise ValueError(f"version {old!r} not found in {path}")
    path.write_text(updated)


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in ("patch", "minor", "major"):
        print("usage: bump_version.py <patch|minor|major>", file=sys.stderr)
        sys.exit(1)

    kind = sys.argv[1]
    old = current_version()
    new = bump(old, kind)

    replace_version(ROOT / "Cargo.toml", old, new)
    replace_version(ROOT / "pyproject.toml", old, new)

    print(new)


if __name__ == "__main__":
    main()
