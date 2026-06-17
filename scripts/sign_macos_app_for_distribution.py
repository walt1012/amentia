#!/usr/bin/env python3
"""Sign Amentia.app for Developer ID distribution."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


MACHO_MAGICS = {
  b"\xca\xfe\xba\xbe",
  b"\xbe\xba\xfe\xca",
  b"\xfe\xed\xfa\xce",
  b"\xce\xfa\xed\xfe",
  b"\xfe\xed\xfa\xcf",
  b"\xcf\xfa\xed\xfe",
}


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("app_path", type=Path, help="Path to Amentia.app.")
  parser.add_argument(
    "--identity",
    required=True,
    help="Developer ID Application signing identity.",
  )
  parser.add_argument(
    "--entitlements",
    type=Path,
    help="Optional app entitlements plist.",
  )
  return parser.parse_args()


def main() -> int:
  args = parse_args()
  app_path = args.app_path.resolve()
  entitlements = args.entitlements.resolve() if args.entitlements else None

  try:
    require_tool("codesign")
    require_app(app_path)
    if entitlements is not None and not entitlements.is_file():
      raise FileNotFoundError(f"Missing entitlements plist: {entitlements}")
    for target in nested_code_targets(app_path):
      sign_path(target, args.identity)
    sign_path(app_path, args.identity, entitlements)
    verify_signature(app_path)
  except Exception as error:
    print(f"macOS distribution signing failed: {error}", file=sys.stderr)
    return 1

  print(f"Signed macOS app for distribution: {app_path}")
  return 0


def require_tool(name: str) -> None:
  if shutil.which(name) is None:
    raise FileNotFoundError(f"Required signing tool is missing: {name}")


def require_app(app_path: Path) -> None:
  if not app_path.is_dir():
    raise FileNotFoundError(f"Missing app bundle: {app_path}")
  info_plist = app_path / "Contents" / "Info.plist"
  if not info_plist.is_file():
    raise FileNotFoundError(f"Missing app Info.plist: {info_plist}")


def nested_code_targets(app_path: Path) -> list[Path]:
  contents_path = app_path / "Contents"
  candidates: list[Path] = []
  for path in sorted(contents_path.rglob("*")):
    if path.is_file() and is_macho(path):
      candidates.append(path)
  return candidates


def is_macho(path: Path) -> bool:
  try:
    with path.open("rb") as handle:
      return handle.read(4) in MACHO_MAGICS
  except OSError:
    return False


def sign_path(path: Path, identity: str, entitlements: Path | None = None) -> None:
  command = [
    "codesign",
    "--force",
    "--timestamp",
    "--options",
    "runtime",
    "--sign",
    identity,
  ]
  if entitlements is not None:
    command.extend(["--entitlements", str(entitlements)])
  command.append(str(path))
  run(command)


def verify_signature(app_path: Path) -> None:
  run(
    [
      "codesign",
      "--verify",
      "--deep",
      "--strict",
      "--verbose=2",
      str(app_path),
    ]
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
