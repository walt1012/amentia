#!/usr/bin/env python3
"""Validate M14 reference connector proof evidence for Amentia."""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path


REFERENCE_PLUGIN_ID = "notion-connector"
REFERENCE_CONNECTOR_ID = "notion-connector::notion"
REFERENCE_SERVICE = "notion"
ALLOWED_ACTION_OR_WORKFLOW_IDS = (
  "notion.prepare-page-draft",
  "notion.inspect-page-write",
  "notion.publish-page-draft",
  "notion.create-page",
)
REQUIRED_TRUE_CHECKS = (
  "pluginInstallOrRefreshCompleted",
  "connectorAuthorized",
  "actionOrWorkflowRun",
  "receiptInspected",
  "credentialCleared",
  "pluginRemoved",
  "noStaleCredentialState",
)
REQUIRED_TEXT_FIELDS = (
  "actionOrWorkflowId",
  "receiptProof",
  "credentialClearProof",
  "pluginRemovalProof",
  "acceptedBy",
  "acceptedAt",
)
REQUIRED_EMPTY_LIST_FIELDS = (
  "remainingConnectorCredentialIds",
  "remainingLocalCredentialHandles",
)


def reference_connector_template() -> dict[str, object]:
  return {
    "pluginId": REFERENCE_PLUGIN_ID,
    "connectorId": REFERENCE_CONNECTOR_ID,
    "service": REFERENCE_SERVICE,
    "pluginInstallOrRefreshCompleted": False,
    "connectorAuthorized": False,
    "actionOrWorkflowRun": False,
    "actionOrWorkflowId": "",
    "receiptInspected": False,
    "receiptProof": "",
    "credentialCleared": False,
    "credentialClearProof": "",
    "pluginRemoved": False,
    "pluginRemovalProof": "",
    "remainingConnectorCredentialIds": [],
    "remainingLocalCredentialHandles": [],
    "noStaleCredentialState": False,
    "acceptedBy": "",
    "acceptedAt": "",
  }


def validate_reference_connector_evidence(data: dict[str, object]) -> None:
  require_equal(data, "pluginId", REFERENCE_PLUGIN_ID)
  require_equal(data, "connectorId", REFERENCE_CONNECTOR_ID)
  require_equal(data, "service", REFERENCE_SERVICE)
  for check in REQUIRED_TRUE_CHECKS:
    require_true(data, check)
  for field in REQUIRED_TEXT_FIELDS:
    require_string(data, field)
    reject_placeholder_text(data[field], field)
  require_utc_timestamp(data["acceptedAt"])
  require_credential_cleanup_proof(data["credentialClearProof"])
  require_allowed_action(data["actionOrWorkflowId"])
  for field in REQUIRED_EMPTY_LIST_FIELDS:
    require_empty_list(data, field)


def load_json_object(path: Path) -> dict[str, object]:
  if not path.is_file():
    raise FileNotFoundError(f"reference connector evidence is missing: {path}")
  value = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(value, dict):
    raise RuntimeError("reference connector evidence must be a JSON object")
  return value


def write_json(path: Path, payload: dict[str, object]) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def require_equal(data: dict[str, object], key: str, expected: str) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(f"reference connector {key} must be {expected!r}, got {actual!r}")


def require_true(data: dict[str, object], key: str) -> None:
  if data.get(key) is not True:
    raise RuntimeError(f"reference connector {key} must be true")


def require_string(data: dict[str, object], key: str) -> None:
  actual = data.get(key)
  if not isinstance(actual, str) or not actual.strip():
    raise RuntimeError(f"reference connector {key} must be a non-empty string")


def reject_placeholder_text(value: object, key: str) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"reference connector {key} must be a non-empty string")
  normalized = value.strip().lower()
  placeholder_values = {"todo", "tbd", "n/a", "na", "none", "placeholder"}
  if normalized in placeholder_values or normalized.startswith("todo") or "fill" in normalized:
    raise RuntimeError(f"reference connector {key} must not be placeholder text")


def require_utc_timestamp(value: object) -> None:
  if not isinstance(value, str):
    raise RuntimeError("reference connector acceptedAt must be a UTC timestamp")
  try:
    datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
  except ValueError as error:
    raise RuntimeError(
      "reference connector acceptedAt must use UTC ISO format like 2026-06-22T00:00:00Z"
    ) from error


def require_credential_cleanup_proof(value: object) -> None:
  if not isinstance(value, str):
    raise RuntimeError("reference connector credentialClearProof must be a non-empty string")
  normalized = value.lower()
  mentions_storage = "storage" in normalized or "sqlite" in normalized
  mentions_local_handles = "local credential" in normalized or "credential handle" in normalized
  if not mentions_storage or not mentions_local_handles:
    raise RuntimeError(
      "reference connector credentialClearProof must mention storage cleanup and local credential handles"
    )


def require_empty_list(data: dict[str, object], key: str) -> None:
  actual = data.get(key)
  if actual != []:
    raise RuntimeError(f"reference connector {key} must be an empty list")


def require_allowed_action(value: object) -> None:
  if value not in ALLOWED_ACTION_OR_WORKFLOW_IDS:
    allowed = ", ".join(ALLOWED_ACTION_OR_WORKFLOW_IDS)
    raise RuntimeError(f"reference connector actionOrWorkflowId must be one of: {allowed}")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--evidence", type=Path, help="Validate filled reference connector evidence.")
  parser.add_argument(
    "--template-output",
    type=Path,
    help="Write a safe-to-fill reference connector evidence template.",
  )
  args = parser.parse_args(argv)
  if args.evidence is None and args.template_output is None:
    parser.error("provide --evidence or --template-output")
  return args


def main(argv: list[str] | None = None) -> int:
  args = parse_args(argv)
  try:
    if args.template_output is not None:
      write_json(args.template_output, reference_connector_template())
    if args.evidence is not None:
      validate_reference_connector_evidence(load_json_object(args.evidence))
  except Exception as error:
    print(f"error: {error}", file=sys.stderr)
    return 1

  if args.template_output is not None:
    print(f"wrote reference connector template to {args.template_output}")
  if args.evidence is not None:
    print("reference connector proof validation passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
