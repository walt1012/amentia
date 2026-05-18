#!/usr/bin/env python3
"""Unit checks for macOS packaging helpers that do not require macOS."""

from __future__ import annotations

from package_macos_app import parse_lipo_architectures


def assert_equal(actual: object, expected: object) -> None:
  if actual != expected:
    raise AssertionError(f"expected {expected!r}, got {actual!r}")


def main() -> int:
  assert_equal(
    parse_lipo_architectures("Non-fat file: Pith is architecture: x86_64"),
    {"x86_64"},
  )
  assert_equal(
    parse_lipo_architectures(
      "Architectures in the fat file: Pith are: x86_64 arm64"
    ),
    {"x86_64", "arm64"},
  )
  try:
    parse_lipo_architectures("not a lipo architecture line")
  except RuntimeError:
    pass
  else:
    raise AssertionError("invalid lipo output should fail")
  print("package helper tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
