#!/usr/bin/env python3
"""Unit checks for manual release acceptance evidence validation."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from manual_acceptance_contract import manual_acceptance_template
from manual_acceptance_contract import main as manual_acceptance_main
from manual_acceptance_contract import validate_manual_acceptance_evidence
from package_contract import DEFAULT_MODEL_ID
from release_artifacts import release_installer_asset_names


TAG = "v1.2.3"
SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
RUN_URL = "https://github.com/walt1012/pith/actions/runs/123456"
CHECKSUM = "a" * 64


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


def assert_cli_template_and_validation() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    template_path = root / "manual-acceptance.json"
    evidence_path = root / "filled-manual-acceptance.json"
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
    finally:
      sys.argv = original_argv


def main() -> int:
  validate_manual_acceptance_evidence(valid_payload(), tag=TAG)
  expect_failure({**valid_payload(), "tag": "v9.9.9"}, "tag")
  expect_failure({**valid_payload(), "sourceCommit": "short"}, "40 characters")
  expect_failure({**valid_payload(), "checksum": "short"}, "64 characters")
  expect_failure({**valid_payload(), "checksum": "g" * 64}, "SHA-256 hex digest")
  expect_failure({**valid_payload(), "dmgAssetName": "Pith.zip"}, "dmgAssetName")
  expect_failure({**valid_payload(), "selectedModelId": "other-model"}, "selectedModelId")
  expect_failure({**valid_payload(), "workspaceOpened": False}, "workspaceOpened")
  expect_failure({**valid_payload(), "approvalReceipt": ""}, "approvalReceipt")
  assert_template_is_safe_to_fill()
  assert_cli_template_and_validation()
  print("manual acceptance contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
