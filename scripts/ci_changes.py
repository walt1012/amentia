#!/usr/bin/env python3
"""Classify changed files into CI execution lanes."""

from __future__ import annotations

import argparse
import fnmatch
from dataclasses import dataclass
from pathlib import Path


WORKFLOW_PATTERNS = (
  ".github/workflows/ci.yml",
  ".github/workflows/release.yml",
)
RUST_PATTERNS = (
  "Cargo.toml",
  "Cargo.lock",
  "crates/*",
  "models/*",
  "plugins/*",
  "scripts/runtime_smoke_test.py",
)
SWIFT_PATTERNS = (
  "apps/pith-macos/*",
)
LLAMA_PATTERNS = (
  "scripts/macos_llama_backend.py",
  "scripts/package_macos_app.py",
)
PACKAGE_PATTERNS = (
  "Cargo.toml",
  "Cargo.lock",
  "crates/*",
  "apps/pith-macos/*",
  "models/*",
  "plugins/*",
  "scripts/create_macos_dmg.py",
  "scripts/macos_llama_backend.py",
  "scripts/package_contract.py",
  "scripts/package_macos_app.py",
  "scripts/installer_artifact_contract.py",
  "scripts/release_artifacts.py",
  "scripts/release_copy_contract.py",
  "scripts/release_identity.py",
  "scripts/release_text.py",
  "scripts/sign_macos_app_for_distribution.py",
  "scripts/smoke_launch_macos_app.py",
  "scripts/validate_macos_distribution.py",
  "scripts/validate_model_pack.py",
)


@dataclass(frozen=True)
class CiChanges:
  rust: bool
  swift: bool
  package: bool
  llama: bool

  @classmethod
  def all(cls) -> CiChanges:
    return cls(rust=True, swift=True, package=True, llama=True)

  def output_lines(self) -> list[str]:
    return [
      f"rust={bool_output(self.rust)}",
      f"swift={bool_output(self.swift)}",
      f"package={bool_output(self.package)}",
      f"llama={bool_output(self.llama)}",
    ]


def classify_changed_paths(paths: list[str], force_all: bool = False) -> CiChanges:
  if force_all or matches_any(paths, WORKFLOW_PATTERNS):
    return CiChanges.all()
  return CiChanges(
    rust=matches_any(paths, RUST_PATTERNS),
    swift=matches_any(paths, SWIFT_PATTERNS),
    package=matches_any(paths, PACKAGE_PATTERNS),
    llama=matches_any(paths, LLAMA_PATTERNS),
  )


def matches_any(paths: list[str], patterns: tuple[str, ...]) -> bool:
  normalized_paths = [path.replace("\\", "/") for path in paths]
  return any(
    fnmatch.fnmatch(path, pattern)
    for path in normalized_paths
    for pattern in patterns
  )


def bool_output(value: bool) -> str:
  return "true" if value else "false"


def read_changed_paths(path: Path) -> list[str]:
  return [
    line.strip()
    for line in path.read_text(encoding="utf-8").splitlines()
    if line.strip()
  ]


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--changed-files", type=Path)
  parser.add_argument("--github-output", type=Path)
  parser.add_argument("--all", action="store_true")
  args = parser.parse_args()

  changed_paths = read_changed_paths(args.changed_files) if args.changed_files else []
  changes = classify_changed_paths(changed_paths, force_all=args.all)
  output = "\n".join(changes.output_lines()) + "\n"
  if args.github_output:
    with args.github_output.open("a", encoding="utf-8") as file:
      file.write(output)
  else:
    print(output, end="")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
