#!/usr/bin/env python3
"""Unit checks for macOS Developer ID signing helper."""

from __future__ import annotations

import tempfile
from pathlib import Path

import sign_macos_app_for_distribution as signing


def assert_equal(actual: object, expected: object) -> None:
  if actual != expected:
    raise AssertionError(f"expected {expected!r}, got {actual!r}")


def assert_raises(action, message: str) -> None:
  try:
    action()
  except Exception:
    return
  raise AssertionError(message)


def main() -> int:
  with tempfile.TemporaryDirectory(prefix="pith-signing-helper-") as root:
    root_path = Path(root)
    app_path = root_path / "Amentia.app"
    contents_path = app_path / "Contents"
    macos_path = contents_path / "MacOS"
    resources_path = contents_path / "Resources"
    macos_path.mkdir(parents=True)
    resources_path.mkdir()
    (contents_path / "Info.plist").write_text("<plist></plist>", encoding="utf-8")
    executable = macos_path / "Amentia"
    executable.write_bytes(b"\xcf\xfa\xed\xfe" + b"placeholder")
    resource = resources_path / "data.txt"
    resource.write_text("not executable", encoding="utf-8")

    signing.require_app(app_path)
    assert_equal(signing.is_macho(executable), True)
    assert_equal(signing.is_macho(resource), False)
    assert_equal(signing.nested_code_targets(app_path), [executable])
    assert_raises(
      lambda: signing.require_app(root_path / "Missing.app"),
      "missing app bundle should fail",
    )

    captured: list[list[str]] = []
    original_run = signing.run
    try:
      signing.run = lambda command: captured.append(command) or ""
      entitlements = root_path / "Amentia.entitlements"
      entitlements.write_text("<plist></plist>", encoding="utf-8")
      signing.sign_path(executable, "Developer ID Application: Example")
      signing.sign_path(app_path, "Developer ID Application: Example", entitlements)
      signing.verify_signature(app_path)
    finally:
      signing.run = original_run

    assert_equal(captured[0][0], "codesign")
    if "--timestamp" not in captured[0] or "--options" not in captured[0]:
      raise AssertionError("Developer ID signing should request timestamped runtime signing")
    if "--entitlements" not in captured[1]:
      raise AssertionError("app signing should include entitlements when provided")
    if captured[2][:4] != ["codesign", "--verify", "--deep", "--strict"]:
      raise AssertionError("signature validation should use strict deep verification")

  print("distribution signing helper tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
