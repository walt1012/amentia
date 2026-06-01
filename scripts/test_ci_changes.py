#!/usr/bin/env python3
"""Unit checks for CI change classification."""

from __future__ import annotations

from ci_changes import CiChanges, classify_changed_paths


def assert_equal(actual: object, expected: object) -> None:
  if actual != expected:
    raise AssertionError(f"expected {expected!r}, got {actual!r}")


def main() -> int:
  assert_equal(
    classify_changed_paths(["docs/development-plan.md"]),
    CiChanges(False, False, False, False),
  )
  assert_equal(
    classify_changed_paths(["crates/pith-core/src/lib.rs"]),
    CiChanges(True, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["apps/pith-macos/Sources/PithApp/App/AppViewModel.swift"]),
    CiChanges(False, True, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/release_artifacts.py"]),
    CiChanges(False, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/release_copy_contract.py"]),
    CiChanges(False, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/package_contract.py"]),
    CiChanges(False, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/release_identity.py"]),
    CiChanges(False, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/sign_macos_app_for_distribution.py"]),
    CiChanges(False, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/installer_artifact_contract.py"]),
    CiChanges(False, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/validate_macos_distribution.py"]),
    CiChanges(False, False, True, False),
  )
  assert_equal(
    classify_changed_paths(["scripts/package_macos_app.py"]),
    CiChanges(False, False, True, True),
  )
  assert_equal(classify_changed_paths([".github/workflows/ci.yml"]), CiChanges.all())
  assert_equal(classify_changed_paths([], force_all=True), CiChanges.all())
  print("CI change classification tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
