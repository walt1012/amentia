#!/usr/bin/env python3
"""Validate structured manual release acceptance evidence for Pith."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from package_contract import DEFAULT_MODEL_ID
from release_artifacts import release_installer_asset_names
from release_identity import validate_public_release_tag


REQUIRED_TRUE_CHECKS = (
  "checksumVerified",
  "manifestReviewed",
  "gatekeeperHandled",
  "modelDownloadedAndActivated",
  "workspaceOpened",
  "coworkTurnCompleted",
  "webSearchProofInspected",
  "approvalDiffReceiptInspected",
  "restartRecoveryVerified",
  "noPithLoginRequired",
  "acceptedForVisiblePrerelease",
)
REQUIRED_TEXT_FIELDS = (
  "tag",
  "sourceCommit",
  "releaseWorkflowRunUrl",
  "dmgAssetName",
  "checksum",
  "gatekeeperPath",
  "selectedModelId",
  "workspaceDescription",
  "coworkRequest",
  "webSearchProof",
  "approvalReceipt",
  "restartRecoveryProof",
  "acceptedBy",
  "acceptedAt",
)


def validate_manual_acceptance_evidence(data: dict[str, object], *, tag: str) -> None:
  validate_public_release_tag(tag)
  require_equal(data, "tag", tag)
  require_string(data, "sourceCommit", length=40)
  require_string(data, "releaseWorkflowRunUrl", prefix="https://github.com/walt1012/pith/actions/runs/")
  require_equal(data, "dmgAssetName", release_installer_asset_names(tag)[0])
  require_string(data, "checksum", length=64)
  require_sha256(data, "checksum")
  require_equal(data, "selectedModelId", DEFAULT_MODEL_ID)
  for field in REQUIRED_TEXT_FIELDS:
    require_string(data, field)
  for check in REQUIRED_TRUE_CHECKS:
    require_true(data, check)


def manual_acceptance_template(*, tag: str, source_commit: str, release_workflow_run_url: str) -> dict[str, object]:
  validate_public_release_tag(tag)
  return {
    "tag": tag,
    "sourceCommit": source_commit,
    "releaseWorkflowRunUrl": release_workflow_run_url,
    "dmgAssetName": release_installer_asset_names(tag)[0],
    "checksum": "",
    "checksumVerified": False,
    "manifestReviewed": False,
    "gatekeeperPath": "",
    "gatekeeperHandled": False,
    "selectedModelId": DEFAULT_MODEL_ID,
    "modelDownloadedAndActivated": False,
    "workspaceDescription": "",
    "workspaceOpened": False,
    "coworkRequest": "",
    "coworkTurnCompleted": False,
    "webSearchProof": "",
    "webSearchProofInspected": False,
    "approvalReceipt": "",
    "approvalDiffReceiptInspected": False,
    "restartRecoveryProof": "",
    "restartRecoveryVerified": False,
    "noPithLoginRequired": False,
    "acceptedForVisiblePrerelease": False,
    "acceptedBy": "",
    "acceptedAt": "",
  }


def require_equal(data: dict[str, object], key: str, expected: str) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(f"manual acceptance {key} must be {expected!r}, got {actual!r}")


def require_string(
  data: dict[str, object],
  key: str,
  *,
  length: int | None = None,
  prefix: str | None = None,
) -> None:
  actual = data.get(key)
  if not isinstance(actual, str) or not actual.strip():
    raise RuntimeError(f"manual acceptance {key} must be a non-empty string")
  value = actual.strip()
  if length is not None and len(value) != length:
    raise RuntimeError(f"manual acceptance {key} must be {length} characters")
  if prefix is not None and not value.startswith(prefix):
    raise RuntimeError(f"manual acceptance {key} must start with {prefix}")


def require_true(data: dict[str, object], key: str) -> None:
  if data.get(key) is not True:
    raise RuntimeError(f"manual acceptance {key} must be true")


def require_sha256(data: dict[str, object], key: str) -> None:
  value = str(data[key]).strip().lower()
  if any(character not in "0123456789abcdef" for character in value):
    raise RuntimeError(f"manual acceptance {key} must be a SHA-256 hex digest")


def load_json(path: Path) -> dict[str, object]:
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError("manual acceptance evidence must be a JSON object")
  return data


def write_json(path: Path, data: dict[str, object]) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--evidence", type=Path)
  parser.add_argument("--template-output", type=Path)
  parser.add_argument("--source-commit", default="")
  parser.add_argument("--release-workflow-run-url", default="")
  args = parser.parse_args()

  try:
    if args.template_output is not None:
      write_json(
        args.template_output,
        manual_acceptance_template(
          tag=args.tag,
          source_commit=args.source_commit,
          release_workflow_run_url=args.release_workflow_run_url,
        ),
      )
      print("Manual acceptance evidence template written")
      return 0
    if args.evidence is None:
      raise RuntimeError("manual acceptance validation requires --evidence")
    validate_manual_acceptance_evidence(load_json(args.evidence), tag=args.tag)
  except Exception as error:
    print(f"manual acceptance contract failed: {error}", file=sys.stderr)
    return 1

  print("Manual acceptance contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
