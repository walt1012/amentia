#!/usr/bin/env python3
"""Unit checks for shared macOS package contract helpers."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from package_contract import (
  APP_NAME,
  DAILY_DRIVER_CONTRACT,
  DEFAULT_MAX_APP_BUNDLE_BYTES,
  DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
  DEFAULT_MODEL_ID,
  MINIMUM_SYSTEM_VERSION,
  MODEL_DELIVERY_MODE,
  MODEL_METADATA_BUNDLED,
  MODEL_WEIGHTS_BUNDLED,
  PACKAGE_MANIFEST_SCHEMA_VERSION,
  PROHIBITED_MODEL_SUFFIXES,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  assert_size_under_budget,
  bundled_model_weight_files,
  directory_size_bytes,
  package_size_budget,
  read_json_object,
  validate_package_manifest_contract,
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


def valid_manifest() -> dict[str, object]:
  return {
    "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
    "appName": APP_NAME,
    "bundleVersion": "1.2.3",
    "minimumSystemVersion": MINIMUM_SYSTEM_VERSION,
    "architecture": SUPPORTED_ARCH,
    "sourceCommit": "0123456789abcdef0123456789abcdef01234567",
    "signing": "ad-hoc",
    "modelDelivery": MODEL_DELIVERY_MODE,
    "defaultModelId": DEFAULT_MODEL_ID,
    "modelWeightsBundled": MODEL_WEIGHTS_BUNDLED,
    "modelMetadataBundled": MODEL_METADATA_BUNDLED,
    "sandboxMode": SANDBOX_CONTRACT["mode"],
    "sandboxBackend": SANDBOX_CONTRACT["backend"],
    "sandboxFallback": SANDBOX_CONTRACT["fallback"],
    "sandboxNetworkDefault": SANDBOX_CONTRACT["networkDefault"],
    "dailyDriverStageSource": DAILY_DRIVER_CONTRACT["stageSource"],
    "dailyDriverNextActionSource": DAILY_DRIVER_CONTRACT["nextActionSource"],
    "dailyDriverPresentation": DAILY_DRIVER_CONTRACT["presentation"],
    "sizeBudget": {
      "maxAppBundleBytes": DEFAULT_MAX_APP_BUNDLE_BYTES,
      "maxZipArtifactBytes": DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
    },
  }


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

  manifest = valid_manifest()
  validated_budget = validate_package_manifest_contract(
    manifest,
    "test manifest",
    source_commit="0123456789abcdef0123456789abcdef01234567",
    signing_mode="ad-hoc",
    bundle_version="1.2.3",
  )
  assert_equal(validated_budget["maxAppBundleBytes"], DEFAULT_MAX_APP_BUNDLE_BYTES)
  wrong_manifest = dict(manifest)
  wrong_manifest["modelDelivery"] = "bundled"
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "wrong model delivery should fail manifest contract validation",
  )
  wrong_manifest = dict(manifest)
  wrong_manifest["sourceCommit"] = "wrong"
  assert_raises(
    lambda: validate_package_manifest_contract(
      wrong_manifest,
      "test manifest",
      source_commit="0123456789abcdef0123456789abcdef01234567",
    ),
    "wrong source commit should fail manifest contract validation",
  )
  wrong_manifest = dict(manifest)
  wrong_manifest.pop("bundleVersion")
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "missing bundle version should fail manifest contract validation",
  )
  wrong_manifest = dict(manifest)
  wrong_manifest["sizeBudget"] = {
    "maxAppBundleBytes": DEFAULT_MAX_APP_BUNDLE_BYTES * 2,
    "maxZipArtifactBytes": DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
  }
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "wrong package budget should fail manifest contract validation",
  )
  custom_budget = {
    "maxAppBundleBytes": 1024,
    "maxZipArtifactBytes": 2048,
  }
  custom_manifest = dict(manifest)
  custom_manifest["sizeBudget"] = custom_budget
  assert_equal(
    validate_package_manifest_contract(
      custom_manifest,
      "test manifest",
      expected_size_budget=custom_budget,
    ),
    custom_budget,
  )

  with tempfile.TemporaryDirectory(prefix="pith-package-contract-") as root:
    root_path = Path(root)
    (root_path / "metadata.json").write_text("{}", encoding="utf-8")
    manifest_path = root_path / "PithPackage.json"
    manifest_path.write_text('{"ok": true}', encoding="utf-8")
    assert_equal(read_json_object(manifest_path, "test manifest"), {"ok": True})
    manifest_path.write_text(
      json.dumps(valid_manifest()),
      encoding="utf-8",
    )
    subprocess.run(
      [
        sys.executable,
        str(Path(__file__).with_name("package_contract.py")),
        "--manifest",
        str(manifest_path),
        "--source-commit",
        "0123456789abcdef0123456789abcdef01234567",
        "--signing-mode",
        "ad-hoc",
        "--bundle-version",
        "1.2.3",
      ],
      check=True,
      stdout=subprocess.PIPE,
      stderr=subprocess.STDOUT,
      text=True,
    )
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
