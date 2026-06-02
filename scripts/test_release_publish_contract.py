#!/usr/bin/env python3
"""Unit checks for published GitHub Release contract validation."""

from __future__ import annotations

import json
from pathlib import Path
from tempfile import TemporaryDirectory

from installer_artifact_contract import expected_installer_asset_names
from release_publish_contract import load_release
from release_publish_contract import RELEASE_REPOSITORY
from release_publish_contract import release_asset_names
from release_publish_contract import validate_published_release
from release_state import expected_release_title
from release_text import release_notes


TAG = "v1.2.3"


def release_payload(
  *,
  tag: str = TAG,
  draft: bool = False,
  prerelease: bool = True,
  signing_mode: str = "ad-hoc",
  allow_untrusted_ad_hoc: bool = True,
  assets: list[str] | None = None,
) -> dict:
  asset_names = assets if assets is not None else sorted(expected_installer_asset_names(tag))
  return {
    "tag_name": tag,
    "name": expected_release_title(tag),
    "body": release_notes(
      tag,
      signing_mode,
      allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
      draft=draft,
    ),
    "draft": draft,
    "prerelease": prerelease,
    "assets": [
      {
        "name": name,
        "state": "uploaded",
        "size": 1024,
        "browser_download_url": (
          f"https://github.com/{RELEASE_REPOSITORY}/releases/download/{tag}/{name}"
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
      expected_draft=False,
      expected_prerelease=True,
      signing_mode="ad-hoc",
      allow_untrusted_ad_hoc=True,
    )
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r} in {error!r}")
    return
  raise AssertionError(f"expected published release validation to fail: {expected}")


def main() -> int:
  validate_published_release(
    release_payload(),
    tag=TAG,
    expected_draft=False,
    expected_prerelease=True,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=True,
  )
  validate_published_release(
    release_payload(signing_mode="developer-id", allow_untrusted_ad_hoc=False),
    tag=TAG,
    expected_draft=False,
    expected_prerelease=True,
    signing_mode="developer-id",
    allow_untrusted_ad_hoc=False,
  )
  validate_published_release(
    release_payload(draft=True, prerelease=False, allow_untrusted_ad_hoc=False),
    tag=TAG,
    expected_draft=True,
    expected_prerelease=False,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=False,
  )

  expect_failure(release_payload(tag="v9.9.9"), "tag_name")
  expect_failure({**release_payload(), "name": "Pith wrong"}, "name")
  expect_failure({**release_payload(), "body": "Install Pith."}, "missing required copy")
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
      expected_draft=False,
      expected_prerelease=True,
      signing_mode="ad-hoc",
      allow_untrusted_ad_hoc=True,
    )

  print("Published release contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
