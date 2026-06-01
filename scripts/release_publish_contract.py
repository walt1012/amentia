#!/usr/bin/env python3
"""Validate the final GitHub Release state after publishing Pith assets."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from installer_artifact_contract import expected_installer_asset_names
from installer_artifact_contract import validate_installer_asset_name
from release_identity import validate_public_release_tag
from release_state import expected_release_title
from release_state import parse_bool


def validate_published_release(
  release: dict,
  *,
  tag: str,
  expected_draft: bool,
  expected_prerelease: bool,
) -> None:
  validate_public_release_tag(tag)
  validate_release_field(release, "tag_name", tag)
  validate_release_field(release, "name", expected_release_title(tag))
  validate_release_bool(release, "draft", expected_draft)
  validate_release_bool(release, "prerelease", expected_prerelease)
  validate_release_assets(release, tag)


def validate_release_field(release: dict, field: str, expected: str) -> None:
  actual = release.get(field)
  if actual != expected:
    raise RuntimeError(
      f"Published GitHub Release field {field} must be {expected!r}, got {actual!r}"
    )


def validate_release_bool(release: dict, field: str, expected: bool) -> None:
  actual = release.get(field)
  if actual is not expected:
    raise RuntimeError(
      f"Published GitHub Release field {field} must be {str(expected).lower()}, got {actual!r}"
    )


def validate_release_assets(release: dict, tag: str) -> None:
  actual_names = release_asset_names(release)
  expected_names = expected_installer_asset_names(tag)
  missing = sorted(expected_names - actual_names)
  extra = sorted(actual_names - expected_names)
  if missing or extra:
    details: list[str] = []
    if missing:
      details.append("missing " + ", ".join(missing))
    if extra:
      details.append("extra " + ", ".join(extra))
    raise RuntimeError(
      "Published GitHub Release assets must exactly match the installer contract: "
      + "; ".join(details)
    )


def release_asset_names(release: dict) -> frozenset[str]:
  assets = release.get("assets")
  if not isinstance(assets, list):
    raise RuntimeError("Published GitHub Release response must include an assets list")

  names: set[str] = set()
  for asset in assets:
    if not isinstance(asset, dict):
      raise RuntimeError("Published GitHub Release asset entries must be objects")
    name = asset.get("name")
    if not isinstance(name, str):
      raise RuntimeError("Published GitHub Release asset names must be strings")
    validate_installer_asset_name(name)
    if name in names:
      raise RuntimeError(f"Published GitHub Release has duplicate asset: {name}")
    names.add(name)
  return frozenset(names)


def load_release(path: Path) -> dict:
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError("Published GitHub Release response must be a JSON object")
  return data


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--release-json", required=True, type=Path)
  parser.add_argument("--expected-draft", required=True)
  parser.add_argument("--expected-prerelease", required=True)
  args = parser.parse_args()

  try:
    validate_published_release(
      load_release(args.release_json),
      tag=args.tag,
      expected_draft=parse_bool(args.expected_draft),
      expected_prerelease=parse_bool(args.expected_prerelease),
    )
  except Exception as error:
    print(f"published release contract failed: {error}", file=sys.stderr)
    return 1

  print("Published release contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
