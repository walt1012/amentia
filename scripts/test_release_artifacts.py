#!/usr/bin/env python3
"""Unit checks for release artifact sidecar helpers."""

from __future__ import annotations

import tempfile
import json
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
  packaged_smoke_package_metadata,
)
from release_artifacts import (
  FIRST_RUN_CONTRACT,
  PACKAGED_SMOKE_RECEIPT_KIND,
  PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION,
  PACKAGED_SMOKE_PROOF_SCOPE,
  PACKAGED_SMOKE_REQUIRED_CHECK_IDS,
  checksum_text,
  packaged_smoke_journey,
  release_gatekeeper_guidance,
  release_manifest as build_release_manifest,
  release_installer_asset_names,
  validate_checksum_file,
  validate_install_guide,
  validate_install_guide_for_tag,
  validate_release_manifest,
  write_checksum_file,
  write_release_manifest as build_write_release_manifest,
)
from release_copy_contract import FIRST_APP_OPEN_CONTRACT_ID
from release_text import install_guide as release_install_guide


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
WORKFLOW_RUN_ID = "123456789"
WORKFLOW_RUN_URL = "https://github.com/walt1012/pith/actions/runs/123456789"


def smoke_metadata_from_package_manifest(path: Path) -> dict:
  return packaged_smoke_package_metadata(
    json.loads(path.read_text(encoding="utf-8"))
  )


def write_smoke_receipt(path: Path, package_metadata: dict) -> Path:
  path.write_text(
    json.dumps(
      {
        "schemaVersion": PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION,
        "kind": PACKAGED_SMOKE_RECEIPT_KIND,
        "result": "passed",
        "proofScope": PACKAGED_SMOKE_PROOF_SCOPE,
        "packageMetadata": package_metadata,
        "journey": packaged_smoke_journey(),
        "checks": [
          {
            "id": check_id,
            "proof": f"{check_id} proof",
          }
          for check_id in PACKAGED_SMOKE_REQUIRED_CHECK_IDS
        ],
      },
      indent=2,
      sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
  )
  return path


def release_manifest(**kwargs) -> dict:
  return build_release_manifest(
    workflow_run_id=WORKFLOW_RUN_ID,
    workflow_run_url=WORKFLOW_RUN_URL,
    **kwargs,
  )


def write_release_manifest(**kwargs) -> Path:
  return build_write_release_manifest(
    workflow_run_id=WORKFLOW_RUN_ID,
    workflow_run_url=WORKFLOW_RUN_URL,
    **kwargs,
  )


def write_package_manifest(
  path: Path,
  *,
  bundle_version: str = "0.1.0",
  source_commit: str = SOURCE_COMMIT,
  signing: str = "ad-hoc",
  model_delivery: str = MODEL_DELIVERY_MODE,
  sandbox_mode: str = SANDBOX_CONTRACT["mode"],
  daily_driver_stage_source: str = DAILY_DRIVER_CONTRACT["stageSource"],
) -> Path:
  path.write_text(
    json.dumps(
      {
        "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
        "appName": "Pith",
        "bundleVersion": bundle_version,
        "minimumSystemVersion": MINIMUM_SYSTEM_VERSION,
        "architecture": SUPPORTED_ARCH,
        "sourceCommit": source_commit,
        "signing": signing,
        "distributionTrust": {
          "ad-hoc": "ad-hoc-not-notarized",
          "developer-id": "developer-id-signed-notarized",
          "unsigned": "unsigned-local-build",
        }[signing],
        "modelDelivery": model_delivery,
        "defaultModelId": DEFAULT_MODEL_ID,
        "modelWeightsBundled": MODEL_WEIGHTS_BUNDLED,
        "modelMetadataBundled": MODEL_METADATA_BUNDLED,
        "pithAccountRequired": PITH_ACCOUNT_REQUIRED,
        "defaultLocalExecutionSafetyMode": DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
        "localExecutionSafetyModes": list(LOCAL_EXECUTION_SAFETY_MODES),
        "dailyDriverStageSource": daily_driver_stage_source,
        "dailyDriverNextActionSource": DAILY_DRIVER_CONTRACT["nextActionSource"],
        "dailyDriverPresentation": DAILY_DRIVER_CONTRACT["presentation"],
        "firstAppOpenActionContract": FIRST_APP_OPEN_CONTRACT_ID,
        "sandboxMode": sandbox_mode,
        "sandboxBackend": SANDBOX_CONTRACT["backend"],
        "sandboxFallback": SANDBOX_CONTRACT["fallback"],
        "sandboxNetworkDefault": SANDBOX_CONTRACT["networkDefault"],
        "sizeBudget": {
          "maxAppBundleBytes": DEFAULT_MAX_APP_BUNDLE_BYTES,
          "maxZipArtifactBytes": DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
        },
      },
      indent=2,
      sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
  )
  return path


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


def main() -> int:
  with tempfile.TemporaryDirectory(prefix="pith-release-artifacts-") as root:
    root_path = Path(root)
    artifact = root_path / "Pith-v0.1.0-macos-x86_64.dmg"
    install_guide = root_path / "README-FIRST.txt"
    package_manifest = write_package_manifest(root_path / "PithPackage.json")
    smoke_receipt = write_smoke_receipt(
      root_path / "packaged-smoke-receipt.json",
      smoke_metadata_from_package_manifest(package_manifest),
    )
    artifact.write_bytes(b"pith release artifact\n")
    install_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc"),
      encoding="utf-8",
    )

    text = checksum_text(artifact)
    if not text.endswith(f"  {artifact.name}\n"):
      raise AssertionError("checksum should reference the artifact basename")
    if str(root_path) in text:
      raise AssertionError("checksum should not expose runner-local paths")

    checksum_path = write_checksum_file(artifact)
    validate_checksum_file(artifact, checksum_path)
    validate_install_guide(install_guide)
    validate_install_guide_for_tag(install_guide, "v0.1.0")

    manifest = release_manifest(
      tag="v0.1.0",
      source_commit=SOURCE_COMMIT,
      signing_mode="ad-hoc",
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
      package_manifest_path=package_manifest,
      smoke_receipt_path=smoke_receipt,
    )
    if manifest["platform"]["architecture"] != SUPPORTED_ARCH:
      raise AssertionError("release manifest should lock the macOS architecture")
    if manifest["schemaVersion"] != PACKAGE_MANIFEST_SCHEMA_VERSION:
      raise AssertionError("release manifest should record its schema version")
    if manifest["releaseKind"] != "public":
      raise AssertionError("public release tags should produce public manifests")
    if manifest["trust"] != "ad-hoc-not-notarized":
      raise AssertionError("ad-hoc releases should not claim notarization")
    if manifest["gatekeeper"] != release_gatekeeper_guidance("ad-hoc"):
      raise AssertionError("ad-hoc releases should record Gatekeeper manual approval")
    if "Open Anyway" not in manifest["gatekeeper"]:
      raise AssertionError("ad-hoc release Gatekeeper guidance should name manual approval")
    if manifest["sourceCommit"] != SOURCE_COMMIT:
      raise AssertionError("release manifest should record the source commit")
    if manifest["modelDelivery"]["modelWeightsBundled"] is not False:
      raise AssertionError("release manifest should not claim bundled model weights")
    if manifest["identity"]["pithAccountRequired"] is not False:
      raise AssertionError("release manifest should not require a Pith account")
    if manifest["localExecution"]["defaultSafetyMode"] != DEFAULT_LOCAL_EXECUTION_SAFETY_MODE:
      raise AssertionError("release manifest should record default action safety mode")
    if manifest["localExecution"]["safetyModes"] != list(LOCAL_EXECUTION_SAFETY_MODES):
      raise AssertionError("release manifest should record action safety modes")
    if manifest["sandbox"]["fallback"] != SANDBOX_CONTRACT["fallback"]:
      raise AssertionError("release manifest should disclose the sandbox fallback")
    if manifest["dailyDriver"]["stageSource"] != DAILY_DRIVER_CONTRACT["stageSource"]:
      raise AssertionError("release manifest should disclose daily-driver stage source")
    if manifest["dailyDriver"]["presentation"] != DAILY_DRIVER_CONTRACT["presentation"]:
      raise AssertionError("release manifest should disclose daily-driver presentation")
    if manifest["firstRun"] != FIRST_RUN_CONTRACT:
      raise AssertionError("release manifest should disclose the first-run path contract")
    if manifest["firstRun"]["firstAppOpen"] != FIRST_APP_OPEN_CONTRACT_ID:
      raise AssertionError("release manifest should disclose the first app-open action")
    if manifest["verification"]["workflowRunId"] != WORKFLOW_RUN_ID:
      raise AssertionError("release manifest should record the workflow run id")
    if manifest["verification"]["packagedSmoke"] != "mounted-dmg-before-upload":
      raise AssertionError("release manifest should record the packaged smoke proof")
    if manifest["verification"]["packagedSmokeReceipt"]["checkIds"] != list(
      PACKAGED_SMOKE_REQUIRED_CHECK_IDS
    ):
      raise AssertionError("release manifest should record packaged smoke receipt checks")
    if manifest["verification"]["packagedSmokeReceipt"]["proofScope"] != PACKAGED_SMOKE_PROOF_SCOPE:
      raise AssertionError("release manifest should summarize packaged smoke proof scope")
    if manifest["verification"]["packagedSmokeReceipt"]["journey"] != packaged_smoke_journey():
      raise AssertionError("release manifest should summarize packaged smoke journey")
    if "sha256" not in manifest["verification"]["packagedSmokeReceipt"]:
      raise AssertionError("release manifest should hash the packaged smoke receipt")
    if (
      manifest["verification"]["packagedSmokeReceipt"]["packageMetadata"]
      != smoke_metadata_from_package_manifest(package_manifest)
    ):
      raise AssertionError("release manifest should record packaged smoke package metadata")
    if manifest["verification"]["assetNames"] != list(release_installer_asset_names("v0.1.0")):
      raise AssertionError("release manifest should record the exact installer asset names")
    if manifest["verification"]["checksumCommand"] != (
      "shasum -a 256 -c Pith-v0.1.0-macos-x86_64.dmg.sha256"
    ):
      raise AssertionError("release manifest should record the exact checksum command")
    if manifest["appPackage"]["sourceCommit"] != SOURCE_COMMIT:
      raise AssertionError("release manifest should summarize packaged app source")
    if manifest["appPackage"]["signing"] != "ad-hoc":
      raise AssertionError("release manifest should summarize packaged app signing")
    if manifest["appPackage"]["distributionTrust"] != "ad-hoc-not-notarized":
      raise AssertionError("release manifest should summarize packaged app trust")
    if manifest["appPackage"]["schemaVersion"] != PACKAGE_MANIFEST_SCHEMA_VERSION:
      raise AssertionError("release manifest should summarize package schema version")
    if "sha256" not in manifest["appPackage"]:
      raise AssertionError("release manifest should hash the package manifest")
    if manifest["appPackage"]["pithAccountRequired"] is not False:
      raise AssertionError("release manifest should summarize account-free package policy")
    if manifest["appPackage"]["defaultLocalExecutionSafetyMode"] != DEFAULT_LOCAL_EXECUTION_SAFETY_MODE:
      raise AssertionError("release manifest should summarize package default execution mode")
    if manifest["appPackage"]["localExecutionSafetyModes"] != list(LOCAL_EXECUTION_SAFETY_MODES):
      raise AssertionError("release manifest should summarize package execution modes")
    if manifest["appPackage"]["sandboxMode"] != SANDBOX_CONTRACT["mode"]:
      raise AssertionError("release manifest should summarize packaged sandbox mode")
    if manifest["appPackage"]["dailyDriverStageSource"] != DAILY_DRIVER_CONTRACT["stageSource"]:
      raise AssertionError("release manifest should summarize packaged daily-driver stage")
    if manifest["appPackage"]["firstAppOpenActionContract"] != FIRST_APP_OPEN_CONTRACT_ID:
      raise AssertionError("release manifest should summarize packaged first app-open action")
    if manifest["appPackage"]["sizeBudget"]["maxZipArtifactBytes"] != DEFAULT_MAX_ZIP_ARTIFACT_BYTES:
      raise AssertionError("release manifest should summarize package size budget")
    manifest_artifacts = {
      item["name"]: item
      for item in manifest["artifacts"]
    }
    if manifest_artifacts[checksum_path.name]["kind"] != "checksum":
      raise AssertionError("release manifest should include the checksum sidecar")
    if manifest_artifacts[checksum_path.name]["checks"] != artifact.name:
      raise AssertionError("checksum sidecar should target the DMG basename")
    if "sha256" not in manifest_artifacts[install_guide.name]:
      raise AssertionError("install guide manifest entry should be hashable")

    assert_raises(
      lambda: write_release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      package_manifest_path=package_manifest,
      smoke_receipt_path=smoke_receipt,
      output_path=root_path / "release-manifest.json",
      ),
      "public release manifest file names should match the release tag",
    )
    manifest_path = write_release_manifest(
      tag="v0.1.0",
      source_commit=SOURCE_COMMIT,
      signing_mode="ad-hoc",
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
      package_manifest_path=package_manifest,
      smoke_receipt_path=smoke_receipt,
      output_path=root_path / "Pith-v0.1.0-release-manifest.json",
    )
    validate_release_manifest(
      manifest_path,
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
      package_manifest_path=package_manifest,
      smoke_receipt_path=smoke_receipt,
    )
    validate_release_manifest(
      manifest_path,
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
      package_manifest_path=package_manifest,
    )

    manifest_data = manifest_path.read_text(encoding="utf-8")
    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["verification"]["packagedSmokeReceipt"]["checkIds"] = ["appLaunch"]
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        smoke_receipt_path=smoke_receipt,
      ),
      "wrong packaged smoke receipt should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["verification"]["packagedSmokeReceipt"]["journey"][0][
      "checkIds"
    ] = ["appLaunch"]
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        smoke_receipt_path=smoke_receipt,
      ),
      "wrong packaged smoke journey should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_smoke_receipt = write_smoke_receipt(
      root_path / "tampered-smoke-receipt.json",
      smoke_metadata_from_package_manifest(package_manifest),
    )
    tampered_smoke_data = json.loads(tampered_smoke_receipt.read_text(encoding="utf-8"))
    tampered_smoke_data["checks"] = tampered_smoke_data["checks"][:-1]
    tampered_smoke_receipt.write_text(json.dumps(tampered_smoke_data), encoding="utf-8")
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        smoke_receipt_path=tampered_smoke_receipt,
      ),
      "incomplete packaged smoke receipt should fail release manifest validation",
    )

    wrong_scope_receipt = write_smoke_receipt(
      root_path / "wrong-scope-smoke-receipt.json",
      smoke_metadata_from_package_manifest(package_manifest),
    )
    wrong_scope_data = json.loads(wrong_scope_receipt.read_text(encoding="utf-8"))
    wrong_scope_data["proofScope"] = "workspace only"
    wrong_scope_receipt.write_text(json.dumps(wrong_scope_data), encoding="utf-8")
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        smoke_receipt_path=wrong_scope_receipt,
      ),
      "wrong packaged smoke receipt scope should fail release manifest validation",
    )

    wrong_package_receipt = write_smoke_receipt(
      root_path / "wrong-package-smoke-receipt.json",
      {
        **smoke_metadata_from_package_manifest(package_manifest),
        "firstAppOpenActionContract": "static-checklist",
      },
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=package_manifest,
        smoke_receipt_path=wrong_package_receipt,
      ),
      "wrong packaged smoke package metadata should fail release manifest validation",
    )

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["schemaVersion"] = 2
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release manifest schema version should fail validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["releaseKind"] = "internal-ci"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release manifest kind should fail validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["verification"]["workflowRunUrl"] = "https://example.com/run"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release workflow run URL should fail validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["gatekeeper"] = "Developer ID signed and notarized."
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release Gatekeeper guidance should fail validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["verification"]["assetNames"] = ["Pith.dmg"]
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release asset set should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["verification"]["checksumCommand"] = "shasum Pith.dmg"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release checksum command should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["platform"]["architecture"] = "arm64"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release platform should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["sourceCommit"] = "short"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong source commit should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["appPackage"]["signing"] = "developer-id"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=package_manifest,
      ),
      "wrong release app package summary should fail validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["artifacts"].append({"name": "../Pith.dmg", "kind": "dmg"})
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "unsafe release artifact names should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["artifacts"].append(
      {
        "name": artifact.name,
        "kind": "duplicate",
      }
    )
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "duplicate release artifact names should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["artifacts"].append("Pith.dmg")
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "non-object release artifact entries should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["artifacts"].append(
      {
        "name": "extra.txt",
        "kind": "extra",
        "sizeBytes": 0,
      }
    )
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "extra release manifest artifacts should fail validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    wrong_artifact = root_path / "Pith-wrong-macos-x86_64.dmg"
    wrong_artifact.write_bytes(b"pith release artifact\n")
    wrong_checksum_path = write_checksum_file(wrong_artifact)
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=wrong_artifact,
        checksum_path=wrong_checksum_path,
        install_guide_path=install_guide,
      ),
      "public release DMG names should match the release tag",
    )

    wrong_checksum = root_path / "Pith-v0.1.0-macos-x86_64.checksum"
    wrong_checksum.write_text(checksum_text(artifact), encoding="utf-8")
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=wrong_checksum,
        install_guide_path=install_guide,
      ),
      "release checksum names should match the DMG sidecar name",
    )

    wrong_guide = root_path / "INSTALL.txt"
    wrong_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc"),
      encoding="utf-8",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=wrong_guide,
      ),
      "release install guide names should be stable",
    )

    assert_raises(
      lambda: release_manifest(
        tag="latest",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "release tags should be public vX.Y.Z tags or internal ci-* tags",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v1.2",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "partial public release tags should fail validation",
    )

    developer_package_manifest = write_package_manifest(
      root_path / "DeveloperIdPithPackage.json",
      signing="developer-id",
    )
    developer_manifest = release_manifest(
      tag="v0.1.0",
      source_commit=SOURCE_COMMIT,
      signing_mode="developer-id",
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
      package_manifest_path=developer_package_manifest,
    )
    if developer_manifest["trust"] != "developer-id-signed-notarized":
      raise AssertionError("Developer ID releases should record notarized trust")
    if developer_manifest["gatekeeper"] != release_gatekeeper_guidance("developer-id"):
      raise AssertionError("Developer ID releases should record normal Gatekeeper launch")
    if "Open Anyway" in developer_manifest["gatekeeper"]:
      raise AssertionError("Developer ID Gatekeeper guidance should not require manual approval")

    checksum_data = checksum_path.read_text(encoding="utf-8")
    checksum_path.write_text("0" * 64 + f"  {artifact.name}\n", encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "tampered release checksum should fail release manifest validation",
    )
    checksum_path.write_text(checksum_data, encoding="utf-8")

    install_guide.write_text(
      release_install_guide("v0.1.0", "developer-id"),
      encoding="utf-8",
    )
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "tampered install guide should fail release manifest validation",
    )
    install_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc"),
      encoding="utf-8",
    )

    install_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc").replace(
        "Pith-v0.1.0-macos-x86_64.dmg.sha256",
        "Pith-v0.2.0-macos-x86_64.dmg.sha256",
      ),
      encoding="utf-8",
    )
    assert_raises(
      lambda: validate_install_guide_for_tag(install_guide, "v0.1.0"),
      "install guide checksum command should match the release tag",
    )
    install_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc"),
      encoding="utf-8",
    )

    install_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc").replace(
        "Pith-v0.1.0-release-manifest.json",
        "Pith-v0.2.0-release-manifest.json",
      ),
      encoding="utf-8",
    )
    assert_raises(
      lambda: validate_install_guide_for_tag(install_guide, "v0.1.0"),
      "install guide manifest name should match the release tag",
    )
    install_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc"),
      encoding="utf-8",
    )

    wrong_package_manifest = write_package_manifest(
      root_path / "WrongSourcePithPackage.json",
      source_commit="abcdef0123456789abcdef0123456789abcdef01",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=wrong_package_manifest,
      ),
      "release manifest should reject mismatched packaged app source commits",
    )

    wrong_package_manifest = write_package_manifest(
      root_path / "WrongVersionPithPackage.json",
      bundle_version="0.2.0",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=wrong_package_manifest,
      ),
      "release manifest should reject packaged app versions that do not match the tag",
    )

    wrong_package_manifest = write_package_manifest(root_path / "WrongSchemaPithPackage.json")
    wrong_package_data = json.loads(wrong_package_manifest.read_text(encoding="utf-8"))
    wrong_package_data["schemaVersion"] = 2
    wrong_package_manifest.write_text(json.dumps(wrong_package_data), encoding="utf-8")
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=wrong_package_manifest,
      ),
      "release manifest should reject unsupported packaged app schema versions",
    )

    wrong_package_manifest = write_package_manifest(
      root_path / "WrongSigningPithPackage.json",
      signing="developer-id",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=wrong_package_manifest,
      ),
      "release manifest should reject mismatched packaged app signing",
    )

    wrong_package_manifest = write_package_manifest(
      root_path / "WrongModelDeliveryPithPackage.json",
      model_delivery="bundled",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=wrong_package_manifest,
      ),
      "release manifest should reject wrong packaged app model delivery",
    )

    wrong_package_manifest = write_package_manifest(
      root_path / "WrongSandboxPithPackage.json",
      sandbox_mode="none",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=wrong_package_manifest,
      ),
      "release manifest should reject wrong packaged sandbox metadata",
    )

    wrong_package_manifest = write_package_manifest(
      root_path / "WrongDailyDriverPithPackage.json",
      daily_driver_stage_source="app-only",
    )
    assert_raises(
      lambda: release_manifest(
        tag="v0.1.0",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=wrong_package_manifest,
      ),
      "release manifest should reject wrong packaged daily-driver metadata",
    )

    artifact.write_bytes(b"tampered release artifact\n")
    assert_raises(
      lambda: validate_checksum_file(artifact, checksum_path),
      "tampered release artifact should fail checksum validation",
    )
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "tampered release artifact should fail release manifest validation",
    )

  with tempfile.TemporaryDirectory(prefix="pith-release-artifacts-") as root:
    root_path = Path(root)
    artifact = root_path / "Pith-macos-x86_64.dmg"
    install_guide = root_path / "README-FIRST.txt"
    package_manifest = write_package_manifest(root_path / "PithPackage.json")
    artifact.write_bytes(b"pith internal ci artifact\n")
    install_guide.write_text(
      release_install_guide("ci-0123456789ab", "ad-hoc"),
      encoding="utf-8",
    )
    checksum_path = write_checksum_file(artifact)
    assert_raises(
      lambda: write_release_manifest(
        tag="ci-0123456789ab",
        source_commit=SOURCE_COMMIT,
        signing_mode="ad-hoc",
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
        package_manifest_path=package_manifest,
        output_path=root_path / "release-manifest.json",
      ),
      "internal release manifest file names should be stable",
    )
    manifest = release_manifest(
      tag="ci-0123456789ab",
      source_commit=SOURCE_COMMIT,
      signing_mode="ad-hoc",
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
      package_manifest_path=package_manifest,
    )
    if manifest["releaseKind"] != "internal-ci":
      raise AssertionError("ci tags should produce internal release manifests")
    expected_internal_assets = list(release_installer_asset_names("ci-0123456789ab"))
    if manifest["verification"]["assetNames"] != expected_internal_assets:
      raise AssertionError("internal release manifests should record internal asset names")
    if manifest["verification"]["checksumCommand"] != (
      "shasum -a 256 -c Pith-macos-x86_64.dmg.sha256"
    ):
      raise AssertionError("internal release manifests should record the checksum command")

  with tempfile.TemporaryDirectory(prefix="pith-release-artifacts-") as root:
    root_path = Path(root)
    weak_guide = root_path / "README-FIRST.txt"
    weak_guide.write_text("Install Pith from this DMG.\n", encoding="utf-8")
    assert_raises(
      lambda: validate_install_guide(weak_guide),
      "weak install guide should fail release guidance validation",
    )

  print("release artifact tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
