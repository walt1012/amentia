#!/usr/bin/env python3
"""Plan GitHub Release state for Pith distribution builds."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

from package_contract import RELEASE_SIGNING_MODES
from release_text import validate_release_notes


@dataclass(frozen=True)
class ReleaseState:
  draft: bool
  prerelease: bool


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


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--title", required=True)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--notes-file", required=True)
  parser.add_argument("--signing-mode", required=True, choices=sorted(RELEASE_SIGNING_MODES))
  parser.add_argument("--requested-draft", required=True)
  parser.add_argument("--requested-prerelease", required=True)
  parser.add_argument("--allow-untrusted-ad-hoc", required=True)
  parser.add_argument("--release-exists", required=True)
  parser.add_argument("--existing-draft", default="")
  parser.add_argument("--state-output", required=True)
  parser.add_argument("--env-output", required=True)
  args = parser.parse_args()

  try:
    release_exists = parse_bool(args.release_exists)
    existing_draft = (
      parse_bool(args.existing_draft)
      if args.existing_draft.strip()
      else None
    )
    state = plan_release_state(
      signing_mode=args.signing_mode,
      requested_draft=parse_bool(args.requested_draft),
      requested_prerelease=parse_bool(args.requested_prerelease),
      allow_untrusted_ad_hoc=parse_bool(args.allow_untrusted_ad_hoc),
      release_exists=release_exists,
      existing_draft=existing_draft,
    )
  except ValueError as error:
    print(f"release state planning failed: {error}", file=sys.stderr)
    return 1

  notes = Path(args.notes_file).read_text(encoding="utf-8")
  try:
    validate_release_notes(
      notes,
      tag=args.tag,
      signing_mode=args.signing_mode,
      allow_untrusted_ad_hoc=parse_bool(args.allow_untrusted_ad_hoc),
      draft=state.draft,
    )
  except RuntimeError as error:
    print(f"release state planning failed: {error}", file=sys.stderr)
    return 1
  Path(args.state_output).write_text(
    json.dumps(
      {
        "name": args.title,
        "body": notes,
        "draft": state.draft,
        "prerelease": state.prerelease,
      }
    ),
    encoding="utf-8",
  )
  write_env(Path(args.env_output), state)
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
