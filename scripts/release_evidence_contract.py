#!/usr/bin/env python3
"""Validate internal release evidence artifacts before workflow upload."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from release_artifacts import release_installer_asset_names
from package_contract import RELEASE_SIGNING_MODES


DRY_RUN_EXTRA_NAMES = (
  "release-readiness.md",
  "release-readiness.json",
  "release-plan.md",
  "release-plan.json",
  "release-dry-run-rehearsal.md",
  "release-dry-run-rehearsal.json",
  "release-dry-run-manual-acceptance.md",
)
PUBLISH_REHEARSAL_NAMES = (
  "release-readiness.md",
  "release-readiness.json",
  "release-plan.md",
  "release-plan.json",
  "release-rehearsal.md",
  "release-rehearsal.json",
  "release-manual-acceptance.md",
)
REQUIRED_JSON_KEYS_BY_NAME = {
  "release-readiness.json": (
    "status",
    "tag",
    "sourceCommit",
    "successfulCiRunUrl",
    "workflowMode",
    "signingMode",
    "requestedDraft",
    "requestedPrerelease",
    "allowUntrustedAdHoc",
    "plannedDraft",
    "plannedPrerelease",
    "workingTreeClean",
    "tagPointsAtSourceCommit",
    "releaseWorkflowInputsReady",
    "manualAcceptanceConfirmed",
    "manualAcceptanceEvidence",
    "preDispatchChecklist",
    "expectedPublicAssets",
    "expectedDryRunEvidence",
    "tagCommands",
    "remoteTagVerificationCommand",
    "successfulCiLookupCommand",
    "dryRunArtifactLookupCommand",
    "dryRunArtifactDownloadCommand",
    "dryRunEvidenceValidationCommand",
    "postAcceptancePublishCommand",
    "blockers",
    "nextCommand",
  ),
  "release-plan.json": (
    "tag",
    "title",
    "sourceCommit",
    "successfulCiRunUrl",
    "releaseWorkflowRunUrl",
    "workflowMode",
    "githubMutation",
    "signingMode",
    "releaseExists",
    "existingReleaseState",
    "requestedDraft",
    "requestedPrerelease",
    "allowVisibleAdHoc",
    "manualAcceptanceConfirmed",
    "manualAcceptanceEvidence",
    "plannedDraft",
    "plannedPrerelease",
    "trustPath",
    "nextMaintainerActions",
  ),
}


def expected_evidence_names(mode: str, tag: str) -> tuple[str, ...]:
  if mode == "dry-run":
    return release_installer_asset_names(tag) + DRY_RUN_EXTRA_NAMES
  if mode == "publish-rehearsal":
    return PUBLISH_REHEARSAL_NAMES
  raise RuntimeError(f"Unknown release evidence mode: {mode}")


def validate_release_evidence_set(
  *,
  mode: str,
  tag: str,
  evidence_paths: list[Path],
) -> None:
  if not evidence_paths:
    raise RuntimeError("Release evidence validation requires evidence files.")
  expected_names = set(expected_evidence_names(mode, tag))
  evidence_by_name: dict[str, Path] = {}
  for evidence_path in evidence_paths:
    path = evidence_path.resolve()
    validate_evidence_path(path)
    name = path.name
    if name in evidence_by_name:
      raise RuntimeError(f"Release evidence contains duplicate file: {name}")
    evidence_by_name[name] = path

  actual_names = set(evidence_by_name)
  missing = sorted(expected_names - actual_names)
  extra = sorted(actual_names - expected_names)
  if missing or extra:
    details: list[str] = []
    if missing:
      details.append("missing " + ", ".join(missing))
    if extra:
      details.append("extra " + ", ".join(extra))
    raise RuntimeError(
      "Release evidence set must exactly match the workflow contract: "
      + "; ".join(details)
    )

  for path in evidence_by_name.values():
    validate_evidence_content(path, mode=mode, tag=tag)
  validate_release_json_consistency(evidence_by_name)


def validate_evidence_path(path: Path) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Release evidence file is missing: {path}")
  if path.name in {"", ".", ".."} or "/" in path.name or "\\" in path.name:
    raise RuntimeError(f"Release evidence names must be basenames: {path.name}")


def validate_evidence_content(path: Path, *, mode: str, tag: str) -> None:
  if path.stat().st_size == 0:
    raise RuntimeError(f"Release evidence file must not be empty: {path.name}")
  if path.suffix == ".json":
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
      raise RuntimeError(f"Release evidence JSON must be an object: {path.name}")
    validate_required_json_keys(path.name, data)
    validate_structured_json(path.name, data, mode=mode, tag=tag)
  elif path.suffix == ".md":
    text = path.read_text(encoding="utf-8").strip()
    if not text.startswith("# "):
      raise RuntimeError(f"Release evidence Markdown must start with a heading: {path.name}")


def validate_required_json_keys(name: str, data: dict[str, object]) -> None:
  required_keys = REQUIRED_JSON_KEYS_BY_NAME.get(name)
  if required_keys is None:
    return
  missing = [key for key in required_keys if key not in data]
  if missing:
    raise RuntimeError(
      f"Release evidence JSON is missing required keys for {name}: "
      + ", ".join(missing)
    )


def validate_release_json_consistency(evidence_by_name: dict[str, Path]) -> None:
  readiness = read_evidence_json(evidence_by_name["release-readiness.json"])
  plan = read_evidence_json(evidence_by_name["release-plan.json"])
  require_matching_json_value(readiness, plan, "tag")
  require_matching_json_value(readiness, plan, "sourceCommit")
  require_matching_json_value(readiness, plan, "successfulCiRunUrl")
  require_matching_json_value(readiness, plan, "workflowMode")
  require_matching_json_value(readiness, plan, "signingMode")
  require_matching_json_value(readiness, plan, "requestedDraft")
  require_matching_json_value(readiness, plan, "requestedPrerelease")
  require_matching_json_value(readiness, plan, "plannedDraft")
  require_matching_json_value(readiness, plan, "plannedPrerelease")
  require_matching_json_value(readiness, plan, "manualAcceptanceConfirmed")
  require_matching_json_value(readiness, plan, "manualAcceptanceEvidence")
  require_matching_json_value(
    readiness,
    plan,
    "allowUntrustedAdHoc",
    right_key="allowVisibleAdHoc",
  )


def read_evidence_json(path: Path) -> dict[str, object]:
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError(f"Release evidence JSON must be an object: {path.name}")
  return data


def require_matching_json_value(
  left: dict[str, object],
  right: dict[str, object],
  left_key: str,
  *,
  right_key: str | None = None,
) -> None:
  actual_right_key = right_key or left_key
  if left.get(left_key) != right.get(actual_right_key):
    raise RuntimeError(
      "Release readiness and release plan JSON disagree on "
      f"{left_key}/{actual_right_key}."
    )


def validate_structured_json(
  name: str,
  data: dict[str, object],
  *,
  mode: str,
  tag: str,
) -> None:
  if name == "release-readiness.json":
    validate_release_readiness_json(data, mode=mode, tag=tag)
  elif name == "release-plan.json":
    validate_release_plan_json(data, mode=mode, tag=tag)


def validate_release_readiness_json(
  data: dict[str, object],
  *,
  mode: str,
  tag: str,
) -> None:
  expected_workflow_mode = expected_workflow_mode_for_evidence(mode)
  require_equal(data, "tag", tag)
  require_equal(data, "workflowMode", expected_workflow_mode)
  require_one_of(data, "status", {"ready", "blocked"})
  require_one_of(data, "signingMode", RELEASE_SIGNING_MODES)
  require_string(data, "sourceCommit")
  require_string(data, "successfulCiRunUrl")
  require_string(data, "manualAcceptanceEvidence", allow_empty=True)
  require_string(data, "remoteTagVerificationCommand")
  require_string(data, "successfulCiLookupCommand")
  require_string(data, "dryRunArtifactLookupCommand")
  require_string(data, "dryRunArtifactDownloadCommand")
  require_string(data, "dryRunEvidenceValidationCommand")
  require_string(data, "postAcceptancePublishCommand")
  require_string(data, "nextCommand")
  for key in (
    "requestedDraft",
    "requestedPrerelease",
    "allowUntrustedAdHoc",
    "plannedDraft",
    "plannedPrerelease",
    "workingTreeClean",
    "tagPointsAtSourceCommit",
    "releaseWorkflowInputsReady",
    "manualAcceptanceConfirmed",
  ):
    require_bool(data, key)
  require_string_list(data, "preDispatchChecklist")
  require_string_list(data, "tagCommands")
  require_string_list(data, "blockers", allow_empty=True)
  require_exact_string_list(
    data,
    "expectedPublicAssets",
    release_installer_asset_names(tag),
  )
  require_exact_string_list(
    data,
    "expectedDryRunEvidence",
    expected_evidence_names("dry-run", tag),
  )


def validate_release_plan_json(
  data: dict[str, object],
  *,
  mode: str,
  tag: str,
) -> None:
  expected_workflow_mode = expected_workflow_mode_for_evidence(mode)
  require_equal(data, "tag", tag)
  require_equal(data, "title", f"Pith {tag}")
  require_equal(data, "workflowMode", expected_workflow_mode)
  require_equal(
    data,
    "githubMutation",
    "none" if expected_workflow_mode == "dry-run" else "create-or-update",
  )
  require_one_of(data, "signingMode", RELEASE_SIGNING_MODES)
  require_one_of(data, "existingReleaseState", {"none", "draft", "visible"})
  require_string(data, "sourceCommit")
  require_string(data, "successfulCiRunUrl")
  require_string(data, "releaseWorkflowRunUrl")
  require_string(data, "manualAcceptanceEvidence", allow_empty=True)
  require_string(data, "trustPath")
  for key in (
    "releaseExists",
    "requestedDraft",
    "requestedPrerelease",
    "allowVisibleAdHoc",
    "manualAcceptanceConfirmed",
    "plannedDraft",
    "plannedPrerelease",
  ):
    require_bool(data, key)
  require_string_list(data, "nextMaintainerActions")


def expected_workflow_mode_for_evidence(mode: str) -> str:
  if mode == "dry-run":
    return "dry-run"
  if mode == "publish-rehearsal":
    return "publish"
  raise RuntimeError(f"Unknown release evidence mode: {mode}")


def require_equal(data: dict[str, object], key: str, expected: str) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(
      f"Release evidence JSON key {key} must be {expected!r}, got {actual!r}"
    )


def require_one_of(data: dict[str, object], key: str, expected: set[str]) -> None:
  actual = data.get(key)
  if not isinstance(actual, str) or actual not in expected:
    expected_values = ", ".join(sorted(expected))
    raise RuntimeError(
      f"Release evidence JSON key {key} must be one of {expected_values}."
    )


def require_string(
  data: dict[str, object],
  key: str,
  *,
  allow_empty: bool = False,
) -> None:
  actual = data.get(key)
  if not isinstance(actual, str) or (not allow_empty and not actual.strip()):
    raise RuntimeError(f"Release evidence JSON key {key} must be a string.")


def require_bool(data: dict[str, object], key: str) -> None:
  if not isinstance(data.get(key), bool):
    raise RuntimeError(f"Release evidence JSON key {key} must be a boolean.")


def require_string_list(
  data: dict[str, object],
  key: str,
  *,
  allow_empty: bool = False,
) -> None:
  actual = data.get(key)
  if (
    not isinstance(actual, list)
    or (not allow_empty and not actual)
    or any(not isinstance(item, str) or not item.strip() for item in actual)
  ):
    raise RuntimeError(f"Release evidence JSON key {key} must be a string list.")


def require_exact_string_list(
  data: dict[str, object],
  key: str,
  expected: tuple[str, ...],
) -> None:
  actual = data.get(key)
  if list(expected) != actual:
    raise RuntimeError(
      f"Release evidence JSON key {key} must match the release contract."
    )


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--mode", required=True, choices=("dry-run", "publish-rehearsal"))
  parser.add_argument("--tag", required=True)
  parser.add_argument("--evidence", action="append", default=[], type=Path)
  args = parser.parse_args()

  try:
    validate_release_evidence_set(
      mode=args.mode,
      tag=args.tag,
      evidence_paths=args.evidence,
    )
  except Exception as error:
    print(f"release evidence contract failed: {error}", file=sys.stderr)
    return 1

  print("Release evidence contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
