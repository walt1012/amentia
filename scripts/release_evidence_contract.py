#!/usr/bin/env python3
"""Validate internal release evidence artifacts before workflow upload."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from installer_artifact_contract import validate_installer_asset_set
from release_artifacts import release_installer_asset_names
from package_contract import DEFAULT_MODEL_ID
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
REQUIRED_REHEARSAL_JSON_KEYS = (
  "tag",
  "result",
  "assetNames",
  "checksumCommand",
  "sourceCommit",
  "signingMode",
  "trust",
  "defaultModelId",
  "appPackage",
  "firstRun",
  "dailyDriver",
  "releaseDecision",
  "firstAppOpenChecks",
  "manualPrereleaseChecks",
  "packagedSmokeReceipt",
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
  "release-dry-run-rehearsal.json": REQUIRED_REHEARSAL_JSON_KEYS,
  "release-rehearsal.json": REQUIRED_REHEARSAL_JSON_KEYS,
}
REQUIRED_REHEARSAL_MARKDOWN_TERMS = (
  "Release Rehearsal",
  "Result: `passed`",
  "## Assets",
  "## Verification",
  "## Packaged Smoke Journey",
  "## First App Open",
  "## Manual Prerelease Acceptance",
  DEFAULT_MODEL_ID,
  "Web Search",
  "reviewing the diff",
)
REQUIRED_MANUAL_ACCEPTANCE_TERMS = (
  "Manual Release Acceptance",
  "Accept this build for visible ad-hoc prerelease.",
  "Keep this build draft-only",
  "## Downloaded Assets",
  "## Required Manual Checks",
  "SHA-256 sidecar",
  "Gatekeeper",
  DEFAULT_MODEL_ID,
  "Web Search",
  "Approval and diff receipt",
  "Restart recovery",
  "structured manual acceptance JSON receipt",
  "--asset-dir",
  "--template-output",
)
REQUIRED_MARKDOWN_TERMS_BY_NAME = {
  "release-dry-run-rehearsal.md": REQUIRED_REHEARSAL_MARKDOWN_TERMS,
  "release-rehearsal.md": REQUIRED_REHEARSAL_MARKDOWN_TERMS,
  "release-dry-run-manual-acceptance.md": REQUIRED_MANUAL_ACCEPTANCE_TERMS,
  "release-manual-acceptance.md": REQUIRED_MANUAL_ACCEPTANCE_TERMS,
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
  validate_release_asset_evidence(mode, tag, evidence_by_name)
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
    validate_required_markdown_terms(path.name, text)


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


def validate_required_markdown_terms(name: str, text: str) -> None:
  required_terms = REQUIRED_MARKDOWN_TERMS_BY_NAME.get(name)
  if required_terms is None:
    return
  missing = [term for term in required_terms if term not in text]
  if missing:
    raise RuntimeError(
      f"Release evidence Markdown is missing required terms for {name}: "
      + ", ".join(missing)
    )


def validate_release_asset_evidence(
  mode: str,
  tag: str,
  evidence_by_name: dict[str, Path],
) -> None:
  if mode != "dry-run":
    return
  validate_installer_asset_set(
    tag,
    [evidence_by_name[name] for name in release_installer_asset_names(tag)],
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
  elif name in {"release-dry-run-rehearsal.json", "release-rehearsal.json"}:
    validate_release_rehearsal_json(data, tag=tag)


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
  validate_release_readiness_state(data)
  validate_release_readiness_checklist(data, tag=tag)
  validate_release_readiness_commands(data, tag=tag)


def validate_release_readiness_state(data: dict[str, object]) -> None:
  blockers = data["blockers"]
  if blockers:
    raise RuntimeError("release readiness evidence must not contain blockers.")
  if data["status"] != "ready":
    raise RuntimeError("release readiness evidence status must be ready.")
  for key in (
    "workingTreeClean",
    "tagPointsAtSourceCommit",
    "releaseWorkflowInputsReady",
  ):
    if data[key] is not True:
      raise RuntimeError(f"release readiness evidence key {key} must be true.")


def validate_release_readiness_checklist(
  data: dict[str, object],
  *,
  tag: str,
) -> None:
  source_commit = str(data["sourceCommit"])
  artifact_name = f"release-dry-run-{tag}"
  require_string_list_contains(
    data,
    "preDispatchChecklist",
    (
      f"Create tag {tag}",
      source_commit,
      "Push the tag to origin",
      "remote tag verification command",
      "CI lookup command",
      "successful CI run matches the source commit",
      "release workflow as a dry-run",
      artifact_name,
      "dry-run evidence validation command",
      "DMG checksum",
      "manual acceptance",
      "post-acceptance publish command",
    ),
  )


def validate_release_readiness_commands(
  data: dict[str, object],
  *,
  tag: str,
) -> None:
  source_commit = str(data["sourceCommit"])
  expected_tag_commands = [
    f"git tag {tag} {source_commit}",
    f"git push origin {tag}",
  ]
  if data["tagCommands"] != expected_tag_commands:
    raise RuntimeError(
      "release readiness tag commands must target the source commit."
    )
  remote_tag_command = str(data["remoteTagVerificationCommand"])
  tag_ref = f"refs/tags/{tag}"
  require_text_contains(
    remote_tag_command,
    (
      "git ls-remote --exit-code --tags origin",
      tag_ref,
      f"'{tag_ref}^{{}}'",
      "tail -n 1",
      source_commit,
    ),
    "release readiness remote tag verification command",
  )
  ci_lookup_command = str(data["successfulCiLookupCommand"])
  require_text_contains(
    ci_lookup_command,
    (
      "gh run list",
      "--workflow CI",
      f"--commit {source_commit}",
      "--status success",
      "--json conclusion,headSha,url",
      "--limit 20",
      f'.headSha == "{source_commit}"',
      '.conclusion == "success"',
    ),
    "release readiness successful CI lookup command",
  )
  artifact_name = f"release-dry-run-{tag}"
  lookup_command = str(data["dryRunArtifactLookupCommand"])
  require_text_contains(
    lookup_command,
    (
      "gh api repos/:owner/:repo/actions/artifacts --paginate",
      artifact_name,
      source_commit,
      ".expired == false",
      ".workflow_run.head_sha",
      'test -n "$release_workflow_run_id"',
    ),
    "release readiness dry-run artifact lookup command",
  )
  download_command = str(data["dryRunArtifactDownloadCommand"])
  require_text_contains(
    download_command,
    (
      'gh run view "$release_workflow_run_id" --json conclusion --jq .conclusion',
      'test "$release_workflow_conclusion" = success',
      f"gh run download \"$release_workflow_run_id\" --name {artifact_name}",
    ),
    "release readiness dry-run artifact download command",
  )
  validation_command = str(data["dryRunEvidenceValidationCommand"])
  required_validation_terms = [
    "python3 scripts/release_evidence_contract.py",
    "--mode dry-run",
    f"--tag {tag}",
  ]
  required_validation_terms.extend(
    f"--evidence {artifact_name}/{name}"
    for name in expected_evidence_names("dry-run", tag)
  )
  require_text_contains(
    validation_command,
    tuple(required_validation_terms),
    "release readiness dry-run evidence validation command",
  )
  next_command = str(data["nextCommand"])
  dry_run_value = "true" if str(data["workflowMode"]) == "dry-run" else "false"
  require_text_contains(
    next_command,
    (
      "gh workflow run release.yml",
      f"-f tag={tag}",
      f"-f dry_run={dry_run_value}",
      f"-f draft={str(data['requestedDraft']).lower()}",
      f"-f prerelease={str(data['requestedPrerelease']).lower()}",
      f"-f publish_untrusted_ad_hoc={str(data['allowUntrustedAdHoc']).lower()}",
      f"-f manual_acceptance_confirmed={str(data['manualAcceptanceConfirmed']).lower()}",
      f"-f manual_acceptance_evidence={data['manualAcceptanceEvidence']}",
    ),
    "release readiness next command",
  )
  publish_command = str(data["postAcceptancePublishCommand"])
  if str(data["signingMode"]) == "developer-id":
    publish_trust_terms = (
      "-f publish_untrusted_ad_hoc=false",
      "-f manual_acceptance_confirmed=false",
      "-f manual_acceptance_evidence=",
    )
  else:
    publish_trust_terms = (
      "-f publish_untrusted_ad_hoc=true",
      "-f manual_acceptance_confirmed=true",
      "-f manual_acceptance_evidence='<manual-acceptance-receipt-url>'",
    )
  require_text_contains(
    publish_command,
    (
      "gh workflow run release.yml",
      f"-f tag={tag}",
      "-f dry_run=false",
      "-f draft=false",
      "-f prerelease=true",
      *publish_trust_terms,
    ),
    "release readiness post-acceptance publish command",
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
  validate_release_plan_actions(data)


def validate_release_plan_actions(data: dict[str, object]) -> None:
  if data["workflowMode"] == "dry-run":
    require_string_list_contains(
      data,
      "nextMaintainerActions",
      (
        "release-dry-run-*",
        "DMG checksum",
        "release manifest",
        "dry-run rehearsal summary",
        "manual acceptance receipt",
        "dry_run=false",
      ),
    )
    return
  if data["plannedDraft"] is True:
    require_string_list_contains(
      data,
      "nextMaintainerActions",
      (
        "draft GitHub Release assets",
        "downloaded-release rehearsal summary",
        "manual prerelease acceptance",
        "release page",
        "manifest",
        "checksum",
        "install guide",
      ),
    )
    return
  require_string_list_contains(
    data,
    "nextMaintainerActions",
    (
      "visible GitHub Release page",
      "exact four public assets",
      "manual acceptance receipt",
      "published DMG",
      "withdraw the release deliberately",
    ),
  )


def validate_release_rehearsal_json(data: dict[str, object], *, tag: str) -> None:
  require_equal(data, "tag", tag)
  require_equal(data, "result", "passed")
  require_one_of(data, "signingMode", RELEASE_SIGNING_MODES)
  require_string(data, "checksumCommand")
  require_string(data, "sourceCommit")
  require_equal(data, "defaultModelId", DEFAULT_MODEL_ID)
  require_exact_string_list(data, "assetNames", release_installer_asset_names(tag))
  require_string_list(data, "firstAppOpenChecks")
  require_string_list(data, "manualPrereleaseChecks")
  validate_first_app_open_checks(data)
  validate_manual_prerelease_checks(data)
  trust = require_object(data, "trust")
  require_object_string(trust, "mode")
  require_object_string(trust, "gatekeeper")
  app_package = require_object(data, "appPackage")
  require_object_equal(app_package, "sourceCommit", data["sourceCommit"])
  require_object_string(app_package, "modelDelivery")
  require_object_string(app_package, "firstAppOpenActionContract")
  require_object(data, "firstRun")
  require_object(data, "dailyDriver")
  release_decision = require_object(data, "releaseDecision")
  require_object_equal(release_decision, "automatedRehearsal", "passed")
  require_object_equal(
    release_decision,
    "manualAcceptance",
    "required-before-visible-prerelease",
  )
  require_object_equal(
    release_decision,
    "publishGate",
    "do-not-publish-visible-ad-hoc-until-manual-acceptance-passes",
  )
  smoke = require_object(data, "packagedSmokeReceipt")
  require_object_string(smoke, "phrase")
  require_object_string(smoke, "proofScope")
  require_object_bool(smoke, "packageMetadataMatched", True)
  if not isinstance(smoke.get("checkCount"), int) or smoke["checkCount"] <= 0:
    raise RuntimeError("Release rehearsal packaged smoke checkCount must be positive.")
  if not isinstance(smoke.get("journey"), list) or not smoke["journey"]:
    raise RuntimeError("Release rehearsal packaged smoke journey must be present.")


def validate_first_app_open_checks(data: dict[str, object]) -> None:
  require_string_list_contains(
    data,
    "firstAppOpenChecks",
    (
      "Launch Pith",
      "Gatekeeper",
      DEFAULT_MODEL_ID,
      "Open a workspace folder",
      "Web Search",
      "sandbox status",
      "daily-driver next action",
    ),
  )


def validate_manual_prerelease_checks(data: dict[str, object]) -> None:
  require_string_list_contains(
    data,
    "manualPrereleaseChecks",
    (
      "SHA-256 sidecar",
      "release manifest",
      "macOS x86_64",
      "in-app model delivery",
      "no bundled model weights",
      "no Pith login",
      "Gatekeeper",
      DEFAULT_MODEL_ID,
      "real workspace folder",
      "Map Workspace",
      "Plan Next Step",
      "Web Search",
      "reviewing the diff",
      "Restart Pith",
      "runtime readiness",
      "model state",
      "recent proof recover",
    ),
  )


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


def require_text_contains(
  text: str,
  required_terms: tuple[str, ...],
  label: str,
) -> None:
  missing = [term for term in required_terms if term not in text]
  if missing:
    raise RuntimeError(f"{label} is missing required terms: " + ", ".join(missing))


def require_string_list_contains(
  data: dict[str, object],
  key: str,
  required_terms: tuple[str, ...],
) -> None:
  actual = data.get(key)
  if not isinstance(actual, list):
    raise RuntimeError(f"Release evidence JSON key {key} must be a string list.")
  combined = "\n".join(item for item in actual if isinstance(item, str))
  missing = [term for term in required_terms if term not in combined]
  if missing:
    raise RuntimeError(
      f"Release evidence JSON key {key} is missing required terms: "
      + ", ".join(missing)
    )


def require_object(data: dict[str, object], key: str) -> dict[str, object]:
  actual = data.get(key)
  if not isinstance(actual, dict):
    raise RuntimeError(f"Release evidence JSON key {key} must be an object.")
  return actual


def require_object_string(data: dict[str, object], key: str) -> None:
  actual = data.get(key)
  if not isinstance(actual, str) or not actual.strip():
    raise RuntimeError(f"Release evidence JSON object key {key} must be a string.")


def require_object_equal(
  data: dict[str, object],
  key: str,
  expected: object,
) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(
      f"Release evidence JSON object key {key} must be {expected!r}, got {actual!r}"
    )


def require_object_bool(
  data: dict[str, object],
  key: str,
  expected: bool,
) -> None:
  actual = data.get(key)
  if actual is not expected:
    raise RuntimeError(
      f"Release evidence JSON object key {key} must be {str(expected).lower()}."
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
