#!/usr/bin/env python3
"""Validate accepted milestone evidence files."""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from installed_app_proof import load_json_object as load_installed_app_json
from installed_app_proof import validate_installed_app_evidence
from reference_connector_proof import load_json_object as load_reference_connector_json
from reference_connector_proof import validate_reference_connector_evidence


INSTALLED_APP_EVIDENCE = "docs/evidence/m14-installed-app-proof.json"
REFERENCE_CONNECTOR_EVIDENCE = "docs/evidence/m14-reference-connector-proof.json"


@dataclass(frozen=True)
class EvidenceSpec:
  relative_path: str
  load_json: Callable[[Path], dict[str, object]]
  validate: Callable[[dict[str, object]], None]


@dataclass(frozen=True)
class EvidenceValidationResult:
  validated_files: list[str]


EVIDENCE_SPECS = (
  EvidenceSpec(
    relative_path=INSTALLED_APP_EVIDENCE,
    load_json=load_installed_app_json,
    validate=validate_installed_app_evidence,
  ),
  EvidenceSpec(
    relative_path=REFERENCE_CONNECTOR_EVIDENCE,
    load_json=load_reference_connector_json,
    validate=validate_reference_connector_evidence,
  ),
)
KNOWN_EVIDENCE_PATHS = {spec.relative_path for spec in EVIDENCE_SPECS}


def validate_milestone_evidence(
  root: Path,
  *,
  require_all: bool = False,
) -> EvidenceValidationResult:
  evidence_dir = root / "docs" / "evidence"
  if not evidence_dir.is_dir():
    raise RuntimeError(f"milestone evidence directory is missing: {evidence_dir}")

  reject_unknown_json_evidence(root, evidence_dir)

  validated_files: list[str] = []
  for spec in EVIDENCE_SPECS:
    path = root / spec.relative_path
    if path.is_file():
      spec.validate(spec.load_json(path))
      validated_files.append(spec.relative_path)
    elif require_all:
      raise RuntimeError(f"required milestone evidence is missing: {spec.relative_path}")

  return EvidenceValidationResult(validated_files=validated_files)


def reject_unknown_json_evidence(root: Path, evidence_dir: Path) -> None:
  for path in sorted(evidence_dir.glob("*.json")):
    relative_path = path.relative_to(root).as_posix()
    if relative_path not in KNOWN_EVIDENCE_PATHS:
      known = ", ".join(sorted(KNOWN_EVIDENCE_PATHS))
      raise RuntimeError(
        f"unsupported milestone evidence file: {relative_path}. Known evidence files: {known}"
      )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--root", type=Path, default=Path("."))
  parser.add_argument(
    "--require-all",
    action="store_true",
    help="Require every known M14 evidence file to exist and validate.",
  )
  return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
  args = parse_args(argv)
  try:
    result = validate_milestone_evidence(args.root, require_all=args.require_all)
  except Exception as error:
    print(f"milestone evidence validation failed: {error}", file=sys.stderr)
    return 1

  if result.validated_files:
    files = ", ".join(result.validated_files)
    print(f"milestone evidence validation passed: {files}")
  else:
    print("milestone evidence validation passed: no accepted evidence files found")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
