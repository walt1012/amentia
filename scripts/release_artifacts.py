#!/usr/bin/env python3
"""Prepare and validate user-facing release artifact sidecar files."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

from package_contract import (
  DAILY_DRIVER_CONTRACT,
  DEFAULT_MODEL_ID,
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  LOCAL_EXECUTION_SAFETY_MODES,
  MINIMUM_SYSTEM_VERSION,
  MODEL_DELIVERY_MODE,
  MODEL_WEIGHTS_BUNDLED,
  PACKAGE_MANIFEST_SCHEMA_VERSION,
  PACKAGED_SMOKE_RECEIPT_KIND,
  PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION,
  PACKAGED_SMOKE_PROOF_SCOPE,
  PACKAGED_SMOKE_REQUIRED_CHECK_IDS,
  PITH_ACCOUNT_REQUIRED,
  RELEASE_SIGNING_MODES,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  package_distribution_trust,
  validate_package_manifest_contract,
)
from release_copy_contract import require_install_guide_copy
from release_identity import product_version_from_tag
from release_identity import validate_public_release_tag


SOURCE_COMMIT_HEX_LENGTH = 40
INTERNAL_CI_TAG_PATTERN = re.compile(r"^ci-[0-9a-f]{12,40}$")
INSTALL_GUIDE_NAME = "README-FIRST.txt"
INTERNAL_CI_DMG_NAME = "Pith-macos-x86_64.dmg"
VERIFICATION_CONTRACT = {
  "ciGate": "successful-ci-required-for-source-commit",
  "packagedSmoke": "mounted-dmg-before-upload",
  "checksum": "sha256-sidecar",
  "installGuide": INSTALL_GUIDE_NAME,
  "assetSet": "dmg-checksum-install-guide-manifest",
}
FIRST_RUN_CONTRACT = {
  "model": "download-default-verified-local-model",
  "workspace": "open-workspace-folder",
  "retrieval": "web-search-readiness",
  "sandbox": "status-visible",
  "approval": "review-before-local-change",
  "proof": "timeline-proof",
  "nextAction": "runtime-readiness",
}
GITHUB_RUN_ID_PATTERN = re.compile(r"^[1-9][0-9]*$")


def sha256_hex(path: Path) -> str:
  hasher = hashlib.sha256()
  with path.open("rb") as file:
    for chunk in iter(lambda: file.read(1024 * 1024), b""):
      hasher.update(chunk)
  return hasher.hexdigest()


def checksum_text(artifact_path: Path) -> str:
  if not artifact_path.is_file():
    raise FileNotFoundError(f"Release artifact is missing: {artifact_path}")
  return f"{sha256_hex(artifact_path)}  {artifact_path.name}\n"


def write_checksum_file(artifact_path: Path, checksum_path: Path | None = None) -> Path:
  output_path = checksum_path or artifact_path.with_name(f"{artifact_path.name}.sha256")
  output_path.parent.mkdir(parents=True, exist_ok=True)
  output_path.write_text(checksum_text(artifact_path), encoding="utf-8")
  validate_checksum_file(artifact_path, output_path)
  return output_path


def release_manifest(
  *,
  tag: str,
  source_commit: str,
  signing_mode: str,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
  workflow_run_id: str,
  workflow_run_url: str,
  package_manifest_path: Path | None = None,
  smoke_receipt_path: Path | None = None,
) -> dict:
  validate_release_identity(tag, source_commit, signing_mode)
  validate_release_asset_names(
    tag=tag,
    artifact_path=artifact_path,
    checksum_path=checksum_path,
    install_guide_path=install_guide_path,
  )
  validate_checksum_file(artifact_path, checksum_path)
  validate_install_guide_for_tag(install_guide_path, tag)
  package_summary = package_manifest_summary(
    package_manifest_path,
    source_commit=source_commit,
    signing_mode=signing_mode,
  )
  validate_package_version_matches_tag(tag, package_summary)

  manifest = {
    "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
    "tag": tag,
    "releaseKind": release_kind(tag),
    "sourceCommit": source_commit,
    "product": "Pith",
    "platform": {
      "os": "macOS",
      "minimumVersion": MINIMUM_SYSTEM_VERSION,
      "architecture": SUPPORTED_ARCH,
    },
    "signingMode": signing_mode,
    "trust": release_trust(signing_mode),
    "modelDelivery": {
      "mode": MODEL_DELIVERY_MODE,
      "defaultModelId": DEFAULT_MODEL_ID,
      "modelWeightsBundled": MODEL_WEIGHTS_BUNDLED,
    },
    "identity": {
      "pithAccountRequired": PITH_ACCOUNT_REQUIRED,
    },
    "localExecution": {
      "defaultSafetyMode": DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
      "safetyModes": list(LOCAL_EXECUTION_SAFETY_MODES),
    },
    "sandbox": dict(SANDBOX_CONTRACT),
    "dailyDriver": dict(DAILY_DRIVER_CONTRACT),
    "firstRun": dict(FIRST_RUN_CONTRACT),
    "verification": release_verification(
      tag=tag,
      workflow_run_id=workflow_run_id,
      workflow_run_url=workflow_run_url,
      smoke_receipt_path=smoke_receipt_path,
    ),
    "artifacts": [
      {
        "name": artifact_path.name,
        "kind": "dmg",
        "sizeBytes": artifact_path.stat().st_size,
        "sha256": sha256_hex(artifact_path),
        "checksum": checksum_path.name,
      },
      {
        "name": checksum_path.name,
        "kind": "checksum",
        "sizeBytes": checksum_path.stat().st_size,
        "sha256": sha256_hex(checksum_path),
        "checks": artifact_path.name,
      },
      {
        "name": install_guide_path.name,
        "kind": "install-guide",
        "sizeBytes": install_guide_path.stat().st_size,
        "sha256": sha256_hex(install_guide_path),
      },
    ],
  }
  if package_summary is not None:
    manifest["appPackage"] = package_summary
  return manifest


def write_release_manifest(
  *,
  tag: str,
  source_commit: str,
  signing_mode: str,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
  output_path: Path,
  workflow_run_id: str,
  workflow_run_url: str,
  package_manifest_path: Path | None = None,
  smoke_receipt_path: Path | None = None,
) -> Path:
  validate_release_manifest_name(tag, output_path)
  output_path.parent.mkdir(parents=True, exist_ok=True)
  output_path.write_text(
    json.dumps(
      release_manifest(
        tag=tag,
        source_commit=source_commit,
        signing_mode=signing_mode,
        artifact_path=artifact_path,
        checksum_path=checksum_path,
        install_guide_path=install_guide_path,
        package_manifest_path=package_manifest_path,
        smoke_receipt_path=smoke_receipt_path,
        workflow_run_id=workflow_run_id,
        workflow_run_url=workflow_run_url,
      ),
      indent=2,
      sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
  )
  validate_release_manifest(
    output_path,
    artifact_path=artifact_path,
    checksum_path=checksum_path,
    install_guide_path=install_guide_path,
    package_manifest_path=package_manifest_path,
    smoke_receipt_path=smoke_receipt_path,
  )
  return output_path


def validate_release_manifest(
  manifest_path: Path,
  *,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
  package_manifest_path: Path | None = None,
  smoke_receipt_path: Path | None = None,
) -> None:
  if not manifest_path.is_file():
    raise FileNotFoundError(f"Release manifest is missing: {manifest_path}")
  manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
  validate_manifest_identity(manifest, smoke_receipt_path=smoke_receipt_path)
  tag = manifest["tag"]
  validate_release_manifest_name(tag, manifest_path)
  validate_release_asset_names(
    tag=tag,
    artifact_path=artifact_path,
    checksum_path=checksum_path,
    install_guide_path=install_guide_path,
  )
  artifacts = manifest.get("artifacts")
  if not isinstance(artifacts, list):
    raise RuntimeError("Release manifest must include an artifacts list")
  validate_checksum_file(artifact_path, checksum_path)
  by_name = validate_manifest_artifacts(artifacts)
  expected_artifact_names = {
    artifact_path.name,
    checksum_path.name,
    install_guide_path.name,
  }
  if set(by_name) != expected_artifact_names:
    raise RuntimeError("Release manifest artifacts must exactly match release assets")
  package_summary = package_manifest_summary(
    package_manifest_path,
    source_commit=manifest["sourceCommit"],
    signing_mode=manifest["signingMode"],
  )
  validate_package_version_matches_tag(tag, package_summary)
  if package_summary is not None and manifest.get("appPackage") != package_summary:
    raise RuntimeError("Release manifest app package summary does not match PithPackage.json")
  artifact_entry = by_name.get(artifact_path.name)
  if artifact_entry is None:
    raise RuntimeError("Release manifest is missing the DMG artifact entry")
  if artifact_entry.get("kind") != "dmg":
    raise RuntimeError("Release manifest DMG entry kind is wrong")
  if artifact_entry.get("sizeBytes") != artifact_path.stat().st_size:
    raise RuntimeError("Release manifest DMG size does not match the artifact")
  if artifact_entry.get("sha256") != sha256_hex(artifact_path):
    raise RuntimeError("Release manifest DMG SHA-256 does not match the artifact")
  if artifact_entry.get("checksum") != checksum_path.name:
    raise RuntimeError("Release manifest DMG checksum file name is wrong")
  checksum_entry = by_name.get(checksum_path.name)
  if checksum_entry is None:
    raise RuntimeError("Release manifest is missing the checksum artifact entry")
  if checksum_entry.get("kind") != "checksum":
    raise RuntimeError("Release manifest checksum entry kind is wrong")
  if checksum_entry.get("checks") != artifact_path.name:
    raise RuntimeError("Release manifest checksum entry target is wrong")
  if checksum_entry.get("sizeBytes") != checksum_path.stat().st_size:
    raise RuntimeError("Release manifest checksum size does not match")
  if checksum_entry.get("sha256") != sha256_hex(checksum_path):
    raise RuntimeError("Release manifest checksum SHA-256 does not match")
  guide_entry = by_name.get(install_guide_path.name)
  if guide_entry is None:
    raise RuntimeError("Release manifest is missing the install guide entry")
  if guide_entry.get("kind") != "install-guide":
    raise RuntimeError("Release manifest install guide entry kind is wrong")
  if guide_entry.get("sizeBytes") != install_guide_path.stat().st_size:
    raise RuntimeError("Release manifest install guide size does not match")
  if guide_entry.get("sha256") != sha256_hex(install_guide_path):
    raise RuntimeError("Release manifest install guide SHA-256 does not match")
  validate_install_guide_for_tag(install_guide_path, tag)


def validate_manifest_artifacts(artifacts: list) -> dict[str, dict]:
  by_name: dict[str, dict] = {}
  for item in artifacts:
    if not isinstance(item, dict):
      raise RuntimeError("Release manifest artifact entries must be objects")
    name = item.get("name")
    if not isinstance(name, str) or not name.strip():
      raise RuntimeError("Release manifest artifact entries must include names")
    if name in {".", ".."} or "/" in name or "\\" in name:
      raise RuntimeError("Release manifest artifact names must be basenames")
    if name in by_name:
      raise RuntimeError("Release manifest artifact names must be unique")
    by_name[name] = item
  return by_name


def package_manifest_summary(
  package_manifest_path: Path | None,
  *,
  source_commit: str,
  signing_mode: str,
) -> dict | None:
  if package_manifest_path is None:
    return None
  package_manifest = read_json_object(package_manifest_path, "PithPackage.json")
  bundle_version = package_manifest.get("bundleVersion")
  if not isinstance(bundle_version, str) or not bundle_version.strip():
    raise RuntimeError(f"PithPackage.json bundleVersion is required: {package_manifest_path}")
  size_budget = validate_package_manifest_contract(
    package_manifest,
    f"PithPackage.json: {package_manifest_path}",
    source_commit=source_commit,
    signing_mode=signing_mode,
  )
  return {
    "manifest": package_manifest_path.name,
    "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
    "sha256": sha256_hex(package_manifest_path),
    "bundleVersion": bundle_version,
    "sourceCommit": source_commit,
    "signing": signing_mode,
    "distributionTrust": package_distribution_trust(signing_mode),
    "architecture": SUPPORTED_ARCH,
    "minimumSystemVersion": MINIMUM_SYSTEM_VERSION,
    "modelDelivery": MODEL_DELIVERY_MODE,
    "defaultModelId": DEFAULT_MODEL_ID,
    "modelWeightsBundled": MODEL_WEIGHTS_BUNDLED,
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
    "sizeBudget": size_budget,
  }

def validate_package_version_matches_tag(tag: str, package_summary: dict | None) -> None:
  if package_summary is None or release_kind(tag) != "public":
    return
  expected_version = product_version_from_tag(tag)
  if package_summary.get("bundleVersion") != expected_version:
    raise RuntimeError(
      "PithPackage.json bundleVersion must match the public release tag"
    )


def read_json_object(path: Path, label: str) -> dict:
  if not path.is_file():
    raise FileNotFoundError(f"{label} is missing: {path}")
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError(f"{label} must be a JSON object: {path}")
  return data


def validate_release_identity(tag: str, source_commit: str, signing_mode: str) -> None:
  release_kind(tag)
  validate_source_commit(source_commit)
  if signing_mode not in RELEASE_SIGNING_MODES:
    raise RuntimeError(f"Unsupported release signing mode: {signing_mode}")


def release_kind(tag: str) -> str:
  if not isinstance(tag, str) or not tag.strip():
    raise RuntimeError("Release manifest tag is required")
  try:
    validate_public_release_tag(tag)
    return "public"
  except RuntimeError:
    pass
  if INTERNAL_CI_TAG_PATTERN.fullmatch(tag):
    return "internal-ci"
  raise RuntimeError(
    "Release manifest tag must be a public vX.Y.Z tag or internal ci-* tag"
  )


def validate_release_asset_names(
  *,
  tag: str,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
) -> None:
  expected_dmg_name = release_dmg_name(tag)
  if artifact_path.name != expected_dmg_name:
    raise RuntimeError(
      f"Release DMG name must be {expected_dmg_name}: {artifact_path.name}"
    )
  expected_checksum_name = f"{artifact_path.name}.sha256"
  if checksum_path.name != expected_checksum_name:
    raise RuntimeError(
      f"Release checksum name must be {expected_checksum_name}: {checksum_path.name}"
    )
  if install_guide_path.name != INSTALL_GUIDE_NAME:
    raise RuntimeError(
      f"Release install guide must be named {INSTALL_GUIDE_NAME}: {install_guide_path.name}"
    )


def validate_release_manifest_name(tag: str, manifest_path: Path) -> None:
  expected_name = release_manifest_name(tag)
  if manifest_path.name != expected_name:
    raise RuntimeError(
      f"Release manifest name must be {expected_name}: {manifest_path.name}"
    )


def release_dmg_name(tag: str) -> str:
  if release_kind(tag) == "internal-ci":
    return INTERNAL_CI_DMG_NAME
  return f"Pith-{tag}-macos-x86_64.dmg"


def release_manifest_name(tag: str) -> str:
  if release_kind(tag) == "internal-ci":
    return "internal-release-manifest.json"
  return f"Pith-{tag}-release-manifest.json"


def release_installer_asset_names(tag: str) -> tuple[str, str, str, str]:
  dmg_name = release_dmg_name(tag)
  return (
    dmg_name,
    f"{dmg_name}.sha256",
    INSTALL_GUIDE_NAME,
    release_manifest_name(tag),
  )


def validate_source_commit(source_commit: str) -> None:
  if len(source_commit) != SOURCE_COMMIT_HEX_LENGTH:
    raise RuntimeError("Release manifest source commit must be a full SHA-1 hash")
  if any(character not in "0123456789abcdef" for character in source_commit):
    raise RuntimeError("Release manifest source commit must be lowercase hex")


def validate_manifest_identity(
  manifest: dict,
  *,
  smoke_receipt_path: Path | None = None,
) -> None:
  if manifest.get("schemaVersion") != PACKAGE_MANIFEST_SCHEMA_VERSION:
    raise RuntimeError(
      f"Release manifest schema version must be {PACKAGE_MANIFEST_SCHEMA_VERSION}"
    )
  tag = manifest.get("tag")
  kind = release_kind(tag)
  if manifest.get("releaseKind") != kind:
    raise RuntimeError("Release manifest kind does not match its tag")
  source_commit = manifest.get("sourceCommit")
  if not isinstance(source_commit, str):
    raise RuntimeError("Release manifest source commit is required")
  validate_source_commit(source_commit)
  if manifest.get("product") != "Pith":
    raise RuntimeError("Release manifest product must be Pith")
  platform = manifest.get("platform")
  if not isinstance(platform, dict):
    raise RuntimeError("Release manifest platform must be an object")
  expected_platform = {
    "os": "macOS",
    "minimumVersion": MINIMUM_SYSTEM_VERSION,
    "architecture": SUPPORTED_ARCH,
  }
  for key, expected in expected_platform.items():
    if platform.get(key) != expected:
      raise RuntimeError(f"Release manifest platform {key} must be {expected}")

  signing_mode = manifest.get("signingMode")
  if signing_mode not in RELEASE_SIGNING_MODES:
    raise RuntimeError("Release manifest signing mode is unsupported")
  if manifest.get("trust") != release_trust(signing_mode):
    raise RuntimeError("Release manifest trust does not match signing mode")

  model_delivery = manifest.get("modelDelivery")
  if not isinstance(model_delivery, dict):
    raise RuntimeError("Release manifest model delivery must be an object")
  if model_delivery.get("mode") != MODEL_DELIVERY_MODE:
    raise RuntimeError(f"Release manifest model delivery mode must be {MODEL_DELIVERY_MODE}")
  if model_delivery.get("defaultModelId") != DEFAULT_MODEL_ID:
    raise RuntimeError(f"Release manifest default model id must be {DEFAULT_MODEL_ID}")
  if model_delivery.get("modelWeightsBundled") is not MODEL_WEIGHTS_BUNDLED:
    raise RuntimeError("Release manifest must state that model weights are not bundled")
  validate_identity_contract(manifest.get("identity"), "Release manifest identity")
  validate_local_execution_contract(
    manifest.get("localExecution"),
    "Release manifest local execution",
  )
  validate_sandbox_contract(manifest.get("sandbox"), "Release manifest sandbox")
  validate_daily_driver_contract(
    manifest.get("dailyDriver"),
    "Release manifest daily driver",
  )
  validate_first_run_contract(manifest.get("firstRun"), "Release manifest first run")
  validate_verification_contract(
    manifest.get("verification"),
    "Release manifest verification",
    tag=tag,
    smoke_receipt_path=smoke_receipt_path,
  )


def validate_sandbox_contract(value: object, label: str) -> None:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} must be an object")
  for field, expected in SANDBOX_CONTRACT.items():
    if value.get(field) != expected:
      raise RuntimeError(f"{label} {field} must be {expected}")


def validate_identity_contract(value: object, label: str) -> None:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} must be an object")
  if value.get("pithAccountRequired") is not PITH_ACCOUNT_REQUIRED:
    raise RuntimeError(f"{label} pithAccountRequired must be false")


def validate_local_execution_contract(value: object, label: str) -> None:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} must be an object")
  if value.get("defaultSafetyMode") != DEFAULT_LOCAL_EXECUTION_SAFETY_MODE:
    raise RuntimeError(
      f"{label} defaultSafetyMode must be {DEFAULT_LOCAL_EXECUTION_SAFETY_MODE}"
    )
  if value.get("safetyModes") != list(LOCAL_EXECUTION_SAFETY_MODES):
    raise RuntimeError(f"{label} safetyModes must match the package contract")


def validate_daily_driver_contract(value: object, label: str) -> None:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} must be an object")
  for field, expected in DAILY_DRIVER_CONTRACT.items():
    if value.get(field) != expected:
      raise RuntimeError(f"{label} {field} must be {expected}")


def validate_first_run_contract(value: object, label: str) -> None:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} must be an object")
  for field, expected in FIRST_RUN_CONTRACT.items():
    if value.get(field) != expected:
      raise RuntimeError(f"{label} {field} must be {expected}")


def release_verification(
  *,
  tag: str,
  workflow_run_id: str,
  workflow_run_url: str,
  smoke_receipt_path: Path | None = None,
) -> dict:
  validate_workflow_run_metadata(workflow_run_id, workflow_run_url)
  _dmg_name, checksum_name, _guide_name, _manifest_name = release_installer_asset_names(tag)
  smoke_receipt = packaged_smoke_receipt_summary(smoke_receipt_path)
  return {
    **VERIFICATION_CONTRACT,
    "assetNames": list(release_installer_asset_names(tag)),
    "checksumCommand": f"shasum -a 256 -c {checksum_name}",
    "workflowRunId": workflow_run_id,
    "workflowRunUrl": workflow_run_url,
    "packagedSmokeReceipt": smoke_receipt,
  }


def validate_verification_contract(
  value: object,
  label: str,
  *,
  tag: str,
  smoke_receipt_path: Path | None = None,
) -> None:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} must be an object")
  for field, expected in VERIFICATION_CONTRACT.items():
    if value.get(field) != expected:
      raise RuntimeError(f"{label} {field} must be {expected}")
  expected_asset_names = list(release_installer_asset_names(tag))
  if value.get("assetNames") != expected_asset_names:
    raise RuntimeError(f"{label} assetNames must match the installer asset contract")
  expected_checksum_command = f"shasum -a 256 -c {expected_asset_names[1]}"
  if value.get("checksumCommand") != expected_checksum_command:
    raise RuntimeError(f"{label} checksumCommand must match the checksum sidecar")
  workflow_run_id = value.get("workflowRunId")
  workflow_run_url = value.get("workflowRunUrl")
  if not isinstance(workflow_run_id, str) or not isinstance(workflow_run_url, str):
    raise RuntimeError(f"{label} must include workflow run metadata")
  validate_workflow_run_metadata(workflow_run_id, workflow_run_url)
  packaged_smoke_receipt = value.get("packagedSmokeReceipt")
  if smoke_receipt_path is None:
    validate_packaged_smoke_receipt_summary(packaged_smoke_receipt, label)
  else:
    expected_smoke_receipt = packaged_smoke_receipt_summary(smoke_receipt_path)
    if packaged_smoke_receipt != expected_smoke_receipt:
      raise RuntimeError(f"{label} packagedSmokeReceipt must match the smoke receipt")


def packaged_smoke_receipt_summary(smoke_receipt_path: Path | None) -> dict:
  if smoke_receipt_path is None:
    return {
      "schemaVersion": PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION,
      "kind": PACKAGED_SMOKE_RECEIPT_KIND,
      "result": "passed",
      "proofScope": PACKAGED_SMOKE_PROOF_SCOPE,
      "checkIds": list(PACKAGED_SMOKE_REQUIRED_CHECK_IDS),
    }
  receipt = read_json_object(smoke_receipt_path, "Packaged smoke receipt")
  if receipt.get("schemaVersion") != PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION:
    raise RuntimeError("Packaged smoke receipt schema version is unsupported")
  if receipt.get("kind") != PACKAGED_SMOKE_RECEIPT_KIND:
    raise RuntimeError("Packaged smoke receipt kind is unsupported")
  if receipt.get("result") != "passed":
    raise RuntimeError("Packaged smoke receipt must record a passed result")
  checks = receipt.get("checks")
  if not isinstance(checks, list):
    raise RuntimeError("Packaged smoke receipt checks must be a list")
  check_ids = []
  for item in checks:
    if not isinstance(item, dict):
      raise RuntimeError("Packaged smoke receipt checks must be objects")
    check_id = item.get("id")
    if not isinstance(check_id, str) or not check_id:
      raise RuntimeError("Packaged smoke receipt check id is required")
    check_ids.append(check_id)
  if check_ids != list(PACKAGED_SMOKE_REQUIRED_CHECK_IDS):
    raise RuntimeError("Packaged smoke receipt check ids do not match the first-run contract")
  return {
    "schemaVersion": PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION,
    "kind": PACKAGED_SMOKE_RECEIPT_KIND,
    "result": "passed",
    "proofScope": PACKAGED_SMOKE_PROOF_SCOPE,
    "checkIds": check_ids,
    "sha256": sha256_hex(smoke_receipt_path),
  }


def validate_packaged_smoke_receipt_summary(value: object, label: str) -> None:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} packagedSmokeReceipt must be an object")
  expected_values = {
    "schemaVersion": PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION,
    "kind": PACKAGED_SMOKE_RECEIPT_KIND,
    "result": "passed",
    "proofScope": PACKAGED_SMOKE_PROOF_SCOPE,
    "checkIds": list(PACKAGED_SMOKE_REQUIRED_CHECK_IDS),
  }
  for field, expected in expected_values.items():
    if value.get(field) != expected:
      raise RuntimeError(f"{label} packagedSmokeReceipt {field} must be {expected!r}")
  receipt_hash = value.get("sha256")
  if receipt_hash is not None and (
    not isinstance(receipt_hash, str)
    or len(receipt_hash) != 64
    or any(character not in "0123456789abcdef" for character in receipt_hash)
  ):
    raise RuntimeError(f"{label} packagedSmokeReceipt sha256 must be lowercase hex")


def validate_workflow_run_metadata(workflow_run_id: str, workflow_run_url: str) -> None:
  if GITHUB_RUN_ID_PATTERN.fullmatch(workflow_run_id) is None:
    raise RuntimeError("Release manifest workflow run id must be a GitHub Actions run id")
  expected_suffix = f"/actions/runs/{workflow_run_id}"
  if not workflow_run_url.startswith("https://github.com/") or not workflow_run_url.endswith(
    expected_suffix
  ):
    raise RuntimeError("Release manifest workflow run URL must match the run id")


def release_trust(signing_mode: str) -> str:
  if signing_mode == "developer-id":
    return "developer-id-signed-notarized"
  return "ad-hoc-not-notarized"


def validate_install_guide(install_guide_path: Path) -> None:
  if not install_guide_path.is_file():
    raise FileNotFoundError(f"Release install guide is missing: {install_guide_path}")
  text = install_guide_path.read_text(encoding="utf-8")
  require_install_guide_copy(text, "release install guide")


def validate_install_guide_for_tag(install_guide_path: Path, tag: str) -> None:
  validate_install_guide(install_guide_path)
  text = install_guide_path.read_text(encoding="utf-8")
  if f"Pith {tag}" not in text:
    raise RuntimeError("Release install guide tag does not match the release manifest")
  dmg_name, checksum_name, _guide_name, manifest_name = release_installer_asset_names(tag)
  required_tag_phrases = (
    dmg_name,
    checksum_name,
    manifest_name,
    f"shasum -a 256 -c {checksum_name}",
  )
  missing = [
    phrase
    for phrase in required_tag_phrases
    if phrase not in text
  ]
  if missing:
    raise RuntimeError(
      "Release install guide is missing tag-specific asset guidance: "
      + ", ".join(missing)
    )


def validate_checksum_file(artifact_path: Path, checksum_path: Path) -> None:
  if not checksum_path.is_file():
    raise FileNotFoundError(f"Release checksum is missing: {checksum_path}")

  fields = checksum_path.read_text(encoding="utf-8").strip().split()
  if len(fields) != 2:
    raise RuntimeError(f"Release checksum must contain hash and file name: {checksum_path}")

  expected_hash, expected_name = fields
  if expected_name != artifact_path.name:
    raise RuntimeError(
      "Release checksum must reference only the artifact file name: "
      f"{expected_name} != {artifact_path.name}"
    )
  if expected_hash != sha256_hex(artifact_path):
    raise RuntimeError(f"Release checksum does not match artifact: {checksum_path}")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--artifact", required=True, type=Path)
  parser.add_argument("--checksum-output", type=Path)
  parser.add_argument("--manifest-output", type=Path)
  parser.add_argument("--install-guide", type=Path)
  parser.add_argument("--package-manifest", type=Path)
  parser.add_argument("--tag")
  parser.add_argument("--source-commit")
  parser.add_argument("--signing-mode", choices=sorted(RELEASE_SIGNING_MODES))
  parser.add_argument("--workflow-run-id")
  parser.add_argument("--workflow-run-url")
  parser.add_argument("--smoke-receipt", type=Path)
  args = parser.parse_args()

  try:
    checksum_path = write_checksum_file(
      args.artifact.resolve(),
      args.checksum_output.resolve() if args.checksum_output else None,
    )
    manifest_path = None
    if args.manifest_output:
      if (
        not args.tag
        or not args.source_commit
        or not args.signing_mode
        or not args.install_guide
        or not args.package_manifest
        or not args.workflow_run_id
        or not args.workflow_run_url
        or not args.smoke_receipt
      ):
        raise RuntimeError(
          "--manifest-output requires --tag, --source-commit, --signing-mode, "
          "--install-guide, --package-manifest, --workflow-run-id, "
          "--workflow-run-url, and --smoke-receipt"
        )
      manifest_path = write_release_manifest(
        tag=args.tag,
        source_commit=args.source_commit,
        signing_mode=args.signing_mode,
        artifact_path=args.artifact.resolve(),
        checksum_path=checksum_path,
        install_guide_path=args.install_guide.resolve(),
        package_manifest_path=args.package_manifest.resolve(),
        smoke_receipt_path=args.smoke_receipt.resolve(),
        workflow_run_id=args.workflow_run_id,
        workflow_run_url=args.workflow_run_url,
        output_path=args.manifest_output.resolve(),
      )
  except Exception as error:
    print(f"release artifact preparation failed: {error}", file=sys.stderr)
    return 1

  print(f"Created release checksum: {checksum_path}")
  if manifest_path is not None:
    print(f"Created release manifest: {manifest_path}")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
