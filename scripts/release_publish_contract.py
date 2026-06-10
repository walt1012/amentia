#!/usr/bin/env python3
"""Validate the final GitHub Release state after publishing Pith assets."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from installer_artifact_contract import expected_installer_asset_names
from installer_artifact_contract import validate_installer_asset_name
from release_artifacts import validate_source_commit
from release_identity import validate_public_release_tag
from package_contract import RELEASE_SIGNING_MODES
from release_state import expected_release_title
from release_state import parse_bool
from release_state import validate_manual_acceptance_evidence as validate_manual_acceptance_receipt_url
from release_text import validate_release_notes


RELEASE_REPOSITORY = "walt1012/pith"


def validate_published_release(
  release: dict,
  *,
  tag: str,
  source_commit: str,
  tag_commit: str,
  expected_draft: bool,
  expected_prerelease: bool,
  signing_mode: str,
  allow_untrusted_ad_hoc: bool,
  manual_acceptance_evidence: str = "",
) -> None:
  validate_public_release_tag(tag)
  validate_source_commit(source_commit)
  validate_source_commit(tag_commit)
  if tag_commit != source_commit:
    raise RuntimeError("Published GitHub Release tag must point at the source commit")
  validate_release_field(release, "tag_name", tag)
  validate_release_field(release, "name", expected_release_title(tag))
  validate_release_bool(release, "draft", expected_draft)
  validate_release_bool(release, "prerelease", expected_prerelease)
  validate_release_body(
    release,
    tag=tag,
    signing_mode=signing_mode,
    allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
    draft=expected_draft,
    manual_acceptance_evidence=manual_acceptance_evidence,
  )
  validate_release_assets(release, tag, draft=expected_draft)


def validate_existing_release_assets_before_upload(release: dict, *, tag: str) -> None:
  validate_public_release_tag(tag)
  actual_names = release_asset_names(release)
  expected_names = expected_installer_asset_names(tag)
  extra = sorted(actual_names - expected_names)
  if extra:
    raise RuntimeError(
      "Existing GitHub Release has non-contract assets before upload: "
      + ", ".join(extra)
    )


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


def validate_release_body(
  release: dict,
  *,
  tag: str,
  signing_mode: str,
  allow_untrusted_ad_hoc: bool,
  draft: bool,
  manual_acceptance_evidence: str = "",
) -> None:
  body = release.get("body")
  if not isinstance(body, str) or not body.strip():
    raise RuntimeError("Published GitHub Release body must be present")
  validate_release_notes(
    body,
    tag=tag,
    signing_mode=signing_mode,
    allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
    draft=draft,
  )
  if signing_mode != "developer-id" and allow_untrusted_ad_hoc and not draft:
    if not manual_acceptance_evidence.strip():
      raise RuntimeError(
        "Published visible ad-hoc release must include the manual acceptance receipt URL"
      )
    validate_manual_acceptance_receipt_url(manual_acceptance_evidence)
    required_terms = (
      "## Manual Acceptance",
      "Visible ad-hoc prerelease acceptance receipt:",
      manual_acceptance_evidence.strip(),
    )
    missing = [term for term in required_terms if term not in body]
    if missing:
      raise RuntimeError(
        "Published visible ad-hoc release body is missing manual acceptance receipt: "
        + ", ".join(missing)
      )


def validate_release_assets(release: dict, tag: str, *, draft: bool) -> None:
  actual_names = release_asset_names(release, tag=tag, draft=draft)
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


def release_asset_names(
  release: dict,
  *,
  tag: str | None = None,
  draft: bool = False,
) -> frozenset[str]:
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
    if tag is not None:
      validate_release_asset_download(asset, tag=tag, name=name, draft=draft)
    if name in names:
      raise RuntimeError(f"Published GitHub Release has duplicate asset: {name}")
    names.add(name)
  return frozenset(names)


def validate_release_asset_download(
  asset: dict,
  *,
  tag: str,
  name: str,
  draft: bool,
) -> None:
  if asset.get("state") != "uploaded":
    raise RuntimeError(f"Published GitHub Release asset must be uploaded: {name}")

  size = asset.get("size")
  if not isinstance(size, int) or size <= 0:
    raise RuntimeError(f"Published GitHub Release asset must be non-empty: {name}")

  download_url = asset.get("browser_download_url")
  if draft:
    expected_prefix = f"https://github.com/{RELEASE_REPOSITORY}/releases/download/"
    if (
      not isinstance(download_url, str)
      or not download_url.startswith(expected_prefix)
      or not download_url.endswith(f"/{name}")
    ):
      raise RuntimeError(
        f"Draft GitHub Release asset download URL must be a release download URL: {name}"
      )
    return
  expected_url = (
    f"https://github.com/{RELEASE_REPOSITORY}/releases/download/{tag}/{name}"
  )
  if download_url != expected_url:
    raise RuntimeError(
      f"Published GitHub Release asset download URL must be {expected_url}: {name}"
    )


def load_release(path: Path) -> dict:
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError("Published GitHub Release response must be a JSON object")
  return data


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument(
    "--mode",
    choices=("final", "preupload-existing-assets"),
    default="final",
  )
  parser.add_argument("--tag", required=True)
  parser.add_argument("--release-json", required=True, type=Path)
  parser.add_argument("--source-commit")
  parser.add_argument("--tag-commit")
  parser.add_argument("--expected-draft")
  parser.add_argument("--expected-prerelease")
  parser.add_argument("--signing-mode", choices=sorted(RELEASE_SIGNING_MODES))
  parser.add_argument("--allow-untrusted-ad-hoc")
  parser.add_argument("--manual-acceptance-evidence", default="")
  args = parser.parse_args()

  try:
    release = load_release(args.release_json)
    if args.mode == "preupload-existing-assets":
      validate_existing_release_assets_before_upload(release, tag=args.tag)
      print("Existing release asset preupload contract passed")
      return 0
    if (
      args.source_commit is None
      or args.tag_commit is None
      or args.expected_draft is None
      or args.expected_prerelease is None
      or args.signing_mode is None
      or args.allow_untrusted_ad_hoc is None
    ):
      raise RuntimeError(
        "Final release validation requires source commit, tag commit, "
        "expected state, and signing arguments"
      )
    validate_published_release(
      release,
      tag=args.tag,
      source_commit=args.source_commit,
      tag_commit=args.tag_commit,
      expected_draft=parse_bool(args.expected_draft),
      expected_prerelease=parse_bool(args.expected_prerelease),
      signing_mode=args.signing_mode,
      allow_untrusted_ad_hoc=parse_bool(args.allow_untrusted_ad_hoc),
      manual_acceptance_evidence=args.manual_acceptance_evidence,
    )
  except Exception as error:
    print(f"published release contract failed: {error}", file=sys.stderr)
    return 1

  print("Published release contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
