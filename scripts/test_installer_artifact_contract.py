#!/usr/bin/env python3
"""Unit checks for installer artifact set validation."""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory

from installer_artifact_contract import expected_installer_asset_names
from installer_artifact_contract import installer_asset_paths
from installer_artifact_contract import installer_asset_paths_from_directory
from installer_artifact_contract import validate_installer_asset_set
from release_artifacts import write_checksum_file
from release_artifacts import write_release_manifest
from release_text import install_guide as release_install_guide


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
WORKFLOW_RUN_ID = "123456789"
WORKFLOW_RUN_URL = "https://github.com/walt1012/amentia/actions/runs/123456789"


def write_bytes(path: Path, content: bytes = b"amentia installer asset\n") -> Path:
  path.write_bytes(content)
  return path


def valid_assets(root: Path, tag: str) -> list[Path]:
  names = sorted(expected_installer_asset_names(tag))
  dmg_name = next(name for name in names if name.endswith(".dmg"))
  checksum_name = f"{dmg_name}.sha256"
  guide_name = "README-FIRST.txt"
  manifest_name = next(name for name in names if name.endswith("-manifest.json"))
  artifact = write_bytes(root / dmg_name)
  guide = root / guide_name
  guide.write_text(release_install_guide(tag, "ad-hoc"), encoding="utf-8")
  checksum = write_checksum_file(artifact, root / checksum_name)
  write_release_manifest(
    tag=tag,
    source_commit=SOURCE_COMMIT,
    signing_mode="ad-hoc",
    artifact_path=artifact,
    checksum_path=checksum,
    install_guide_path=guide,
    output_path=root / manifest_name,
    workflow_run_id=WORKFLOW_RUN_ID,
    workflow_run_url=WORKFLOW_RUN_URL,
  )
  return [
    root / name
    for name in names
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
  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    validate_installer_asset_set("v0.1.0", valid_assets(root, "v0.1.0"))
    validate_installer_asset_set(
      "v0.1.0",
      installer_asset_paths_from_directory("v0.1.0", root),
    )
    validate_installer_asset_set(
      "v0.1.0",
      installer_asset_paths(
        tag="v0.1.0",
        asset_paths=[],
        asset_dir=root,
      ),
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    valid_assets(root, "v0.1.0")
    write_bytes(root / "unexpected.txt")
    assert_raises(
      lambda: installer_asset_paths_from_directory("v0.1.0", root),
      "must not include extra entries",
    )
    validate_installer_asset_set(
      "v0.1.0",
      installer_asset_paths_from_directory(
        "v0.1.0",
        root,
        allow_extra_assets=True,
      ),
    )
    validate_installer_asset_set(
      "v0.1.0",
      installer_asset_paths(
        tag="v0.1.0",
        asset_paths=[],
        asset_dir=root,
        allow_extra_assets=True,
      ),
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    assert_raises(
      lambda: installer_asset_paths(
        tag="v0.1.0",
        asset_paths=assets,
        asset_dir=root,
      ),
      "Use either --asset-dir",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    assert_raises(
      lambda: installer_asset_paths(
        tag="v0.1.0",
        asset_paths=assets,
        asset_dir=None,
        allow_extra_assets=True,
      ),
      "--allow-extra-assets only applies to --asset-dir",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    validate_installer_asset_set("ci-0123456789ab", valid_assets(root, "ci-0123456789ab"))

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    missing_manifest = [
      asset
      for asset in assets
      if asset.name != "Amentia-v0.1.0-release-manifest.json"
    ]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", missing_manifest),
      "missing Amentia-v0.1.0-release-manifest.json",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0") + [write_bytes(root / "Amentia-v0.1.0-macos-x86_64.zip")]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "must not include .zip payloads",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0") + [write_bytes(root / "internal-release-notes.md")]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "must not include internal notes",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0") + [write_bytes(root / "MiniCPM5-1B-Q4_K_M.gguf")]
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "must not include .gguf payloads",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    assets[0] = write_bytes(root / "Amentia-v0.2.0-macos-x86_64.dmg")
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "missing Amentia-v0.1.0-macos-x86_64.dmg",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets + [assets[0]]),
      "duplicate asset",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    missing_file = root / "README-FIRST.txt"
    missing_file.unlink()
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "Installer asset is missing",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    (root / "Amentia-v0.1.0-macos-x86_64.dmg").write_bytes(b"tampered dmg\n")
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "Release checksum does not match artifact",
    )

  with TemporaryDirectory(prefix="amentia-installer-assets-") as directory:
    root = Path(directory)
    assets = valid_assets(root, "v0.1.0")
    (root / "README-FIRST.txt").write_text("Install Amentia.\n", encoding="utf-8")
    assert_raises(
      lambda: validate_installer_asset_set("v0.1.0", assets),
      "Release manifest install guide size does not match",
    )

  print("installer artifact contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
