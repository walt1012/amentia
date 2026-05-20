#!/usr/bin/env python3
"""Validate a macOS app bundle before public distribution."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


DEVELOPER_ID_MARKER = "Authority=Developer ID Application:"


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("app_path", type=Path, help="Path to Pith.app.")
  return parser.parse_args()


def main() -> int:
  args = parse_args()
  app_path = args.app_path.resolve()
  try:
    require_file(app_path / "Contents" / "Info.plist", "Info.plist")
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
