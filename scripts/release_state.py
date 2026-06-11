#!/usr/bin/env python3
"""Plan GitHub Release state for Pith distribution builds."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

from package_contract import RELEASE_SIGNING_MODES
from release_identity import validate_public_release_tag
from release_text import release_trust_note
from release_text import validate_release_notes


@dataclass(frozen=True)
class ReleaseState:
  draft: bool
  prerelease: bool


def expected_release_title(tag: str) -> str:
  return f"Pith {tag}"


def validate_release_title(title: str, tag: str) -> None:
  expected = expected_release_title(tag)
  if title != expected:
    raise ValueError(f"Release title must be {expected!r}")


def parse_bool(value: str) -> bool:
  normalized = value.strip().lower()
  if normalized in {"1", "true", "yes", "on"}:
    return True
  if normalized in {"0", "false", "no", "off", ""}:
    return False
  raise ValueError(f"invalid boolean value: {value!r}")


def plan_release_state(
  *,
  signing_mode: str,
  requested_draft: bool,
  requested_prerelease: bool,
  allow_untrusted_ad_hoc: bool,
  release_exists: bool,
  existing_draft: bool | None,
) -> ReleaseState:
  is_developer_id = signing_mode == "developer-id"
  if release_exists and existing_draft is None:
    raise ValueError("Existing GitHub Release draft state is required.")
  if release_exists and existing_draft is False and requested_draft:
    raise ValueError(
      "Refusing to move an existing public GitHub Release back to draft. "
      "Delete or manually manage the release before retrying."
    )
  if (
    not is_developer_id
    and not allow_untrusted_ad_hoc
    and release_exists
    and existing_draft is False
  ):
    raise ValueError(
      "Refusing to update a public GitHub Release with an ad-hoc DMG. "
      "Run with publish_untrusted_ad_hoc=true or configure Developer ID signing."
    )

  final_draft = requested_draft or (
    not is_developer_id and not allow_untrusted_ad_hoc
  )
  final_prerelease = requested_prerelease or not is_developer_id

  desired_draft = final_draft
  if (
    is_developer_id
    and release_exists
    and existing_draft is False
    and not requested_draft
  ):
    desired_draft = False

  return ReleaseState(draft=desired_draft, prerelease=final_prerelease)


def validate_manual_acceptance_gate(
  *,
  signing_mode: str,
  dry_run: bool,
  allow_untrusted_ad_hoc: bool,
  manual_acceptance_confirmed: bool,
  manual_acceptance_evidence: str,
  state: ReleaseState,
) -> None:
  if dry_run or signing_mode == "developer-id" or state.draft:
    return
  if not allow_untrusted_ad_hoc:
    return
  if not manual_acceptance_confirmed:
    raise ValueError(
      "Visible ad-hoc prereleases require manual_acceptance_confirmed=true. "
      "Generate and validate the manual acceptance receipt on a fresh Mac before publishing."
    )
  if not manual_acceptance_evidence.strip():
    raise ValueError(
      "Visible ad-hoc prereleases require a manual acceptance receipt URL. "
      "Set manual_acceptance_evidence to a validated receipt URL."
    )
  validate_manual_acceptance_evidence(manual_acceptance_evidence)


def validate_manual_acceptance_evidence(value: str) -> None:
  normalized = value.strip().lower()
  placeholder_terms = ("<", ">", "todo", "tbd", "not recorded", "placeholder")
  if any(term in normalized for term in placeholder_terms):
    raise ValueError(
      "Visible ad-hoc prereleases require a real manual acceptance receipt. "
      "Replace placeholder text with a validated manual acceptance receipt URL."
    )
  if not normalized.startswith("https://"):
    raise ValueError(
      "Visible ad-hoc prereleases require the manual acceptance receipt as an HTTPS URL."
    )
  if not normalized.startswith("https://github.com/walt1012/pith/"):
    raise ValueError(
      "Visible ad-hoc prereleases require a repository-scoped manual acceptance receipt URL."
    )
  if "manual-acceptance" not in normalized or "receipt" not in normalized:
    raise ValueError(
      "Visible ad-hoc prereleases require a validated manual acceptance receipt URL."
    )


def write_env(path: Path, state: ReleaseState) -> None:
  path.write_text(
    "\n".join(
      [
        f"PITH_RELEASE_STATE_DRAFT={str(state.draft).lower()}",
        f"PITH_RELEASE_STATE_PRERELEASE={str(state.prerelease).lower()}",
        "",
      ]
    ),
    encoding="utf-8",
  )


def release_state_summary(
  *,
  tag: str,
  title: str,
  source_commit: str,
  ci_run_url: str,
  workflow_run_url: str,
  dry_run: bool,
  signing_mode: str,
  requested_draft: bool,
  requested_prerelease: bool,
  allow_untrusted_ad_hoc: bool,
  manual_acceptance_confirmed: bool,
  manual_acceptance_evidence: str,
  release_exists: bool,
  existing_draft: bool | None,
  state: ReleaseState,
) -> str:
  existing_state = "none"
  if release_exists:
    existing_state = "draft" if existing_draft else "visible"
  mutation = "none; dry-run does not create or update a GitHub Release"
  if not dry_run:
    mutation = "create-or-update GitHub Release after asset rehearsal"
  visibility = release_visibility_label(dry_run=dry_run, state=state)
  next_actions = release_next_actions(dry_run=dry_run, state=state)
  trust_note = release_trust_note(
    signing_mode,
    allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
    draft=state.draft,
  )
  return f"""# Release Plan

- Tag: `{tag}`
- Title: `{title}`
- Source commit: `{summary_value(source_commit)}`
- Successful CI: {summary_value(ci_run_url)}
- Release workflow: {summary_value(workflow_run_url)}
- Workflow mode: `{"dry-run" if dry_run else "publish"}`
- GitHub mutation: {mutation}
- Signing mode: `{signing_mode}`
- Existing release: `{existing_state}`
- Requested draft: `{str(requested_draft).lower()}`
- Requested prerelease: `{str(requested_prerelease).lower()}`
- Allow visible ad-hoc: `{str(allow_untrusted_ad_hoc).lower()}`
- Manual acceptance confirmed: `{str(manual_acceptance_confirmed).lower()}`
- Manual acceptance receipt: {summary_value(manual_acceptance_evidence)}
- Release visibility: `{visibility}`
- Trust path: {trust_note}

## Next Maintainer Actions
{next_actions}
"""


def release_body(
  notes: str,
  *,
  signing_mode: str,
  allow_untrusted_ad_hoc: bool,
  manual_acceptance_evidence: str,
  state: ReleaseState,
) -> str:
  if signing_mode == "developer-id" or state.draft or not allow_untrusted_ad_hoc:
    return notes
  receipt = manual_acceptance_evidence.strip()
  return (
    notes.rstrip()
    + "\n\n## Manual Acceptance\n\n"
    + f"Visible ad-hoc prerelease acceptance receipt: {receipt}\n"
  )


def release_plan_json(
  *,
  tag: str,
  title: str,
  source_commit: str,
  ci_run_url: str,
  workflow_run_url: str,
  dry_run: bool,
  signing_mode: str,
  requested_draft: bool,
  requested_prerelease: bool,
  allow_untrusted_ad_hoc: bool,
  manual_acceptance_confirmed: bool,
  manual_acceptance_evidence: str,
  release_exists: bool,
  existing_draft: bool | None,
  state: ReleaseState,
) -> dict[str, object]:
  existing_state = "none"
  if release_exists:
    existing_state = "draft" if existing_draft else "visible"
  mutation = "none" if dry_run else "create-or-update"
  return {
    "tag": tag,
    "title": title,
    "sourceCommit": source_commit.strip(),
    "successfulCiRunUrl": ci_run_url.strip(),
    "releaseWorkflowRunUrl": workflow_run_url.strip(),
    "workflowMode": "dry-run" if dry_run else "publish",
    "githubMutation": mutation,
    "signingMode": signing_mode,
    "releaseExists": release_exists,
    "existingReleaseState": existing_state,
    "requestedDraft": requested_draft,
    "requestedPrerelease": requested_prerelease,
    "allowVisibleAdHoc": allow_untrusted_ad_hoc,
    "manualAcceptanceConfirmed": manual_acceptance_confirmed,
    "manualAcceptanceEvidence": manual_acceptance_evidence.strip(),
    "releaseVisibility": release_visibility_label(dry_run=dry_run, state=state),
    "plannedDraft": state.draft,
    "plannedPrerelease": state.prerelease,
    "trustPath": release_trust_note(
      signing_mode,
      allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
      draft=state.draft,
    ),
    "nextMaintainerActions": release_next_action_items(
      dry_run=dry_run,
      state=state,
    ),
  }


def release_visibility_label(*, dry_run: bool, state: ReleaseState) -> str:
  if dry_run:
    return "not published; dry-run only"
  visibility = "draft" if state.draft else "visible"
  release_class = "prerelease" if state.prerelease else "stable"
  return f"{visibility} {release_class}"


def release_next_action_items(*, dry_run: bool, state: ReleaseState) -> list[str]:
  return [
    line.removeprefix("- ").strip()
    for line in release_next_actions(dry_run=dry_run, state=state).splitlines()
    if line.strip()
  ]


def release_next_actions(*, dry_run: bool, state: ReleaseState) -> str:
  if dry_run:
    return "\n".join(
      [
        "- Download the `release-dry-run-*` artifact from this workflow.",
        "- Verify the DMG checksum, release manifest, release plan, and dry-run rehearsal summary.",
        "- Generate, fill, and validate the manual acceptance receipt on a fresh Mac before any visible ad-hoc release.",
        "- If acceptance passes, rerun the release workflow with `dry_run=false` and the intended visibility inputs.",
      ]
    )
  if state.draft:
    return "\n".join(
      [
        "- Review the draft GitHub Release assets and downloaded-release rehearsal summary.",
        "- Complete manual prerelease acceptance before making any ad-hoc release visible.",
        "- Edit release visibility only after the release page, manifest, checksum, and install guide all match.",
      ]
    )
  return "\n".join(
    [
      "- Inspect the visible GitHub Release page and confirm the exact four public assets.",
      "- Confirm the recorded manual acceptance receipt matches the published DMG.",
      "- If acceptance fails, withdraw the release deliberately rather than moving it back to draft in automation.",
    ]
  )


def summary_value(value: str) -> str:
  stripped = value.strip()
  return stripped if stripped else "not recorded"


def write_summary(path: Path, summary: str) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(summary, encoding="utf-8")


def write_plan_json(path: Path, plan: dict[str, object]) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(
    json.dumps(plan, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
  )


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--title", required=True)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--notes-file", required=True)
  parser.add_argument("--signing-mode", required=True, choices=sorted(RELEASE_SIGNING_MODES))
  parser.add_argument("--requested-draft", required=True)
  parser.add_argument("--requested-prerelease", required=True)
  parser.add_argument("--allow-untrusted-ad-hoc", required=True)
  parser.add_argument("--manual-acceptance-confirmed", default="false")
  parser.add_argument("--manual-acceptance-evidence", default="")
  parser.add_argument("--release-exists", required=True)
  parser.add_argument("--existing-draft", default="")
  parser.add_argument("--state-output", required=True)
  parser.add_argument("--env-output", required=True)
  parser.add_argument("--summary-output", type=Path)
  parser.add_argument("--plan-output", type=Path)
  parser.add_argument("--source-commit", default="")
  parser.add_argument("--ci-run-url", default="")
  parser.add_argument("--workflow-run-url", default="")
  parser.add_argument("--dry-run", default="false")
  args = parser.parse_args()

  try:
    validate_public_release_tag(args.tag)
    validate_release_title(args.title, args.tag)
    release_exists = parse_bool(args.release_exists)
    requested_draft = parse_bool(args.requested_draft)
    requested_prerelease = parse_bool(args.requested_prerelease)
    allow_untrusted_ad_hoc = parse_bool(args.allow_untrusted_ad_hoc)
    manual_acceptance_confirmed = parse_bool(args.manual_acceptance_confirmed)
    dry_run = parse_bool(args.dry_run)
    existing_draft = (
      parse_bool(args.existing_draft)
      if args.existing_draft.strip()
      else None
    )
    state = plan_release_state(
      signing_mode=args.signing_mode,
      requested_draft=requested_draft,
      requested_prerelease=requested_prerelease,
      allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
      release_exists=release_exists,
      existing_draft=existing_draft,
    )
    validate_manual_acceptance_gate(
      signing_mode=args.signing_mode,
      dry_run=dry_run,
      allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
      manual_acceptance_confirmed=manual_acceptance_confirmed,
      manual_acceptance_evidence=args.manual_acceptance_evidence,
      state=state,
    )
  except (RuntimeError, ValueError) as error:
    print(f"release state planning failed: {error}", file=sys.stderr)
    return 1

  notes = Path(args.notes_file).read_text(encoding="utf-8")
  try:
    validate_release_notes(
      notes,
      tag=args.tag,
      signing_mode=args.signing_mode,
      allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
      draft=state.draft,
    )
  except RuntimeError as error:
    print(f"release state planning failed: {error}", file=sys.stderr)
    return 1
  Path(args.state_output).write_text(
    json.dumps(
      {
        "tag_name": args.tag,
        "target_commitish": args.source_commit,
        "name": args.title,
        "body": release_body(
          notes,
          signing_mode=args.signing_mode,
          allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
          manual_acceptance_evidence=args.manual_acceptance_evidence,
          state=state,
        ),
        "draft": state.draft,
        "prerelease": state.prerelease,
      }
    ),
    encoding="utf-8",
  )
  write_env(Path(args.env_output), state)
  if args.summary_output is not None:
    write_summary(
      args.summary_output,
      release_state_summary(
        tag=args.tag,
        title=args.title,
        source_commit=args.source_commit,
        ci_run_url=args.ci_run_url,
        workflow_run_url=args.workflow_run_url,
        dry_run=dry_run,
        signing_mode=args.signing_mode,
        requested_draft=requested_draft,
        requested_prerelease=requested_prerelease,
        allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
        manual_acceptance_confirmed=manual_acceptance_confirmed,
        manual_acceptance_evidence=args.manual_acceptance_evidence,
        release_exists=release_exists,
        existing_draft=existing_draft,
        state=state,
      ),
    )
  if args.plan_output is not None:
    write_plan_json(
      args.plan_output,
      release_plan_json(
        tag=args.tag,
        title=args.title,
        source_commit=args.source_commit,
        ci_run_url=args.ci_run_url,
        workflow_run_url=args.workflow_run_url,
        dry_run=dry_run,
        signing_mode=args.signing_mode,
        requested_draft=requested_draft,
        requested_prerelease=requested_prerelease,
        allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
        manual_acceptance_confirmed=manual_acceptance_confirmed,
        manual_acceptance_evidence=args.manual_acceptance_evidence,
        release_exists=release_exists,
        existing_draft=existing_draft,
        state=state,
      ),
    )
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
