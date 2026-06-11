#!/usr/bin/env python3
"""Unit checks for release readiness reporting."""

from __future__ import annotations

from release_readiness import plan_readiness
from release_readiness import readiness_checklist
from release_readiness import readiness_json
from release_readiness import readiness_report
from release_readiness import readiness_visibility_label
from release_readiness import REQUIRED_RELEASE_INPUTS


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
    "Release visibility: `not published; dry-run only`",
    "Tag points at source commit: `true`",
    "## Pre-Dispatch Checklist",
    "## Tag Preparation",
    f"git tag v0.1.0 {VALID_COMMIT}",
    "git push origin v0.1.0",
    "## Remote Tag Verification",
    "git ls-remote --exit-code --tags origin refs/tags/v0.1.0",
    "'refs/tags/v0.1.0^{}'",
    "tail -n 1",
    f'test "${{remote_tag_line%%[[:space:]]*}}" = {VALID_COMMIT}',
    "## CI Lookup",
    f"--commit {VALID_COMMIT}",
    f'select(.headSha == "{VALID_COMMIT}"',
    "## Expected Dry-Run Evidence",
    "Pith-v0.1.0-macos-x86_64.dmg",
    "release-dry-run-rehearsal.json",
    "gh workflow run release.yml",
    "-f dry_run=true",
    "## Dry-Run Artifact Verification",
    "python3 scripts/release_evidence_contract.py",
    "--mode dry-run",
    "release-dry-run-v0.1.0/release-dry-run-manual-acceptance.md",
    "Download the DMG, checksum, install guide, and manifest into one folder",
    "## Post-Acceptance Publish Command",
    "Use only after the generated manual acceptance receipt is filled and validated.",
    "-f dry_run=false",
    "-f draft=false",
    "-f publish_untrusted_ad_hoc=true",
    "-f manual_acceptance_confirmed=true",
    "-f manual_acceptance_evidence='<manual-acceptance-receipt-url>'",
    "release_workflow_run_id=\"$(",
    "gh api repos/:owner/:repo/actions/artifacts --paginate",
    'select(.name == "release-dry-run-v0.1.0"',
    ".expired == false",
    f'.workflow_run.head_sha == "{VALID_COMMIT}"',
    'gh run view "$release_workflow_run_id" --json conclusion --jq .conclusion',
    'test "$release_workflow_conclusion" = success',
    'gh run download "$release_workflow_run_id" --name release-dry-run-v0.1.0',
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
    "releaseVisibility": "not published; dry-run only",
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
  checklist = payload.get("preDispatchChecklist")
  if not isinstance(checklist, list) or len(checklist) < 5:
    raise AssertionError("readiness JSON should include the pre-dispatch checklist")
  if "Pith-v0.1.0-macos-x86_64.dmg" not in payload.get("expectedPublicAssets", []):
    raise AssertionError("readiness JSON should include expected public assets")
  if "release-dry-run-rehearsal.json" not in payload.get("expectedDryRunEvidence", []):
    raise AssertionError("readiness JSON should include expected dry-run evidence")
  if payload.get("tagCommands") != [
    f"git tag v0.1.0 {VALID_COMMIT}",
    "git push origin v0.1.0",
  ]:
    raise AssertionError("readiness JSON should include deterministic tag preparation commands")
  remote_tag_command = str(payload.get("remoteTagVerificationCommand", ""))
  for phrase in (
    "git ls-remote --exit-code --tags origin refs/tags/v0.1.0",
    "'refs/tags/v0.1.0^{}'",
    "tail -n 1",
    f'test "${{remote_tag_line%%[[:space:]]*}}" = {VALID_COMMIT}',
  ):
    if phrase not in remote_tag_command:
      raise AssertionError(f"readiness JSON remote tag verification should include {phrase}")
  ci_lookup = str(payload.get("successfulCiLookupCommand", ""))
  for phrase in (
    "gh run list",
    "--workflow CI",
    f"--commit {VALID_COMMIT}",
    f'select(.headSha == "{VALID_COMMIT}"',
  ):
    if phrase not in ci_lookup:
      raise AssertionError(f"readiness JSON CI lookup should include {phrase}")
  download_command = str(payload.get("dryRunArtifactDownloadCommand", ""))
  lookup_command = str(payload.get("dryRunArtifactLookupCommand", ""))
  for phrase in (
    "gh api repos/:owner/:repo/actions/artifacts --paginate",
    'select(.name == "release-dry-run-v0.1.0"',
    ".expired == false",
    f'.workflow_run.head_sha == "{VALID_COMMIT}"',
    "workflow_run.id",
    'test -n "$release_workflow_run_id"',
  ):
    if phrase not in lookup_command:
      raise AssertionError(f"readiness JSON dry-run lookup should include {phrase}")
  if 'gh run download "$release_workflow_run_id" --name release-dry-run-v0.1.0' not in download_command:
    raise AssertionError("readiness JSON should include the dry-run artifact download command")
  for phrase in (
    'gh run view "$release_workflow_run_id" --json conclusion --jq .conclusion',
    'test "$release_workflow_conclusion" = success',
  ):
    if phrase not in download_command:
      raise AssertionError(f"readiness JSON dry-run download should include {phrase}")
  validation_command = str(payload.get("dryRunEvidenceValidationCommand", ""))
  for phrase in (
    "python3 scripts/release_evidence_contract.py",
    "--mode dry-run",
    "--tag v0.1.0",
    "release-dry-run-v0.1.0/Pith-v0.1.0-macos-x86_64.dmg",
    "release-dry-run-v0.1.0/release-dry-run-manual-acceptance.md",
  ):
    if phrase not in validation_command:
      raise AssertionError(f"readiness JSON dry-run validation should include {phrase}")
  publish_command = str(payload.get("postAcceptancePublishCommand", ""))
  for phrase in (
    "gh workflow run release.yml",
    "-f dry_run=false",
    "-f draft=false",
    "-f prerelease=true",
    "-f publish_untrusted_ad_hoc=true",
    "-f manual_acceptance_confirmed=true",
    "-f manual_acceptance_evidence='<manual-acceptance-receipt-url>'",
  ):
    if phrase not in publish_command:
      raise AssertionError(f"readiness JSON post-acceptance publish command should include {phrase}")


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


def assert_tag_push_draft_release_does_not_claim_visible_release() -> None:
  readiness = plan_readiness(
    tag="v0.1.0",
    source_commit=VALID_COMMIT,
    working_tree_clean_value=True,
    tag_points_at_commit_value=True,
    workflow_inputs_ready=True,
    ci_run_url=VALID_CI_URL,
    dry_run=False,
    signing_mode="ad-hoc",
    requested_draft=True,
    requested_prerelease=True,
    allow_untrusted_ad_hoc=False,
    manual_acceptance_confirmed=False,
    manual_acceptance_evidence="",
  )
  if not readiness.ready:
    raise AssertionError(f"tag-push draft release should be ready: {readiness.blockers}")
  report = readiness_report(readiness)
  if "visible stable" in report:
    raise AssertionError("tag-push draft release report must not claim a visible stable release")
  expected_visibility = "draft prerelease"
  if readiness_visibility_label(readiness) != expected_visibility:
    raise AssertionError("tag-push visibility should describe a draft prerelease")
  if readiness_json(readiness).get("releaseVisibility") != expected_visibility:
    raise AssertionError("readiness JSON should expose draft prerelease visibility")


def assert_accepted_visible_ad_hoc_report_preserves_inputs() -> None:
  evidence = "https://github.com/walt1012/pith/issues/1#manual-acceptance-receipt"
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
    "Release visibility: `visible prerelease`",
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
    "releaseVisibility": "visible prerelease",
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


def assert_required_release_inputs_cover_dispatch_controls() -> None:
  for name in (
    "tag",
    "draft",
    "prerelease",
    "dry_run",
    "publish_untrusted_ad_hoc",
    "manual_acceptance_confirmed",
    "manual_acceptance_evidence",
  ):
    if name not in REQUIRED_RELEASE_INPUTS:
      raise AssertionError(f"release readiness should require dispatch input {name}")


def assert_readiness_checklist_names_release_candidate_flow() -> None:
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
  checklist = "\n".join(readiness_checklist(readiness))
  for phrase in (
    "Create tag v0.1.0",
    "Push the tag to origin",
    "tag-push release events create or update a draft prerelease",
    "remote tag verification command",
    "CI lookup command",
    "manual dry-run",
    "dry-run artifact lookup command",
    "release-dry-run-v0.1.0",
    "dry-run evidence validation command",
    "fresh-Mac manual acceptance",
    "validate the receipt",
    "post-acceptance publish command",
  ):
    if phrase not in checklist:
      raise AssertionError(f"readiness checklist should include {phrase}")


def main() -> int:
  assert_ready_dry_run_report()
  assert_blocks_missing_ci_and_tag()
  assert_blocks_visible_ad_hoc_without_acceptance()
  assert_tag_push_draft_release_does_not_claim_visible_release()
  assert_accepted_visible_ad_hoc_report_preserves_inputs()
  assert_rejects_invalid_tag()
  assert_required_release_inputs_cover_dispatch_controls()
  assert_readiness_checklist_names_release_candidate_flow()
  print("release readiness tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
