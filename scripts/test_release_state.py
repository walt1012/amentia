#!/usr/bin/env python3
"""Unit checks for release state planning."""

from __future__ import annotations

import sys
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from release_state import main as release_state_main
from release_state import expected_release_title
from release_state import plan_release_state
from release_state import release_state_summary
from release_state import validate_release_title
from release_text import release_notes


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
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=True,
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
    "Final visibility: `visible prerelease`",
    "Allow visible ad-hoc: `true`",
    "Untrusted ad-hoc prerelease",
    "Open Anyway",
  ):
    if phrase not in summary:
      raise AssertionError(f"release summary should include {phrase}")


def assert_main_writes_release_summary() -> None:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    notes_file = root / "release-notes.md"
    state_file = root / "release-state.json"
    env_file = root / "release-state.env"
    summary_file = root / "release-plan.md"
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
        "--release-exists",
        "false",
        "--state-output",
        str(state_file),
        "--env-output",
        str(env_file),
        "--summary-output",
        str(summary_file),
      ]
      exit_code = release_state_main()
      if exit_code != 0:
        raise AssertionError("release state summary command should pass")
    finally:
      sys.argv = original_argv
    summary = summary_file.read_text(encoding="utf-8")
    if "Final visibility: `visible prerelease`" not in summary:
      raise AssertionError("release summary should record final visibility")
    if "PITH_RELEASE_STATE_DRAFT=false" not in env_file.read_text(encoding="utf-8"):
      raise AssertionError("release env should record final draft state")


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
  assert_rejects_tampered_release_notes()
  assert_rejects_wrong_release_title()
  assert_rejects_non_release_tag()
  assert_release_summary_names_visibility_and_trust()
  assert_main_writes_release_summary()
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
