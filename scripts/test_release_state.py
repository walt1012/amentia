#!/usr/bin/env python3
"""Unit checks for release state planning."""

from __future__ import annotations

from release_state import plan_release_state


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


def main() -> int:
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
  assert_state(
    signing_mode="ad-hoc",
    requested_draft=False,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=False,
    release_exists=True,
    existing_draft=False,
    expected_draft=False,
    expected_prerelease=True,
  )
  assert_state(
    signing_mode="developer-id",
    requested_draft=True,
    requested_prerelease=False,
    allow_untrusted_ad_hoc=False,
    release_exists=True,
    existing_draft=False,
    expected_draft=True,
    expected_prerelease=False,
  )
  print("release state tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
