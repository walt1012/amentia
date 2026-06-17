#!/usr/bin/env python3
"""Unit checks for shared release identity rules."""

from __future__ import annotations

from release_identity import (
  RELEASE_ACTIONS_RUN_URL_PREFIX,
  RELEASE_REPOSITORY,
  RELEASE_REPOSITORY_URL,
  normalize_product_version,
  release_actions_run_url,
  release_repository_url,
  product_version_from_tag,
  validate_public_release_tag,
)


def assert_equal(actual: object, expected: object) -> None:
  if actual != expected:
    raise AssertionError(f"expected {expected!r}, got {actual!r}")


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


def main() -> int:
  assert_equal(normalize_product_version("0.1.0"), "0.1.0")
  assert_equal(normalize_product_version("v1.2.3"), "1.2.3")
  assert_equal(product_version_from_tag("v1.2.3"), "1.2.3")
  assert_equal(RELEASE_REPOSITORY, "walt1012/amentia")
  assert_equal(RELEASE_REPOSITORY_URL, "https://github.com/walt1012/amentia")
  assert_equal(
    RELEASE_ACTIONS_RUN_URL_PREFIX,
    "https://github.com/walt1012/amentia/actions/runs/",
  )
  assert_equal(
    release_actions_run_url(123456),
    "https://github.com/walt1012/amentia/actions/runs/123456",
  )
  assert_equal(
    release_repository_url("issues/1#manual-acceptance-receipt"),
    "https://github.com/walt1012/amentia/issues/1#manual-acceptance-receipt",
  )
  validate_public_release_tag("v0.1.0")
  assert_raises(
    lambda: normalize_product_version("1.2"),
    "partial product versions should fail validation",
  )
  assert_raises(
    lambda: validate_public_release_tag("v1.2"),
    "partial public release tags should fail validation",
  )
  assert_raises(
    lambda: validate_public_release_tag("1.2.3"),
    "public release tags require the leading v",
  )
  assert_raises(
    lambda: validate_public_release_tag("v1.2.3-beta"),
    "prerelease suffixes should stay out of public release tags",
  )
  print("release identity tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
