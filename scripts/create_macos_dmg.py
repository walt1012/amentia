#!/usr/bin/env python3
"""Create and validate the user-facing macOS DMG installer for Pith."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from package_contract import DEFAULT_MODEL_ID

APP_NAME = "Pith.app"
APPLICATIONS_LINK_NAME = "Applications"
DEFAULT_VOLUME_NAME = "Pith"
README_NAME = "README-FIRST.txt"
REQUIRED_README_PHRASES = (
  "Launch Pith and download one verified local model when prompted",
  DEFAULT_MODEL_ID,
  "Open a workspace folder.",
  "Start a cowork session",
  "Follow the next action",
  "runtime readiness",
  "package size budget",
)


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("app_path", type=Path, help="Path to Pith.app.")
  parser.add_argument("dmg_path", type=Path, help="Output DMG path.")
  parser.add_argument(
    "--volume-name",
    default=DEFAULT_VOLUME_NAME,
    help="Mounted disk image volume name.",
  )
  parser.add_argument(
    "--smoke-launch-script",
    type=Path,
    help="Optional packaged app smoke script to run against the app inside the mounted DMG.",
  )
  parser.add_argument(
    "--readme-file",
    type=Path,
    help="Optional plain-text install guide to copy into the DMG root.",
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
      readme_file = args.readme_file.resolve() if args.readme_file else None
      stage_dmg_contents(
        app_path,
        staging_dir,
        readme_file,
      )
      create_dmg(staging_dir, dmg_path, args.volume_name)
    validate_dmg(
      dmg_path,
      args.smoke_launch_script.resolve() if args.smoke_launch_script else None,
      readme_file,
    )
  except Exception as error:
    print(f"macOS DMG creation failed: {error}", file=sys.stderr)
    return 1

  print(f"Created macOS DMG: {dmg_path}")
  return 0


def require_tool(name: str) -> None:
  if shutil.which(name) is None:
    raise FileNotFoundError(f"Required macOS packaging tool is missing: {name}")


def require_file(path: Path) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Required file is missing: {path}")


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


def stage_dmg_contents(app_path: Path, staging_dir: Path, readme_file: Path | None) -> None:
  copy_app_bundle(app_path, staging_dir / APP_NAME)
  applications_link = staging_dir / APPLICATIONS_LINK_NAME
  applications_link.symlink_to("/Applications", target_is_directory=True)
  if readme_file is not None:
    require_file(readme_file)
    validate_install_readme_text(readme_file.read_text(encoding="utf-8"))
    shutil.copy2(readme_file, staging_dir / README_NAME)


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


def validate_dmg(
  dmg_path: Path,
  smoke_launch_script: Path | None = None,
  readme_file: Path | None = None,
) -> None:
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
      if readme_file is not None:
        require_file(readme_file)
        validate_install_readme_file(mountpoint / README_NAME, readme_file)
      if smoke_launch_script is not None:
        require_file(smoke_launch_script)
        run([sys.executable, str(smoke_launch_script), str(mountpoint / APP_NAME)])
    finally:
      if attached:
        run(["hdiutil", "detach", str(mountpoint)])


def validate_install_readme_file(staged_readme: Path, expected_readme: Path) -> None:
  require_file(staged_readme)
  require_file(expected_readme)
  staged_text = staged_readme.read_text(encoding="utf-8")
  expected_text = expected_readme.read_text(encoding="utf-8")
  validate_install_readme_text(staged_text)
  if staged_text != expected_text:
    raise RuntimeError("macOS DMG README-FIRST.txt does not match the generated install guide")


def validate_install_readme_text(text: str) -> None:
  for phrase in REQUIRED_README_PHRASES:
    if phrase not in text:
      raise RuntimeError(f"macOS DMG install guide is missing: {phrase}")


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
