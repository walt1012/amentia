#!/usr/bin/env python3
"""Unit checks for macOS distribution validators that do not require signing tools."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from package_contract import (
  DAILY_DRIVER_CONTRACT,
  DEFAULT_MAX_APP_BUNDLE_BYTES,
  DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  DEFAULT_MODEL_ID,
  FIRST_APP_OPEN_CONTRACT_ID,
  LOCAL_EXECUTION_SAFETY_MODES,
  MINIMUM_SYSTEM_VERSION,
  MODEL_DELIVERY_MODE,
  MODEL_METADATA_BUNDLED,
  MODEL_WEIGHTS_BUNDLED,
  PACKAGE_MANIFEST_SCHEMA_VERSION,
  PITH_ACCOUNT_REQUIRED,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  package_distribution_trust,
)
from validate_macos_distribution import validate_app_icon, validate_package_manifest


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"


def assert_raises(action, message: str) -> None:
  try:
    action()
  except (RuntimeError, FileNotFoundError):
    return
  raise AssertionError(message)


def write_manifest(app_path: Path, signing: str, source_commit: str) -> None:
  manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
  manifest_path.parent.mkdir(parents=True)
  manifest_path.write_text(
    json.dumps(
      {
        "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
        "appName": "Pith",
        "bundleVersion": "0.1.0",
        "signing": signing,
        "distributionTrust": package_distribution_trust(signing),
        "sourceCommit": source_commit,
        "architecture": SUPPORTED_ARCH,
        "minimumSystemVersion": MINIMUM_SYSTEM_VERSION,
        "defaultModelId": DEFAULT_MODEL_ID,
        "modelDelivery": MODEL_DELIVERY_MODE,
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
    ),
    encoding="utf-8",
  )


def write_icns(path: Path, declared_size: int, body: bytes = b"") -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_bytes(b"icns" + declared_size.to_bytes(4, "big") + body)


def main() -> int:
  with tempfile.TemporaryDirectory(prefix="pith-distribution-icon-") as root:
    app_path = Path(root) / "Pith.app"
    write_icns(app_path / "Contents" / "Resources" / "Pith.icns", 12, b"abcd")
    validate_app_icon(app_path)

  with tempfile.TemporaryDirectory(prefix="pith-distribution-icon-") as root:
    app_path = Path(root) / "Pith.app"
    assert_raises(
      lambda: validate_app_icon(app_path),
      "public distribution should require a packaged app icon",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-icon-") as root:
    app_path = Path(root) / "Pith.app"
    write_icns(app_path / "Contents" / "Resources" / "Pith.icns", 8, b"abcd")
    assert_raises(
      lambda: validate_app_icon(app_path),
      "public distribution should reject invalid ICNS headers",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    validate_package_manifest(app_path)

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "ad-hoc", SOURCE_COMMIT)
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require developer-id signing metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", "development")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require full source commit metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["schemaVersion"] = 2
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require package schema version 1",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["architecture"] = "arm64"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require x86_64 package metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["modelDelivery"] = "bundled"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require in-app model delivery",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["modelWeightsBundled"] = True
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should reject bundled model weights metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["sandboxFallback"] = "none"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require sandbox fallback metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["dailyDriverStageSource"] = "app-only"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require daily-driver readiness metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["firstAppOpenActionContract"] = "static-checklist"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should require first app-open action metadata",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["sizeBudget"]["maxAppBundleBytes"] = 1
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should enforce package size budget",
    )

  with tempfile.TemporaryDirectory(prefix="pith-distribution-") as root:
    app_path = Path(root) / "Pith.app"
    write_manifest(app_path, "developer-id", SOURCE_COMMIT)
    model_path = app_path / "Contents" / "Resources" / "models" / "local.gguf"
    model_path.parent.mkdir(parents=True)
    model_path.write_bytes(b"model")
    assert_raises(
      lambda: validate_package_manifest(app_path),
      "public distribution should reject bundled model files",
    )

  print("macOS distribution validator tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
