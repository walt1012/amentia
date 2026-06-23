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
  APP_NAME,
  DAILY_DRIVER_CONTRACT,
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  DEFAULT_MODEL_ID,
  FIRST_APP_OPEN_CONTRACT_ID,
  LOCAL_EXECUTION_SAFETY_MODES,
  MODEL_DELIVERY_MODE,
  MODEL_WEIGHTS_BUNDLED,
  AMENTIA_ACCOUNT_REQUIRED,
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
  PACKAGED_FIRST_RUN_RECEIPT_PHRASE,
)


FIRST_APP_OPEN_CHECKS = (
  f"Launch {APP_NAME} from Applications after handling Gatekeeper if needed.",
  f"Download one verified local model; {DEFAULT_MODEL_ID} is the default.",
  "Open a project folder.",
  "Check that Web Search and project safety are ready.",
  FIRST_APP_OPEN_ACTION_COPY,
  "Follow the daily-driver next action shown in the app header and inspector.",
)
MANUAL_PRERELEASE_CHECKS = (
  "Verify the downloaded DMG with the SHA-256 sidecar before opening it.",
  f"Open the release manifest and confirm macOS x86_64, in-app model delivery, no bundled model weights, and no {APP_NAME} login.",
  f"Install {APP_NAME} from the DMG and handle Gatekeeper according to the manifest guidance.",
  f"Download and activate one verified local model; {DEFAULT_MODEL_ID} is the default choice.",
  "Open a real project folder and confirm the header or inspector reports project readiness.",
  "Run Understand Project, Pick Next Step, or a short cowork prompt from the first app-open surface.",
  "Let the model use Web Search when useful and inspect the source receipt in the timeline.",
  "Approve one safe local project change only after reviewing the diff, then confirm the timeline receipt.",
  f"Restart {APP_NAME} and confirm {APP_NAME} status, selected project, model state, and recent receipts recover.",
)
RELEASE_DECISION = {
  "automatedRehearsal": "passed",
  "manualAcceptance": "required-before-visible-prerelease",
  "publishGate": "do-not-publish-visible-ad-hoc-until-manual-acceptance-passes",
}


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
  if manifest.get("product") != APP_NAME:
    raise RuntimeError(f"Downloaded release manifest product must be {APP_NAME}")
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
  if not isinstance(identity, dict) or identity.get("amentiaAccountRequired") is not AMENTIA_ACCOUNT_REQUIRED:
    raise RuntimeError(f"Downloaded release manifest must keep {APP_NAME} account-free")
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
  if app_package.get("amentiaAccountRequired") is not AMENTIA_ACCOUNT_REQUIRED:
    raise RuntimeError(f"Downloaded release app package must keep {APP_NAME} account-free")
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
    "releaseDecision": dict(RELEASE_DECISION),
    "firstAppOpenChecks": list(FIRST_APP_OPEN_CHECKS),
    "manualPrereleaseChecks": list(MANUAL_PRERELEASE_CHECKS),
    "packagedSmokeReceipt": {
      "phrase": PACKAGED_FIRST_RUN_RECEIPT_PHRASE,
      "receiptScope": smoke_receipt["receiptScope"],
      "checkCount": len(smoke_receipt["checkIds"]),
      "journey": list(smoke_receipt["journey"]),
      "packageMetadata": dict(smoke_receipt["packageMetadata"]),
      "packageMetadataMatched": smoke_receipt["packageMetadata"] == expected_smoke_metadata,
    },
  }


def write_summary(path: Path, summary: dict) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(summary_markdown(summary), encoding="utf-8")


def write_acceptance(path: Path, summary: dict) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(acceptance_markdown(summary), encoding="utf-8")


def write_json(path: Path, summary: dict) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(
    json.dumps(summary, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
  )


def acceptance_markdown(summary: dict) -> str:
  assets = "\n".join(f"- [ ] Download `{name}`" for name in summary["assetNames"])
  manual_acceptance = "\n".join(
    f"- [ ] {check}" for check in summary["manualPrereleaseChecks"]
  )
  return f"""# Amentia {summary["tag"]} Manual Release Acceptance

Use this acceptance worksheet only after automated release rehearsal passes.

Decision:
- [ ] Accept this build for visible ad-hoc prerelease.
- [ ] Keep this build draft-only and fix issues before publishing.

## Release Inputs
- Source commit: `{summary["sourceCommit"]}`
- Signing mode: `{summary["signingMode"]}`
- Trust: `{summary["trust"]["mode"]}`
- Gatekeeper: {summary["trust"]["gatekeeper"]}
- Checksum command: `{summary["checksumCommand"]}`
- Default model: `{summary["defaultModelId"]}`

## Downloaded Assets
{assets}

## Required Manual Checks
{manual_acceptance}

## Receipt To Record
- Fresh Mac or clean macOS user profile used.
- Checksum verification result.
- Gatekeeper path used.
- Model selected and activated.
- Workspace path used for acceptance.
- Web Search receipt inspected.
- Approval and diff receipt inspected.
- Restart recovery result.
- Generate one structured manual acceptance JSON receipt with `python3 scripts/manual_acceptance_contract.py --tag {summary["tag"]} --asset-dir <downloaded-assets> --template-output manual-acceptance.json`, fill it from the same installed-app run, then validate it with `python3 scripts/manual_acceptance_contract.py --tag {summary["tag"]} --receipt manual-acceptance.json`.
"""


def summary_markdown(summary: dict) -> str:
  assets = "\n".join(f"- `{name}`" for name in summary["assetNames"])
  first_run = "\n".join(
    f"- `{key}`: {value}"
    for key, value in summary["firstRun"].items()
  )
  release_decision = "\n".join(
    f"- `{key}`: {value}"
    for key, value in summary["releaseDecision"].items()
  )
  app_open = "\n".join(f"- {check}" for check in summary["firstAppOpenChecks"])
  manual_acceptance = "\n".join(
    f"- [ ] {check}" for check in summary["manualPrereleaseChecks"]
  )
  smoke_journey = "\n".join(
    f"- {stage['title']}: {', '.join(stage['checkIds'])}"
    for stage in summary["packagedSmokeReceipt"]["journey"]
  )
  return f"""# Amentia {summary["tag"]} Release Rehearsal

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
- Packaged receipt: `{summary["packagedSmokeReceipt"]["phrase"]}`
- Receipt scope: `{summary["packagedSmokeReceipt"]["receiptScope"]}`
- Receipt checks: `{summary["packagedSmokeReceipt"]["checkCount"]}`
- Smoke package metadata: `matches app package metadata`

## Packaged Smoke Journey
{smoke_journey}

## First Run
{first_run}

## Release Decision
{release_decision}

## First App Open
{app_open}

## Manual Prerelease Acceptance
{manual_acceptance}
"""


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--asset-dir", required=True, type=Path)
  parser.add_argument("--summary-output", type=Path)
  parser.add_argument("--acceptance-output", type=Path)
  parser.add_argument("--json-output", type=Path)
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
    if args.acceptance_output:
      write_acceptance(args.acceptance_output, summary)
    if args.json_output:
      write_json(args.json_output, summary)
  except Exception as error:
    print(f"release rehearsal contract failed: {error}", file=sys.stderr)
    return 1

  print("Release rehearsal contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
