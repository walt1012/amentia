#!/usr/bin/env python3
"""Unit checks for release artifact sidecar helpers."""

from __future__ import annotations

import tempfile
from pathlib import Path

from release_artifacts import (
  checksum_text,
  validate_checksum_file,
  write_checksum_file,
)


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


def main() -> int:
  with tempfile.TemporaryDirectory(prefix="pith-release-artifacts-") as root:
    root_path = Path(root)
    artifact = root_path / "Pith-v0.1.0-macos-x86_64.dmg"
    artifact.write_bytes(b"pith release artifact\n")

    text = checksum_text(artifact)
    if not text.endswith(f"  {artifact.name}\n"):
      raise AssertionError("checksum should reference the artifact basename")
    if str(root_path) in text:
      raise AssertionError("checksum should not expose runner-local paths")

    checksum_path = write_checksum_file(artifact)
    validate_checksum_file(artifact, checksum_path)

    artifact.write_bytes(b"tampered release artifact\n")
    assert_raises(
      lambda: validate_checksum_file(artifact, checksum_path),
      "tampered release artifact should fail checksum validation",
    )

  print("release artifact tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
