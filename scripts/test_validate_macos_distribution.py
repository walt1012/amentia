#!/usr/bin/env python3
"""Unit checks for macOS distribution validators that do not require signing tools."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from validate_macos_distribution import validate_package_manifest


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


def write_manifest(app_path: Path, signing: str, source_commit: str) -> None:
  manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
  manifest_path.parent.mkdir(parents=True)
  manifest_path.write_text(
    json.dumps(
      {
        "schemaVersion": 1,
        "signing": signing,
        "sourceCommit": source_commit,
        "architecture": "x86_64",
        "minimumSystemVersion": "12.0",
        "defaultModelId": "lfm2.5-350m",
        "modelDelivery": "in-app-download",
        "modelWeightsBundled": False,
        "modelMetadataBundled": True,
        "sandboxMode": "workspaceReadWrite",
        "sandboxBackend": "runtime-detected",
        "sandboxFallback": "processOnlyWhenNativeUnavailable",
        "sandboxNetworkDefault": "disabled",
        "dailyDriverStageSource": "runtime/readiness",
        "dailyDriverNextActionSource": "runtime/readiness",
        "dailyDriverPresentation": "app-header-inspector",
        "sizeBudget": {
          "maxAppBundleBytes": 262144000,
          "maxZipArtifactBytes": 157286400,
        },
      }
    ),
    encoding="utf-8",
  )


def main() -> int:
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
