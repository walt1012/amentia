#!/usr/bin/env python3
"""Validate M14 installed-app proof evidence for Amentia."""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime
from pathlib import Path


APP_NAME = "Amentia"
PROOF_SCOPE = "installed-app"
ALLOWED_MODEL_IDS = (
  "lfm2.5-350m",
  "granite-4.0-h-350m",
  "minicpm5-1b",
)
REQUIRED_TRUE_CHECKS = (
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
)
REQUIRED_TEXT_FIELDS = (
  "releaseTag",
  "dmgName",
  "dmgSha256",
  "modelId",
  "modelProof",
  "inferenceProof",
  "webSearchProof",
  "sessionCleanupProof",
  "resetProof",
  "pluginLifecycleProof",
  "acceptedBy",
  "acceptedAt",
)


def installed_app_template() -> dict[str, object]:
  return {
    "appName": APP_NAME,
    "proofScope": PROOF_SCOPE,
    "releaseTag": "",
    "dmgName": "",
    "dmgSha256": "",
    "dmgChecksumVerified": False,
    "installedFromDmg": False,
    "firstLaunchCompleted": False,
    "modelDownloaded": False,
    "modelVerified": False,
    "modelActivated": False,
    "modelSelfCheckPassed": False,
    "modelId": "",
    "modelProof": "",
    "localInferenceCompleted": False,
    "inferenceProof": "",
    "webSearchProofInspected": False,
    "webSearchProof": "",
    "sessionDeleteVerified": False,
    "sessionRevertVerified": False,
    "sessionCleanupProof": "",
    "resetAmentiaVerified": False,
    "resetProof": "",
    "pluginLifecycleVerified": False,
    "pluginLifecycleProof": "",
    "unexpectedAppOwnedResiduePaths": [],
    "noUnexpectedAppOwnedResidue": False,
    "acceptedBy": "",
    "acceptedAt": "",
  }


def validate_installed_app_evidence(data: dict[str, object]) -> None:
  require_equal(data, "appName", APP_NAME)
  require_equal(data, "proofScope", PROOF_SCOPE)
  for check in REQUIRED_TRUE_CHECKS:
    require_true(data, check)
  for field in REQUIRED_TEXT_FIELDS:
    require_string(data, field)
    reject_placeholder_text(data[field], field)
  require_sha256(data["dmgSha256"])
  require_allowed_model(data["modelId"])
  require_utc_timestamp(data["acceptedAt"])
  require_empty_list(data, "unexpectedAppOwnedResiduePaths")
  require_model_proof(data["modelProof"])
  require_inference_proof(data["inferenceProof"])
  require_web_search_proof(data["webSearchProof"])
  require_session_cleanup_proof(data["sessionCleanupProof"])
  require_reset_proof(data["resetProof"])
  require_plugin_lifecycle_proof(data["pluginLifecycleProof"])


def load_json_object(path: Path) -> dict[str, object]:
  if not path.is_file():
    raise FileNotFoundError(f"installed-app evidence is missing: {path}")
  value = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(value, dict):
    raise RuntimeError("installed-app evidence must be a JSON object")
  return value


def write_json(path: Path, payload: dict[str, object]) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def require_equal(data: dict[str, object], key: str, expected: str) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(f"installed-app {key} must be {expected!r}, got {actual!r}")


def require_true(data: dict[str, object], key: str) -> None:
  if data.get(key) is not True:
    raise RuntimeError(f"installed-app {key} must be true")


def require_string(data: dict[str, object], key: str) -> None:
  actual = data.get(key)
  if not isinstance(actual, str) or not actual.strip():
    raise RuntimeError(f"installed-app {key} must be a non-empty string")


def reject_placeholder_text(value: object, key: str) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"installed-app {key} must be a non-empty string")
  normalized = value.strip().lower()
  placeholder_values = {"todo", "tbd", "n/a", "na", "none", "placeholder"}
  if normalized in placeholder_values or normalized.startswith("todo") or "fill" in normalized:
    raise RuntimeError(f"installed-app {key} must not be placeholder text")


def require_sha256(value: object) -> None:
  if not isinstance(value, str) or re.fullmatch(r"[0-9a-fA-F]{64}", value) is None:
    raise RuntimeError("installed-app dmgSha256 must be a 64-character SHA-256 hex digest")


def require_allowed_model(value: object) -> None:
  if value not in ALLOWED_MODEL_IDS:
    allowed = ", ".join(ALLOWED_MODEL_IDS)
    raise RuntimeError(f"installed-app modelId must be one of: {allowed}")


def require_utc_timestamp(value: object) -> None:
  if not isinstance(value, str):
    raise RuntimeError("installed-app acceptedAt must be a UTC timestamp")
  try:
    datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
  except ValueError as error:
    raise RuntimeError(
      "installed-app acceptedAt must use UTC ISO format like 2026-06-23T00:00:00Z"
    ) from error


def require_empty_list(data: dict[str, object], key: str) -> None:
  actual = data.get(key)
  if actual != []:
    raise RuntimeError(f"installed-app {key} must be an empty list")


def require_words(value: object, key: str, words: tuple[str, ...]) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"installed-app {key} must be a non-empty string")
  normalized = value.lower()
  missing = [word for word in words if word not in normalized]
  if missing:
    missing_label = ", ".join(missing)
    raise RuntimeError(f"installed-app {key} must mention: {missing_label}")


def require_model_proof(value: object) -> None:
  require_words(value, "modelProof", ("download", "verified", "activated", "self-check"))


def require_inference_proof(value: object) -> None:
  require_words(value, "inferenceProof", ("local",))


def require_web_search_proof(value: object) -> None:
  require_words(value, "webSearchProof", ("source", "proof"))


def require_session_cleanup_proof(value: object) -> None:
  require_words(value, "sessionCleanupProof", ("delete", "revert"))


def require_reset_proof(value: object) -> None:
  require_words(
    value,
    "resetProof",
    ("support", "cache", "preference", "saved state", "paused download", "credential"),
  )


def require_plugin_lifecycle_proof(value: object) -> None:
  require_words(value, "pluginLifecycleProof", ("installed", "disabled", "removed"))


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--evidence", type=Path, help="Validate filled installed-app evidence.")
  parser.add_argument(
    "--template-output",
    type=Path,
    help="Write a safe-to-fill installed-app evidence template.",
  )
  args = parser.parse_args(argv)
  if args.evidence is None and args.template_output is None:
    parser.error("provide --evidence or --template-output")
  return args


def main(argv: list[str] | None = None) -> int:
  args = parse_args(argv)
  try:
    if args.template_output is not None:
      write_json(args.template_output, installed_app_template())
    if args.evidence is not None:
      validate_installed_app_evidence(load_json_object(args.evidence))
  except Exception as error:
    print(f"error: {error}", file=sys.stderr)
    return 1

  if args.template_output is not None:
    print(f"wrote installed-app template to {args.template_output}")
  if args.evidence is not None:
    print("installed-app proof validation passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
