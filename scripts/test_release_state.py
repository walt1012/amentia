#!/usr/bin/env python3
"""Unit checks for release state planning."""

from __future__ import annotations

import json
import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from release_state import main as release_state_main
from release_state import expected_release_title
from release_state import plan_release_state
from release_state import ReleaseState
from release_state import release_plan_json
from release_state import release_body
from release_state import release_next_actions
from release_state import release_state_summary
from release_state import validate_manual_acceptance_gate
from release_state import validate_release_title
from release_text import release_notes


ACCEPTANCE_RECEIPT = "https://github.com/walt1012/pith/issues/1#manual-acceptance-receipt"


def assert_state(
  *,
  signing_mode: str,
  requested_draft: bool,
  requested_prerelease: bool,
  allow_untrusted_ad_hoc: bool,
  release_exists: bool,
  existing_draft: bool | None,
  expected_draft: bool,
  expected_prerelease: bool,
) -> None:
  state = plan_release_state(
    signing_mode=signing_mode,
    requested_draft=requested_draft,
    requested_prerelease=requested_prerelease,
    allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
    release_exists=release_exists,
    existing_draft=existing_draft,
  )
  if state.draft != expected_draft or state.prerelease != expected_prerelease:
    raise AssertionError(
      f"expected draft={expected_draft}, prerelease={expected_prerelease}; "
      f"got draft={state.draft}, prerelease={state.prerelease}"
    )


def assert_rejects_public_ad_hoc_without_explicit_publish() -> None:
  try:
    plan_release_state(
      signing_mode="ad-hoc",
      requested_draft=False,
      requested_prerelease=False,
      allow_untrusted_ad_hoc=False,
      release_exists=True,
      existing_draft=False,
    )
  except ValueError:
    return
  raise AssertionError("public ad-hoc release updates should be rejected")


def assert_rejects_visible_ad_hoc_without_manual_acceptance() -> None:
  try:
    validate_manual_acceptance_gate(
      signing_mode="ad-hoc",
      dry_run=False,
      allow_untrusted_ad_hoc=True,
      manual_acceptance_confirmed=False,
      manual_acceptance_evidence="",
      state=ReleaseState(draft=False, prerelease=True),
    )
  except ValueError:
    return
  raise AssertionError("visible ad-hoc publishing should require manual acceptance")


def assert_rejects_visible_ad_hoc_without_acceptance_evidence() -> None:
  try:
    validate_manual_acceptance_gate(
      signing_mode="ad-hoc",
      dry_run=False,
      allow_untrusted_ad_hoc=True,
      manual_acceptance_confirmed=True,
      manual_acceptance_evidence="",
      state=ReleaseState(draft=False, prerelease=True),
    )
  except ValueError:
    return
  raise AssertionError("visible ad-hoc publishing should require acceptance evidence")


def assert_rejects_visible_ad_hoc_with_placeholder_evidence() -> None:
  for evidence in (
    "<manual-acceptance-receipt-url>",
    "TODO",
    "not recorded",
    "placeholder",
    "https://github.com/walt1012/pith/actions/runs/100",
    "https://example.com/manual-acceptance-receipt",
  ):
    try:
      validate_manual_acceptance_gate(
        signing_mode="ad-hoc",
        dry_run=False,
        allow_untrusted_ad_hoc=True,
        manual_acceptance_confirmed=True,
        manual_acceptance_evidence=evidence,
        state=ReleaseState(draft=False, prerelease=True),
      )
    except ValueError:
      continue
    raise AssertionError("visible ad-hoc publishing should reject placeholder evidence")


def assert_manual_acceptance_gate_allows_safe_modes() -> None:
  for signing_mode, dry_run, draft, confirmed, evidence in (
    ("ad-hoc", True, False, False, ""),
    ("ad-hoc", False, True, False, ""),
    ("ad-hoc", False, False, True, "https://github.com/walt1012/pith/issues/1#manual-acceptance-receipt"),
    ("developer-id", False, False, False, ""),
  ):
    validate_manual_acceptance_gate(
      signing_mode=signing_mode,
      dry_run=dry_run,
      allow_untrusted_ad_hoc=True,
      manual_acceptance_confirmed=confirmed,
      manual_acceptance_evidence=evidence,
      state=ReleaseState(draft=draft, prerelease=signing_mode != "developer-id"),
    )


def assert_rejects_unknown_existing_release_state() -> None:
  try:
    plan_release_state(
      signing_mode="developer-id",
      requested_draft=False,
      requested_prerelease=False,
      allow_untrusted_ad_hoc=False,
      release_exists=True,
      existing_draft=None,
    )
  except ValueError:
    return
  raise AssertionError("existing release updates require known draft state")


def assert_rejects_public_release_to_draft() -> None:
  try:
    plan_release_state(
      signing_mode="developer-id",
      requested_draft=True,
      requested_prerelease=False,
      allow_untrusted_ad_hoc=False,
      release_exists=True,
      existing_draft=False,
    )
  except ValueError:
    return
  raise AssertionError("public release updates should not move back to draft")


def assert_rejects_tampered_release_notes() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    notes_file.write_text(
      release_notes("v0.1.0", "developer-id", False, False).replace(
        "README-FIRST.txt",
        "install guide",
      ),
      encoding="utf-8",
    )
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_state.py",
        "--title",
        "Pith v0.1.0",
        "--tag",
        "v0.1.0",
        "--notes-file",
        str(notes_file),
        "--signing-mode",
        "developer-id",
        "--requested-draft",
        "false",
        "--requested-prerelease",
        "false",
        "--allow-untrusted-ad-hoc",
        "false",
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
      ]
      with redirect_stderr(StringIO()):
        exit_code = release_state_main()
      if exit_code == 0:
        raise AssertionError("tampered release notes should be rejected")
    finally:
      sys.argv = original_argv


def assert_rejects_wrong_release_title() -> None:
  try:
    validate_release_title("Pith latest", "v0.1.0")
  except ValueError:
    return
  raise AssertionError("release title must match the release tag")


def assert_rejects_non_release_tag() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    notes_file.write_text("Pith latest\n", encoding="utf-8")
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_state.py",
        "--title",
        "Pith latest",
        "--tag",
        "latest",
        "--notes-file",
        str(notes_file),
        "--signing-mode",
        "developer-id",
        "--requested-draft",
        "false",
        "--requested-prerelease",
        "false",
        "--allow-untrusted-ad-hoc",
        "false",
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
      ]
      with redirect_stderr(StringIO()):
        exit_code = release_state_main()
      if exit_code == 0:
        raise AssertionError("non-version release tags should be rejected")
    finally:
      sys.argv = original_argv


def assert_release_summary_names_visibility_and_trust() -> None:
  summary = release_state_summary(
    tag="v0.1.0",
    title="Pith v0.1.0",
    source_commit="0123456789abcdef0123456789abcdef01234567",
    ci_run_url="https://github.com/walt1012/pith/actions/runs/100",
    workflow_run_url="https://github.com/walt1012/pith/actions/runs/101",
    dry_run=True,
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=True,
    manual_acceptance_confirmed=False,
    manual_acceptance_evidence="not recorded",
    release_exists=False,
    existing_draft=None,
    state=plan_release_state(
      signing_mode="ad-hoc",
      requested_draft=False,
      requested_prerelease=False,
      allow_untrusted_ad_hoc=True,
      release_exists=False,
      existing_draft=None,
    ),
  )
  for phrase in (
    "# Release Plan",
    "0123456789abcdef0123456789abcdef01234567",
    "https://github.com/walt1012/pith/actions/runs/100",
    "https://github.com/walt1012/pith/actions/runs/101",
    "Workflow mode: `dry-run`",
    "GitHub mutation: none; dry-run does not create or update a GitHub Release",
    "## Next Maintainer Actions",
    "release-dry-run-*",
    "manual prerelease acceptance checklist",
    "Planned final visibility: `visible prerelease`",
    "Allow visible ad-hoc: `true`",
    "Manual acceptance confirmed: `false`",
    "Manual acceptance receipt: not recorded",
    "Untrusted ad-hoc prerelease",
    "Open Anyway",
  ):
    if phrase not in summary:
      raise AssertionError(f"release summary should include {phrase}")


def assert_release_plan_json_preserves_release_decision() -> None:
  state = plan_release_state(
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=True,
    release_exists=False,
    existing_draft=None,
  )
  plan = release_plan_json(
    tag="v0.1.0",
    title="Pith v0.1.0",
    source_commit="0123456789abcdef0123456789abcdef01234567",
    ci_run_url="https://github.com/walt1012/pith/actions/runs/100",
    workflow_run_url="https://github.com/walt1012/pith/actions/runs/101",
    dry_run=True,
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=True,
    manual_acceptance_confirmed=False,
    manual_acceptance_evidence="",
    release_exists=False,
    existing_draft=None,
    state=state,
  )
  expected_values = {
    "workflowMode": "dry-run",
    "githubMutation": "none",
    "signingMode": "ad-hoc",
    "existingReleaseState": "none",
    "allowVisibleAdHoc": True,
    "manualAcceptanceConfirmed": False,
    "plannedDraft": False,
    "plannedPrerelease": True,
  }
  for key, expected in expected_values.items():
    if plan.get(key) != expected:
      raise AssertionError(f"release plan JSON {key} should be {expected!r}")
  actions = plan.get("nextMaintainerActions")
  if not isinstance(actions, list) or not actions:
    raise AssertionError("release plan JSON should include next maintainer actions")
  if not any("manual prerelease acceptance" in action for action in actions):
    raise AssertionError("release plan JSON should preserve manual acceptance guidance")
  if "Untrusted ad-hoc prerelease" not in str(plan.get("trustPath", "")):
    raise AssertionError("release plan JSON should preserve trust guidance")


def assert_release_body_records_visible_ad_hoc_receipt() -> None:
  notes = release_notes("v0.1.0", "ad-hoc", True, False)
  body = release_body(
    notes,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=True,
    manual_acceptance_evidence=ACCEPTANCE_RECEIPT,
    state=ReleaseState(draft=False, prerelease=True),
  )
  if "## Manual Acceptance" not in body:
    raise AssertionError("visible ad-hoc release body should include manual acceptance")
  if ACCEPTANCE_RECEIPT not in body:
    raise AssertionError("visible ad-hoc release body should include acceptance receipt")
  draft_body = release_body(
    notes,
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=True,
    manual_acceptance_evidence=ACCEPTANCE_RECEIPT,
    state=ReleaseState(draft=True, prerelease=True),
  )
  if "## Manual Acceptance" in draft_body:
    raise AssertionError("draft release body should not expose manual acceptance as published proof")


def assert_main_writes_release_summary() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    summary_file = root / "release-plan.md"
    plan_file = root / "release-plan.json"
    notes_file.write_text(
      release_notes("v0.1.0", "ad-hoc", True, False),
      encoding="utf-8",
    )
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_state.py",
        "--title",
        "Pith v0.1.0",
        "--tag",
        "v0.1.0",
        "--notes-file",
        str(notes_file),
        "--signing-mode",
        "ad-hoc",
        "--requested-draft",
        "false",
        "--requested-prerelease",
        "false",
        "--allow-untrusted-ad-hoc",
        "true",
        "--manual-acceptance-confirmed",
        "false",
        "--manual-acceptance-evidence",
        "",
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
        "--summary-output",
        str(summary_file),
        "--plan-output",
        str(plan_file),
        "--source-commit",
        "0123456789abcdef0123456789abcdef01234567",
        "--ci-run-url",
        "https://github.com/walt1012/pith/actions/runs/100",
        "--workflow-run-url",
        "https://github.com/walt1012/pith/actions/runs/101",
        "--dry-run",
        "true",
      ]
      exit_code = release_state_main()
      if exit_code != 0:
        raise AssertionError("release state summary command should pass")
    finally:
      sys.argv = original_argv
    summary = summary_file.read_text(encoding="utf-8")
    if "Planned final visibility: `visible prerelease`" not in summary:
      raise AssertionError("release summary should record planned final visibility")
    if "https://github.com/walt1012/pith/actions/runs/100" not in summary:
      raise AssertionError("release summary should record the successful CI run")
    if "Workflow mode: `dry-run`" not in summary:
      raise AssertionError("release summary should record the workflow mode")
    if "GitHub mutation: none" not in summary:
      raise AssertionError("release summary should record dry-run mutation behavior")
    if "release-dry-run-*" not in summary:
      raise AssertionError("release summary should include dry-run next actions")
    if "PITH_RELEASE_STATE_DRAFT=false" not in env_file.read_text(encoding="utf-8"):
      raise AssertionError("release env should record final draft state")
    plan = json.loads(plan_file.read_text(encoding="utf-8"))
    if plan["workflowMode"] != "dry-run":
      raise AssertionError("release plan JSON should record workflow mode")
    if plan["sourceCommit"] != "0123456789abcdef0123456789abcdef01234567":
      raise AssertionError("release plan JSON should record source commit")
    if plan["plannedDraft"] is not False:
      raise AssertionError("release plan JSON should record final draft state")


def assert_main_rejects_unaccepted_visible_ad_hoc_publish() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    notes_file.write_text(
      release_notes("v0.1.0", "ad-hoc", True, False),
      encoding="utf-8",
    )
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_state.py",
        "--title",
        "Pith v0.1.0",
        "--tag",
        "v0.1.0",
        "--notes-file",
        str(notes_file),
        "--signing-mode",
        "ad-hoc",
        "--requested-draft",
        "false",
        "--requested-prerelease",
        "false",
        "--allow-untrusted-ad-hoc",
        "true",
        "--manual-acceptance-confirmed",
        "false",
        "--manual-acceptance-evidence",
        "",
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
      ]
      with redirect_stderr(StringIO()) as stderr:
        exit_code = release_state_main()
      if exit_code == 0:
        raise AssertionError("unaccepted visible ad-hoc publish should be rejected")
      if "manual_acceptance_confirmed=true" not in stderr.getvalue():
        raise AssertionError("manual acceptance rejection should explain the required input")
    finally:
      sys.argv = original_argv


def assert_main_rejects_visible_ad_hoc_without_acceptance_evidence() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    notes_file.write_text(
      release_notes("v0.1.0", "ad-hoc", True, False),
      encoding="utf-8",
    )
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_state.py",
        "--title",
        "Pith v0.1.0",
        "--tag",
        "v0.1.0",
        "--notes-file",
        str(notes_file),
        "--signing-mode",
        "ad-hoc",
        "--requested-draft",
        "false",
        "--requested-prerelease",
        "false",
        "--allow-untrusted-ad-hoc",
        "true",
        "--manual-acceptance-confirmed",
        "true",
        "--manual-acceptance-evidence",
        "",
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
      ]
      with redirect_stderr(StringIO()) as stderr:
        exit_code = release_state_main()
      if exit_code == 0:
        raise AssertionError("visible ad-hoc publish without evidence should be rejected")
      if "manual acceptance receipt" not in stderr.getvalue():
        raise AssertionError("acceptance receipt rejection should explain the required input")
    finally:
      sys.argv = original_argv


def assert_main_rejects_visible_ad_hoc_with_placeholder_evidence() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    notes_file.write_text(
      release_notes("v0.1.0", "ad-hoc", True, False),
      encoding="utf-8",
    )
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_state.py",
        "--title",
        "Pith v0.1.0",
        "--tag",
        "v0.1.0",
        "--notes-file",
        str(notes_file),
        "--signing-mode",
        "ad-hoc",
        "--requested-draft",
        "false",
        "--requested-prerelease",
        "false",
        "--allow-untrusted-ad-hoc",
        "true",
        "--manual-acceptance-confirmed",
        "true",
        "--manual-acceptance-evidence",
        "<manual-acceptance-receipt-url>",
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
      ]
      with redirect_stderr(StringIO()) as stderr:
        exit_code = release_state_main()
      if exit_code == 0:
        raise AssertionError("visible ad-hoc publish with placeholder evidence should be rejected")
      if "real manual acceptance receipt" not in stderr.getvalue():
        raise AssertionError("placeholder receipt rejection should explain the required input")
    finally:
      sys.argv = original_argv


def assert_main_allows_accepted_visible_ad_hoc_publish() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    notes_file.write_text(
      release_notes("v0.1.0", "ad-hoc", True, False),
      encoding="utf-8",
    )
    original_argv = sys.argv[:]
    try:
      sys.argv = [
        "release_state.py",
        "--title",
        "Pith v0.1.0",
        "--tag",
        "v0.1.0",
        "--notes-file",
        str(notes_file),
        "--signing-mode",
        "ad-hoc",
        "--requested-draft",
        "false",
        "--requested-prerelease",
        "false",
        "--allow-untrusted-ad-hoc",
        "true",
        "--manual-acceptance-confirmed",
        "true",
        "--manual-acceptance-evidence",
        ACCEPTANCE_RECEIPT,
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
      ]
      exit_code = release_state_main()
      if exit_code != 0:
        raise AssertionError("accepted visible ad-hoc publish should pass")
    finally:
      sys.argv = original_argv
    if "PITH_RELEASE_STATE_DRAFT=false" not in env_file.read_text(encoding="utf-8"):
      raise AssertionError("accepted visible ad-hoc publish should stay visible")
    state = json.loads(state_file.read_text(encoding="utf-8"))
    if ACCEPTANCE_RECEIPT not in state["body"]:
      raise AssertionError("accepted visible ad-hoc release body should include receipt")


def assert_release_next_actions_match_mode() -> None:
  dry_run_actions = release_next_actions(
    dry_run=True,
    state=ReleaseState(draft=False, prerelease=True),
  )
  if "release-dry-run-*" not in dry_run_actions:
    raise AssertionError("dry-run next actions should point to the dry-run artifact")
  draft_actions = release_next_actions(
    dry_run=False,
    state=ReleaseState(draft=True, prerelease=True),
  )
  if "draft GitHub Release" not in draft_actions:
    raise AssertionError("draft publish next actions should keep draft review visible")
  visible_actions = release_next_actions(
    dry_run=False,
    state=ReleaseState(draft=False, prerelease=True),
  )
  if "visible GitHub Release page" not in visible_actions:
    raise AssertionError("visible publish next actions should inspect the release page")
  if "recorded manual acceptance receipt" not in visible_actions:
    raise AssertionError("visible publish next actions should confirm recorded acceptance")

def main() -> int:
  if expected_release_title("v0.1.0") != "Pith v0.1.0":
    raise AssertionError("release title should be derived from the tag")
  validate_release_title("Pith v0.1.0", "v0.1.0")
  assert_state(
    signing_mode="developer-id",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=False,
    release_exists=False,
    existing_draft=None,
    expected_draft=False,
    expected_prerelease=False,
  )
  assert_state(
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=False,
    release_exists=False,
    existing_draft=None,
    expected_draft=True,
    expected_prerelease=True,
  )
  assert_state(
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=True,
    release_exists=False,
    existing_draft=None,
    expected_draft=False,
    expected_prerelease=True,
  )
  assert_rejects_unknown_existing_release_state()
  assert_rejects_public_release_to_draft()
  assert_rejects_public_ad_hoc_without_explicit_publish()
  assert_rejects_visible_ad_hoc_without_manual_acceptance()
  assert_rejects_visible_ad_hoc_with_placeholder_evidence()
  assert_manual_acceptance_gate_allows_safe_modes()
  assert_rejects_tampered_release_notes()
  assert_rejects_wrong_release_title()
  assert_rejects_non_release_tag()
  assert_release_summary_names_visibility_and_trust()
  assert_release_plan_json_preserves_release_decision()
  assert_release_body_records_visible_ad_hoc_receipt()
  assert_main_writes_release_summary()
  assert_main_rejects_unaccepted_visible_ad_hoc_publish()
  assert_main_rejects_visible_ad_hoc_without_acceptance_evidence()
  assert_main_rejects_visible_ad_hoc_with_placeholder_evidence()
  assert_main_allows_accepted_visible_ad_hoc_publish()
  assert_release_next_actions_match_mode()
  assert_state(
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=True,
    release_exists=True,
    existing_draft=False,
    expected_draft=False,
    expected_prerelease=True,
  )
  assert_state(
    signing_mode="developer-id",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=False,
    release_exists=True,
    existing_draft=False,
    expected_draft=False,
    expected_prerelease=False,
  )
  print("release state tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
