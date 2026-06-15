#!/usr/bin/env python3
"""Unit checks for downloaded release rehearsal validation."""

from __future__ import annotations

import json
from pathlib import Path
from tempfile import TemporaryDirectory

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
  packaged_smoke_package_metadata,
)
from release_copy_contract import FIRST_APP_OPEN_ACTION_COPY
from release_artifacts import packaged_smoke_journey
from release_artifacts import release_gatekeeper_guidance
from release_artifacts import release_installer_asset_names
from release_artifacts import write_checksum_file
from release_artifacts import write_release_manifest
from release_rehearsal_contract import summary_markdown
from release_rehearsal_contract import acceptance_markdown
from release_rehearsal_contract import write_acceptance
from release_rehearsal_contract import write_json
from release_rehearsal_contract import validate_release_rehearsal
from release_rehearsal_contract import write_summary
from release_text import install_guide as release_install_guide
from smoke_launch_macos_app import write_packaged_smoke_receipt


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
WORKFLOW_RUN_ID = "123456789"
WORKFLOW_RUN_URL = "https://github.com/walt1012/pith/actions/runs/123456789"


def package_manifest_data(tag: str, signing: str = "ad-hoc") -> dict:
  return {
    "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
    "appName": "Pith",
    "bundleVersion": tag.removeprefix("v"),
    "minimumSystemVersion": MINIMUM_SYSTEM_VERSION,
    "architecture": SUPPORTED_ARCH,
    "sourceCommit": SOURCE_COMMIT,
    "signing": signing,
    "distributionTrust": package_distribution_trust(signing),
    "modelDelivery": MODEL_DELIVERY_MODE,
    "defaultModelId": DEFAULT_MODEL_ID,
    "modelWeightsBundled": MODEL_WEIGHTS_BUNDLED,
    "modelMetadataBundled": MODEL_METADATA_BUNDLED,
    "pithAccountRequired": PITH_ACCOUNT_REQUIRED,
    "defaultLocalExecutionSafetyMode": DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
    "localExecutionSafetyModes": list(LOCAL_EXECUTION_SAFETY_MODES),
    "dailyDriverStageSource": DAILY_DRIVER_CONTRACT["stageSource"],
    "dailyDriverNextActionSource": DAILY_DRIVER_CONTRACT["nextActionSource"],
    "dailyDriverPresentation": DAILY_DRIVER_CONTRACT["presentation"],
    "firstAppOpenActionContract": FIRST_APP_OPEN_CONTRACT_ID,
    "sandboxMode": SANDBOX_CONTRACT["mode"],
    "sandboxBackend": SANDBOX_CONTRACT["backend"],
    "sandboxFallback": SANDBOX_CONTRACT["fallback"],
    "sandboxNetworkDefault": SANDBOX_CONTRACT["networkDefault"],
    "sizeBudget": {
      "maxAppBundleBytes": DEFAULT_MAX_APP_BUNDLE_BYTES,
      "maxZipArtifactBytes": DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
    },
  }


def write_downloaded_assets(root: Path, tag: str = "v0.1.0") -> None:
  dmg_name, checksum_name, guide_name, manifest_name = release_installer_asset_names(tag)
  artifact = root / dmg_name
  artifact.write_bytes(b"pith downloaded release artifact\n")
  guide = root / guide_name
  guide.write_text(release_install_guide(tag, "ad-hoc"), encoding="utf-8")
  checksum = write_checksum_file(artifact, root / checksum_name)
  package_manifest = root / "PithPackage.json"
  package_manifest.write_text(
    json.dumps(package_manifest_data(tag), indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
  )
  smoke_receipt = root / "packaged-smoke-receipt.json"
  write_packaged_smoke_receipt(
    smoke_receipt,
    package_metadata=packaged_smoke_package_metadata(package_manifest_data(tag)),
  )
  write_release_manifest(
    tag=tag,
    source_commit=SOURCE_COMMIT,
    signing_mode="ad-hoc",
    artifact_path=artifact,
    checksum_path=checksum,
    install_guide_path=guide,
    package_manifest_path=package_manifest,
    smoke_receipt_path=smoke_receipt,
    output_path=root / manifest_name,
    workflow_run_id=WORKFLOW_RUN_ID,
    workflow_run_url=WORKFLOW_RUN_URL,
  )
  package_manifest.unlink()
  smoke_receipt.unlink()


def expect_failure(action, expected: str) -> None:
  try:
    action()
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r}, got {error!r}") from error
    return
  raise AssertionError(f"expected release rehearsal validation to fail: {expected}")


def main() -> int:
  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    summary = validate_release_rehearsal("v0.1.0", root)
    if summary["result"] != "passed":
      raise AssertionError("release rehearsal summary should pass")
    if summary["defaultModelId"] != DEFAULT_MODEL_ID:
      raise AssertionError("release rehearsal summary should record the default model")
    if summary["trust"]["mode"] != "ad-hoc-not-notarized":
      raise AssertionError("release rehearsal summary should record ad-hoc trust")
    if "Open Anyway" not in summary["trust"]["gatekeeper"]:
      raise AssertionError("release rehearsal summary should record Gatekeeper manual approval")
    if summary["trust"]["gatekeeper"] != release_gatekeeper_guidance("ad-hoc"):
      raise AssertionError("release rehearsal summary should use manifest Gatekeeper guidance")
    if "download-default-verified-local-model" not in summary["firstRun"].values():
      raise AssertionError("release rehearsal summary should include the first-run model step")
    if summary["firstRun"].get("firstAppOpen") != FIRST_APP_OPEN_CONTRACT_ID:
      raise AssertionError("release rehearsal summary should include the first app-open contract")
    if summary["dailyDriver"] != DAILY_DRIVER_CONTRACT:
      raise AssertionError("release rehearsal summary should record daily-driver readiness")
    if (
      summary["releaseDecision"]["manualAcceptance"]
      != "required-before-visible-prerelease"
    ):
      raise AssertionError("release rehearsal summary should require manual acceptance")
    if "do-not-publish-visible-ad-hoc" not in summary["releaseDecision"]["publishGate"]:
      raise AssertionError("release rehearsal summary should guard visible ad-hoc publish")
    if summary["appPackage"]["firstAppOpenActionContract"] != FIRST_APP_OPEN_CONTRACT_ID:
      raise AssertionError("release rehearsal summary should record app package first app-open proof")
    if (
      summary["packagedSmokeReceipt"]["packageMetadata"]["firstAppOpenActionContract"]
      != FIRST_APP_OPEN_CONTRACT_ID
    ):
      raise AssertionError("release rehearsal summary should record smoke package metadata")
    if summary["packagedSmokeReceipt"]["packageMetadataMatched"] is not True:
      raise AssertionError("release rehearsal summary should prove smoke package metadata match")
    if summary["packagedSmokeReceipt"]["journey"] != packaged_smoke_journey():
      raise AssertionError("release rehearsal summary should include packaged smoke journey")
    manual_checks = "\n".join(summary["manualPrereleaseChecks"])
    for phrase in (
      "SHA-256 sidecar",
      DEFAULT_MODEL_ID,
      "project readiness",
      "Web Search",
      "reviewing the diff",
      "Pith status",
    ):
      if phrase not in manual_checks:
        raise AssertionError(f"manual prerelease checks should include {phrase}")
    if "runtime readiness" in manual_checks or "local service status" in manual_checks:
      raise AssertionError("manual prerelease checks should use Pith status language")
    if FIRST_APP_OPEN_ACTION_COPY not in summary["firstAppOpenChecks"]:
      raise AssertionError("release rehearsal summary should name the first cowork prompts")
    first_app_open_checks = "\n".join(summary["firstAppOpenChecks"])
    if "Check that Web Search and project safety are ready." not in first_app_open_checks:
      raise AssertionError("first app-open checks should use product-level setup language")
    if "Web Search readiness" in first_app_open_checks:
      raise AssertionError("first app-open checks should not use internal readiness wording")
    markdown = summary_markdown(summary)
    if "Pith v0.1.0 Release Rehearsal" not in markdown:
      raise AssertionError("release rehearsal markdown should name the tag")
    if "## First App Open" not in markdown:
      raise AssertionError("release rehearsal markdown should include first app open checks")
    if "First app-open contract" not in markdown:
      raise AssertionError("release rehearsal markdown should include package contract proof")
    if "Gatekeeper" not in markdown or "Open Anyway" not in markdown:
      raise AssertionError("release rehearsal markdown should include Gatekeeper guidance")
    if "Smoke package metadata: `matches app package metadata`" not in markdown:
      raise AssertionError("release rehearsal markdown should include smoke metadata match proof")
    if "## Packaged Smoke Journey" not in markdown or "Web Search retrieval" not in markdown:
      raise AssertionError("release rehearsal markdown should include smoke journey stages")
    if "## Manual Prerelease Acceptance" not in markdown:
      raise AssertionError("release rehearsal markdown should include manual acceptance")
    if "## Release Decision" not in markdown:
      raise AssertionError("release rehearsal markdown should include release decision")
    if "required-before-visible-prerelease" not in markdown:
      raise AssertionError("release rehearsal markdown should require manual acceptance")
    if "- [ ] Let the model use Web Search" not in markdown:
      raise AssertionError("release rehearsal markdown should include Web Search acceptance")
    acceptance = acceptance_markdown(summary)
    if "Manual Release Acceptance" not in acceptance:
      raise AssertionError("manual acceptance markdown should name its purpose")
    if "Accept this build for visible ad-hoc prerelease" not in acceptance:
      raise AssertionError("manual acceptance markdown should include an accept decision")
    if "Keep this build draft-only" not in acceptance:
      raise AssertionError("manual acceptance markdown should include a reject decision")
    for name in release_installer_asset_names("v0.1.0"):
      if f"Download `{name}`" not in acceptance:
        raise AssertionError("manual acceptance markdown should list every release asset")
    for phrase in (
      "Checksum verification result",
      "Gatekeeper path used",
      "Web Search proof inspected",
      "Approval and diff receipt inspected",
      "Restart recovery result",
    ):
      if phrase not in acceptance:
        raise AssertionError(f"manual acceptance markdown should request evidence for {phrase}")
    output = root / "rehearsal.md"
    write_summary(output, summary)
    if "Result: `passed`" not in output.read_text(encoding="utf-8"):
      raise AssertionError("release rehearsal summary file should record the result")
    acceptance_output = root / "acceptance.md"
    write_acceptance(acceptance_output, summary)
    if "Required Manual Checks" not in acceptance_output.read_text(encoding="utf-8"):
      raise AssertionError("manual acceptance file should record required checks")
    json_output = root / "rehearsal.json"
    write_json(json_output, summary)
    summary_payload = json.loads(json_output.read_text(encoding="utf-8"))
    if summary_payload["result"] != "passed":
      raise AssertionError("release rehearsal JSON should record the result")
    if summary_payload["releaseDecision"] != summary["releaseDecision"]:
      raise AssertionError("release rehearsal JSON should preserve release decision evidence")
    if summary_payload["manualPrereleaseChecks"] != summary["manualPrereleaseChecks"]:
      raise AssertionError("release rehearsal JSON should preserve manual acceptance checks")

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    (root / "unexpected.txt").write_text("extra release asset\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "must not include extra entries",
    )
    validate_release_rehearsal(
      "v0.1.0",
      root,
      allow_extra_assets=True,
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["gatekeeper"] = "Developer ID signed and notarized."
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "Gatekeeper guidance",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["verification"]["packagedSmokeReceipt"]["journey"][3]["checkIds"] = []
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "journey",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["modelDelivery"]["defaultModelId"] = "wrong-model"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "default model",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["dailyDriver"]["nextActionSource"] = "static-checklist"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "daily driver nextActionSource must be runtime/readiness",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["sandbox"]["fallback"] = "processOnly"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "sandbox fallback must be processOnlyWhenNativeUnavailable",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["verification"]["packagedSmokeReceipt"]["result"] = "failed"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "packagedSmokeReceipt result must be 'passed'",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["verification"]["packagedSmokeReceipt"]["packageMetadata"][
      "firstAppOpenActionContract"
    ] = "static-checklist"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "smoke metadata must match app package metadata",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    (root / "Pith-v0.1.0-macos-x86_64.dmg").write_bytes(b"tampered artifact\n")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "Release checksum does not match artifact",
    )

  print("release rehearsal contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
