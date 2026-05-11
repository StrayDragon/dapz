#!/usr/bin/env python3
"""
Validate git commit messages follow Conventional Commits format.

Reads the commit message from .git/COMMIT_EDITMSG (passed as arg or default).
Exits with code 0 if valid, 1 if invalid.
"""

import re
import sys
from pathlib import Path

# Conventional Commits pattern
# type(scope)!: description
PATTERN = re.compile(
    r"^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)"
    r"(\([\w\s/.-]+\))?"
    r"(!)?"
    r":\s"
    r".+"
)

# Allowed types
ALLOWED_TYPES = {
    "build", "chore", "ci", "docs", "feat", "fix",
    "perf", "refactor", "revert", "style", "test",
}


def main():
    if len(sys.argv) > 1:
        msg_file = sys.argv[1]
    else:
        msg_file = ".git/COMMIT_EDITMSG"

    path = Path(msg_file)
    if not path.exists():
        print(f"  SKIP: commit message file not found: {msg_file}")
        sys.exit(0)

    lines = path.read_text(encoding="utf-8").split("\n")

    # Skip comments (lines starting with #)
    msg_lines = [l for l in lines if l and not l.startswith("#")]

    if not msg_lines:
        print("  SKIP: empty commit message")
        sys.exit(0)

    subject = msg_lines[0]
    subject = subject.strip()

    if not subject:
        print("  SKIP: empty subject line")
        sys.exit(0)

    # Allow merge commits
    if subject.startswith("Merge "):
        sys.exit(0)

    m = PATTERN.match(subject)
    if not m:
        print(f"  FAIL: '{subject}'")
        print(f"  Expected format: type(scope)!: description")
        print(f"  Allowed types: {', '.join(sorted(ALLOWED_TYPES))}")
        sys.exit(1)

    commit_type = m.group(1)
    if commit_type not in ALLOWED_TYPES:
        print(f"  FAIL: unknown type '{commit_type}'")
        sys.exit(1)

    print(f"  OK: {subject}")
    sys.exit(0)


if __name__ == "__main__":
    main()
