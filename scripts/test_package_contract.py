#!/usr/bin/env python3
"""Unit checks for shared macOS package contract helpers."""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

from package_contract import (
  DEFAULT_MAX_APP_BUNDLE_BYTES,
  DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
  DEFAULT_MODEL_ID,
  MODEL_DELIVERY_MODE,
  MODEL_WEIGHTS_BUNDLED,
  PROHIBITED_MODEL_SUFFIXES,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  assert_size_under_budget,
  bundled_model_weight_files,
  directory_size_bytes,
  package_size_budget,
  validate_package_size_budget,
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


def with_env(name: str, value: str | None, action) -> None:
  original = os.environ.get(name)
  try:
    if value is None:
      os.environ.pop(name, None)
    else:
      os.environ[name] = value
    action()
  finally:
    if original is None:
      os.environ.pop(name, None)
    else:
      os.environ[name] = original


def main() -> int:
  assert_equal(SUPPORTED_ARCH, "x86_64")
  assert_equal(DEFAULT_MODEL_ID, "lfm2.5-350m")
  assert_equal(MODEL_DELIVERY_MODE, "in-app-download")
  assert_equal(MODEL_WEIGHTS_BUNDLED, False)
  assert_equal(SANDBOX_CONTRACT["mode"], "workspaceReadWrite")
  assert_equal(DEFAULT_MAX_APP_BUNDLE_BYTES, 250 * 1024 * 1024)
  assert_equal(DEFAULT_MAX_ZIP_ARTIFACT_BYTES, 150 * 1024 * 1024)
  assert_equal(
    package_size_budget(),
    {
      "maxAppBundleBytes": DEFAULT_MAX_APP_BUNDLE_BYTES,
      "maxZipArtifactBytes": DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
    },
  )

  with_env(
    "PITH_MAX_APP_BUNDLE_BYTES",
    "1024",
    lambda: assert_equal(package_size_budget()["maxAppBundleBytes"], 1024),
  )
  with_env(
    "PITH_MAX_ZIP_ARTIFACT_BYTES",
    "4096",
    lambda: assert_equal(package_size_budget()["maxZipArtifactBytes"], 4096),
  )
  with_env(
    "PITH_MAX_APP_BUNDLE_BYTES",
    "0",
    lambda: assert_raises(package_size_budget, "zero package budgets should fail"),
  )
  with_env(
    "PITH_MAX_ZIP_ARTIFACT_BYTES",
    "not-a-number",
    lambda: assert_raises(package_size_budget, "non-integer package budgets should fail"),
  )

  budget = validate_package_size_budget(
    {
      "maxAppBundleBytes": 1024,
      "maxZipArtifactBytes": 2048,
    },
    "test contract",
  )
  assert_equal(budget["maxZipArtifactBytes"], 2048)
  assert_raises(
    lambda: validate_package_size_budget({}, "test contract"),
    "missing package budget fields should fail",
  )
  assert_size_under_budget(1024, 2048, "test package")
  assert_raises(
    lambda: assert_size_under_budget(2049, 2048, "test package"),
    "oversized packages should fail",
  )

  with tempfile.TemporaryDirectory(prefix="pith-package-contract-") as root:
    root_path = Path(root)
    (root_path / "metadata.json").write_text("{}", encoding="utf-8")
    model_path = root_path / "models" / "model.gguf"
    model_path.parent.mkdir()
    model_path.write_bytes(b"gguf")
    weights = bundled_model_weight_files(root_path)
    assert_equal(weights, [model_path])
    if directory_size_bytes(root_path) <= 0:
      raise AssertionError("directory size should count files")

  expected_suffixes = {".gguf", ".bin", ".safetensors"}
  assert_equal(PROHIBITED_MODEL_SUFFIXES, expected_suffixes)

  print("package contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
