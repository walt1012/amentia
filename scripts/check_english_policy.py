#!/usr/bin/env python3
"""Reject CJK text in repository-managed source artifacts."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path


CJK_RE = re.compile(r"[\u3400-\u9fff\uf900-\ufaff]")
TEXT_SUFFIXES = {
    ".json",
    ".md",
    ".py",
    ".rs",
    ".swift",
    ".toml",
    ".txt",
    ".yml",
    ".yaml",
}
TEXT_FILENAMES = {
    ".editorconfig",
    ".gitattributes",
    ".gitignore",
    ".swiftlint.yml",
    "LICENSE",
}


def tracked_files() -> list[Path]:
    output = subprocess.check_output(["git", "ls-files"], text=True)
    return [Path(line) for line in output.splitlines() if line.strip()]


def should_scan(path: Path) -> bool:
    return path.suffix.lower() in TEXT_SUFFIXES or path.name in TEXT_FILENAMES


def main() -> int:
    violations: list[str] = []
    for path in tracked_files():
        if not should_scan(path):
            continue
        if not path.exists():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            if CJK_RE.search(line):
                violations.append(f"{path}:{line_number}: contains non-English CJK text")

    if violations:
        print("English-only policy check failed:")
        print("\n".join(violations))
        return 1

    print("English-only policy check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
