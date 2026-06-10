#!/usr/bin/env python3
"""Unit checks for shared release identity rules."""

from __future__ import annotations

from release_identity import (
  normalize_product_version,
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
