#!/usr/bin/env python3
"""Prepend a new version entry to CHANGELOG.md.

Usage: python update_changelog.py <version> <bump_type> <commit_message>
"""

import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).parent.parent.parent
CHANGELOG = ROOT / "CHANGELOG.md"

BUMP_SECTION = {
    "major": "Breaking Changes",
    "minor": "New Features",
    "patch": "Bug Fixes",
}


def main() -> None:
    if len(sys.argv) < 4:
        print("usage: update_changelog.py <version> <patch|minor|major> <message>", file=sys.stderr)
        sys.exit(1)

    version = sys.argv[1]
    bump_type = sys.argv[2]
    message = " ".join(sys.argv[3:])
    today = date.today().isoformat()
    section = BUMP_SECTION.get(bump_type, "Changes")

    entry = f"## [{version}] - {today}\n\n### {section}\n\n- {message}\n\n"

    existing = CHANGELOG.read_text() if CHANGELOG.exists() else "# Changelog\n\n"

    # Find the end of the leading header block (heading + description lines)
    # and insert the new entry just before the first "## [" version section.
    marker = "\n## ["
    idx = existing.find(marker)
    if idx != -1:
        updated = existing[:idx] + "\n\n" + entry + existing[idx + 1:]
    else:
        # No prior version sections — append after header.
        updated = existing.rstrip("\n") + "\n\n" + entry

    CHANGELOG.write_text(updated)
    print(f"CHANGELOG.md updated with v{version}")


if __name__ == "__main__":
    main()
