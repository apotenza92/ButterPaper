#!/usr/bin/env python3
"""Extract GitHub release notes for a version from CHANGELOG.md.

Usage:
  python3 scripts/release_notes_from_changelog.py --tag v0.0.2 > /tmp/notes.md

Rules:
- Looks for a section header like: '## [0.0.2] - YYYY-MM-DD' (leading 'v' stripped from tag).
- Emits the body until the next '## ' header.
- Fails non-zero if the section is missing.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--tag", required=True, help="Git tag, e.g. v0.0.2 or v0.0.2-beta.1")
    p.add_argument(
        "--changelog",
        default="CHANGELOG.md",
        help="Path to CHANGELOG.md",
    )
    return p.parse_args()


def main() -> int:
    args = parse_args()
    tag = args.tag.strip()
    version = tag[1:] if tag.startswith("v") else tag

    changelog_path = Path(args.changelog)
    text = changelog_path.read_text(encoding="utf-8")

    # Match: ## [0.0.2] - 2026-02-10
    header_re = re.compile(
        r"^## \[" + re.escape(version) + r"\]\s*-\s*\d{4}-\d{2}-\d{2}\s*$"
    )
    lines = text.splitlines()

    start = None
    for i, line in enumerate(lines):
        if header_re.match(line):
            start = i
            break

    if start is None:
        sys.stderr.write(f"error: missing changelog entry for {version} in {changelog_path}\n")
        return 2

    end = len(lines)
    for j in range(start + 1, len(lines)):
        if lines[j].startswith("## "):
            end = j
            break

    section = "\n".join(lines[start:end]).strip() + "\n"

    # Strip the top-level header line; GH release already has tag/title.
    section_lines = section.splitlines()
    body = "\n".join(section_lines[1:]).lstrip("\n").rstrip() + "\n"
    sys.stdout.write(body)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
