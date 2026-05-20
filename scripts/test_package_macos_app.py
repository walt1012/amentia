#!/usr/bin/env python3
"""Unit checks for macOS packaging helpers that do not require macOS."""

from __future__ import annotations

import stat
import tempfile
from pathlib import Path

from macos_llama_backend import (
  LLAMA_BACKEND_EXECUTABLE_NAME,
  LLAMA_BACKEND_LIB_DIRECTORY_NAME,
  is_packaged_backend_dependency,
  parse_otool_dependencies,
)
from package_macos_app import (
  LLAMA_BACKEND_RELATIVE_PARENT,
  copy_required_llama_backend,
  parse_lipo_architectures,
)


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
  assert_equal(
    parse_otool_dependencies(
      """/tmp/llama-cli:
\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)
\t@executable_path/lib/libllama.dylib (compatibility version 0.0.0, current version 0.0.0)
"""
    ),
    ["/usr/lib/libSystem.B.dylib", "@executable_path/lib/libllama.dylib"],
  )
  if not is_packaged_backend_dependency("@executable_path/lib/libllama.dylib", True):
    raise AssertionError("backend dependencies should allow packaged executable paths")
  if not is_packaged_backend_dependency("@loader_path/libggml.dylib", False):
    raise AssertionError("dylib dependencies should allow packaged loader paths")
  if is_packaged_backend_dependency("/external/package-manager/lib/libllama.dylib", True):
    raise AssertionError("absolute non-system dependency paths should be rejected")

  with tempfile.TemporaryDirectory(prefix="pith-package-test-") as root:
    root_path = Path(root)
    source_backend = root_path / "llama-cli"
    source_backend.write_text("#!/bin/sh\n", encoding="utf-8")
    source_backend.chmod(source_backend.stat().st_mode | stat.S_IXUSR)
    source_lib = root_path / LLAMA_BACKEND_LIB_DIRECTORY_NAME
    source_lib.mkdir()
    (source_lib / "libllama.dylib").write_text("placeholder", encoding="utf-8")
    packaged_backend = copy_required_llama_backend(
      root_path,
      root_path / "Resources",
      source_backend,
    )
    assert_equal(
      packaged_backend,
      root_path / "Resources" / LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME,
    )
    if not packaged_backend.is_file():
      raise AssertionError("packaged llama backend should exist")
    if not (
      packaged_backend.parent / LLAMA_BACKEND_LIB_DIRECTORY_NAME / "libllama.dylib"
    ).is_file():
      raise AssertionError("packaged llama backend should include sibling dylib bundle")

  with tempfile.TemporaryDirectory(prefix="pith-package-missing-backend-") as root:
    try:
      copy_required_llama_backend(Path(root), Path(root) / "Resources", None)
    except FileNotFoundError:
      pass
    else:
      raise AssertionError("missing llama backend should fail packaging")
  print("package helper tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
