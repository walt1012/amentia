#!/usr/bin/env python3
"""Validate installed-app release acceptance evidence for Amentia."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from evidence_contracts import load_json_object as load_evidence_json_object
from evidence_contracts import reject_placeholder_text
from evidence_contracts import require_empty_list
from evidence_contracts import require_equal
from evidence_contracts import require_sha256_hex
from evidence_contracts import require_string
from evidence_contracts import require_true
from evidence_contracts import require_utc_timestamp
from evidence_contracts import require_words
from evidence_contracts import write_json


APP_NAME = "Amentia"
PROOF_SCOPE = "installed-app"
PROOF_LABEL = "installed-app"
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
  require_equal(data, "appName", APP_NAME, label=PROOF_LABEL)
  require_equal(data, "proofScope", PROOF_SCOPE, label=PROOF_LABEL)
  for check in REQUIRED_TRUE_CHECKS:
    require_true(data, check, label=PROOF_LABEL)
  for field in REQUIRED_TEXT_FIELDS:
    require_string(data, field, label=PROOF_LABEL)
    reject_placeholder_text(data[field], field, label=PROOF_LABEL)
  require_sha256_hex(data["dmgSha256"], label="installed-app dmgSha256")
  require_allowed_model(data["modelId"])
  require_utc_timestamp(
    data["acceptedAt"],
    label="installed-app acceptedAt",
    example="2026-06-23T00:00:00Z",
  )
  require_empty_list(data, "unexpectedAppOwnedResiduePaths", label=PROOF_LABEL)
  require_model_proof(data["modelProof"])
  require_inference_proof(data["inferenceProof"])
  require_web_search_proof(data["webSearchProof"])
  require_session_cleanup_proof(data["sessionCleanupProof"])
  require_reset_proof(data["resetProof"])
  require_plugin_lifecycle_proof(data["pluginLifecycleProof"])


def load_json_object(path: Path) -> dict[str, object]:
  return load_evidence_json_object(path, label="installed-app evidence")


def require_allowed_model(value: object) -> None:
  if value not in ALLOWED_MODEL_IDS:
    allowed = ", ".join(ALLOWED_MODEL_IDS)
    raise RuntimeError(f"installed-app modelId must be one of: {allowed}")


def require_model_proof(value: object) -> None:
  require_words(
    value,
    "modelProof",
    ("download", "verified", "activated", "self-check"),
    label=PROOF_LABEL,
  )


def require_inference_proof(value: object) -> None:
  require_words(value, "inferenceProof", ("local",), label=PROOF_LABEL)


def require_web_search_proof(value: object) -> None:
  require_words(value, "webSearchProof", ("source", "proof"), label=PROOF_LABEL)


def require_session_cleanup_proof(value: object) -> None:
  require_words(value, "sessionCleanupProof", ("delete", "revert"), label=PROOF_LABEL)


def require_reset_proof(value: object) -> None:
  require_words(
    value,
    "resetProof",
    ("support", "cache", "preference", "saved state", "paused download", "credential"),
    label=PROOF_LABEL,
  )


def require_plugin_lifecycle_proof(value: object) -> None:
  require_words(
    value,
    "pluginLifecycleProof",
    ("installed", "disabled", "removed"),
    label=PROOF_LABEL,
  )


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
