#!/usr/bin/env python3
"""Unit checks for milestone evidence validation."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from milestone_evidence import (
  INSTALLED_APP_EVIDENCE,
  REFERENCE_CONNECTOR_EVIDENCE,
  main as milestone_evidence_main,
  validate_milestone_evidence,
)


def valid_installed_app_payload() -> dict[str, object]:
  return {
    "appName": "Amentia",
    "proofScope": "installed-app",
    "releaseTag": "v0.14.0",
    "dmgName": "Amentia-v0.14.0-macos-x86_64.dmg",
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


def valid_reference_connector_payload() -> dict[str, object]:
  return {
    "pluginId": "notion-connector",
    "connectorId": "notion-connector::notion",
    "service": "notion",
    "pluginInstallOrRefreshCompleted": True,
    "connectorAuthorized": True,
    "actionOrWorkflowRun": True,
    "actionOrWorkflowId": "notion.create-page",
    "receiptInspected": True,
    "receiptProof": "Timeline showed a Notion workflow receipt with remote proof.",
    "credentialCleared": True,
    "credentialClearProof": (
      "Storage showed no connector credential row and local credential handles were empty after clearing."
    ),
    "pluginRemoved": True,
    "pluginRemovalProof": "The Notion bundle no longer appears in installed plugins.",
    "remainingConnectorCredentialIds": [],
    "remainingLocalCredentialHandles": [],
    "noStaleCredentialState": True,
    "acceptedBy": "Maintainer",
    "acceptedAt": "2026-06-23T00:00:00Z",
  }


def write_json(root: Path, relative: str, payload: dict[str, object]) -> None:
  path = root / relative
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(payload) + "\n", encoding="utf-8")


def expect_failure(action, expected: str) -> None:
  try:
    action()
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r} in {error!r}") from error
    return
  raise AssertionError(f"expected milestone evidence validation to fail: {expected}")


def assert_accepts_empty_evidence_directory() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    (root / "docs" / "evidence").mkdir(parents=True)
    result = validate_milestone_evidence(root)
    if result.validated_files != []:
      raise AssertionError("empty evidence directory should not validate files")


def assert_validates_known_evidence_files() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_json(root, INSTALLED_APP_EVIDENCE, valid_installed_app_payload())
    write_json(root, REFERENCE_CONNECTOR_EVIDENCE, valid_reference_connector_payload())
    result = validate_milestone_evidence(root)
    if result.validated_files != [INSTALLED_APP_EVIDENCE, REFERENCE_CONNECTOR_EVIDENCE]:
      raise AssertionError(f"unexpected validated files: {result.validated_files}")


def assert_rejects_invalid_present_evidence() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    payload = valid_installed_app_payload()
    payload["modelSelfCheckPassed"] = False
    write_json(root, INSTALLED_APP_EVIDENCE, payload)
    expect_failure(lambda: validate_milestone_evidence(root), "modelSelfCheckPassed")


def assert_rejects_unknown_json_evidence() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_json(root, "docs/evidence/random-proof.json", {"proof": "untracked"})
    expect_failure(lambda: validate_milestone_evidence(root), "unsupported milestone evidence")


def assert_require_all_blocks_missing_m14_evidence() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_json(root, INSTALLED_APP_EVIDENCE, valid_installed_app_payload())
    expect_failure(
      lambda: validate_milestone_evidence(root, require_all=True),
      REFERENCE_CONNECTOR_EVIDENCE,
    )


def assert_cli_paths() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    evidence_dir = root / "docs" / "evidence"
    evidence_dir.mkdir(parents=True)
    original_argv = sys.argv[:]
    try:
      sys.argv = ["milestone_evidence.py", "--root", str(root)]
      if milestone_evidence_main() != 0:
        raise AssertionError("empty milestone evidence CLI should pass")

      write_json(root, INSTALLED_APP_EVIDENCE, valid_installed_app_payload())
      write_json(root, REFERENCE_CONNECTOR_EVIDENCE, valid_reference_connector_payload())
      sys.argv = ["milestone_evidence.py", "--root", str(root), "--require-all"]
      if milestone_evidence_main() != 0:
        raise AssertionError("complete milestone evidence CLI should pass")

      (root / REFERENCE_CONNECTOR_EVIDENCE).unlink()
      sys.argv = ["milestone_evidence.py", "--root", str(root), "--require-all"]
      stderr = StringIO()
      with redirect_stderr(stderr):
        result = milestone_evidence_main()
      if result == 0 or REFERENCE_CONNECTOR_EVIDENCE not in stderr.getvalue():
        raise AssertionError("require-all CLI should explain the missing evidence file")
    finally:
      sys.argv = original_argv


def main() -> int:
  assert_accepts_empty_evidence_directory()
  assert_validates_known_evidence_files()
  assert_rejects_invalid_present_evidence()
  assert_rejects_unknown_json_evidence()
  assert_require_all_blocks_missing_m14_evidence()
  assert_cli_paths()
  print("milestone evidence tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
