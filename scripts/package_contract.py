#!/usr/bin/env python3
"""Shared package contract for Pith macOS release automation."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


APP_NAME = "Pith"
PACKAGE_MANIFEST_SCHEMA_VERSION = 1
SUPPORTED_ARCH = "x86_64"
MINIMUM_SYSTEM_VERSION = "12.0"
DEFAULT_MODEL_ID = "lfm2.5-350m"
DEFAULT_MODEL_MANIFEST_RELATIVE_PATH = Path(
  "models/builtin/lfm2.5-350m/model-pack.json"
)
MODEL_DELIVERY_MODE = "in-app-download"
MODEL_WEIGHTS_BUNDLED = False
MODEL_METADATA_BUNDLED = True
PITH_ACCOUNT_REQUIRED = False
LOCAL_EXECUTION_SAFETY_MODES = (
  "explore",
  "askBeforeChange",
  "approvedWorkspaceExecution",
)
DEFAULT_LOCAL_EXECUTION_SAFETY_MODE = "askBeforeChange"
DEFAULT_MAX_APP_BUNDLE_BYTES = 250 * 1024 * 1024
DEFAULT_MAX_ZIP_ARTIFACT_BYTES = 150 * 1024 * 1024
PROHIBITED_MODEL_SUFFIXES = {".gguf", ".bin", ".safetensors"}
PACKAGE_SIGNING_MODES = {"unsigned", "ad-hoc", "developer-id"}
RELEASE_SIGNING_MODES = {"ad-hoc", "developer-id"}
PACKAGE_DISTRIBUTION_TRUST_BY_SIGNING = {
  "unsigned": "unsigned-local-build",
  "ad-hoc": "ad-hoc-not-notarized",
  "developer-id": "developer-id-signed-notarized",
}

SANDBOX_CONTRACT = {
  "mode": "workspaceReadWrite",
  "backend": "runtime-detected",
  "fallback": "processOnlyWhenNativeUnavailable",
  "networkDefault": "disabled",
}
DAILY_DRIVER_CONTRACT = {
  "stageSource": "runtime/readiness",
  "nextActionSource": "runtime/readiness",
  "presentation": "app-header-inspector",
  "packagedSmoke": "required",
}
PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION = 1
PACKAGED_SMOKE_RECEIPT_KIND = "pith.packagedSmokeReceipt"
PACKAGED_SMOKE_REQUIRED_CHECK_IDS = (
  "mountedDmgAppBundle",
  "appLaunch",
  "runtimeProtocol",
  "defaultModelMetadata",
  "appOwnedModelActivation",
  "workspaceBootstrap",
  "firstCoworkTurn",
  "webSearchExecution",
  "approvalDenied",
  "approvalApproved",
  "connectorAuthorization",
  "connectorExecution",
  "runnerMemoryCapture",
  "runtimeRecovery",
  "sandboxReadiness",
)
PACKAGED_SMOKE_PROOF_SCOPE = (
  "model setup, workspace, Web Search, approval, connector, sandbox, "
  "and runtime recovery checks"
)


def package_size_budget() -> dict[str, int]:
  return {
    "maxAppBundleBytes": env_positive_int(
      "PITH_MAX_APP_BUNDLE_BYTES",
      DEFAULT_MAX_APP_BUNDLE_BYTES,
    ),
    "maxZipArtifactBytes": env_positive_int(
      "PITH_MAX_ZIP_ARTIFACT_BYTES",
      DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
    ),
  }


def env_positive_int(name: str, default: int) -> int:
  raw_value = os.environ.get(name)
  if raw_value is None:
    return default
  try:
    value = int(raw_value)
  except ValueError as error:
    raise RuntimeError(f"{name} must be a positive integer") from error
  if value <= 0:
    raise RuntimeError(f"{name} must be a positive integer")
  return value


def validate_package_size_budget(value: object, label: str) -> dict[str, int]:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} sizeBudget is required")
  result: dict[str, int] = {}
  for field in ("maxAppBundleBytes", "maxZipArtifactBytes"):
    field_value = value.get(field)
    if not isinstance(field_value, int) or field_value <= 0:
      raise RuntimeError(f"{label} sizeBudget {field} must be a positive integer")
    result[field] = field_value
  return result


def validate_package_manifest_contract(
  manifest: dict,
  label: str,
  *,
  source_commit: str | None = None,
  signing_mode: str | None = None,
  bundle_version: str | None = None,
  expected_size_budget: dict[str, int] | None = None,
) -> dict[str, int]:
  expected_values = {
    "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
    "appName": APP_NAME,
    "minimumSystemVersion": MINIMUM_SYSTEM_VERSION,
    "architecture": SUPPORTED_ARCH,
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
  }
  if source_commit is not None:
    expected_values["sourceCommit"] = source_commit
  if signing_mode is not None:
    expected_values["signing"] = signing_mode
  if bundle_version is not None:
    expected_values["bundleVersion"] = bundle_version

  for field, expected in expected_values.items():
    if manifest.get(field) != expected:
      raise RuntimeError(f"{label} field {field} must be {expected!r}")

  actual_bundle_version = manifest.get("bundleVersion")
  if not isinstance(actual_bundle_version, str) or not actual_bundle_version.strip():
    raise RuntimeError(f"{label} bundleVersion is required")
  actual_source_commit = manifest.get("sourceCommit")
  if not isinstance(actual_source_commit, str) or not actual_source_commit.strip():
    raise RuntimeError(f"{label} sourceCommit is required")
  actual_signing = manifest.get("signing")
  if not isinstance(actual_signing, str) or not actual_signing.strip():
    raise RuntimeError(f"{label} signing is required")
  if actual_signing not in PACKAGE_SIGNING_MODES:
    expected = ", ".join(sorted(PACKAGE_SIGNING_MODES))
    raise RuntimeError(f"{label} signing must be one of: {expected}")
  actual_distribution_trust = manifest.get("distributionTrust")
  expected_distribution_trust = package_distribution_trust(actual_signing)
  if actual_distribution_trust != expected_distribution_trust:
    raise RuntimeError(
      f"{label} distributionTrust must be {expected_distribution_trust!r}"
    )

  actual_size_budget = validate_package_size_budget(manifest.get("sizeBudget"), label)
  expected_budget = (
    package_size_budget() if expected_size_budget is None else expected_size_budget
  )
  if actual_size_budget != expected_budget:
    raise RuntimeError(f"{label} sizeBudget must be {expected_budget!r}")
  return actual_size_budget


def package_distribution_trust(signing_mode: str) -> str:
  try:
    return PACKAGE_DISTRIBUTION_TRUST_BY_SIGNING[signing_mode]
  except KeyError as error:
    raise RuntimeError(f"Unsupported signing mode: {signing_mode}") from error


def read_json_object(path: Path, label: str) -> dict:
  if not path.is_file():
    raise FileNotFoundError(f"{label} is missing: {path}")
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError(f"{label} must be a JSON object: {path}")
  return data


def assert_size_under_budget(size_bytes: int, max_bytes: int, label: str) -> None:
  if size_bytes > max_bytes:
    raise RuntimeError(
      f"{label} is {size_bytes} bytes, above the {max_bytes} byte release budget. "
      "Keep model weights, multi-arch runtimes, and optional connectors out of the package."
    )


def directory_size_bytes(path: Path) -> int:
  total = 0
  for entry in path.rglob("*"):
    if entry.is_file():
      total += entry.stat().st_size
  return total


def bundled_model_weight_files(path: Path) -> list[Path]:
  return sorted(
    entry
    for entry in path.rglob("*")
    if entry.is_file() and entry.suffix.lower() in PROHIBITED_MODEL_SUFFIXES
  )


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--manifest", type=Path, required=True)
  parser.add_argument("--source-commit")
  parser.add_argument("--signing-mode")
  parser.add_argument("--bundle-version")
  return parser.parse_args()


def main() -> int:
  args = parse_args()
  try:
    manifest_path = args.manifest.resolve()
    manifest = read_json_object(manifest_path, "PithPackage.json")
    validate_package_manifest_contract(
      manifest,
      f"PithPackage.json: {manifest_path}",
      source_commit=args.source_commit,
      signing_mode=args.signing_mode,
      bundle_version=args.bundle_version,
    )
  except Exception as error:
    print(f"package contract validation failed: {error}", file=sys.stderr)
    return 1
  print("package contract validation passed.")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
