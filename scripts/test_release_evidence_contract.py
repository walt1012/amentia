#!/usr/bin/env python3
"""Unit checks for internal release evidence artifact validation."""

from __future__ import annotations

import json
from pathlib import Path
from tempfile import TemporaryDirectory

from package_contract import DEFAULT_MODEL_ID
from release_artifacts import release_installer_asset_names
from release_artifacts import write_checksum_file
from release_artifacts import write_release_manifest
from release_evidence_contract import expected_evidence_names
from release_evidence_contract import validate_release_evidence_set
from release_text import install_guide as release_install_guide


TAG = "v0.1.0"
SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
WORKFLOW_RUN_ID = "123456789"
WORKFLOW_RUN_URL = "https://github.com/walt1012/pith/actions/runs/123456789"


def write_evidence_file(path: Path, mode: str = "dry-run") -> None:
  if path.suffix == ".json":
    payload = {"result": "passed"}
    if path.name == "release-readiness.json":
      payload = release_readiness_payload(mode)
    elif path.name == "release-plan.json":
      payload = release_plan_payload(mode)
    elif path.name in {"release-dry-run-rehearsal.json", "release-rehearsal.json"}:
      payload = release_rehearsal_payload()
    path.write_text(json.dumps(payload) + "\n", encoding="utf-8")
  elif path.suffix == ".md":
    if path.name in {"release-dry-run-manual-acceptance.md", "release-manual-acceptance.md"}:
      path.write_text(manual_acceptance_markdown() + "\n", encoding="utf-8")
    elif path.name in {"release-dry-run-rehearsal.md", "release-rehearsal.md"}:
      path.write_text(release_rehearsal_markdown() + "\n", encoding="utf-8")
    else:
      path.write_text(f"# {path.stem}\n\nEvidence.\n", encoding="utf-8")
  else:
    path.write_bytes(b"release asset\n")


def release_readiness_payload(mode: str = "dry-run") -> dict[str, object]:
  workflow_mode = "dry-run" if mode == "dry-run" else "publish"
  return {
    "status": "ready",
    "tag": TAG,
    "sourceCommit": SOURCE_COMMIT,
    "successfulCiRunUrl": "https://github.com/walt1012/pith/actions/runs/1",
    "workflowMode": workflow_mode,
    "signingMode": "ad-hoc",
    "requestedDraft": True,
    "requestedPrerelease": True,
    "allowUntrustedAdHoc": False,
    "plannedDraft": True,
    "plannedPrerelease": True,
    "workingTreeClean": True,
    "tagPointsAtSourceCommit": True,
    "releaseWorkflowInputsReady": True,
    "manualAcceptanceConfirmed": False,
    "manualAcceptanceEvidence": "",
    "preDispatchChecklist": pre_dispatch_checklist(),
    "expectedPublicAssets": list(release_installer_asset_names(TAG)),
    "expectedDryRunEvidence": list(expected_evidence_names("dry-run", TAG)),
    "tagCommands": release_tag_commands(),
    "remoteTagVerificationCommand": remote_tag_verification_command(),
    "successfulCiLookupCommand": successful_ci_lookup_command(),
    "dryRunArtifactLookupCommand": dry_run_artifact_lookup_command(),
    "dryRunArtifactDownloadCommand": dry_run_artifact_download_command(),
    "dryRunEvidenceValidationCommand": dry_run_evidence_validation_command(),
    "postAcceptancePublishCommand": post_acceptance_publish_command(),
    "blockers": [],
    "nextCommand": release_next_command(workflow_mode),
  }


def pre_dispatch_checklist() -> list[str]:
  return [
    (
      f"Create tag {TAG} at source commit {SOURCE_COMMIT} "
      "if it does not already exist."
    ),
    f"Confirm tag {TAG} points at source commit {SOURCE_COMMIT}.",
    "Push the tag to origin; tag-push release events run as dry-run by default.",
    (
      "Run the remote tag verification command before dispatching "
      "a manual release workflow."
    ),
    (
      "Use the CI lookup command to copy the successful CI URL "
      "for this exact source commit."
    ),
    (
      "Confirm the successful CI run matches the source commit: "
      "https://github.com/walt1012/pith/actions/runs/1."
    ),
    "Run the release workflow as a dry-run before any publish attempt.",
    (
      "Use the dry-run artifact lookup command to find the "
      f"release-dry-run-{TAG} workflow run."
    ),
    (
      f"Download the release-dry-run-{TAG} workflow artifact "
      "after the dry-run passes."
    ),
    "Run the dry-run evidence validation command before manual acceptance.",
    (
      "Verify the DMG checksum, release manifest, release plan, "
      "rehearsal summary, and manual acceptance checklist."
    ),
    "Complete fresh-Mac manual acceptance before any visible ad-hoc prerelease.",
    (
      "Use the post-acceptance publish command only after manual acceptance "
      "evidence is recorded."
    ),
  ]


def release_tag_commands() -> list[str]:
  return [
    f"git tag {TAG} {SOURCE_COMMIT}",
    f"git push origin {TAG}",
  ]


def remote_tag_verification_command() -> str:
  tag_ref = f"refs/tags/{TAG}"
  return "\n".join(
    [
      'remote_tag_line="$(',
      "  git ls-remote --exit-code --tags origin "
      f"{tag_ref} '{tag_ref}^{{}}' | tail -n 1",
      ')"',
      f'test "${{remote_tag_line%%[[:space:]]*}}" = {SOURCE_COMMIT}',
    ]
  )


def successful_ci_lookup_command() -> str:
  return (
    "gh run list "
    "--workflow CI "
    f"--commit {SOURCE_COMMIT} "
    "--status success "
    "--json conclusion,headSha,url "
    "--limit 20 "
    "--jq "
    + "'"
    + (
      f'[.[] | select(.headSha == "{SOURCE_COMMIT}" '
      'and .conclusion == "success")][0].url // ""'
    )
    + "'"
  )


def developer_id_readiness_payload(mode: str = "dry-run") -> dict[str, object]:
  payload = release_readiness_payload(mode)
  payload["signingMode"] = "developer-id"
  payload["postAcceptancePublishCommand"] = developer_id_publish_command()
  return payload


def release_next_command(workflow_mode: str) -> str:
  return "\n".join(
    [
      "gh workflow run release.yml \\",
      f"  -f tag={TAG} \\",
      f"  -f dry_run={'true' if workflow_mode == 'dry-run' else 'false'} \\",
      "  -f draft=true \\",
      "  -f prerelease=true \\",
      "  -f publish_untrusted_ad_hoc=false \\",
      "  -f manual_acceptance_confirmed=false \\",
      "  -f manual_acceptance_evidence=",
    ]
  )


def post_acceptance_publish_command() -> str:
  return "\n".join(
    [
      "gh workflow run release.yml \\",
      f"  -f tag={TAG} \\",
      "  -f dry_run=false \\",
      "  -f draft=false \\",
      "  -f prerelease=true \\",
      "  -f publish_untrusted_ad_hoc=true \\",
      "  -f manual_acceptance_confirmed=true \\",
      "  -f manual_acceptance_evidence='<manual-acceptance-receipt-url>'",
    ]
  )


def developer_id_publish_command() -> str:
  return "\n".join(
    [
      "gh workflow run release.yml \\",
      f"  -f tag={TAG} \\",
      "  -f dry_run=false \\",
      "  -f draft=false \\",
      "  -f prerelease=true \\",
      "  -f publish_untrusted_ad_hoc=false \\",
      "  -f manual_acceptance_confirmed=false \\",
      "  -f manual_acceptance_evidence=",
    ]
  )


def dry_run_artifact_lookup_command() -> str:
  return (
    'release_workflow_run_id="$(\n'
    "  gh api repos/:owner/:repo/actions/artifacts --paginate \\\n"
    "    --jq "
    + "'"
    + "[.artifacts[] | select("
    + f'.name == "release-dry-run-{TAG}" '
    + "and .expired == false "
    + f'and .workflow_run.head_sha == "{SOURCE_COMMIT}"'
    + ')][0].workflow_run.id // ""'
    + "'\n"
    ')"\n'
    'test -n "$release_workflow_run_id"'
  )


def dry_run_artifact_download_command() -> str:
  return (
    'release_workflow_conclusion="$(gh run view "$release_workflow_run_id" --json conclusion --jq .conclusion)"\n'
    'test "$release_workflow_conclusion" = success\n'
    f'gh run download "$release_workflow_run_id" --name release-dry-run-{TAG} --dir release-dry-run-{TAG}'
  )


def dry_run_evidence_validation_command() -> str:
  lines = [
    "python3 scripts/release_evidence_contract.py \\",
    "  --mode dry-run \\",
    f"  --tag {TAG} \\",
  ]
  evidence_names = expected_evidence_names("dry-run", TAG)
  for index, name in enumerate(evidence_names):
    suffix = " \\" if index < len(evidence_names) - 1 else ""
    lines.append(f"  --evidence release-dry-run-{TAG}/{name}{suffix}")
  return "\n".join(lines)


def release_plan_payload(mode: str = "dry-run") -> dict[str, object]:
  workflow_mode = "dry-run" if mode == "dry-run" else "publish"
  github_mutation = "none" if workflow_mode == "dry-run" else "create-or-update"
  planned_draft = True
  return {
    "tag": TAG,
    "title": f"Pith {TAG}",
    "sourceCommit": SOURCE_COMMIT,
    "successfulCiRunUrl": "https://github.com/walt1012/pith/actions/runs/1",
    "releaseWorkflowRunUrl": "https://github.com/walt1012/pith/actions/runs/2",
    "workflowMode": workflow_mode,
    "githubMutation": github_mutation,
    "signingMode": "ad-hoc",
    "releaseExists": False,
    "existingReleaseState": "none",
    "requestedDraft": True,
    "requestedPrerelease": True,
    "allowVisibleAdHoc": False,
    "manualAcceptanceConfirmed": False,
    "manualAcceptanceEvidence": "",
    "plannedDraft": planned_draft,
    "plannedPrerelease": True,
    "trustPath": "Ad-hoc signed prerelease.",
    "nextMaintainerActions": release_plan_actions(
      workflow_mode=workflow_mode,
      planned_draft=planned_draft,
    ),
  }


def release_plan_actions(*, workflow_mode: str, planned_draft: bool) -> list[str]:
  if workflow_mode == "dry-run":
    return [
      "Download the `release-dry-run-*` artifact from this workflow.",
      (
        "Verify the DMG checksum, release manifest, release plan, "
        "and dry-run rehearsal summary."
      ),
      (
        "Run the manual prerelease acceptance checklist on a fresh Mac "
        "before any visible ad-hoc release."
      ),
      (
        "If acceptance passes, rerun the release workflow with "
        "`dry_run=false` and the intended visibility inputs."
      ),
    ]
  if planned_draft:
    return [
      (
        "Review the draft GitHub Release assets and downloaded-release "
        "rehearsal summary."
      ),
      (
        "Complete manual prerelease acceptance before making any ad-hoc "
        "release visible."
      ),
      (
        "Edit release visibility only after the release page, manifest, "
        "checksum, and install guide all match."
      ),
    ]
  return [
    (
      "Inspect the visible GitHub Release page and confirm the exact four "
      "public assets."
    ),
    "Confirm the recorded manual acceptance receipt matches the published DMG.",
    (
      "If acceptance fails, withdraw the release deliberately rather than "
      "moving it back to draft in automation."
    ),
  ]


def release_rehearsal_payload() -> dict[str, object]:
  return {
    "tag": TAG,
    "result": "passed",
    "assetNames": list(release_installer_asset_names(TAG)),
    "checksumCommand": "shasum -a 256 Pith-v0.1.0-macos-x86_64.dmg",
    "sourceCommit": SOURCE_COMMIT,
    "signingMode": "ad-hoc",
    "trust": {
      "mode": "ad-hoc",
      "gatekeeper": "Manual Gatekeeper approval is required.",
    },
    "defaultModelId": DEFAULT_MODEL_ID,
    "appPackage": {
      "sourceCommit": SOURCE_COMMIT,
      "modelDelivery": "in-app-download",
      "firstAppOpenActionContract": "first-app-open-v1",
    },
    "firstRun": {"modelSetup": "required"},
    "dailyDriver": {"loop": "cowork"},
    "releaseDecision": {
      "automatedRehearsal": "passed",
      "manualAcceptance": "required-before-visible-prerelease",
      "publishGate": "do-not-publish-visible-ad-hoc-until-manual-acceptance-passes",
    },
    "firstAppOpenChecks": [
      "Launch Pith from Applications after handling Gatekeeper if needed.",
      f"Download one verified local model; {DEFAULT_MODEL_ID} is the default.",
      "Open a workspace folder.",
      "Confirm Web Search readiness and sandbox status.",
      "Use the first app-open action.",
      "Follow the daily-driver next action shown in the app header and inspector.",
    ],
    "manualPrereleaseChecks": [
      "Verify the downloaded DMG with the SHA-256 sidecar before opening it.",
      (
        "Open the release manifest and confirm macOS x86_64, "
        "in-app model delivery, no bundled model weights, and no Pith login."
      ),
      "Install Pith from the DMG and handle Gatekeeper according to the manifest guidance.",
      f"Download and activate one verified local model; {DEFAULT_MODEL_ID} is the default choice.",
      (
        "Open a real workspace folder and confirm the header or inspector "
        "reports workspace readiness."
      ),
      (
        "Run Map Workspace, Plan Next Step, or a short cowork request "
        "from the first app-open surface."
      ),
      "Let the model use Web Search when useful and inspect the source proof in the timeline.",
      "Approve one safe local workspace change only after reviewing the diff, then confirm the timeline receipt.",
      (
        "Restart Pith and confirm runtime readiness, selected workspace, "
        "model state, and recent proof recover."
      ),
    ],
    "packagedSmokeReceipt": {
      "phrase": "Packaged first-run smoke proof",
      "proofScope": "packaged-app",
      "checkCount": 5,
      "journey": [{"title": "First app open", "checkIds": ["model", "workspace"]}],
      "packageMetadata": {"sourceCommit": SOURCE_COMMIT},
      "packageMetadataMatched": True,
    },
  }


def release_rehearsal_markdown() -> str:
  return f"""# Pith {TAG} Release Rehearsal

Result: `passed`

## Assets
- `Pith-{TAG}-macos-x86_64.dmg`

## Verification
- Default model: `{DEFAULT_MODEL_ID}`

## Packaged Smoke Journey
- First app open: model, workspace

## First App Open
- Confirm Web Search readiness and sandbox status.

## Manual Prerelease Acceptance
- [ ] Let the model use Web Search when useful and inspect the source proof in the timeline.
- [ ] Approve one safe local workspace change only after reviewing the diff, then confirm the timeline receipt.
"""


def manual_acceptance_markdown() -> str:
  return f"""# Pith {TAG} Manual Release Acceptance

Decision:
- [ ] Accept this build for visible ad-hoc prerelease.
- [ ] Keep this build draft-only and fix issues before publishing.

## Downloaded Assets
- [ ] Download `Pith-{TAG}-macos-x86_64.dmg`

## Required Manual Checks
- [ ] Verify the downloaded DMG with the SHA-256 sidecar before opening it.
- [ ] Install Pith from the DMG and handle Gatekeeper according to the manifest guidance.
- [ ] Download and activate one verified local model; {DEFAULT_MODEL_ID} is the default choice.
- [ ] Let the model use Web Search when useful and inspect the source proof in the timeline.
- [ ] Approve one safe local workspace change only after reviewing the diff, then confirm the timeline receipt.
- [ ] Restart Pith and confirm runtime readiness, selected workspace, model state, and recent proof recover.

## Evidence To Record
- Approval and diff receipt inspected.
- Restart recovery result.
- Fill a structured manual acceptance JSON receipt and validate it with `python3 scripts/manual_acceptance_contract.py --tag {TAG} --evidence <manual-acceptance.json>`.
"""


def write_evidence_set(root: Path, mode: str) -> list[Path]:
  if mode == "dry-run":
    write_installer_assets(root)
  paths: list[Path] = []
  for name in expected_evidence_names(mode, TAG):
    path = root / name
    if not path.exists():
      write_evidence_file(path, mode)
    paths.append(path)
  return paths


def write_installer_assets(root: Path) -> None:
  dmg_name, checksum_name, guide_name, manifest_name = release_installer_asset_names(TAG)
  dmg = root / dmg_name
  dmg.write_bytes(b"pith release dmg\n")
  guide = root / guide_name
  guide.write_text(release_install_guide(TAG, "ad-hoc"), encoding="utf-8")
  checksum = write_checksum_file(dmg, root / checksum_name)
  write_release_manifest(
    tag=TAG,
    source_commit=SOURCE_COMMIT,
    signing_mode="ad-hoc",
    artifact_path=dmg,
    checksum_path=checksum,
    install_guide_path=guide,
    output_path=root / manifest_name,
    workflow_run_id=WORKFLOW_RUN_ID,
    workflow_run_url=WORKFLOW_RUN_URL,
  )


def expect_failure(action, expected: str) -> None:
  try:
    action()
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r}, got {error!r}") from error
    return
  raise AssertionError(f"expected release evidence validation to fail: {expected}")


def assert_dry_run_evidence_accepts_exact_set() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    validate_release_evidence_set(mode="dry-run", tag=TAG, evidence_paths=paths)
    expected = set(release_installer_asset_names(TAG))
    if not expected.issubset({path.name for path in paths}):
      raise AssertionError("dry-run evidence should include the public installer assets")


def assert_publish_rehearsal_accepts_exact_set() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    validate_release_evidence_set(
      mode="publish-rehearsal",
      tag=TAG,
      evidence_paths=paths,
    )
    if any(path.suffix == ".dmg" for path in paths):
      raise AssertionError("publish rehearsal evidence should not include public installer assets")


def assert_rejects_missing_extra_empty_and_invalid_json() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths[:-1],
      ),
      "missing",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    extra = root / "unexpected.json"
    write_evidence_file(extra, "dry-run")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths + [extra],
      ),
      "extra",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    paths[0].write_text("", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must not be empty",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    json_path = next(path for path in paths if path.suffix == ".json")
    json_path.write_text("[]\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "JSON must be an object",
    )


def assert_rejects_invalid_dry_run_installer_asset_evidence() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    (root / f"Pith-{TAG}-macos-x86_64.dmg.sha256").write_text(
      "0" * 64 + f"  Pith-{TAG}-macos-x86_64.dmg\n",
      encoding="utf-8",
    )
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "Release checksum does not match artifact",
    )


def assert_rejects_stale_structured_json() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    del stale["postAcceptancePublishCommand"]
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "missing required keys",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("publish-rehearsal")
    del stale["manualAcceptanceEvidence"]
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "missing required keys",
    )


def assert_rejects_stale_readiness_commands() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["tagCommands"] = [f"git tag {TAG} HEAD", f"git push origin {TAG}"]
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "tag commands",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["remoteTagVerificationCommand"] = str(
      stale["remoteTagVerificationCommand"]
    ).replace(SOURCE_COMMIT, "HEAD")
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "remote tag verification command",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["successfulCiLookupCommand"] = str(
      stale["successfulCiLookupCommand"]
    ).replace(f"--commit {SOURCE_COMMIT}", "--branch main")
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "successful CI lookup command",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["dryRunEvidenceValidationCommand"] = "python scripts/release_evidence_contract.py"
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "dry-run evidence validation command",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["dryRunArtifactLookupCommand"] = stale["dryRunArtifactLookupCommand"].replace(
      SOURCE_COMMIT,
      "fedcba9876543210fedcba9876543210fedcba98",
    )
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "dry-run artifact lookup command",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["nextCommand"] = str(stale["nextCommand"]).replace(
      f"-f tag={TAG}",
      "-f tag=v9.9.9",
    )
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "next command",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["postAcceptancePublishCommand"] = str(
      stale["postAcceptancePublishCommand"]
    ).replace("-f dry_run=false", "-f dry_run=true")
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "post-acceptance publish command",
    )


def assert_rejects_stale_readiness_checklist() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["preDispatchChecklist"] = [
      item
      for item in pre_dispatch_checklist()
      if "manual acceptance" not in item
      and "post-acceptance publish command" not in item
    ]
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "preDispatchChecklist",
    )


def assert_rejects_blocked_readiness_state() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["status"] = "blocked"
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "status must be ready",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["blockers"] = ["Release workflow is missing required dispatch inputs."]
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must not contain blockers",
    )

  for key in (
    "workingTreeClean",
    "tagPointsAtSourceCommit",
    "releaseWorkflowInputsReady",
  ):
    with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
      root = Path(directory)
      paths = write_evidence_set(root, "dry-run")
      readiness_path = root / "release-readiness.json"
      stale = release_readiness_payload("dry-run")
      stale[key] = False
      readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
      expect_failure(
        lambda: validate_release_evidence_set(
          mode="dry-run",
          tag=TAG,
          evidence_paths=paths,
        ),
        key,
      )


def assert_rejects_stale_release_plan_actions() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("dry-run")
    stale["nextMaintainerActions"] = [
      action
      for action in release_plan_actions(
        workflow_mode="dry-run",
        planned_draft=True,
      )
      if "dry_run=false" not in action
    ]
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "nextMaintainerActions",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("publish-rehearsal")
    stale["nextMaintainerActions"] = [
      action
      for action in release_plan_actions(
        workflow_mode="publish",
        planned_draft=True,
      )
      if "manual prerelease acceptance" not in action
    ]
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "nextMaintainerActions",
    )


def assert_rejects_stale_manual_acceptance_markdown() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    acceptance_path = root / "release-dry-run-manual-acceptance.md"
    acceptance_path.write_text("# Manual Release Acceptance\n\nIncomplete.\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "missing required terms",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    acceptance_path = root / "release-manual-acceptance.md"
    stale = manual_acceptance_markdown().replace("Web Search", "retrieval")
    acceptance_path.write_text(stale + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "Web Search",
    )


def assert_accepts_developer_id_publish_command() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    plan_path = root / "release-plan.json"
    readiness_path.write_text(
      json.dumps(developer_id_readiness_payload("dry-run")) + "\n",
      encoding="utf-8",
    )
    plan = release_plan_payload("dry-run")
    plan["signingMode"] = "developer-id"
    plan_path.write_text(json.dumps(plan) + "\n", encoding="utf-8")
    validate_release_evidence_set(
      mode="dry-run",
      tag=TAG,
      evidence_paths=paths,
    )


def assert_rejects_stale_release_rehearsal_evidence() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    rehearsal_path = root / "release-dry-run-rehearsal.json"
    stale = release_rehearsal_payload()
    stale["packagedSmokeReceipt"]["packageMetadataMatched"] = False
    rehearsal_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "packageMetadataMatched",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    rehearsal_path = root / "release-rehearsal.md"
    stale = release_rehearsal_markdown().replace("reviewing the diff", "checking output")
    rehearsal_path.write_text(stale + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "reviewing the diff",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    rehearsal_path = root / "release-dry-run-rehearsal.json"
    stale = release_rehearsal_payload()
    stale["firstAppOpenChecks"] = [
      check
      for check in stale["firstAppOpenChecks"]
      if "workspace folder" not in check
    ]
    rehearsal_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "firstAppOpenChecks",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    rehearsal_path = root / "release-rehearsal.json"
    stale = release_rehearsal_payload()
    stale["manualPrereleaseChecks"] = [
      check
      for check in stale["manualPrereleaseChecks"]
      if "Restart Pith" not in check
    ]
    rehearsal_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "manualPrereleaseChecks",
    )


def assert_rejects_inconsistent_structured_json() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["tag"] = "v9.9.9"
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must be 'v0.1.0'",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("publish-rehearsal")
    stale["workflowMode"] = "dry-run"
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must be 'publish'",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["expectedPublicAssets"] = []
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must match the release contract",
    )


def assert_rejects_cross_file_json_disagreement() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("dry-run")
    stale["sourceCommit"] = "fedcba9876543210fedcba9876543210fedcba98"
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "disagree on sourceCommit",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("dry-run")
    stale["allowVisibleAdHoc"] = True
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "disagree on allowUntrustedAdHoc/allowVisibleAdHoc",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("publish-rehearsal")
    stale["successfulCiRunUrl"] = "https://github.com/walt1012/pith/actions/runs/99"
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "disagree on successfulCiRunUrl",
    )


def main() -> int:
  assert_dry_run_evidence_accepts_exact_set()
  assert_publish_rehearsal_accepts_exact_set()
  assert_rejects_missing_extra_empty_and_invalid_json()
  assert_rejects_invalid_dry_run_installer_asset_evidence()
  assert_rejects_stale_structured_json()
  assert_rejects_blocked_readiness_state()
  assert_rejects_stale_release_plan_actions()
  assert_rejects_stale_readiness_checklist()
  assert_rejects_stale_readiness_commands()
  assert_accepts_developer_id_publish_command()
  assert_rejects_stale_manual_acceptance_markdown()
  assert_rejects_stale_release_rehearsal_evidence()
  assert_rejects_inconsistent_structured_json()
  assert_rejects_cross_file_json_disagreement()
  print("release evidence contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
