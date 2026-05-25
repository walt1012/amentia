#!/usr/bin/env python3
"""Prepare and validate user-facing release artifact sidecar files."""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path


def sha256_hex(path: Path) -> str:
  hasher = hashlib.sha256()
  with path.open("rb") as file:
    for chunk in iter(lambda: file.read(1024 * 1024), b""):
      hasher.update(chunk)
  return hasher.hexdigest()


def checksum_text(artifact_path: Path) -> str:
  if not artifact_path.is_file():
    raise FileNotFoundError(f"Release artifact is missing: {artifact_path}")
  return f"{sha256_hex(artifact_path)}  {artifact_path.name}\n"


def write_checksum_file(artifact_path: Path, checksum_path: Path | None = None) -> Path:
  output_path = checksum_path or artifact_path.with_name(f"{artifact_path.name}.sha256")
  output_path.parent.mkdir(parents=True, exist_ok=True)
  output_path.write_text(checksum_text(artifact_path), encoding="utf-8")
  validate_checksum_file(artifact_path, output_path)
  return output_path


def validate_checksum_file(artifact_path: Path, checksum_path: Path) -> None:
  if not checksum_path.is_file():
    raise FileNotFoundError(f"Release checksum is missing: {checksum_path}")

  fields = checksum_path.read_text(encoding="utf-8").strip().split()
  if len(fields) != 2:
    raise RuntimeError(f"Release checksum must contain hash and file name: {checksum_path}")

  expected_hash, expected_name = fields
  if expected_name != artifact_path.name:
    raise RuntimeError(
      "Release checksum must reference only the artifact file name: "
      f"{expected_name} != {artifact_path.name}"
    )
  if expected_hash != sha256_hex(artifact_path):
    raise RuntimeError(f"Release checksum does not match artifact: {checksum_path}")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--artifact", required=True, type=Path)
  parser.add_argument("--checksum-output", type=Path)
  args = parser.parse_args()

  try:
    checksum_path = write_checksum_file(
      args.artifact.resolve(),
      args.checksum_output.resolve() if args.checksum_output else None,
    )
  except Exception as error:
    print(f"release artifact preparation failed: {error}", file=sys.stderr)
    return 1

  print(f"Created release checksum: {checksum_path}")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
