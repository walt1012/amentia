#!/usr/bin/env python3
"""Unit checks for installer artifact set validation."""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory

from installer_artifact_contract import expected_installer_asset_names
from installer_artifact_contract import validate_installer_asset_set


def touch(path: Path) -> Path:
  path.write_bytes(b"pith installer asset\n")
  return path


def valid_assets(root: Path, tag: str) -> list[Path]:
  return [
    touch(root / name)
    for name in sorted(expected_installer_asset_names(tag))
  ]


def assert_raises(action, expected: str) -> None:
  try:
    action()
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r}, got {error!r}") from error
    return
  raise AssertionError(f"expected failure containing {expected!r}")


def main() -> int:
  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    validate_installer_asset_set("v0.1.0", valid_assets(root, "v0.1.0"))

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    validate_installer_asset_set("ci-0123456789ab", valid_assets(root, "ci-0123456789ab"))

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    missing_manifest = [
      asset
      for asset in assets
      if asset.name != "Pith-v0.1.0-release-manifest.json"
    ]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", missing_manifest),
      "missing Pith-v0.1.0-release-manifest.json",
    )

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0") + [touch(root / "Pith-v0.1.0-macos-x86_64.zip")]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "must not include .zip payloads",
    )

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0") + [touch(root / "internal-release-notes.md")]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "must not include internal notes",
    )

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0") + [touch(root / "MiniCPM5-1B-Q4_K_M.gguf")]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "must not include .gguf payloads",
    )

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    assets[0] = touch(root / "Pith-v0.2.0-macos-x86_64.dmg")
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "missing Pith-v0.1.0-macos-x86_64.dmg",
    )

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets + [assets[0]]),
      "duplicate asset",
    )

  with TemporaryDirectory(prefix="pith-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    missing_file = root / "README-FIRST.txt"
    missing_file.unlink()
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "Installer asset is missing",
    )

  print("installer artifact contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
