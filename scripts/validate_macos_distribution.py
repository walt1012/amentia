#!/usr/bin/env python3
"""Validate a macOS app bundle before public distribution."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


DEVELOPER_ID_MARKER = "Authority=Developer ID Application:"
PACKAGE_MANIFEST_RELATIVE_PATH = Path("Contents/Resources/PithPackage.json")
SOURCE_COMMIT_HEX_LENGTH = 40


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("app_path", type=Path, help="Path to Pith.app.")
  parser.add_argument(
    "--dmg-path",
    type=Path,
    help="Optional notarized DMG artifact to validate for public release.",
  )
  return parser.parse_args()


def main() -> int:
  args = parse_args()
  app_path = args.app_path.resolve()
  try:
    require_file(app_path / "Contents" / "Info.plist", "Info.plist")
    validate_package_manifest(app_path)
    require_tool("codesign")
    require_tool("spctl")
    run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app_path)])
    signature = run(["codesign", "-dv", "--verbose=4", str(app_path)])
    if DEVELOPER_ID_MARKER not in signature:
      raise RuntimeError(
        "Pith.app is not signed with a Developer ID Application identity. "
        "Ad-hoc signed builds are valid for CI only, not public distribution."
      )
    run(["spctl", "--assess", "--type", "execute", "--verbose=4", str(app_path)])
    if args.dmg_path:
      validate_dmg(args.dmg_path.resolve())
  except Exception as error:
    print(f"macOS distribution validation failed: {error}", file=sys.stderr)
    return 1
  print("macOS distribution validation passed.")
  return 0


def require_file(path: Path, label: str) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Missing {label}: {path}")


def require_tool(name: str) -> None:
  if shutil.which(name) is None:
    raise FileNotFoundError(f"Required distribution validation tool is missing: {name}")


def validate_dmg(dmg_path: Path) -> None:
  require_file(dmg_path, "macOS release DMG")
  require_tool("xcrun")
  run(["codesign", "--verify", "--verbose=2", str(dmg_path)])
  run(
    [
      "spctl",
      "--assess",
      "--type",
      "open",
      "--context",
      "context:primary-signature",
      "--verbose=4",
      str(dmg_path),
    ]
  )
  run(["xcrun", "stapler", "validate", str(dmg_path)])


def validate_package_manifest(app_path: Path) -> None:
  manifest_path = app_path / PACKAGE_MANIFEST_RELATIVE_PATH
  require_file(manifest_path, "PithPackage.json")
  manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
  if manifest.get("schemaVersion") != 1:
    raise RuntimeError(
      "Public distribution builds must record PithPackage schema version 1 in "
      f"{manifest_path}"
    )
  if manifest.get("signing") != "developer-id":
    raise RuntimeError(
      "Public distribution builds must record developer-id signing in "
      f"{manifest_path}"
    )
  expected_sandbox_values = {
    "sandboxMode": "workspaceReadWrite",
    "sandboxBackend": "runtime-detected",
    "sandboxFallback": "processOnlyWhenNativeUnavailable",
    "sandboxNetworkDefault": "disabled",
  }
  for field, expected in expected_sandbox_values.items():
    if manifest.get(field) != expected:
      raise RuntimeError(
        f"Public distribution builds must record {field} as {expected} in "
        f"{manifest_path}"
      )
  source_commit = manifest.get("sourceCommit", "")
  if (
    not isinstance(source_commit, str)
    or len(source_commit) != SOURCE_COMMIT_HEX_LENGTH
    or any(character not in "0123456789abcdef" for character in source_commit)
  ):
    raise RuntimeError(
      "Public distribution builds must record a full source commit in "
      f"{manifest_path}"
    )


def run(command: list[str]) -> str:
  completed = subprocess.run(
    command,
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
  )
  if completed.returncode != 0:
    output = completed.stdout.strip()
    detail = f": {output}" if output else ""
    raise RuntimeError(f"command failed with status {completed.returncode}: {' '.join(command)}{detail}")
  return completed.stdout


if __name__ == "__main__":
  raise SystemExit(main())
