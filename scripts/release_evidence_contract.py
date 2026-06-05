#!/usr/bin/env python3
"""Validate internal release evidence artifacts before workflow upload."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from release_artifacts import release_installer_asset_names


DRY_RUN_EXTRA_NAMES = (
  "release-readiness.md",
  "release-readiness.json",
  "release-plan.md",
  "release-plan.json",
  "release-dry-run-rehearsal.md",
  "release-dry-run-rehearsal.json",
  "release-dry-run-manual-acceptance.md",
)
PUBLISH_REHEARSAL_NAMES = (
  "release-readiness.md",
  "release-readiness.json",
  "release-plan.md",
  "release-plan.json",
  "release-rehearsal.md",
  "release-rehearsal.json",
  "release-manual-acceptance.md",
)


def expected_evidence_names(mode: str, tag: str) -> tuple[str, ...]:
  if mode == "dry-run":
    return release_installer_asset_names(tag) + DRY_RUN_EXTRA_NAMES
  if mode == "publish-rehearsal":
    return PUBLISH_REHEARSAL_NAMES
  raise RuntimeError(f"Unknown release evidence mode: {mode}")


def validate_release_evidence_set(
  *,
  mode: str,
  tag: str,
  evidence_paths: list[Path],
) -> None:
  if not evidence_paths:
    raise RuntimeError("Release evidence validation requires evidence files.")
  expected_names = set(expected_evidence_names(mode, tag))
  evidence_by_name: dict[str, Path] = {}
  for evidence_path in evidence_paths:
    path = evidence_path.resolve()
    validate_evidence_path(path)
    name = path.name
    if name in evidence_by_name:
      raise RuntimeError(f"Release evidence contains duplicate file: {name}")
    evidence_by_name[name] = path

  actual_names = set(evidence_by_name)
  missing = sorted(expected_names - actual_names)
  extra = sorted(actual_names - expected_names)
  if missing or extra:
    details: list[str] = []
    if missing:
      details.append("missing " + ", ".join(missing))
    if extra:
      details.append("extra " + ", ".join(extra))
    raise RuntimeError(
      "Release evidence set must exactly match the workflow contract: "
      + "; ".join(details)
    )

  for path in evidence_by_name.values():
    validate_evidence_content(path)


def validate_evidence_path(path: Path) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Release evidence file is missing: {path}")
  if path.name in {"", ".", ".."} or "/" in path.name or "\\" in path.name:
    raise RuntimeError(f"Release evidence names must be basenames: {path.name}")


def validate_evidence_content(path: Path) -> None:
  if path.stat().st_size == 0:
    raise RuntimeError(f"Release evidence file must not be empty: {path.name}")
  if path.suffix == ".json":
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
      raise RuntimeError(f"Release evidence JSON must be an object: {path.name}")
  elif path.suffix == ".md":
    text = path.read_text(encoding="utf-8").strip()
    if not text.startswith("# "):
      raise RuntimeError(f"Release evidence Markdown must start with a heading: {path.name}")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--mode", required=True, choices=("dry-run", "publish-rehearsal"))
  parser.add_argument("--tag", required=True)
  parser.add_argument("--evidence", action="append", default=[], type=Path)
  args = parser.parse_args()

  try:
    validate_release_evidence_set(
      mode=args.mode,
      tag=args.tag,
      evidence_paths=args.evidence,
    )
  except Exception as error:
    print(f"release evidence contract failed: {error}", file=sys.stderr)
    return 1

  print("Release evidence contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
