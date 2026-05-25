#!/usr/bin/env python3
"""Unit checks for macOS DMG staging helpers that do not require macOS."""

from __future__ import annotations

import tempfile
from pathlib import Path

from create_macos_dmg import APPLICATIONS_LINK_NAME, APP_NAME, stage_dmg_contents


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


def main() -> int:
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
    readme_file.write_text("Install Pith by dragging it to Applications.", encoding="utf-8")

    stage_dmg_contents(app_path, staging_dir, readme_file)

    require((staging_dir / APP_NAME / "Contents" / "Info.plist").is_file(), "app was not staged")
    applications_link = staging_dir / APPLICATIONS_LINK_NAME
    require(applications_link.is_symlink(), "Applications shortcut was not staged")
    require(
      applications_link.readlink() == Path("/Applications"),
      "Applications shortcut points to the wrong target",
    )
    require(
      (staging_dir / "README-FIRST.txt").read_text(encoding="utf-8")
      == "Install Pith by dragging it to Applications.",
      "install readme was not staged",
    )

  print("DMG staging helper tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
