#!/usr/bin/env python3
"""Unit checks for manual release acceptance evidence validation."""

from __future__ import annotations

import json
import hashlib
import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from manual_acceptance_contract import manual_acceptance_template
from manual_acceptance_contract import manual_acceptance_template_from_asset_dir
from manual_acceptance_contract import main as manual_acceptance_main
from manual_acceptance_contract import validate_manual_acceptance_evidence
from package_contract import DEFAULT_MODEL_ID
from release_artifacts import release_installer_asset_names


TAG = "v1.2.3"
SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
RUN_URL = "https://github.com/walt1012/pith/actions/runs/123456"
DMG_BYTES = b"release dmg\n"
CHECKSUM = hashlib.sha256(DMG_BYTES).hexdigest()


def valid_payload() -> dict[str, object]:
  return {
    "tag": TAG,
    "sourceCommit": SOURCE_COMMIT,
    "releaseWorkflowRunUrl": RUN_URL,
    "dmgAssetName": release_installer_asset_names(TAG)[0],
    "checksum": CHECKSUM,
    "checksumVerified": True,
    "manifestReviewed": True,
    "gatekeeperPath": "Control-click Open",
    "gatekeeperHandled": True,
    "selectedModelId": DEFAULT_MODEL_ID,
    "modelDownloadedAndActivated": True,
    "workspaceDescription": "Fresh test workspace with a small text file.",
    "workspaceOpened": True,
    "coworkRequest": "Map this workspace and plan the next safe step.",
    "coworkTurnCompleted": True,
    "webSearchProof": "Timeline showed Web Search source proof.",
    "webSearchProofInspected": True,
    "approvalReceipt": "Approved one safe diff after reviewing it.",
    "approvalDiffReceiptInspected": True,
    "restartRecoveryProof": "Restart restored runtime, model, workspace, and recent proof.",
    "restartRecoveryVerified": True,
    "noPithLoginRequired": True,
    "acceptedForVisiblePrerelease": True,
    "acceptedBy": "Maintainer",
    "acceptedAt": "2026-06-08T00:00:00Z",
  }


def expect_failure(payload: dict[str, object], expected: str) -> None:
  try:
    validate_manual_acceptance_evidence(payload, tag=TAG)
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r} in {error!r}")
    return
  raise AssertionError(f"expected manual acceptance validation to fail: {expected}")


def assert_template_is_safe_to_fill() -> None:
  template = manual_acceptance_template(
    tag=TAG,
    source_commit=SOURCE_COMMIT,
    release_workflow_run_url=RUN_URL,
  )
  if template["tag"] != TAG:
    raise AssertionError("template should preserve tag")
  if template["dmgAssetName"] != release_installer_asset_names(TAG)[0]:
    raise AssertionError("template should use the release DMG asset")
  if template["selectedModelId"] != DEFAULT_MODEL_ID:
    raise AssertionError("template should default to the product model")
  for key in ("checksumVerified", "acceptedForVisiblePrerelease"):
    if template[key] is not False:
      raise AssertionError(f"template {key} should be false until manually filled")


def write_asset_dir(
  root: Path,
  *,
  checksum: str = CHECKSUM,
  manifest_checksum: str = CHECKSUM,
  source_commit: str = SOURCE_COMMIT,
  workflow_run_url: str = RUN_URL,
) -> None:
  dmg_name, checksum_name, _guide_name, manifest_name = release_installer_asset_names(TAG)
  manifest: dict[str, object] = {
    "tag": TAG,
    "sourceCommit": source_commit,
    "verification": {
      "workflowRunUrl": workflow_run_url,
    },
    "artifacts": [
      {
        "kind": "dmg",
        "name": dmg_name,
        "sha256": manifest_checksum,
      },
      {
        "kind": "checksum",
        "name": checksum_name,
        "checks": dmg_name,
      },
    ],
  }
  root.mkdir(parents=True, exist_ok=True)
  (root / dmg_name).write_bytes(DMG_BYTES)
  (root / checksum_name).write_text(f"{checksum}  {dmg_name}\n", encoding="utf-8")
  (root / manifest_name).write_text(json.dumps(manifest) + "\n", encoding="utf-8")


def assert_template_can_be_derived_from_release_assets() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_asset_dir(root)
    template = manual_acceptance_template_from_asset_dir(tag=TAG, asset_dir=root)
    if template["sourceCommit"] != SOURCE_COMMIT:
      raise AssertionError("asset-derived template should use manifest source commit")
    if template["releaseWorkflowRunUrl"] != RUN_URL:
      raise AssertionError("asset-derived template should use manifest workflow URL")
    if template["dmgAssetName"] != release_installer_asset_names(TAG)[0]:
      raise AssertionError("asset-derived template should use the release DMG asset")
    if template["checksum"] != CHECKSUM:
      raise AssertionError("asset-derived template should use the verified DMG checksum")
    if template["checksumVerified"] is not False:
      raise AssertionError("asset-derived template should still require manual checksum confirmation")


def assert_asset_template_rejects_stale_assets() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_asset_dir(root, checksum="b" * 64)
    expect_template_failure(
      lambda: manual_acceptance_template_from_asset_dir(tag=TAG, asset_dir=root),
      "checksum sidecar digest",
    )

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_asset_dir(root, checksum="g" * 64, manifest_checksum="g" * 64)
    expect_template_failure(
      lambda: manual_acceptance_template_from_asset_dir(tag=TAG, asset_dir=root),
      "SHA-256 hex digest",
    )

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_asset_dir(root, source_commit="z" * 40)
    expect_template_failure(
      lambda: manual_acceptance_template_from_asset_dir(tag=TAG, asset_dir=root),
      "Git SHA hex digest",
    )

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_asset_dir(root, workflow_run_url="")
    expect_template_failure(
      lambda: manual_acceptance_template_from_asset_dir(tag=TAG, asset_dir=root),
      "workflowRunUrl",
    )

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_asset_dir(root)
    (root / release_installer_asset_names(TAG)[0]).unlink()
    expect_template_failure(
      lambda: manual_acceptance_template_from_asset_dir(tag=TAG, asset_dir=root),
      "DMG asset is missing",
    )

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_asset_dir(root)
    (root / release_installer_asset_names(TAG)[0]).write_bytes(b"tampered dmg\n")
    expect_template_failure(
      lambda: manual_acceptance_template_from_asset_dir(tag=TAG, asset_dir=root),
      "DMG asset digest",
    )


def expect_template_failure(action, expected: str) -> None:
  try:
    action()
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r} in {error!r}") from error
    return
  raise AssertionError(f"expected manual acceptance template to fail: {expected}")


def assert_cli_template_and_validation() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    template_path = root / "manual-acceptance.json"
    evidence_path = root / "filled-manual-acceptance.json"
    asset_template_path = root / "asset-manual-acceptance.json"
    asset_dir = root / "assets"
    write_asset_dir(asset_dir)
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "manual_acceptance_contract.py",
        "--tag",
        TAG,
        "--template-output",
        str(template_path),
        "--source-commit",
        SOURCE_COMMIT,
        "--release-workflow-run-url",
        RUN_URL,
      ]
      if manual_acceptance_main() != 0:
        raise AssertionError("manual acceptance template CLI should pass")
      data = json.loads(template_path.read_text(encoding="utf-8"))
      if data["acceptedForVisiblePrerelease"] is not False:
        raise AssertionError("template should not pre-accept release")
      sys.argv = [
        "manual_acceptance_contract.py",
        "--tag",
        TAG,
        "--asset-dir",
        str(asset_dir),
        "--template-output",
        str(asset_template_path),
      ]
      if manual_acceptance_main() != 0:
        raise AssertionError("asset-derived manual acceptance template CLI should pass")
      asset_data = json.loads(asset_template_path.read_text(encoding="utf-8"))
      if asset_data["checksum"] != CHECKSUM:
        raise AssertionError("asset-derived CLI template should include the DMG checksum")
      evidence_path.write_text(json.dumps(valid_payload()), encoding="utf-8")
      sys.argv = [
        "manual_acceptance_contract.py",
        "--tag",
        TAG,
        "--evidence",
        str(evidence_path),
      ]
      if manual_acceptance_main() != 0:
        raise AssertionError("manual acceptance validation CLI should pass")
      invalid_path = root / "invalid.json"
      invalid = valid_payload()
      invalid["webSearchProofInspected"] = False
      invalid_path.write_text(json.dumps(invalid), encoding="utf-8")
      sys.argv = [
        "manual_acceptance_contract.py",
        "--tag",
        TAG,
        "--evidence",
        str(invalid_path),
      ]
      with redirect_stderr(StringIO()) as stderr:
        result = manual_acceptance_main()
      if result == 0 or "webSearchProofInspected" not in stderr.getvalue():
        raise AssertionError("manual acceptance CLI should reject incomplete evidence")
      sys.argv = [
        "manual_acceptance_contract.py",
        "--tag",
        TAG,
        "--template-output",
        str(root / "missing-inputs.json"),
      ]
      with redirect_stderr(StringIO()) as stderr:
        result = manual_acceptance_main()
      if result == 0 or "--source-commit" not in stderr.getvalue():
        raise AssertionError("manual acceptance CLI should reject template inputs without an asset dir")
    finally:
      sys.argv = original_argv


def main() -> int:
  validate_manual_acceptance_evidence(valid_payload(), tag=TAG)
  expect_failure({**valid_payload(), "tag": "v9.9.9"}, "tag")
  expect_failure({**valid_payload(), "sourceCommit": "short"}, "40 characters")
  expect_failure({**valid_payload(), "sourceCommit": "z" * 40}, "Git SHA hex digest")
  expect_failure({**valid_payload(), "checksum": "short"}, "64 characters")
  expect_failure({**valid_payload(), "checksum": "g" * 64}, "SHA-256 hex digest")
  expect_failure({**valid_payload(), "dmgAssetName": "Pith.zip"}, "dmgAssetName")
  expect_failure({**valid_payload(), "selectedModelId": "other-model"}, "selectedModelId")
  expect_failure({**valid_payload(), "workspaceOpened": False}, "workspaceOpened")
  expect_failure({**valid_payload(), "approvalReceipt": ""}, "approvalReceipt")
  assert_template_is_safe_to_fill()
  assert_template_can_be_derived_from_release_assets()
  assert_asset_template_rejects_stale_assets()
  assert_cli_template_and_validation()
  print("manual acceptance contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
