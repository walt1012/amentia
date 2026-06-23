#!/usr/bin/env python3
"""Unit checks for published GitHub Release contract validation."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from installer_artifact_contract import expected_installer_asset_names
from release_publish_contract import load_release
from release_publish_contract import main as release_publish_main
from release_publish_contract import RELEASE_REPOSITORY
from release_publish_contract import release_asset_names
from release_publish_contract import validate_existing_release_assets_before_upload
from release_publish_contract import validate_published_release
from release_identity import release_repository_url
from release_state import expected_release_title
from release_text import release_notes


TAG = "v1.2.3"
SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
ACCEPTANCE_RECEIPT = release_repository_url("issues/1#manual-acceptance-receipt")


def release_payload(
  *,
  tag: str = TAG,
  draft: bool = False,
  prerelease: bool = True,
  signing_mode: str = "ad-hoc",
  allow_untrusted_ad_hoc: bool = True,
  assets: list[str] | None = None,
  acceptance_receipt: str = ACCEPTANCE_RECEIPT,
  draft_download_urls: bool = False,
) -> dict:
  asset_names = assets if assets is not None else sorted(expected_installer_asset_names(tag))
  body = release_notes(
    tag,
    signing_mode,
    allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
    draft=draft,
  )
  if signing_mode != "developer-id" and allow_untrusted_ad_hoc and not draft:
    body = (
      body.rstrip()
      + "\n\n## Manual Acceptance\n\n"
      + f"Visible ad-hoc prerelease acceptance receipt: {acceptance_receipt}\n"
    )
  return {
    "tag_name": tag,
    "name": expected_release_title(tag),
    "body": body,
    "draft": draft,
    "prerelease": prerelease,
    "assets": [
      {
        "name": name,
        "state": "uploaded",
        "size": 1024,
        "browser_download_url": (
          f"https://github.com/{RELEASE_REPOSITORY}/releases/download/"
          f"{'untagged-test-draft' if draft_download_urls else tag}/{name}"
        ),
      }
      for name in asset_names
    ],
  }


def expect_failure(payload: dict, expected: str) -> None:
  try:
    validate_published_release(
      payload,
      tag=TAG,
      source_commit=SOURCE_COMMIT,
      tag_commit=SOURCE_COMMIT,
      expected_draft=False,
      expected_prerelease=True,
      signing_mode="ad-hoc",
      allow_untrusted_ad_hoc=True,
      manual_acceptance_receipt_url=ACCEPTANCE_RECEIPT,
    )
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r} in {error!r}")
    return
  raise AssertionError(f"expected published release validation to fail: {expected}")


def assert_main_validates_final_release_source_commit() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    release_file = root / "release.json"
    release_file.write_text(json.dumps(release_payload()), encoding="utf-8")
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_publish_contract.py",
        "--tag",
        TAG,
        "--release-json",
        str(release_file),
        "--source-commit",
        SOURCE_COMMIT,
        "--tag-commit",
        SOURCE_COMMIT,
        "--expected-draft",
        "false",
        "--expected-prerelease",
        "true",
        "--signing-mode",
        "ad-hoc",
        "--allow-untrusted-ad-hoc",
        "true",
        "--manual-acceptance-receipt-url",
        ACCEPTANCE_RECEIPT,
      ]
      if release_publish_main() != 0:
        raise AssertionError("published release CLI should pass with matching tag commit")
      sys.argv = [
        "release_publish_contract.py",
        "--tag",
        TAG,
        "--release-json",
        str(release_file),
        "--source-commit",
        SOURCE_COMMIT,
        "--tag-commit",
        "fedcba9876543210fedcba9876543210fedcba98",
        "--expected-draft",
        "false",
        "--expected-prerelease",
        "true",
        "--signing-mode",
        "ad-hoc",
        "--allow-untrusted-ad-hoc",
        "true",
        "--manual-acceptance-receipt-url",
        ACCEPTANCE_RECEIPT,
      ]
      with redirect_stderr(StringIO()) as stderr:
        exit_code = release_publish_main()
      if exit_code == 0:
        raise AssertionError("published release CLI should reject a mismatched tag commit")
      if "tag must point at the source commit" not in stderr.getvalue():
        raise AssertionError("published release CLI should explain tag commit mismatch")
    finally:
      sys.argv = original_argv


def main() -> int:
  validate_published_release(
    release_payload(),
    tag=TAG,
    source_commit=SOURCE_COMMIT,
    tag_commit=SOURCE_COMMIT,
    expected_draft=False,
    expected_prerelease=True,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=True,
    manual_acceptance_receipt_url=ACCEPTANCE_RECEIPT,
  )
  validate_published_release(
    release_payload(signing_mode="developer-id", allow_untrusted_ad_hoc=False),
    tag=TAG,
    source_commit=SOURCE_COMMIT,
    tag_commit=SOURCE_COMMIT,
    expected_draft=False,
    expected_prerelease=True,
    signing_mode="developer-id",
    allow_untrusted_ad_hoc=False,
  )
  validate_published_release(
    release_payload(draft=True, prerelease=False, allow_untrusted_ad_hoc=False),
    tag=TAG,
    source_commit=SOURCE_COMMIT,
    tag_commit=SOURCE_COMMIT,
    expected_draft=True,
    expected_prerelease=False,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=False,
  )
  validate_published_release(
    release_payload(
      draft=True,
      prerelease=True,
      allow_untrusted_ad_hoc=False,
      draft_download_urls=True,
    ),
    tag=TAG,
    source_commit=SOURCE_COMMIT,
    tag_commit=SOURCE_COMMIT,
    expected_draft=True,
    expected_prerelease=True,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=False,
  )
  missing_draft_url = release_payload(
    draft=True,
    prerelease=True,
    allow_untrusted_ad_hoc=False,
  )
  for asset in missing_draft_url["assets"]:
    asset["browser_download_url"] = ""
  validate_published_release(
    missing_draft_url,
    tag=TAG,
    source_commit=SOURCE_COMMIT,
    tag_commit=SOURCE_COMMIT,
    expected_draft=True,
    expected_prerelease=True,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=False,
  )
  validate_existing_release_assets_before_upload(
    release_payload(assets=sorted(expected_installer_asset_names(TAG))[:2]),
    tag=TAG,
  )
  try:
    validate_existing_release_assets_before_upload(
      release_payload(
        assets=[*sorted(expected_installer_asset_names(TAG))[:2], "unexpected.txt"]
      ),
      tag=TAG,
    )
  except RuntimeError as error:
    if "non-contract assets" not in str(error):
      raise
  else:
    raise AssertionError("existing release asset preupload guard should reject extras")

  expect_failure(release_payload(tag="v9.9.9"), "tag_name")
  expect_failure({**release_payload(), "name": "Amentia wrong"}, "name")
  try:
    validate_published_release(
      release_payload(),
      tag=TAG,
      source_commit=SOURCE_COMMIT,
      tag_commit="fedcba9876543210fedcba9876543210fedcba98",
      expected_draft=False,
      expected_prerelease=True,
      signing_mode="ad-hoc",
      allow_untrusted_ad_hoc=True,
      manual_acceptance_receipt_url=ACCEPTANCE_RECEIPT,
    )
  except RuntimeError as error:
    if "tag must point at the source commit" not in str(error):
      raise
  else:
    raise AssertionError("final release validation should reject wrong tag commit")
  expect_failure({**release_payload(), "body": "Install Amentia."}, "missing required copy")
  expect_failure(
    release_payload(acceptance_receipt=""),
    "manual acceptance receipt",
  )
  try:
    validate_published_release(
      release_payload(
        acceptance_receipt="https://example.com/manual-acceptance-receipt"
      ),
      tag=TAG,
      source_commit=SOURCE_COMMIT,
      tag_commit=SOURCE_COMMIT,
      expected_draft=False,
      expected_prerelease=True,
      signing_mode="ad-hoc",
      allow_untrusted_ad_hoc=True,
      manual_acceptance_receipt_url="https://example.com/manual-acceptance-receipt",
    )
  except Exception as error:
    if "repository-scoped" not in str(error):
      raise
  else:
    raise AssertionError("final release validation should reject external receipt URLs")
  expect_failure(
    {
      **release_payload(),
      "body": release_notes(
        TAG,
        "developer-id",
        allow_untrusted_ad_hoc=False,
        draft=False,
      ),
    },
    "Untrusted ad-hoc prerelease.",
  )
  expect_failure(release_payload(draft=True), "draft")
  expect_failure(release_payload(prerelease=False), "prerelease")

  expected_assets = sorted(expected_installer_asset_names(TAG))
  expect_failure(release_payload(assets=expected_assets[:-1]), "missing")
  expect_failure(
    release_payload(assets=[*expected_assets, "unexpected.txt"]),
    "extra",
  )
  expect_failure(release_payload(assets=[*expected_assets, "model.gguf"]), "model.gguf")

  missing_url_payload = release_payload()
  missing_url_payload["assets"][0]["browser_download_url"] = ""
  expect_failure(missing_url_payload, "download URL")

  zero_size_payload = release_payload()
  zero_size_payload["assets"][0]["size"] = 0
  expect_failure(zero_size_payload, "non-empty")

  pending_asset_payload = release_payload()
  pending_asset_payload["assets"][0]["state"] = "starter"
  expect_failure(pending_asset_payload, "uploaded")

  duplicate_asset_payload = release_payload(
    assets=[expected_assets[0], expected_assets[0]]
  )
  try:
    release_asset_names(duplicate_asset_payload)
  except RuntimeError as error:
    if "duplicate asset" not in str(error):
      raise
  else:
    raise AssertionError("expected duplicate release assets to fail")

  with TemporaryDirectory() as directory:
    path = Path(directory) / "release.json"
    path.write_text(json.dumps(release_payload()), encoding="utf-8")
    validate_published_release(
      load_release(path),
      tag=TAG,
      source_commit=SOURCE_COMMIT,
      tag_commit=SOURCE_COMMIT,
      expected_draft=False,
      expected_prerelease=True,
      signing_mode="ad-hoc",
      allow_untrusted_ad_hoc=True,
      manual_acceptance_receipt_url=ACCEPTANCE_RECEIPT,
    )

  assert_main_validates_final_release_source_commit()

  print("Published release contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
