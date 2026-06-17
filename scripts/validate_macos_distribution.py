#!/usr/bin/env python3
"""Validate a macOS app bundle before public distribution."""

from __future__ import annotations

import argparse
import json
import shutil
import struct
import subprocess
import sys
from pathlib import Path

from package_contract import (
  APP_NAME,
  PACKAGE_MANIFEST_SCHEMA_VERSION,
  bundled_model_weight_files,
  directory_size_bytes,
  validate_package_manifest_contract,
)


DEVELOPER_ID_MARKER = "Authority=Developer ID Application:"
PACKAGE_MANIFEST_RELATIVE_PATH = Path("Contents/Resources/PithPackage.json")
APP_ICON_RELATIVE_PATH = Path(f"Contents/Resources/{APP_NAME}.icns")
SOURCE_COMMIT_HEX_LENGTH = 40


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("app_path", type=Path, help=f"Path to {APP_NAME}.app.")
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
    validate_app_icon(app_path)
    validate_package_manifest(app_path)
    require_tool("codesign")
    require_tool("spctl")
    run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app_path)])
    signature = run(["codesign", "-dv", "--verbose=4", str(app_path)])
    if DEVELOPER_ID_MARKER not in signature:
      raise RuntimeError(
        f"{APP_NAME}.app is not signed with a Developer ID Application identity. "
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


def validate_app_icon(app_path: Path) -> None:
  icon_path = app_path / APP_ICON_RELATIVE_PATH
  require_file(icon_path, f"{APP_NAME}.icns")
  data = icon_path.read_bytes()
  if len(data) < 8 or data[:4] != b"icns":
    raise RuntimeError(f"{APP_NAME}.icns must be an ICNS file: {icon_path}")
  declared_size = struct.unpack(">I", data[4:8])[0]
  actual_size = icon_path.stat().st_size
  if declared_size != actual_size:
    raise RuntimeError(
      f"{APP_NAME}.icns size header must match file size: {icon_path}: "
      f"{declared_size} != {actual_size}"
    )


def validate_package_manifest(app_path: Path) -> None:
  manifest_path = app_path / PACKAGE_MANIFEST_RELATIVE_PATH
  require_file(manifest_path, "PithPackage.json")
  manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
  if manifest.get("schemaVersion") != PACKAGE_MANIFEST_SCHEMA_VERSION:
    raise RuntimeError(
      "Public distribution builds must record PithPackage schema version "
      f"{PACKAGE_MANIFEST_SCHEMA_VERSION} in "
      f"{manifest_path}"
    )
  if manifest.get("signing") != "developer-id":
    raise RuntimeError(
      "Public distribution builds must record developer-id signing in "
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
  budget = validate_package_manifest_contract(
    manifest,
    f"Public distribution PithPackage.json: {manifest_path}",
    signing_mode="developer-id",
  )
  validate_distribution_size_budget(budget, app_path)
  validate_no_model_weight_files(app_path)


def validate_distribution_size_budget(
  budget: dict[str, int],
  app_path: Path,
) -> None:
  max_app_bundle_bytes = budget["maxAppBundleBytes"]
  app_size = directory_size_bytes(app_path)
  if app_size > max_app_bundle_bytes:
    raise RuntimeError(
      f"Public distribution app bundle is {app_size} bytes, above the "
      f"{max_app_bundle_bytes} byte package budget."
    )

def validate_no_model_weight_files(app_path: Path) -> None:
  bundled_weights = bundled_model_weight_files(app_path)
  if bundled_weights:
    raise RuntimeError(
      "Public distribution builds must not bundle model weight files: "
      + ", ".join(str(path) for path in bundled_weights)
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
