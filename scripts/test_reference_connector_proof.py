#!/usr/bin/env python3
"""Unit checks for M14 reference connector proof evidence."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from reference_connector_proof import (
  REFERENCE_CONNECTOR_ID,
  REFERENCE_PLUGIN_ID,
  REFERENCE_SERVICE,
  reference_connector_template,
  main as reference_connector_main,
  validate_reference_connector_evidence,
)


VALID_ACTION_ID = "notion.create-page"


def valid_payload() -> dict[str, object]:
  return {
    "pluginId": REFERENCE_PLUGIN_ID,
    "connectorId": REFERENCE_CONNECTOR_ID,
    "service": REFERENCE_SERVICE,
    "pluginInstallOrRefreshCompleted": True,
    "connectorAuthorized": True,
    "actionOrWorkflowRun": True,
    "actionOrWorkflowId": VALID_ACTION_ID,
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
    "acceptedAt": "2026-06-22T00:00:00Z",
  }


def expect_failure(payload: dict[str, object], expected: str) -> None:
  try:
    validate_reference_connector_evidence(payload)
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r} in {error!r}") from error
    return
  raise AssertionError(f"expected reference connector validation to fail: {expected}")


def assert_valid_payload_passes() -> None:
  validate_reference_connector_evidence(valid_payload())


def assert_template_is_safe_to_fill() -> None:
  template = reference_connector_template()
  if template["pluginId"] != REFERENCE_PLUGIN_ID:
    raise AssertionError("template should target the reference plugin")
  if template["connectorId"] != REFERENCE_CONNECTOR_ID:
    raise AssertionError("template should target the reference connector")
  if template["service"] != REFERENCE_SERVICE:
    raise AssertionError("template should name the reference service")
  for key in (
    "pluginInstallOrRefreshCompleted",
    "connectorAuthorized",
    "actionOrWorkflowRun",
    "receiptInspected",
    "credentialCleared",
    "pluginRemoved",
    "noStaleCredentialState",
  ):
    if template[key] is not False:
      raise AssertionError(f"template {key} should stay false until manually filled")
  if template["remainingConnectorCredentialIds"] != []:
    raise AssertionError("template should start with no stale connector ids")


def assert_rejects_wrong_reference_connector() -> None:
  payload = valid_payload()
  payload["connectorId"] = "other::notion"
  expect_failure(payload, "connectorId")


def assert_rejects_incomplete_steps() -> None:
  payload = valid_payload()
  payload["credentialCleared"] = False
  expect_failure(payload, "credentialCleared")


def assert_rejects_stale_credentials() -> None:
  payload = valid_payload()
  payload["remainingConnectorCredentialIds"] = [REFERENCE_CONNECTOR_ID]
  expect_failure(payload, "remainingConnectorCredentialIds")

  payload = valid_payload()
  payload["remainingLocalCredentialHandles"] = ["local-keychain-handle"]
  expect_failure(payload, "remainingLocalCredentialHandles")


def assert_rejects_unknown_action() -> None:
  payload = valid_payload()
  payload["actionOrWorkflowId"] = "slack.post-message"
  expect_failure(payload, "actionOrWorkflowId")


def assert_rejects_placeholder_evidence_text() -> None:
  payload = valid_payload()
  payload["receiptProof"] = "TODO: fill after testing"
  expect_failure(payload, "placeholder")

  payload = valid_payload()
  payload["pluginRemovalProof"] = "N/A"
  expect_failure(payload, "placeholder")


def assert_rejects_non_utc_acceptance_time() -> None:
  payload = valid_payload()
  payload["acceptedAt"] = "2026-06-22"
  expect_failure(payload, "acceptedAt")

  payload = valid_payload()
  payload["acceptedAt"] = "2026-06-22T00:00:00+08:00"
  expect_failure(payload, "acceptedAt")


def assert_rejects_cleanup_proof_without_storage_and_local_handles() -> None:
  payload = valid_payload()
  payload["credentialClearProof"] = "Plugin manager showed Notion needs sign in after clearing."
  expect_failure(payload, "credentialClearProof")


def assert_cli_template_and_validation() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    template_path = root / "reference-connector-template.json"
    evidence_path = root / "reference-connector-proof.json"
    invalid_path = root / "invalid-reference-connector-proof.json"
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "reference_connector_proof.py",
        "--template-output",
        str(template_path),
      ]
      if reference_connector_main() != 0:
        raise AssertionError("reference connector template CLI should pass")
      template = json.loads(template_path.read_text(encoding="utf-8"))
      if template["connectorAuthorized"] is not False:
        raise AssertionError("template should not pre-authorize the connector")

      evidence_path.write_text(json.dumps(valid_payload()), encoding="utf-8")
      sys.argv = [
        "reference_connector_proof.py",
        "--evidence",
        str(evidence_path),
      ]
      if reference_connector_main() != 0:
        raise AssertionError("reference connector evidence CLI should pass")

      invalid = valid_payload()
      invalid["noStaleCredentialState"] = False
      invalid_path.write_text(json.dumps(invalid), encoding="utf-8")
      sys.argv = [
        "reference_connector_proof.py",
        "--evidence",
        str(invalid_path),
      ]
      stderr = StringIO()
      with redirect_stderr(stderr):
        result = reference_connector_main()
      if result == 0:
        raise AssertionError("invalid reference connector evidence should fail")
      if "noStaleCredentialState" not in stderr.getvalue():
        raise AssertionError("CLI failure should explain the failing field")
    finally:
      sys.argv = original_argv


def main() -> int:
  assert_valid_payload_passes()
  assert_template_is_safe_to_fill()
  assert_rejects_wrong_reference_connector()
  assert_rejects_incomplete_steps()
  assert_rejects_stale_credentials()
  assert_rejects_unknown_action()
  assert_rejects_placeholder_evidence_text()
  assert_rejects_non_utc_acceptance_time()
  assert_rejects_cleanup_proof_without_storage_and_local_handles()
  assert_cli_template_and_validation()
  print("reference connector proof tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
