#!/usr/bin/env python3
"""Validate the exact installer asset set before upload, publish, or download rehearsal."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from release_artifacts import release_installer_asset_names
from release_artifacts import validate_release_manifest


FORBIDDEN_ASSET_NAMES = {
  "internal-release-notes.md",
  "release-notes.md",
}
FORBIDDEN_SUFFIXES = {
  ".gguf",
  ".safetensors",
  ".zip",
}


def expected_installer_asset_names(tag: str) -> frozenset[str]:
  return frozenset(release_installer_asset_names(tag))


def validate_installer_asset_set(tag: str, asset_paths: list[Path]) -> None:
  if not asset_paths:
    raise RuntimeError("Installer asset validation requires at least one asset")

  expected_names = expected_installer_asset_names(tag)
  assets_by_name: dict[str, Path] = {}
  for asset_path in asset_paths:
    resolved_path = asset_path.resolve()
    validate_installer_asset_path(resolved_path)
    name = resolved_path.name
    validate_installer_asset_name(name)
    if name in assets_by_name:
      raise RuntimeError(f"Installer asset set contains duplicate asset: {name}")
    assets_by_name[name] = resolved_path

  actual_names = set(assets_by_name)
  missing = sorted(expected_names - actual_names)
  extra = sorted(actual_names - expected_names)
  if missing or extra:
    details: list[str] = []
    if missing:
      details.append("missing " + ", ".join(missing))
    if extra:
      details.append("extra " + ", ".join(extra))
    raise RuntimeError("Installer asset set must exactly match the release contract: " + "; ".join(details))

  dmg_name, checksum_name, install_guide_name, manifest_name = release_installer_asset_names(tag)
  validate_release_manifest(
    assets_by_name[manifest_name],
    artifact_path=assets_by_name[dmg_name],
    checksum_path=assets_by_name[checksum_name],
    install_guide_path=assets_by_name[install_guide_name],
  )


def installer_asset_paths_from_directory(tag: str, asset_dir: Path) -> list[Path]:
  if not asset_dir.is_dir():
    raise NotADirectoryError(f"Installer asset directory is missing: {asset_dir}")
  return [
    asset_dir / name
    for name in sorted(expected_installer_asset_names(tag))
  ]


def installer_asset_paths(
  *,
  tag: str,
  asset_paths: list[Path],
  asset_dir: Path | None,
) -> list[Path]:
  if asset_dir is not None and asset_paths:
    raise RuntimeError("Use either --asset-dir or explicit --asset paths, not both")
  if asset_dir is not None:
    return installer_asset_paths_from_directory(tag, asset_dir)
  return asset_paths


def validate_installer_asset_path(asset_path: Path) -> None:
  if not asset_path.is_file():
    raise FileNotFoundError(f"Installer asset is missing: {asset_path}")


def validate_installer_asset_name(name: str) -> None:
  if not name or name in {".", ".."} or "/" in name or "\\" in name:
    raise RuntimeError(f"Installer asset names must be basenames: {name}")
  if name in FORBIDDEN_ASSET_NAMES:
    raise RuntimeError(f"Installer asset set must not include internal notes: {name}")
  suffix = Path(name).suffix
  if suffix in FORBIDDEN_SUFFIXES:
    raise RuntimeError(f"Installer asset set must not include {suffix} payloads: {name}")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--asset", action="append", default=[], type=Path)
  parser.add_argument("--asset-dir", type=Path)
  args = parser.parse_args()

  try:
    validate_installer_asset_set(
      args.tag,
      installer_asset_paths(
        tag=args.tag,
        asset_paths=args.asset,
        asset_dir=args.asset_dir,
      ),
    )
  except Exception as error:
    print(f"installer artifact contract failed: {error}", file=sys.stderr)
    return 1

  print("Installer artifact contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
