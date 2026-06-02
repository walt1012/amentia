#!/usr/bin/env python3
"""Validate downloaded release assets and write a compact rehearsal summary."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from installer_artifact_contract import installer_asset_paths_from_directory
from installer_artifact_contract import validate_installer_asset_set
from package_contract import (
  DAILY_DRIVER_CONTRACT,
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  DEFAULT_MODEL_ID,
  FIRST_APP_OPEN_CONTRACT_ID,
  LOCAL_EXECUTION_SAFETY_MODES,
  MODEL_DELIVERY_MODE,
  MODEL_WEIGHTS_BUNDLED,
  PITH_ACCOUNT_REQUIRED,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  packaged_smoke_package_metadata,
)
from release_artifacts import FIRST_RUN_CONTRACT
from release_artifacts import release_gatekeeper_guidance
from release_artifacts import release_installer_asset_names
from release_artifacts import release_trust
from release_artifacts import validate_packaged_smoke_receipt_summary
from release_copy_contract import (
  FIRST_APP_OPEN_ACTION_COPY,
  PACKAGED_FIRST_RUN_PROOF_PHRASE,
)


FIRST_APP_OPEN_CHECKS = (
  "Launch Pith from Applications after handling Gatekeeper if needed.",
  f"Download one verified local model; {DEFAULT_MODEL_ID} is the default.",
  "Open a workspace folder.",
  "Confirm Web Search readiness and sandbox status.",
  FIRST_APP_OPEN_ACTION_COPY,
  "Follow the daily-driver next action shown in the app header and inspector.",
)


def validate_release_rehearsal(
  tag: str,
  asset_dir: Path,
  *,
  allow_extra_assets: bool = False,
) -> dict:
  asset_paths = installer_asset_paths_from_directory(
    tag,
    asset_dir,
    allow_extra_assets=allow_extra_assets,
  )
  validate_installer_asset_set(tag, asset_paths)
  manifest = load_release_manifest(tag, asset_dir)
  validate_rehearsal_manifest(manifest, tag=tag)
  return release_rehearsal_summary(manifest, tag=tag)


def load_release_manifest(tag: str, asset_dir: Path) -> dict:
  _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names(tag)
  manifest_path = asset_dir / manifest_name
  data = json.loads(manifest_path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError("Downloaded release manifest must be a JSON object")
  return data


def validate_rehearsal_manifest(manifest: dict, *, tag: str) -> None:
  if manifest.get("tag") != tag:
    raise RuntimeError("Downloaded release manifest tag must match the rehearsal tag")
  if manifest.get("product") != "Pith":
    raise RuntimeError("Downloaded release manifest product must be Pith")
  platform = manifest.get("platform")
  if not isinstance(platform, dict) or platform.get("architecture") != SUPPORTED_ARCH:
    raise RuntimeError("Downloaded release manifest must target the supported architecture")
  model_delivery = manifest.get("modelDelivery")
  if not isinstance(model_delivery, dict):
    raise RuntimeError("Downloaded release manifest must include model delivery")
  if model_delivery.get("mode") != MODEL_DELIVERY_MODE:
    raise RuntimeError("Downloaded release manifest must use in-app model delivery")
  if model_delivery.get("defaultModelId") != DEFAULT_MODEL_ID:
    raise RuntimeError("Downloaded release manifest default model must match the product default")
  if model_delivery.get("modelWeightsBundled") is not MODEL_WEIGHTS_BUNDLED:
    raise RuntimeError("Downloaded release manifest must not bundle model weights")
  identity = manifest.get("identity")
  if not isinstance(identity, dict) or identity.get("pithAccountRequired") is not PITH_ACCOUNT_REQUIRED:
    raise RuntimeError("Downloaded release manifest must keep Pith account-free")
  local_execution = manifest.get("localExecution")
  if (
    not isinstance(local_execution, dict)
    or local_execution.get("defaultSafetyMode") != DEFAULT_LOCAL_EXECUTION_SAFETY_MODE
    or local_execution.get("safetyModes") != list(LOCAL_EXECUTION_SAFETY_MODES)
  ):
    raise RuntimeError("Downloaded release manifest local execution contract is incomplete")
  if manifest.get("sandbox") != SANDBOX_CONTRACT:
    raise RuntimeError("Downloaded release manifest sandbox contract is incomplete")
  if manifest.get("firstRun") != FIRST_RUN_CONTRACT:
    raise RuntimeError("Downloaded release manifest first-run contract is incomplete")
  daily_driver = manifest.get("dailyDriver")
  if daily_driver != DAILY_DRIVER_CONTRACT:
    raise RuntimeError("Downloaded release manifest daily-driver contract is incomplete")
  app_package = manifest.get("appPackage")
  if not isinstance(app_package, dict):
    raise RuntimeError("Downloaded release manifest must include app package metadata")
  if app_package.get("sourceCommit") != manifest.get("sourceCommit"):
    raise RuntimeError("Downloaded release app package source must match the release source")
  if app_package.get("architecture") != SUPPORTED_ARCH:
    raise RuntimeError("Downloaded release app package architecture is wrong")
  if app_package.get("modelDelivery") != MODEL_DELIVERY_MODE:
    raise RuntimeError("Downloaded release app package model delivery is wrong")
  if app_package.get("defaultModelId") != DEFAULT_MODEL_ID:
    raise RuntimeError("Downloaded release app package default model is wrong")
  if app_package.get("modelWeightsBundled") is not MODEL_WEIGHTS_BUNDLED:
    raise RuntimeError("Downloaded release app package must not bundle model weights")
  if app_package.get("pithAccountRequired") is not PITH_ACCOUNT_REQUIRED:
    raise RuntimeError("Downloaded release app package must keep Pith account-free")
  if app_package.get("firstAppOpenActionContract") != FIRST_APP_OPEN_CONTRACT_ID:
    raise RuntimeError("Downloaded release app package first app-open contract is wrong")
  if app_package.get("sandboxMode") != SANDBOX_CONTRACT["mode"]:
    raise RuntimeError("Downloaded release app package sandbox mode is wrong")
  if app_package.get("sandboxFallback") != SANDBOX_CONTRACT["fallback"]:
    raise RuntimeError("Downloaded release app package sandbox fallback is wrong")
  if manifest.get("trust") != release_trust(manifest["signingMode"]):
    raise RuntimeError("Downloaded release trust does not match signing mode")
  if manifest.get("gatekeeper") != release_gatekeeper_guidance(manifest["signingMode"]):
    raise RuntimeError("Downloaded release Gatekeeper guidance is wrong")
  verification = manifest.get("verification")
  if not isinstance(verification, dict):
    raise RuntimeError("Downloaded release manifest must include verification")
  if verification.get("assetNames") != list(release_installer_asset_names(tag)):
    raise RuntimeError("Downloaded release manifest asset names must match the release assets")
  smoke_receipt = verification.get("packagedSmokeReceipt")
  if not isinstance(smoke_receipt, dict) or smoke_receipt.get("result") != "passed":
    raise RuntimeError("Downloaded release manifest must include a passed packaged smoke receipt")
  validate_packaged_smoke_receipt_summary(
    smoke_receipt,
    "Downloaded release manifest verification",
  )
  if smoke_receipt.get("packageMetadata") != packaged_smoke_package_metadata(app_package):
    raise RuntimeError(
      "Downloaded release packaged smoke metadata must match app package metadata"
    )


def release_rehearsal_summary(manifest: dict, *, tag: str) -> dict:
  verification = manifest["verification"]
  smoke_receipt = verification["packagedSmokeReceipt"]
  expected_smoke_metadata = packaged_smoke_package_metadata(manifest["appPackage"])
  return {
    "tag": tag,
    "result": "passed",
    "assetNames": list(release_installer_asset_names(tag)),
    "checksumCommand": verification["checksumCommand"],
    "sourceCommit": manifest["sourceCommit"],
    "signingMode": manifest["signingMode"],
    "trust": {
      "mode": manifest["trust"],
      "gatekeeper": manifest["gatekeeper"],
    },
    "defaultModelId": manifest["modelDelivery"]["defaultModelId"],
    "appPackage": {
      "sourceCommit": manifest["appPackage"]["sourceCommit"],
      "modelDelivery": manifest["appPackage"]["modelDelivery"],
      "firstAppOpenActionContract": manifest["appPackage"]["firstAppOpenActionContract"],
    },
    "firstRun": dict(FIRST_RUN_CONTRACT),
    "dailyDriver": dict(DAILY_DRIVER_CONTRACT),
    "firstAppOpenChecks": list(FIRST_APP_OPEN_CHECKS),
    "packagedSmokeReceipt": {
      "phrase": PACKAGED_FIRST_RUN_PROOF_PHRASE,
      "proofScope": smoke_receipt["proofScope"],
      "checkCount": len(smoke_receipt["checkIds"]),
      "packageMetadata": dict(smoke_receipt["packageMetadata"]),
      "packageMetadataMatched": smoke_receipt["packageMetadata"] == expected_smoke_metadata,
    },
  }


def write_summary(path: Path, summary: dict) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(summary_markdown(summary), encoding="utf-8")


def summary_markdown(summary: dict) -> str:
  assets = "\n".join(f"- `{name}`" for name in summary["assetNames"])
  first_run = "\n".join(
    f"- `{key}`: {value}"
    for key, value in summary["firstRun"].items()
  )
  app_open = "\n".join(f"- {check}" for check in summary["firstAppOpenChecks"])
  return f"""# Pith {summary["tag"]} Release Rehearsal

Result: `{summary["result"]}`

## Assets
{assets}

## Verification
- Checksum: `{summary["checksumCommand"]}`
- Source commit: `{summary["sourceCommit"]}`
- Signing mode: `{summary["signingMode"]}`
- Trust: `{summary["trust"]["mode"]}`
- Gatekeeper: {summary["trust"]["gatekeeper"]}
- Default model: `{summary["defaultModelId"]}`
- App package source: `{summary["appPackage"]["sourceCommit"]}`
- App package model delivery: `{summary["appPackage"]["modelDelivery"]}`
- First app-open contract: `{summary["appPackage"]["firstAppOpenActionContract"]}`
- Packaged proof: `{summary["packagedSmokeReceipt"]["phrase"]}`
- Proof scope: `{summary["packagedSmokeReceipt"]["proofScope"]}`
- Proof checks: `{summary["packagedSmokeReceipt"]["checkCount"]}`
- Smoke package metadata: `matches app package metadata`

## First Run
{first_run}

## First App Open
{app_open}
"""


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--asset-dir", required=True, type=Path)
  parser.add_argument("--summary-output", type=Path)
  parser.add_argument("--allow-extra-assets", action="store_true")
  args = parser.parse_args()

  try:
    summary = validate_release_rehearsal(
      args.tag,
      args.asset_dir,
      allow_extra_assets=args.allow_extra_assets,
    )
    if args.summary_output:
      write_summary(args.summary_output, summary)
  except Exception as error:
    print(f"release rehearsal contract failed: {error}", file=sys.stderr)
    return 1

  print("Release rehearsal contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
