#!/usr/bin/env python3
"""Validate GitHub Actions workflow structure for fast, safe CI."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path


CI_WORKFLOW = ".github/workflows/ci.yml"
RELEASE_WORKFLOW = ".github/workflows/release.yml"
REQUIRED_CI_JOBS = (
  "changes",
  "repository-policy",
  "rust-format",
  "rust-clippy",
  "rust-test",
  "runtime-smoke",
  "model-catalog-remote",
  "swift-app",
  "swift-tests",
  "macos-runtime",
  "macos-llama-backend",
  "macos-package",
)
REQUIRED_CI_PACKAGE_ASSETS = (
  "artifacts/macos/${{ env.MACOS_DMG_NAME }}",
  "artifacts/macos/${{ env.MACOS_DMG_NAME }}.sha256",
  "artifacts/macos/README-FIRST.txt",
  "artifacts/macos/internal-release-manifest.json",
)
REQUIRED_CI_ARTIFACT_CONTRACT = (
  "SWIFT_APP_ARTIFACT: internal-PithApp-x86_64",
  "RUNTIME_ARTIFACT: internal-pith-runtime-bin-x86_64",
  "LLAMA_ARTIFACT: internal-llama-cli-x86_64",
  "MACOS_APP_ARTIFACT: Pith-installer-x86_64",
)
REQUIRED_RELEASE_ASSETS = (
  "artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg",
  "artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg.sha256",
  "artifacts/macos/README-FIRST.txt",
  "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json",
)


@dataclass(frozen=True)
class WorkflowIssue:
  path: str
  message: str


def validate_workflows(root: Path) -> list[WorkflowIssue]:
  issues: list[WorkflowIssue] = []
  workflow_texts = read_required_workflows(root, issues)
  for relative_path, text in workflow_texts.items():
    issues.extend(validate_common_workflow_rules(relative_path, text))

  ci_text = workflow_texts.get(CI_WORKFLOW)
  if ci_text is not None:
    issues.extend(validate_ci_workflow(ci_text))

  release_text = workflow_texts.get(RELEASE_WORKFLOW)
  if release_text is not None:
    issues.extend(validate_release_workflow(release_text))
  return issues


def read_required_workflows(
  root: Path,
  issues: list[WorkflowIssue],
) -> dict[str, str]:
  workflow_texts: dict[str, str] = {}
  for relative_path in (CI_WORKFLOW, RELEASE_WORKFLOW):
    path = root / relative_path
    if not path.exists():
      issues.append(WorkflowIssue(relative_path, "required workflow is missing"))
      continue
    workflow_texts[relative_path] = path.read_text(encoding="utf-8")
  return workflow_texts


def validate_common_workflow_rules(
  relative_path: str,
  text: str,
) -> list[WorkflowIssue]:
  issues: list[WorkflowIssue] = []
  for term in (
    "defaults:",
    "shell: bash",
    "concurrency:",
  ):
    if term not in text:
      issues.append(
        WorkflowIssue(relative_path, f"workflow contract is missing {term}")
      )

  blocks = step_blocks(text)
  for block in blocks:
    block_text = "\n".join(block)
    if (
      "uses: actions/checkout@" in block_text
      and "persist-credentials: false" not in block_text
    ):
      issues.append(
        WorkflowIssue(
          relative_path,
          "actions/checkout steps must set persist-credentials: false",
        )
      )
    if (
      "uses: actions/upload-artifact@" in block_text
      and "retention-days:" not in block_text
    ):
      issues.append(
        WorkflowIssue(
          relative_path,
          "actions/upload-artifact steps must set retention-days",
        )
      )

  for job_name, block in job_blocks(text).items():
    if "timeout-minutes:" not in block:
      issues.append(
        WorkflowIssue(relative_path, f"job {job_name} must set timeout-minutes")
      )
  return issues


def validate_ci_workflow(text: str) -> list[WorkflowIssue]:
  issues: list[WorkflowIssue] = []
  for term in (
    "permissions:",
    "actions: read",
    "contents: read",
    "cancel-in-progress: true",
  ):
    if term not in text:
      issues.append(WorkflowIssue(CI_WORKFLOW, f"CI workflow contract is missing {term}"))

  for job in REQUIRED_CI_JOBS:
    if job not in job_blocks(text):
      issues.append(WorkflowIssue(CI_WORKFLOW, f"required CI job {job} is missing"))

  for term in REQUIRED_CI_ARTIFACT_CONTRACT:
    if term not in text:
      issues.append(WorkflowIssue(CI_WORKFLOW, f"CI artifact contract is missing {term}"))

  issues.extend(validate_ci_artifact_uploads(text))

  repository_policy_block = job_block(text, "repository-policy")
  if repository_policy_block:
    required_policy_commands = (
      "python3 scripts/validate_model_pack.py",
      "python3 scripts/check_english_policy.py",
      "python3 scripts/test_package_macos_app.py",
      "python3 scripts/test_ci_changes.py",
      "python3 scripts/validate_workflows.py",
      "python3 scripts/test_validate_workflows.py",
      "python3 scripts/test_create_macos_dmg.py",
      "python3 scripts/test_release_state.py",
      "python3 scripts/test_release_readiness.py",
      "python3 scripts/test_release_publish_contract.py",
      "python3 scripts/test_release_rehearsal_contract.py",
      "python3 scripts/test_release_evidence_contract.py",
      "python3 scripts/test_package_contract.py",
      "python3 scripts/test_release_identity.py",
      "python3 scripts/test_sign_macos_app_for_distribution.py",
      "python3 scripts/test_release_artifacts.py",
      "python3 scripts/test_installer_artifact_contract.py",
      "python3 scripts/test_manual_acceptance_contract.py",
      "python3 scripts/test_release_text.py",
      "python3 scripts/test_first_app_open_contract.py",
      "python3 scripts/test_smoke_launch_macos_app.py",
      "python3 scripts/test_connector_workflow_contracts.py",
      "python3 scripts/test_notion_connector_contract.py",
      "python3 scripts/test_validate_macos_distribution.py",
    )
    for term in required_policy_commands:
      if term not in repository_policy_block:
        issues.append(
          WorkflowIssue(
            CI_WORKFLOW,
            f"repository-policy is missing {term}",
          )
        )

  swift_app_block = job_block(text, "swift-app")
  if swift_app_block:
    for term in (
      'id: swift_app_binary_cache',
      'uses: actions/cache/restore@v5',
      'key: swift-app-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles',
      'Use cached Swift app executable',
    ):
      if term not in swift_app_block:
        issues.append(
          WorkflowIssue(CI_WORKFLOW, f"swift-app cached executable path is missing {term}")
        )

  macos_runtime_block = job_block(text, "macos-runtime")
  if macos_runtime_block:
    for term in (
      'id: runtime_binary_cache',
      'uses: actions/cache/restore@v5',
      'key: runtime-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles',
      'Use cached runtime executable',
    ):
      if term not in macos_runtime_block:
        issues.append(
          WorkflowIssue(CI_WORKFLOW, f"macos-runtime cached executable path is missing {term}")
        )

  package_block = job_block(text, "macos-package")
  if package_block:
    package_needs = job_needs(package_block)
    if package_needs != ["changes"]:
      issues.append(
        WorkflowIssue(
          CI_WORKFLOW,
          "macos-package must depend only on changes; validation lanes must not sit on the installer critical path",
        )
      )
    if re.search(r"(?m)^\s+-\s+swift-tests\s*$", package_block):
      issues.append(
        WorkflowIssue(
          CI_WORKFLOW,
          "macos-package must not wait for swift-tests before artifact assembly",
        )
      )
    if re.search(r"(?m)^\s+-\s+swift-app\s*$", package_block):
      issues.append(
        WorkflowIssue(
          CI_WORKFLOW,
          "macos-package must restore or build the Swift executable directly instead of waiting for the Swift artifact",
        )
      )
    if re.search(r"(?m)^\s+-\s+macos-runtime\s*$", package_block):
      issues.append(
        WorkflowIssue(
          CI_WORKFLOW,
          "macos-package must restore or build the runtime executable directly instead of waiting for the runtime artifact",
        )
      )
    if re.search(r"(?m)^\s+-\s+macos-llama-backend\s*$", package_block):
      issues.append(
        WorkflowIssue(
          CI_WORKFLOW,
          "macos-package must restore cached llama.cpp directly instead of waiting for the llama backend artifact",
        )
      )
    for asset in REQUIRED_CI_PACKAGE_ASSETS:
      if asset not in package_block:
        issues.append(
          WorkflowIssue(CI_WORKFLOW, f"macos-package upload is missing {asset}")
        )
    if "--source-commit" not in package_block:
      issues.append(
        WorkflowIssue(
          CI_WORKFLOW,
          "macos-package release manifest must include --source-commit",
        )
      )
    if "--no-zip" not in package_block:
      issues.append(
        WorkflowIssue(
          CI_WORKFLOW,
          "macos-package installer path must pass --no-zip",
        )
      )
    required_package_terms = (
      'python3 scripts/package_contract.py',
      'id: package_swift_cache',
      'key: swift-app-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles',
      'id: package_runtime_cache',
      'key: runtime-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles',
      'Build missing package executables',
      'id: package_llama_cache',
      'key: llama-backend-${{ runner.os }}-${{ runner.arch }}-${{ env.LLAMA_CPP_REF }}-v1',
      'Build pinned llama.cpp backend',
      'Validate packaged llama.cpp backend',
      '--tag "ci-${GITHUB_SHA::12}"',
      '--signing-mode ad-hoc',
      '--install-guide artifacts/macos/README-FIRST.txt',
      '--smoke-receipt-output artifacts/macos/packaged-smoke-receipt.json',
      '--package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json',
      '--smoke-receipt artifacts/macos/packaged-smoke-receipt.json',
      '--workflow-run-id "$GITHUB_RUN_ID"',
      '--workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"',
      '--manifest-output artifacts/macos/internal-release-manifest.json',
      'python3 scripts/installer_artifact_contract.py',
      '--asset "artifacts/macos/$MACOS_DMG_NAME"',
      '--asset "artifacts/macos/$MACOS_DMG_NAME.sha256"',
      '--asset artifacts/macos/README-FIRST.txt',
      '--asset artifacts/macos/internal-release-manifest.json',
      'python3 scripts/release_rehearsal_contract.py',
      '--asset-dir artifacts/macos',
      '--allow-extra-assets',
      '--summary-output artifacts/macos/internal-release-rehearsal.md',
      'cat artifacts/macos/internal-release-rehearsal.md >> "$GITHUB_STEP_SUMMARY"',
    )
    for term in required_package_terms:
      if term not in package_block:
        issues.append(
          WorkflowIssue(
            CI_WORKFLOW,
            f"macos-package release manifest is missing {term}",
          )
        )
  return issues


def validate_ci_artifact_uploads(text: str) -> list[WorkflowIssue]:
  issues: list[WorkflowIssue] = []
  for block in step_blocks(text):
    block_text = "\n".join(block)
    if "uses: actions/upload-artifact@" not in block_text:
      continue
    if "${{ env.SWIFT_APP_ARTIFACT }}" in block_text:
      validate_upload_retention(block_text, "Swift app internal artifact", "1", issues)
    if "${{ env.RUNTIME_ARTIFACT }}" in block_text:
      validate_upload_retention(block_text, "runtime internal artifact", "1", issues)
    if "${{ env.LLAMA_ARTIFACT }}" in block_text:
      validate_upload_retention(block_text, "llama internal artifact", "1", issues)
    if "${{ env.MACOS_APP_ARTIFACT }}" in block_text:
      validate_upload_retention(block_text, "macOS installer artifact", "7", issues)
      if "artifacts/macos/Pith-macos-x86_64.zip" in block_text:
        issues.append(
          WorkflowIssue(
            CI_WORKFLOW,
            "macOS installer artifact must not upload the internal zip bundle",
          )
        )
      if "artifacts/macos/internal-release-notes.md" in block_text:
        issues.append(
          WorkflowIssue(
            CI_WORKFLOW,
            "macOS installer artifact must not upload internal release notes",
          )
        )
  return issues


def validate_upload_retention(
  block_text: str,
  label: str,
  expected_days: str,
  issues: list[WorkflowIssue],
) -> None:
  expected_line = f"retention-days: {expected_days}"
  if expected_line not in block_text:
    issues.append(
      WorkflowIssue(
        CI_WORKFLOW,
        f"{label} must use retention-days: {expected_days}",
      )
    )


def validate_release_workflow(text: str) -> list[WorkflowIssue]:
  issues: list[WorkflowIssue] = []
  for term in (
    "permissions:",
    "actions: read",
    "contents: write",
    "cancel-in-progress: false",
    "publish_untrusted_ad_hoc:",
    "manual_acceptance_confirmed:",
    "manual_acceptance_evidence:",
    "dry_run:",
    "RELEASE_DRAFT:",
    "RELEASE_DRAFT: ${{ github.event_name == 'workflow_dispatch' && inputs.draft || false }}",
    "RELEASE_PRERELEASE:",
    "RELEASE_PRERELEASE: ${{ github.event_name == 'workflow_dispatch' && inputs.prerelease || false }}",
    "RELEASE_ALLOW_UNTRUSTED_AD_HOC:",
    "RELEASE_MANUAL_ACCEPTANCE_CONFIRMED:",
    "RELEASE_MANUAL_ACCEPTANCE_EVIDENCE:",
    "RELEASE_DRY_RUN:",
    "RELEASE_DRY_RUN: ${{ github.event_name != 'workflow_dispatch' || inputs.dry_run }}",
  ):
    if term not in text:
      issues.append(
        WorkflowIssue(RELEASE_WORKFLOW, f"release workflow contract is missing {term}")
      )

  issues.extend(validate_release_dispatch_inputs(text))

  release_block = job_block(text, "release-dmg")
  if not release_block:
    return [WorkflowIssue(RELEASE_WORKFLOW, "required release-dmg job is missing")]

  required_gate_terms = (
    "gh run list",
    "--workflow CI",
    "--status success",
    "--json conclusion,headSha,url",
    'git fetch --depth 1 origin "refs/tags/$RELEASE_TAG:refs/tags/$RELEASE_TAG"',
    "PITH_RELEASE_CI_RUN_URL",
    r"^v[0-9]+\.[0-9]+\.[0-9]+$",
    "python3 scripts/validate_model_pack.py --remote",
  )
  for term in required_gate_terms:
    if term not in release_block:
      issues.append(
        WorkflowIssue(RELEASE_WORKFLOW, f"release CI gate is missing {term}")
      )

  if "shasum -a 256" in release_block:
    issues.append(
      WorkflowIssue(
        RELEASE_WORKFLOW,
        "release checksum must use scripts/release_artifacts.py",
      )
    )
  if "scripts/release_state.py" not in release_block:
    issues.append(
      WorkflowIssue(RELEASE_WORKFLOW, "release publish state helper is missing")
    )
  elif not command_window_contains(
    release_block,
    "scripts/release_state.py",
    '--tag "$RELEASE_TAG"',
  ):
    issues.append(
      WorkflowIssue(RELEASE_WORKFLOW, "release state helper must receive --tag")
    )
  if 'release_title="Pith $RELEASE_TAG"' not in release_block:
    issues.append(
      WorkflowIssue(
        RELEASE_WORKFLOW,
        "release workflow must derive the GitHub Release title from the tag",
      )
    )
  elif not command_window_contains(
    release_block,
    "scripts/release_state.py",
    '--title "$release_title"',
  ):
    issues.append(
      WorkflowIssue(RELEASE_WORKFLOW, "release state helper must receive --title")
    )
  validate_release_shared_input_contract(release_block, issues)
  validate_release_evidence_step_contracts(release_block, issues)
  for term in (
    "Plan GitHub Release state",
    'cat release-state.env >> "$GITHUB_ENV"',
    '--summary-output release-plan.md',
    '--plan-output release-plan.json',
    '--source-commit "$PITH_RELEASE_SHA"',
    '--ci-run-url "$PITH_RELEASE_CI_RUN_URL"',
    '--workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"',
    '--dry-run "$RELEASE_DRY_RUN"',
    '--manual-acceptance-confirmed "$RELEASE_MANUAL_ACCEPTANCE_CONFIRMED"',
    '--manual-acceptance-evidence "$RELEASE_MANUAL_ACCEPTANCE_EVIDENCE"',
    'cat release-plan.md >> "$GITHUB_STEP_SUMMARY"',
    "Write release readiness report",
    'python3 scripts/release_readiness.py',
    '--output release-readiness.md',
    '--json-output release-readiness.json',
    '--ci-run-url "$PITH_RELEASE_CI_RUN_URL"',
    'cat release-readiness.md >> "$GITHUB_STEP_SUMMARY"',
    'printf \'%s\' "$release_json" > release-existing.json',
    '--mode preupload-existing-assets',
    '--release-json release-existing.json',
    "Upload GitHub Release draft assets",
    "Rehearse downloaded GitHub Release assets",
    "Apply final GitHub Release visibility",
    "Validate final GitHub Release",
    "Rehearse release dry-run assets",
    "Upload release dry-run assets",
    "release-dry-run-${{ env.RELEASE_TAG }}",
    "if: env.RELEASE_DRY_RUN == 'true'",
    "release-dry-run-assets",
    "--asset-dir release-dry-run-assets",
    "--summary-output release-dry-run-rehearsal.md",
    "--acceptance-output release-dry-run-manual-acceptance.md",
    "--json-output release-dry-run-rehearsal.json",
    'cat release-dry-run-rehearsal.md >> "$GITHUB_STEP_SUMMARY"',
    'cat release-dry-run-manual-acceptance.md >> "$GITHUB_STEP_SUMMARY"',
    "Validate release dry-run evidence",
    "python3 scripts/release_evidence_contract.py",
    "--mode dry-run",
    "artifacts/macos/Pith-${{ env.RELEASE_TAG }}-macos-x86_64.dmg",
    "artifacts/macos/Pith-${{ env.RELEASE_TAG }}-macos-x86_64.dmg.sha256",
    "artifacts/macos/Pith-${{ env.RELEASE_TAG }}-release-manifest.json",
    "release-readiness.md",
    "release-readiness.json",
    "release-plan.md",
    "release-plan.json",
    "release-dry-run-rehearsal.md",
    "release-dry-run-rehearsal.json",
    "release-dry-run-manual-acceptance.md",
  ):
    if term not in release_block:
      issues.append(
        WorkflowIssue(
          RELEASE_WORKFLOW,
          f"release workflow stage boundary is missing {term}",
        )
      )
  if "scripts/release_publish_contract.py" not in release_block:
    issues.append(
      WorkflowIssue(
        RELEASE_WORKFLOW,
        "published release contract helper is missing",
      )
    )
  for term in (
    "Upload GitHub Release draft assets\n        if: env.RELEASE_DRY_RUN != 'true'",
    "Rehearse downloaded GitHub Release assets\n        if: env.RELEASE_DRY_RUN != 'true'",
    "Apply final GitHub Release visibility\n        if: env.RELEASE_DRY_RUN != 'true'",
    "Validate final GitHub Release\n        if: env.RELEASE_DRY_RUN != 'true'",
    "Validate release rehearsal evidence\n        if: env.RELEASE_DRY_RUN != 'true'",
    "Upload release rehearsal summary\n        if: always() && env.RELEASE_DRY_RUN != 'true'",
    "-X PATCH",
    "--input release-state.json",
  ):
    if term not in release_block:
      issues.append(
        WorkflowIssue(
          RELEASE_WORKFLOW,
          f"final release state patch is missing {term}",
        )
      )
  for term in (
    'gh api "repos/$GITHUB_REPOSITORY/releases/tags/$RELEASE_TAG" > release-published.json',
    'release_tag_commit="$(',
    "git ls-remote --exit-code --tags origin",
    '"refs/tags/$RELEASE_TAG" "refs/tags/$RELEASE_TAG^{}"',
    '--release-json release-published.json',
    '--source-commit "$SOURCE_COMMIT"',
    '--tag-commit "$release_tag_commit"',
    '--expected-draft "$PITH_RELEASE_STATE_DRAFT"',
    '--expected-prerelease "$PITH_RELEASE_STATE_PRERELEASE"',
    '--signing-mode "$PITH_RELEASE_SIGNING_MODE"',
    '--allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"',
  ):
    if term not in release_block:
      issues.append(
        WorkflowIssue(
          RELEASE_WORKFLOW,
          f"published release contract is missing {term}",
        )
      )
  if "scripts/release_rehearsal_contract.py" not in release_block:
    issues.append(
      WorkflowIssue(
        RELEASE_WORKFLOW,
        "release download rehearsal helper is missing",
      )
    )
  require_release_order(
    release_block,
    "scripts/release_state.py",
    "gh release upload",
    "release state planning must pass before assets are uploaded",
    issues,
  )
  require_release_order(
    release_block,
    "Write release readiness report",
    "Audit remote model catalog metadata",
    "release readiness must be written before remote model audit can fail the job",
    issues,
  )
  require_release_order(
    release_block,
    "Write release readiness report",
    "Build Swift app executable",
    "release readiness must be written before expensive release builds",
    issues,
  )
  require_release_order(
    release_block,
    "gh release upload",
    'gh release download "$RELEASE_TAG"',
    "release assets must be uploaded before downloaded rehearsal",
    issues,
  )
  require_release_order(
    release_block,
    "Rehearse release dry-run assets",
    "Upload release dry-run assets",
    "release dry-run rehearsal must pass before dry-run assets are uploaded",
    issues,
  )
  require_release_order(
    release_block,
    "Validate release dry-run evidence",
    "Upload release dry-run assets",
    "release dry-run evidence validation must pass before dry-run assets are uploaded",
    issues,
  )
  require_release_order(
    release_block,
    'gh release download "$RELEASE_TAG"',
    "--asset-dir release-download",
    "release assets must be downloaded before rehearsal validation",
    issues,
  )
  require_release_order(
    release_block,
    "--asset-dir release-download",
    "Validate release rehearsal evidence",
    "release download rehearsal must pass before publish rehearsal evidence validation",
    issues,
  )
  require_release_order(
    release_block,
    "Validate release rehearsal evidence",
    "-X PATCH",
    "publish rehearsal evidence validation must pass before final release state patch",
    issues,
  )
  require_release_order(
    release_block,
    "-X PATCH",
    "--release-json release-published.json",
    "published release validation must run after final release state patch",
    issues,
  )
  if "--allow-extra-assets" in release_block:
    issues.append(
      WorkflowIssue(
        RELEASE_WORKFLOW,
        "release download rehearsal must not allow extra assets",
      )
    )
  for term in (
    'gh release download "$RELEASE_TAG"',
    "--dir release-download",
    "--asset-dir release-download",
    "--summary-output release-rehearsal.md",
    "--acceptance-output release-manual-acceptance.md",
    "--json-output release-rehearsal.json",
    'cat release-rehearsal.md >> "$GITHUB_STEP_SUMMARY"',
    'cat release-manual-acceptance.md >> "$GITHUB_STEP_SUMMARY"',
    "--mode publish-rehearsal",
    "uses: actions/upload-artifact@v7",
    "release-rehearsal-${{ env.RELEASE_TAG }}",
    "release-plan.md",
    "release-plan.json",
    "release-rehearsal.json",
    "release-manual-acceptance.md",
    "retention-days: 30",
  ):
    if term not in release_block:
      issues.append(
        WorkflowIssue(
          RELEASE_WORKFLOW,
          f"release download rehearsal is missing {term}",
        )
      )
  if "--source-commit" not in release_block:
    issues.append(
      WorkflowIssue(
        RELEASE_WORKFLOW,
        "release manifest must include --source-commit",
      )
    )
  required_release_terms = (
    'python3 scripts/package_contract.py',
    'python3 scripts/sign_macos_app_for_distribution.py',
    '--identity "$MACOS_DEVELOPER_ID_APPLICATION"',
    'python3 scripts/validate_macos_distribution.py',
    'xcrun notarytool submit "$dmg_path"',
    '--apple-id "$APPLE_ID"',
    '--team-id "$APPLE_TEAM_ID"',
    '--password "$APPLE_APP_SPECIFIC_PASSWORD"',
    '--wait',
    'xcrun stapler staple "$dmg_path"',
    '--dmg-path "$dmg_path"',
    '--tag "$RELEASE_TAG"',
    '--signing-mode "$PITH_RELEASE_SIGNING_MODE"',
    '--install-guide artifacts/macos/README-FIRST.txt',
    '--smoke-receipt-output artifacts/macos/packaged-smoke-receipt.json',
    '--package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json',
    '--smoke-receipt artifacts/macos/packaged-smoke-receipt.json',
    '--workflow-run-id "$GITHUB_RUN_ID"',
    '--workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"',
    '--manifest-output "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json"',
    'python3 scripts/installer_artifact_contract.py',
    '--asset "$dmg_path"',
    '--asset "$dmg_path.sha256"',
    '--asset artifacts/macos/README-FIRST.txt',
    '--asset "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json"',
  )
  for term in required_release_terms:
    if term not in release_block:
      issues.append(
        WorkflowIssue(
          RELEASE_WORKFLOW,
          f"release manifest is missing {term}",
        )
      )
  for asset in REQUIRED_RELEASE_ASSETS:
    if asset not in release_block:
      issues.append(
        WorkflowIssue(RELEASE_WORKFLOW, f"release upload is missing {asset}")
      )
  return issues


def validate_release_shared_input_contract(
  release_block: str,
  issues: list[WorkflowIssue],
) -> None:
  shared_inputs = (
    '--dry-run "$RELEASE_DRY_RUN"',
    '--signing-mode "$PITH_RELEASE_SIGNING_MODE"',
    '--requested-draft "$RELEASE_DRAFT"',
    '--requested-prerelease "$RELEASE_PRERELEASE"',
    '--allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"',
    '--manual-acceptance-confirmed "$RELEASE_MANUAL_ACCEPTANCE_CONFIRMED"',
    '--manual-acceptance-evidence "$RELEASE_MANUAL_ACCEPTANCE_EVIDENCE"',
  )
  for anchor, label in (
    ("scripts/release_readiness.py", "release readiness"),
    ("scripts/release_state.py", "release state"),
  ):
    step_text = step_text_containing(release_block, anchor)
    for term in shared_inputs:
      if term not in step_text:
        issues.append(
          WorkflowIssue(
            RELEASE_WORKFLOW,
            f"{label} helper must receive shared release input {term}",
          )
        )


def validate_release_evidence_step_contracts(
  release_block: str,
  issues: list[WorkflowIssue],
) -> None:
  expected_by_step = (
    (
      "Validate release dry-run evidence",
      "dry-run evidence validation",
      (
        "--mode dry-run",
        '--tag "$RELEASE_TAG"',
        'artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg',
        'artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg.sha256',
        "artifacts/macos/README-FIRST.txt",
        'artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json',
        "release-readiness.md",
        "release-readiness.json",
        "release-plan.md",
        "release-plan.json",
        "release-dry-run-rehearsal.md",
        "release-dry-run-rehearsal.json",
        "release-dry-run-manual-acceptance.md",
      ),
    ),
    (
      "Validate release rehearsal evidence",
      "publish rehearsal evidence validation",
      (
        "--mode publish-rehearsal",
        '--tag "$RELEASE_TAG"',
        "release-readiness.md",
        "release-readiness.json",
        "release-plan.md",
        "release-plan.json",
        "release-rehearsal.md",
        "release-rehearsal.json",
        "release-manual-acceptance.md",
      ),
    ),
  )
  for step_name, label, expected_terms in expected_by_step:
    step_text = step_text_containing(release_block, step_name)
    for term in expected_terms:
      if term not in step_text:
        issues.append(
          WorkflowIssue(
            RELEASE_WORKFLOW,
            f"{label} step is missing {term}",
          )
        )


def validate_release_dispatch_inputs(text: str) -> list[WorkflowIssue]:
  issues: list[WorkflowIssue] = []
  required_inputs = {
    "tag": ("type: string",),
    "draft": ("default: true", "type: boolean"),
    "prerelease": ("default: true", "type: boolean"),
    "publish_untrusted_ad_hoc": ("default: false", "type: boolean"),
    "manual_acceptance_confirmed": (
      "validated manual acceptance receipt",
      "default: false",
      "type: boolean",
    ),
    "manual_acceptance_evidence": (
      "HTTPS URL for the validated manual acceptance receipt",
      'default: ""',
      "type: string",
    ),
    "dry_run": ("default: true", "type: boolean"),
  }
  for name, required_terms in required_inputs.items():
    block = workflow_dispatch_input_block(text, name)
    if not block:
      issues.append(
        WorkflowIssue(
          RELEASE_WORKFLOW,
          f"release workflow dispatch input {name} is missing",
        )
      )
      continue
    for term in required_terms:
      if term not in block:
        issues.append(
          WorkflowIssue(
            RELEASE_WORKFLOW,
            f"release workflow dispatch input {name} must include {term}",
          )
        )
  return issues


def workflow_dispatch_input_block(text: str, input_name: str) -> str:
  pattern = re.compile(rf"^\s{{6}}{re.escape(input_name)}:\s*$", re.MULTILINE)
  match = pattern.search(text)
  if not match:
    return ""
  next_match = re.search(r"^\s{6}[A-Za-z0-9_]+:\s*$", text[match.end():], re.MULTILINE)
  if next_match:
    return text[match.start():match.end() + next_match.start()]
  return text[match.start():]


def require_release_order(
  text: str,
  earlier: str,
  later: str,
  message: str,
  issues: list[WorkflowIssue],
) -> None:
  earlier_index = text.find(earlier)
  later_index = text.find(later)
  if earlier_index == -1 or later_index == -1:
    return
  if earlier_index > later_index:
    issues.append(WorkflowIssue(RELEASE_WORKFLOW, message))


def step_text_containing(text: str, anchor: str) -> str:
  anchor_index = text.find(anchor)
  if anchor_index == -1:
    return ""
  step_start = text.rfind("\n      - name:", 0, anchor_index)
  if step_start == -1:
    step_start = 0
  else:
    step_start += 1
  step_end = text.find("\n      - name:", anchor_index)
  if step_end == -1:
    step_end = len(text)
  return text[step_start:step_end]


def step_blocks(text: str) -> list[list[str]]:
  blocks: list[list[str]] = []
  current: list[str] = []
  for line in text.splitlines():
    if re.match(r"^\s{6}-\s+name:", line):
      if current:
        blocks.append(current)
      current = [line]
    elif current:
      current.append(line)
  if current:
    blocks.append(current)
  return blocks


def job_blocks(text: str) -> dict[str, str]:
  blocks: dict[str, str] = {}
  current_name: str | None = None
  current_lines: list[str] = []
  in_jobs = False
  for line in text.splitlines():
    if line == "jobs:":
      in_jobs = True
      continue
    if not in_jobs:
      continue
    if line and not line.startswith(" "):
      break
    match = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
    if match:
      if current_name is not None:
        blocks[current_name] = "\n".join(current_lines)
      current_name = match.group(1)
      current_lines = [line]
    elif current_name is not None:
      current_lines.append(line)
  if current_name is not None:
    blocks[current_name] = "\n".join(current_lines)
  return blocks


def job_block(text: str, job_name: str) -> str:
  return job_blocks(text).get(job_name, "")


def job_needs(block: str) -> list[str]:
  needs: list[str] = []
  in_needs = False
  for line in block.splitlines():
    if re.match(r"^\s{4}needs:\s*$", line):
      in_needs = True
      continue
    if not in_needs:
      continue
    match = re.match(r"^\s{6}-\s+([A-Za-z0-9_-]+)\s*$", line)
    if match:
      needs.append(match.group(1))
      continue
    if line.startswith("    ") and not line.startswith("      "):
      break
  return needs


def command_window_contains(
  text: str,
  anchor: str,
  term: str,
  *,
  span: int = 260,
) -> bool:
  start = text.find(anchor)
  if start == -1:
    return False
  return term in text[start:start + span]


def main() -> int:
  root = Path(__file__).resolve().parents[1]
  issues = validate_workflows(root)
  if issues:
    for issue in issues:
      print(f"{issue.path}: {issue.message}")
    return 1
  print("Workflow structure validation passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
