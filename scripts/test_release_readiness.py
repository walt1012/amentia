#!/usr/bin/env python3
"""Unit checks for release readiness reporting."""

from __future__ import annotations

from release_readiness import plan_readiness
from release_readiness import readiness_json
from release_readiness import readiness_report


VALID_CI_URL = "https://github.com/walt1012/pith/actions/runs/100"
VALID_COMMIT = "0123456789abcdef0123456789abcdef01234567"


def assert_ready_dry_run_report() -> None:
  readiness = plan_readiness(
    tag="v0.1.0",
    source_commit=VALID_COMMIT,
    working_tree_clean_value=True,
    tag_points_at_commit_value=True,
    workflow_inputs_ready=True,
    ci_run_url=VALID_CI_URL,
    dry_run=True,
    signing_mode="ad-hoc",
    requested_draft=True,
    requested_prerelease=True,
    allow_untrusted_ad_hoc=False,
    manual_acceptance_confirmed=False,
    manual_acceptance_evidence="",
  )
  if not readiness.ready:
    raise AssertionError(f"dry-run should be ready: {readiness.blockers}")

  report = readiness_report(readiness)
  for phrase in (
    "Status: `ready`",
    "Workflow mode: `dry-run`",
    "Planned visibility: `draft prerelease`",
    "Tag points at source commit: `true`",
    "gh workflow run release.yml",
    "-f dry_run=true",
  ):
    if phrase not in report:
      raise AssertionError(f"readiness report should include {phrase}")

  payload = readiness_json(readiness)
  expected_values = {
    "status": "ready",
    "tag": "v0.1.0",
    "sourceCommit": VALID_COMMIT,
    "successfulCiRunUrl": VALID_CI_URL,
    "workflowMode": "dry-run",
    "signingMode": "ad-hoc",
    "plannedDraft": True,
    "plannedPrerelease": True,
    "workingTreeClean": True,
    "tagPointsAtSourceCommit": True,
    "releaseWorkflowInputsReady": True,
  }
  for key, expected in expected_values.items():
    if payload.get(key) != expected:
      raise AssertionError(f"readiness JSON {key} should be {expected!r}, got {payload.get(key)!r}")
  if "gh workflow run release.yml" not in str(payload.get("nextCommand", "")):
    raise AssertionError("readiness JSON should include the dispatch command")


def assert_blocks_missing_ci_and_tag() -> None:
  readiness = plan_readiness(
    tag="v0.1.0",
    source_commit=VALID_COMMIT,
    working_tree_clean_value=True,
    tag_points_at_commit_value=False,
    workflow_inputs_ready=True,
    ci_run_url="",
    dry_run=True,
    signing_mode="ad-hoc",
    requested_draft=True,
    requested_prerelease=True,
    allow_untrusted_ad_hoc=False,
    manual_acceptance_confirmed=False,
    manual_acceptance_evidence="",
  )
  if readiness.ready:
    raise AssertionError("missing tag and CI should block release readiness")
  report = readiness_report(readiness)
  for phrase in (
    "Status: `blocked`",
    "Tag v0.1.0 must exist locally",
    "Successful CI run URL must be recorded",
  ):
    if phrase not in report:
      raise AssertionError(f"blocked report should include {phrase}")


def assert_blocks_visible_ad_hoc_without_acceptance() -> None:
  readiness = plan_readiness(
    tag="v0.1.0",
    source_commit=VALID_COMMIT,
    working_tree_clean_value=True,
    tag_points_at_commit_value=True,
    workflow_inputs_ready=True,
    ci_run_url=VALID_CI_URL,
    dry_run=False,
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=True,
    allow_untrusted_ad_hoc=True,
    manual_acceptance_confirmed=False,
    manual_acceptance_evidence="",
  )
  if readiness.ready:
    raise AssertionError("visible ad-hoc publish should require acceptance")
  if not any("manual_acceptance_confirmed=true" in blocker for blocker in readiness.blockers):
    raise AssertionError("visible ad-hoc blocker should name manual acceptance")


def assert_accepted_visible_ad_hoc_report_preserves_inputs() -> None:
  evidence = "https://github.com/walt1012/pith/actions/runs/100#manual-acceptance"
  readiness = plan_readiness(
    tag="v0.1.0",
    source_commit=VALID_COMMIT,
    working_tree_clean_value=True,
    tag_points_at_commit_value=True,
    workflow_inputs_ready=True,
    ci_run_url=VALID_CI_URL,
    dry_run=False,
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=True,
    allow_untrusted_ad_hoc=True,
    manual_acceptance_confirmed=True,
    manual_acceptance_evidence=evidence,
  )
  if not readiness.ready:
    raise AssertionError(f"accepted visible ad-hoc publish should be ready: {readiness.blockers}")

  report = readiness_report(readiness)
  for phrase in (
    "Workflow mode: `publish`",
    "Planned visibility: `visible prerelease`",
    "-f dry_run=false",
    "-f draft=false",
    "-f prerelease=true",
    "-f publish_untrusted_ad_hoc=true",
    "-f manual_acceptance_confirmed=true",
    f"-f manual_acceptance_evidence={evidence}",
  ):
    if phrase not in report:
      raise AssertionError(f"accepted publish report should include {phrase}")

  payload = readiness_json(readiness)
  expected_values = {
    "workflowMode": "publish",
    "allowUntrustedAdHoc": True,
    "manualAcceptanceConfirmed": True,
    "manualAcceptanceEvidence": evidence,
    "plannedDraft": False,
    "plannedPrerelease": True,
  }
  for key, expected in expected_values.items():
    if payload.get(key) != expected:
      raise AssertionError(f"accepted publish JSON {key} should be {expected!r}")


def assert_rejects_invalid_tag() -> None:
  readiness = plan_readiness(
    tag="latest",
    source_commit=VALID_COMMIT,
    working_tree_clean_value=True,
    tag_points_at_commit_value=True,
    workflow_inputs_ready=True,
    ci_run_url=VALID_CI_URL,
    dry_run=True,
    signing_mode="ad-hoc",
    requested_draft=True,
    requested_prerelease=True,
    allow_untrusted_ad_hoc=False,
    manual_acceptance_confirmed=False,
    manual_acceptance_evidence="",
  )
  if readiness.ready:
    raise AssertionError("invalid release tag should block readiness")


def main() -> int:
  assert_ready_dry_run_report()
  assert_blocks_missing_ci_and_tag()
  assert_blocks_visible_ad_hoc_without_acceptance()
  assert_accepted_visible_ad_hoc_report_preserves_inputs()
  assert_rejects_invalid_tag()
  print("release readiness tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
