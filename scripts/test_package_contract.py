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
  FIRST_APP_OPEN_CONTRACT_ID,
  MINIMUM_SYSTEM_VERSION,
  MODEL_DELIVERY_MODE,
  MODEL_METADATA_BUNDLED,
  MODEL_WEIGHTS_BUNDLED,
  PACKAGE_MANIFEST_SCHEMA_VERSION,
  PACKAGE_SIGNING_MODES,
  PACKAGE_DISTRIBUTION_TRUST_BY_SIGNING,
  PACKAGED_SMOKE_JOURNEY,
  PACKAGED_SMOKE_REQUIRED_CHECK_IDS,
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  LOCAL_EXECUTION_SAFETY_MODES,
  PITH_ACCOUNT_REQUIRED,
  PROHIBITED_MODEL_SUFFIXES,
  RELEASE_SIGNING_MODES,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  assert_size_under_budget,
  bundled_model_weight_files,
  directory_size_bytes,
  package_size_budget,
  package_distribution_trust,
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
    "distributionTrust": "ad-hoc-not-notarized",
    "modelDelivery": MODEL_DELIVERY_MODE,
    "defaultModelId": DEFAULT_MODEL_ID,
    "modelWeightsBundled": MODEL_WEIGHTS_BUNDLED,
    "modelMetadataBundled": MODEL_METADATA_BUNDLED,
    "pithAccountRequired": PITH_ACCOUNT_REQUIRED,
    "defaultLocalExecutionSafetyMode": DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
    "localExecutionSafetyModes": list(LOCAL_EXECUTION_SAFETY_MODES),
    "sandboxMode": SANDBOX_CONTRACT["mode"],
    "sandboxBackend": SANDBOX_CONTRACT["backend"],
    "sandboxFallback": SANDBOX_CONTRACT["fallback"],
    "sandboxNetworkDefault": SANDBOX_CONTRACT["networkDefault"],
    "dailyDriverStageSource": DAILY_DRIVER_CONTRACT["stageSource"],
    "dailyDriverNextActionSource": DAILY_DRIVER_CONTRACT["nextActionSource"],
    "dailyDriverPresentation": DAILY_DRIVER_CONTRACT["presentation"],
    "firstAppOpenActionContract": FIRST_APP_OPEN_CONTRACT_ID,
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
  assert_equal(PITH_ACCOUNT_REQUIRED, False)
  assert_equal(DEFAULT_LOCAL_EXECUTION_SAFETY_MODE, "askBeforeChange")
  assert_equal(
    LOCAL_EXECUTION_SAFETY_MODES,
    ("explore", "askBeforeChange", "approvedWorkspaceExecution"),
  )
  assert_equal(SANDBOX_CONTRACT["mode"], "workspaceReadWrite")
  assert_equal(PACKAGE_SIGNING_MODES, {"unsigned", "ad-hoc", "developer-id"})
  assert_equal(
    PACKAGE_DISTRIBUTION_TRUST_BY_SIGNING,
    {
      "unsigned": "unsigned-local-build",
      "ad-hoc": "ad-hoc-not-notarized",
      "developer-id": "developer-id-signed-notarized",
    },
  )
  assert_equal(RELEASE_SIGNING_MODES, {"ad-hoc", "developer-id"})
  journey_check_ids = [
    check_id
    for stage in PACKAGED_SMOKE_JOURNEY
    for check_id in stage["checkIds"]
  ]
  assert_equal(sorted(journey_check_ids), sorted(PACKAGED_SMOKE_REQUIRED_CHECK_IDS))
  assert_equal(len(journey_check_ids), len(set(journey_check_ids)))
  assert_equal(
    [stage["id"] for stage in PACKAGED_SMOKE_JOURNEY],
    [
      "install",
      "modelSetup",
      "workspaceCowork",
      "retrieval",
      "localExecution",
      "connector",
      "recovery",
    ],
  )
  assert_equal(package_distribution_trust("developer-id"), "developer-id-signed-notarized")
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
  wrong_manifest["pithAccountRequired"] = True
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "package contract must not allow a required Pith account",
  )
  wrong_manifest = dict(manifest)
  wrong_manifest["defaultLocalExecutionSafetyMode"] = "alwaysRun"
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "wrong default execution mode should fail manifest contract validation",
  )
  wrong_manifest = dict(manifest)
  wrong_manifest["localExecutionSafetyModes"] = ["alwaysRun"]
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "wrong execution mode list should fail manifest contract validation",
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
  wrong_manifest["signing"] = "package-manager"
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "unsupported signing mode should fail manifest contract validation",
  )
  wrong_manifest = dict(manifest)
  wrong_manifest["distributionTrust"] = "developer-id-signed-notarized"
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "wrong distribution trust should fail manifest contract validation",
  )
  wrong_manifest = dict(manifest)
  wrong_manifest["firstAppOpenActionContract"] = "static-checklist"
  assert_raises(
    lambda: validate_package_manifest_contract(wrong_manifest, "test manifest"),
    "wrong first app-open action should fail manifest contract validation",
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
