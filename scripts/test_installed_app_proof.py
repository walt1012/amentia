#!/usr/bin/env python3
"""Unit checks for M14 installed-app proof evidence."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from installed_app_proof import (
  ALLOWED_MODEL_IDS,
  APP_NAME,
  PROOF_SCOPE,
  installed_app_template,
  main as installed_app_main,
  validate_installed_app_evidence,
)


def valid_payload() -> dict[str, object]:
  return {
    "appName": APP_NAME,
    "proofScope": PROOF_SCOPE,
    "releaseTag": "v0.14.0",
    "dmgName": "Amentia.dmg",
    "dmgSha256": "a" * 64,
    "dmgChecksumVerified": True,
    "installedFromDmg": True,
    "firstLaunchCompleted": True,
    "modelDownloaded": True,
    "modelVerified": True,
    "modelActivated": True,
    "modelSelfCheckPassed": True,
    "modelId": "lfm2.5-350m",
    "modelProof": "Downloaded, checksum verified, activated, and self-check passed in the installed app.",
    "localInferenceCompleted": True,
    "inferenceProof": "The installed app produced a bounded local response without hosted model API access.",
    "webSearchProofInspected": True,
    "webSearchProof": "Timeline showed Web Search source proof and the sources were inspected.",
    "sessionDeleteVerified": True,
    "sessionRevertVerified": True,
    "sessionCleanupProof": "Session delete removed the target session and session revert restored approved project changes.",
    "resetAmentiaVerified": True,
    "resetProof": (
      "Reset Amentia cleared support data, cache files, preferences, saved state, "
      "paused downloads, and local credential handles."
    ),
    "pluginLifecycleVerified": True,
    "pluginLifecycleProof": "Installed, disabled, and removed the reference plugin from the installed app.",
    "unexpectedAppOwnedResiduePaths": [],
    "noUnexpectedAppOwnedResidue": True,
    "acceptedBy": "Maintainer",
    "acceptedAt": "2026-06-23T00:00:00Z",
  }


def expect_failure(payload: dict[str, object], expected: str) -> None:
  try:
    validate_installed_app_evidence(payload)
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r} in {error!r}") from error
    return
  raise AssertionError(f"expected installed-app proof validation to fail: {expected}")


def assert_valid_payload_passes() -> None:
  validate_installed_app_evidence(valid_payload())


def assert_template_is_safe_to_fill() -> None:
  template = installed_app_template()
  if template["appName"] != APP_NAME:
    raise AssertionError("template should target Amentia")
  if template["proofScope"] != PROOF_SCOPE:
    raise AssertionError("template should target installed-app proof")
  for key in (
    "dmgChecksumVerified",
    "installedFromDmg",
    "firstLaunchCompleted",
    "modelDownloaded",
    "modelVerified",
    "modelActivated",
    "modelSelfCheckPassed",
    "localInferenceCompleted",
    "webSearchProofInspected",
    "sessionDeleteVerified",
    "sessionRevertVerified",
    "resetAmentiaVerified",
    "pluginLifecycleVerified",
    "noUnexpectedAppOwnedResidue",
  ):
    if template[key] is not False:
      raise AssertionError(f"template {key} should stay false until manually filled")
  if template["unexpectedAppOwnedResiduePaths"] != []:
    raise AssertionError("template should start with no unexpected app-owned residue")


def assert_rejects_wrong_scope() -> None:
  payload = valid_payload()
  payload["proofScope"] = "developer-machine"
  expect_failure(payload, "proofScope")


def assert_rejects_unknown_model() -> None:
  payload = valid_payload()
  payload["modelId"] = "qwen2.5-old"
  expect_failure(payload, "modelId")
  if "lfm2.5-350m" not in ALLOWED_MODEL_IDS:
    raise AssertionError("allowed model ids should include the first-use default")


def assert_rejects_incomplete_model_path() -> None:
  payload = valid_payload()
  payload["modelSelfCheckPassed"] = False
  expect_failure(payload, "modelSelfCheckPassed")


def assert_rejects_placeholder_text() -> None:
  payload = valid_payload()
  payload["modelProof"] = "TODO"
  expect_failure(payload, "placeholder")

  payload = valid_payload()
  payload["acceptedBy"] = "N/A"
  expect_failure(payload, "placeholder")


def assert_rejects_non_utc_acceptance_time() -> None:
  payload = valid_payload()
  payload["acceptedAt"] = "2026-06-23"
  expect_failure(payload, "acceptedAt")


def assert_rejects_bad_checksum() -> None:
  payload = valid_payload()
  payload["dmgSha256"] = "not-a-sha"
  expect_failure(payload, "dmgSha256")


def assert_rejects_stale_residue() -> None:
  payload = valid_payload()
  payload["unexpectedAppOwnedResiduePaths"] = ["~/Library/Application Support/Amentia/tmp"]
  expect_failure(payload, "unexpectedAppOwnedResiduePaths")


def assert_rejects_weak_cleanup_proof() -> None:
  payload = valid_payload()
  payload["resetProof"] = "Reset Amentia finished."
  expect_failure(payload, "resetProof")

  payload = valid_payload()
  payload["pluginLifecycleProof"] = "Installed the plugin."
  expect_failure(payload, "pluginLifecycleProof")


def assert_cli_template_and_validation() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    template_path = root / "installed-app-template.json"
    evidence_path = root / "installed-app-proof.json"
    invalid_path = root / "invalid-installed-app-proof.json"
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "installed_app_proof.py",
        "--template-output",
        str(template_path),
      ]
      if installed_app_main() != 0:
        raise AssertionError("installed-app template CLI should pass")
      template = json.loads(template_path.read_text(encoding="utf-8"))
      if template["modelActivated"] is not False:
        raise AssertionError("template should not pre-activate the model")

      evidence_path.write_text(json.dumps(valid_payload()), encoding="utf-8")
      sys.argv = [
        "installed_app_proof.py",
        "--evidence",
        str(evidence_path),
      ]
      if installed_app_main() != 0:
        raise AssertionError("installed-app evidence CLI should pass")

      invalid = valid_payload()
      invalid["localInferenceCompleted"] = False
      invalid_path.write_text(json.dumps(invalid), encoding="utf-8")
      sys.argv = [
        "installed_app_proof.py",
        "--evidence",
        str(invalid_path),
      ]
      stderr = StringIO()
      with redirect_stderr(stderr):
        result = installed_app_main()
      if result == 0:
        raise AssertionError("invalid installed-app evidence should fail")
      if "localInferenceCompleted" not in stderr.getvalue():
        raise AssertionError("CLI failure should explain the failing field")
    finally:
      sys.argv = original_argv


def main() -> int:
  assert_valid_payload_passes()
  assert_template_is_safe_to_fill()
  assert_rejects_wrong_scope()
  assert_rejects_unknown_model()
  assert_rejects_incomplete_model_path()
  assert_rejects_placeholder_text()
  assert_rejects_non_utc_acceptance_time()
  assert_rejects_bad_checksum()
  assert_rejects_stale_residue()
  assert_rejects_weak_cleanup_proof()
  assert_cli_template_and_validation()
  print("installed-app proof tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
