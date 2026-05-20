#!/usr/bin/env python3
"""Create and validate the user-facing macOS DMG installer for Pith."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


APP_NAME = "Pith.app"
APPLICATIONS_LINK_NAME = "Applications"
DEFAULT_VOLUME_NAME = "Pith"


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("app_path", type=Path, help="Path to Pith.app.")
  parser.add_argument("dmg_path", type=Path, help="Output DMG path.")
  parser.add_argument(
    "--volume-name",
    default=DEFAULT_VOLUME_NAME,
    help="Mounted disk image volume name.",
  )
  return parser.parse_args()


def main() -> int:
  args = parse_args()
  app_path = args.app_path.resolve()
  dmg_path = args.dmg_path.resolve()

  try:
    require_tool("hdiutil")
    require_app(app_path)
    dmg_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="pith-dmg-") as temp_dir:
      staging_dir = Path(temp_dir) / "staging"
      staging_dir.mkdir()
      copy_app_bundle(app_path, staging_dir / APP_NAME)
      applications_link = staging_dir / APPLICATIONS_LINK_NAME
      applications_link.symlink_to("/Applications", target_is_directory=True)
      create_dmg(staging_dir, dmg_path, args.volume_name)
    validate_dmg(dmg_path)
  except Exception as error:
    print(f"macOS DMG creation failed: {error}", file=sys.stderr)
    return 1

  print(f"Created macOS DMG: {dmg_path}")
  return 0


def require_tool(name: str) -> None:
  if shutil.which(name) is None:
    raise FileNotFoundError(f"Required macOS packaging tool is missing: {name}")


def require_app(app_path: Path) -> None:
  if not app_path.is_dir():
    raise FileNotFoundError(f"Missing app bundle: {app_path}")
  info_plist = app_path / "Contents" / "Info.plist"
  if not info_plist.is_file():
    raise FileNotFoundError(f"Missing app Info.plist: {info_plist}")


def copy_app_bundle(source: Path, destination: Path) -> None:
  if shutil.which("ditto") is not None:
    run(["ditto", str(source), str(destination)])
    return
  shutil.copytree(source, destination, symlinks=False)


def create_dmg(staging_dir: Path, dmg_path: Path, volume_name: str) -> None:
  if dmg_path.exists():
    dmg_path.unlink()
  run(
    [
      "hdiutil",
      "create",
      "-volname",
      volume_name,
      "-srcfolder",
      str(staging_dir),
      "-ov",
      "-format",
      "UDZO",
      "-fs",
      "HFS+",
      str(dmg_path),
    ]
  )


def validate_dmg(dmg_path: Path) -> None:
  if not dmg_path.is_file():
    raise FileNotFoundError(f"Missing DMG artifact: {dmg_path}")
  if dmg_path.suffix.lower() != ".dmg":
    raise RuntimeError(f"macOS installer artifact must be a DMG file: {dmg_path}")
  if dmg_path.stat().st_size <= 0:
    raise RuntimeError(f"macOS DMG artifact is empty: {dmg_path}")

  run(["hdiutil", "imageinfo", str(dmg_path)])
  with tempfile.TemporaryDirectory(prefix="pith-dmg-mount-") as temp_dir:
    mountpoint = Path(temp_dir) / "mount"
    mountpoint.mkdir()
    attached = False
    try:
      run(
        [
          "hdiutil",
          "attach",
          "-nobrowse",
          "-readonly",
          "-mountpoint",
          str(mountpoint),
          str(dmg_path),
        ]
      )
      attached = True
      require_app(mountpoint / APP_NAME)
      applications_link = mountpoint / APPLICATIONS_LINK_NAME
      if not applications_link.is_symlink():
        raise RuntimeError("macOS DMG must include an Applications symlink")
      if applications_link.readlink() != Path("/Applications"):
        raise RuntimeError("macOS DMG Applications symlink must point to /Applications")
    finally:
      if attached:
        run(["hdiutil", "detach", str(mountpoint)])


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
