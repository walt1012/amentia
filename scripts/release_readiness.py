#!/usr/bin/env python3
"""Prepare a maintainer-facing release readiness report."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from release_identity import validate_public_release_tag
from release_artifacts import release_installer_asset_names
from release_evidence_contract import expected_evidence_names
from release_state import ReleaseState
from release_state import plan_release_state
from release_state import validate_manual_acceptance_gate


REPO_ROOT = Path(__file__).resolve().parent.parent
RELEASE_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "release.yml"
REQUIRED_RELEASE_INPUTS = (
  "tag",
  "draft",
  "prerelease",
  "dry_run",
  "publish_untrusted_ad_hoc",
  "manual_acceptance_confirmed",
  "manual_acceptance_evidence",
)


@dataclass(frozen=True)
class ReleaseReadiness:
  tag: str
  source_commit: str
  working_tree_clean: bool
  tag_points_at_commit: bool
  workflow_inputs_ready: bool
  ci_run_url: str
  dry_run: bool
  signing_mode: str
  requested_draft: bool
  requested_prerelease: bool
  allow_untrusted_ad_hoc: bool
  state: ReleaseState
  manual_acceptance_confirmed: bool
  manual_acceptance_evidence: str
  blockers: tuple[str, ...]

  @property
  def ready(self) -> bool:
    return not self.blockers


def git_output(*args: str) -> str:
  return subprocess.check_output(
    ["git", *args],
    cwd=REPO_ROOT,
    text=True,
    stderr=subprocess.STDOUT,
  ).strip()


def git_success(*args: str) -> bool:
  return subprocess.run(
    ["git", *args],
    cwd=REPO_ROOT,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    text=True,
    check=False,
  ).returncode == 0


def working_tree_clean() -> bool:
  return git_output("status", "--porcelain") == ""


def current_commit() -> str:
  return git_output("rev-parse", "HEAD")


def tag_points_at_commit(tag: str, commit: str) -> bool:
  if not git_success("rev-parse", "--verify", f"refs/tags/{tag}"):
    return False
  tag_commit = git_output("rev-list", "-n", "1", tag)
  return tag_commit == commit


def release_workflow_inputs_ready() -> bool:
  if not RELEASE_WORKFLOW.is_file():
    return False
  text = RELEASE_WORKFLOW.read_text(encoding="utf-8")
  return all(f"{name}:" in text for name in REQUIRED_RELEASE_INPUTS)


def plan_readiness(
  *,
  tag: str,
  source_commit: str,
  working_tree_clean_value: bool,
  tag_points_at_commit_value: bool,
  workflow_inputs_ready: bool,
  ci_run_url: str,
  dry_run: bool,
  signing_mode: str,
  requested_draft: bool,
  requested_prerelease: bool,
  allow_untrusted_ad_hoc: bool,
  manual_acceptance_confirmed: bool,
  manual_acceptance_evidence: str,
) -> ReleaseReadiness:
  blockers: list[str] = []
  try:
    validate_public_release_tag(tag)
  except (RuntimeError, ValueError) as error:
    blockers.append(str(error))

  try:
    state = plan_release_state(
      signing_mode=signing_mode,
      requested_draft=requested_draft,
      requested_prerelease=requested_prerelease,
      allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
      release_exists=False,
      existing_draft=None,
    )
  except ValueError as error:
    state = ReleaseState(draft=True, prerelease=True)
    blockers.append(str(error))

  try:
    validate_manual_acceptance_gate(
      signing_mode=signing_mode,
      dry_run=dry_run,
      allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
      manual_acceptance_confirmed=manual_acceptance_confirmed,
      manual_acceptance_evidence=manual_acceptance_evidence,
      state=state,
    )
  except ValueError as error:
    blockers.append(str(error))

  if not source_commit:
    blockers.append("Source commit is required.")
  if not working_tree_clean_value:
    blockers.append("Working tree must be clean before preparing release inputs.")
  if not tag_points_at_commit_value:
    blockers.append(
      f"Tag {tag} must exist locally and point at commit {source_commit} before dispatch."
    )
  if not workflow_inputs_ready:
    blockers.append("Release workflow is missing required dispatch inputs.")
  if not ci_run_url.strip():
    blockers.append("Successful CI run URL must be recorded before release dry-run.")

  return ReleaseReadiness(
    tag=tag,
    source_commit=source_commit,
    working_tree_clean=working_tree_clean_value,
    tag_points_at_commit=tag_points_at_commit_value,
    workflow_inputs_ready=workflow_inputs_ready,
    ci_run_url=ci_run_url.strip(),
    dry_run=dry_run,
    signing_mode=signing_mode,
    requested_draft=requested_draft,
    requested_prerelease=requested_prerelease,
    allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
    state=state,
    manual_acceptance_confirmed=manual_acceptance_confirmed,
    manual_acceptance_evidence=manual_acceptance_evidence.strip(),
    blockers=tuple(blockers),
  )


def readiness_report(readiness: ReleaseReadiness) -> str:
  status = "ready" if readiness.ready else "blocked"
  workflow_mode = "dry-run" if readiness.dry_run else "publish"
  visibility = "draft" if readiness.state.draft else "visible"
  release_class = "prerelease" if readiness.state.prerelease else "stable"
  evidence = readiness.manual_acceptance_evidence or "not recorded"
  blockers = "\n".join(f"- {blocker}" for blocker in readiness.blockers)
  next_command = readiness_next_command(readiness)
  ci_lookup_command = readiness_ci_lookup_command(readiness)
  dry_run_download_command = readiness_dry_run_download_command(readiness)
  dry_run_validation_command = readiness_dry_run_validation_command(readiness)
  tag_commands = "\n".join(readiness_tag_commands(readiness))
  checklist = "\n".join(f"- [ ] {item}" for item in readiness_checklist(readiness))
  dry_run_evidence = "\n".join(
    f"- `{name}`" for name in expected_evidence_names("dry-run", readiness.tag)
  )
  if not blockers:
    blockers = "- None"

  return f"""# Release Readiness

- Status: `{status}`
- Tag: `{readiness.tag}`
- Source commit: `{readiness.source_commit}`
- Successful CI: {readiness.ci_run_url or "not recorded"}
- Workflow mode: `{workflow_mode}`
- Signing mode: `{readiness.signing_mode}`
- Planned visibility: `{visibility} {release_class}`
- Working tree clean: `{str(readiness.working_tree_clean).lower()}`
- Tag points at source commit: `{str(readiness.tag_points_at_commit).lower()}`
- Release workflow inputs ready: `{str(readiness.workflow_inputs_ready).lower()}`
- Manual acceptance confirmed: `{str(readiness.manual_acceptance_confirmed).lower()}`
- Manual acceptance evidence: {evidence}

## Blockers
{blockers}

## Pre-Dispatch Checklist
{checklist}

## Tag Preparation
```bash
{tag_commands}
```

## CI Lookup
```bash
{ci_lookup_command}
```

## Expected Dry-Run Evidence
{dry_run_evidence}

## Next Command
```bash
{next_command}
```

## Dry-Run Artifact Verification
```bash
{dry_run_download_command}
{dry_run_validation_command}
```
"""


def readiness_json(readiness: ReleaseReadiness) -> dict[str, object]:
  return {
    "status": "ready" if readiness.ready else "blocked",
    "tag": readiness.tag,
    "sourceCommit": readiness.source_commit,
    "successfulCiRunUrl": readiness.ci_run_url,
    "workflowMode": "dry-run" if readiness.dry_run else "publish",
    "signingMode": readiness.signing_mode,
    "requestedDraft": readiness.requested_draft,
    "requestedPrerelease": readiness.requested_prerelease,
    "allowUntrustedAdHoc": readiness.allow_untrusted_ad_hoc,
    "plannedDraft": readiness.state.draft,
    "plannedPrerelease": readiness.state.prerelease,
    "workingTreeClean": readiness.working_tree_clean,
    "tagPointsAtSourceCommit": readiness.tag_points_at_commit,
    "releaseWorkflowInputsReady": readiness.workflow_inputs_ready,
    "manualAcceptanceConfirmed": readiness.manual_acceptance_confirmed,
    "manualAcceptanceEvidence": readiness.manual_acceptance_evidence,
    "preDispatchChecklist": readiness_checklist(readiness),
    "expectedPublicAssets": list(release_installer_asset_names(readiness.tag)),
    "expectedDryRunEvidence": list(expected_evidence_names("dry-run", readiness.tag)),
    "tagCommands": readiness_tag_commands(readiness),
    "successfulCiLookupCommand": readiness_ci_lookup_command(readiness),
    "dryRunArtifactDownloadCommand": readiness_dry_run_download_command(readiness),
    "dryRunEvidenceValidationCommand": readiness_dry_run_validation_command(readiness),
    "blockers": list(readiness.blockers),
    "nextCommand": readiness_next_command(readiness),
  }


def readiness_checklist(readiness: ReleaseReadiness) -> list[str]:
  return [
    f"Create tag {readiness.tag} at source commit {readiness.source_commit} if it does not already exist.",
    f"Confirm tag {readiness.tag} points at source commit {readiness.source_commit}.",
    "Push the tag to origin; tag-push release events run as dry-run by default.",
    "Use the CI lookup command to copy the successful CI URL for this exact source commit.",
    f"Confirm the successful CI run matches the source commit: {readiness.ci_run_url or 'not recorded'}.",
    "Run the release workflow as a dry-run before any publish attempt.",
    f"Download the release-dry-run-{readiness.tag} workflow artifact after the dry-run passes.",
    "Run the dry-run evidence validation command before manual acceptance.",
    "Verify the DMG checksum, release manifest, release plan, rehearsal summary, and manual acceptance checklist.",
    "Complete fresh-Mac manual acceptance before any visible ad-hoc prerelease.",
  ]


def readiness_ci_lookup_command(readiness: ReleaseReadiness) -> str:
  return " ".join(
    [
      "gh run list",
      "--workflow CI",
      f"--commit {shell_quote(readiness.source_commit)}",
      "--status success",
      "--json conclusion,headSha,url",
      "--limit 20",
      "--jq",
      shell_quote(
        f'[.[] | select(.headSha == "{readiness.source_commit}" and .conclusion == "success")][0].url // ""'
      ),
    ]
  )


def readiness_dry_run_download_command(readiness: ReleaseReadiness) -> str:
  artifact_name = f"release-dry-run-{readiness.tag}"
  return (
    f"gh run download <release-workflow-run-id> "
    f"--name {shell_quote(artifact_name)} "
    f"--dir {shell_quote(artifact_name)}"
  )


def readiness_dry_run_validation_command(readiness: ReleaseReadiness) -> str:
  artifact_dir = f"release-dry-run-{readiness.tag}"
  lines = [
    "python scripts/release_evidence_contract.py \\",
    "  --mode dry-run \\",
    f"  --tag {shell_quote(readiness.tag)} \\",
  ]
  evidence_names = expected_evidence_names("dry-run", readiness.tag)
  for index, name in enumerate(evidence_names):
    suffix = " \\" if index < len(evidence_names) - 1 else ""
    lines.append(f"  --evidence {shell_quote(f'{artifact_dir}/{name}')}{suffix}")
  return "\n".join(lines)


def readiness_tag_commands(readiness: ReleaseReadiness) -> list[str]:
  return [
    f"git tag {shell_quote(readiness.tag)} {readiness.source_commit}",
    f"git push origin {shell_quote(readiness.tag)}",
  ]


def readiness_next_command(readiness: ReleaseReadiness) -> str:
  return "\n".join(
    [
      "gh workflow run release.yml \\",
      f"  -f tag={readiness.tag} \\",
      f"  -f dry_run={str(readiness.dry_run).lower()} \\",
      f"  -f draft={str(readiness.requested_draft).lower()} \\",
      f"  -f prerelease={str(readiness.requested_prerelease).lower()} \\",
      f"  -f publish_untrusted_ad_hoc={str(readiness.allow_untrusted_ad_hoc).lower()} \\",
      f"  -f manual_acceptance_confirmed={str(readiness.manual_acceptance_confirmed).lower()} \\",
      f"  -f manual_acceptance_evidence={shell_quote(readiness.manual_acceptance_evidence)}",
    ]
  )


def shell_quote(value: str) -> str:
  if not value:
    return ""
  if all(character.isalnum() or character in "-_./:#?=&%" for character in value):
    return value
  return "'" + value.replace("'", "'\"'\"'") + "'"


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--ci-run-url", default="")
  parser.add_argument("--output", type=Path)
  parser.add_argument("--json-output", type=Path)
  parser.add_argument("--dry-run", default="true")
  parser.add_argument("--signing-mode", default="ad-hoc", choices=("ad-hoc", "developer-id"))
  parser.add_argument("--requested-draft", default="true")
  parser.add_argument("--requested-prerelease", default="true")
  parser.add_argument("--allow-untrusted-ad-hoc", default="false")
  parser.add_argument("--manual-acceptance-confirmed", default="false")
  parser.add_argument("--manual-acceptance-evidence", default="")
  args = parser.parse_args()

  try:
    readiness = plan_readiness(
      tag=args.tag,
      source_commit=current_commit(),
      working_tree_clean_value=working_tree_clean(),
      tag_points_at_commit_value=tag_points_at_commit(args.tag, current_commit()),
      workflow_inputs_ready=release_workflow_inputs_ready(),
      ci_run_url=args.ci_run_url,
      dry_run=parse_bool(args.dry_run),
      signing_mode=args.signing_mode,
      requested_draft=parse_bool(args.requested_draft),
      requested_prerelease=parse_bool(args.requested_prerelease),
      allow_untrusted_ad_hoc=parse_bool(args.allow_untrusted_ad_hoc),
      manual_acceptance_confirmed=parse_bool(args.manual_acceptance_confirmed),
      manual_acceptance_evidence=args.manual_acceptance_evidence,
    )
  except (subprocess.CalledProcessError, ValueError) as error:
    print(f"release readiness failed: {error}", file=sys.stderr)
    return 1

  report = readiness_report(readiness)
  if args.output is not None:
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(report, encoding="utf-8")
  else:
    print(report)
  if args.json_output is not None:
    args.json_output.parent.mkdir(parents=True, exist_ok=True)
    args.json_output.write_text(
      json.dumps(readiness_json(readiness), indent=2, sort_keys=True) + "\n",
      encoding="utf-8",
    )
  return 0 if readiness.ready else 1


def parse_bool(value: str) -> bool:
  normalized = value.strip().lower()
  if normalized in {"1", "true", "yes", "on"}:
    return True
  if normalized in {"0", "false", "no", "off", ""}:
    return False
  raise ValueError(f"invalid boolean value: {value!r}")


if __name__ == "__main__":
  raise SystemExit(main())
