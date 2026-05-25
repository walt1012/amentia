#!/usr/bin/env python3
"""Unit checks for macOS distribution validators that do not require signing tools."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from validate_macos_distribution import validate_package_manifest


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


def write_manifest(app_path: Path, signing: str, source_commit: str) -> None:
  manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
  manifest_path.parent.mkdir(parents=True)
  manifest_path.write_text(
    json.dumps(
      {
        "signing": signing,
        "sourceCommit": source_commit,
      }
    ),
    encoding="utf-8",
  )


def main() -> int:
  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    validate_package_manifest(app_path)

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "ad-hoc", SOURCE_COMMIT)
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require developer-id signing metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", "development")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require full source commit metadata",
    )

  print("macOS distribution validator tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
