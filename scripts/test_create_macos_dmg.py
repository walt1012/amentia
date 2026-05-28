#!/usr/bin/env python3
"""Unit checks for macOS DMG staging helpers that do not require macOS."""

from __future__ import annotations

import tempfile
from pathlib import Path

import create_macos_dmg
from create_macos_dmg import (
  APPLICATIONS_LINK_NAME,
  APP_NAME,
  README_NAME,
  detach_dmg,
  stage_dmg_contents,
  validate_install_readme_text,
)
from release_text import install_guide


def require(condition: bool, message: str) -> None:
  if not condition:
    raise AssertionError(message)


def create_fake_app(path: Path) -> None:
  contents = path / "Contents"
  contents.mkdir(parents=True)
  (contents / "Info.plist").write_text("plist", encoding="utf-8")


def can_create_symlink(root: Path) -> bool:
  link_path = root / "link"
  try:
    link_path.symlink_to("/Applications", target_is_directory=True)
  except OSError:
    return False
  link_path.unlink()
  return True


def validate_detach_force_fallback() -> None:
  calls: list[list[str]] = []
  mountpoint = Path("pith-mount")

  def fake_run(command: list[str]) -> str:
    calls.append(command)
    if command == ["hdiutil", "detach", str(mountpoint)]:
      raise RuntimeError("busy")
    return ""

  original_run = create_macos_dmg.run
  try:
    create_macos_dmg.run = fake_run
    detach_dmg(mountpoint)
  finally:
    create_macos_dmg.run = original_run

  require(
    calls
    == [
      ["hdiutil", "detach", str(mountpoint)],
      ["hdiutil", "detach", "-force", str(mountpoint)],
    ],
    "DMG detach should force-detach after a normal detach failure",
  )


def main() -> int:
  validate_install_readme_text(install_guide("v0.1.0", "ad-hoc"))
  validate_install_readme_text(install_guide("ci-0123456789ab", "ad-hoc"))
  validate_detach_force_fallback()
  with tempfile.TemporaryDirectory(prefix="pith-dmg-stage-test-") as root:
    root_path = Path(root)
    if not can_create_symlink(root_path):
      print("Skipping DMG staging helper tests because symlink creation is unavailable.")
      return 0

    app_path = root_path / APP_NAME
    staging_dir = root_path / "staging"
    readme_file = root_path / "install-readme.txt"
    create_fake_app(app_path)
    staging_dir.mkdir()
    readme_file.write_text(install_guide("v0.1.0", "ad-hoc"), encoding="utf-8")

    stage_dmg_contents(app_path, staging_dir, readme_file)

    require((staging_dir / APP_NAME / "Contents" / "Info.plist").is_file(), "app was not staged")
    applications_link = staging_dir / APPLICATIONS_LINK_NAME
    require(applications_link.is_symlink(), "Applications shortcut was not staged")
    require(
      applications_link.readlink() == Path("/Applications"),
      "Applications shortcut points to the wrong target",
    )
    require(
      (staging_dir / README_NAME).read_text(encoding="utf-8")
      == install_guide("v0.1.0", "ad-hoc"),
      "install readme was not staged",
    )

  print("DMG staging helper tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
