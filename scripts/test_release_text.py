#!/usr/bin/env python3
"""Unit checks for release copy generation."""

from __future__ import annotations

from package_contract import (
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  DEFAULT_MODEL_ID,
  MINIMUM_SYSTEM_VERSION,
  PACKAGED_SMOKE_PROOF_SCOPE,
  SUPPORTED_ARCH,
)
from release_copy_contract import (
  FIRST_APP_OPEN_ACTION_COPY,
  FIRST_APP_OPEN_CONTRACT_ID,
  FIRST_APP_OPEN_INSTALL_STEP,
  INSTALL_GUIDE_REQUIRED_PHRASES,
  PACKAGED_FIRST_RUN_RECEIPT_PHRASE,
  RELEASE_NOTES_REQUIRED_PHRASES,
  missing_required_phrases,
  require_install_guide_copy,
  require_release_copy,
)
from release_text import (
  first_app_open_action_clause,
  first_app_open_action_sentence,
  first_run_path_copy,
  first_run_receipt_copy,
  install_guide,
  installer_assets_copy,
  local_execution_copy,
  platform_label,
  release_notes,
  release_size_budget_copy,
  checksum_verification_copy,
  validate_install_guide,
  validate_release_notes,
)


def require_contains(text: str, expected: str) -> None:
  if expected not in text:
    raise AssertionError(f"expected text to contain {expected!r}")


def require_not_contains(text: str, unexpected: str) -> None:
  if unexpected in text:
    raise AssertionError(f"expected text not to contain {unexpected!r}")


def require_raises(action, expected: str) -> None:
  try:
    action()
  except RuntimeError as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r}, got {error!r}") from error
    return
  raise AssertionError(f"expected failure containing {expected!r}")


def main() -> int:
  require_contains(platform_label(), MINIMUM_SYSTEM_VERSION)
  require_contains(platform_label(), SUPPORTED_ARCH)
  require_contains(release_size_budget_copy(), "app <= 250 MiB")
  require_contains(release_size_budget_copy(), "installer artifact <= 150 MiB")
  require_contains(local_execution_copy(), "No Amentia login is required")
  require_contains(local_execution_copy(), DEFAULT_LOCAL_EXECUTION_SAFETY_MODE)
  require_contains(FIRST_APP_OPEN_ACTION_COPY, "Understand Project")
  require_contains(FIRST_APP_OPEN_ACTION_COPY, "Pick Next Step")
  require_contains(FIRST_APP_OPEN_CONTRACT_ID, "map-plan")
  require_contains(first_app_open_action_sentence(), FIRST_APP_OPEN_INSTALL_STEP)
  require_contains(first_app_open_action_clause(), "start coworking")
  require_contains(" ".join(RELEASE_NOTES_REQUIRED_PHRASES), "DMG installer.")
  require_contains(" ".join(RELEASE_NOTES_REQUIRED_PHRASES), "SHA-256 checksum sidecar")
  require_contains(" ".join(RELEASE_NOTES_REQUIRED_PHRASES), "same download folder")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), "shasum -a 256 -c")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), "same download folder")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), "first-run contract")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), "app package metadata")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), "smoke package metadata")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), PACKAGED_FIRST_RUN_RECEIPT_PHRASE)
  if missing_required_phrases("alpha beta", ("alpha", "gamma")) != ["gamma"]:
    raise AssertionError("release copy missing phrase helper should preserve missing phrases")
  require_release_copy("alpha beta", ("alpha",), "test copy")
  require_raises(
    lambda: require_install_guide_copy("Install Amentia.", "test install guide"),
    "test install guide is missing required copy",
  )
  require_contains(first_run_path_copy(), DEFAULT_MODEL_ID)
  require_contains(first_run_path_copy(), "Web Search")
  require_contains(first_run_path_copy(), "project safety")
  require_contains(first_run_path_copy(), first_app_open_action_clause())
  require_contains(first_run_path_copy(), "approve a safe local change")
  require_contains(first_run_path_copy(), "inspect the timeline receipt")
  require_contains(first_run_receipt_copy(), PACKAGED_FIRST_RUN_RECEIPT_PHRASE)
  require_contains(first_run_receipt_copy(), PACKAGED_SMOKE_PROOF_SCOPE)
  require_contains(PACKAGED_SMOKE_PROOF_SCOPE, "first cowork request")
  require_contains(installer_assets_copy("v0.1.0"), "Amentia-v0.1.0-macos-x86_64.dmg")
  require_contains(installer_assets_copy("v0.1.0"), "Amentia-v0.1.0-release-manifest.json")
  require_contains(installer_assets_copy("ci-0123456789ab"), "Amentia-macos-x86_64.dmg")
  require_contains(installer_assets_copy("ci-0123456789ab"), "internal-release-manifest.json")
  require_contains(checksum_verification_copy("v0.1.0"), "shasum -a 256 -c")
  require_contains(checksum_verification_copy("v0.1.0"), "same download folder")
  require_contains(checksum_verification_copy("v0.1.0"), "from that folder")
  require_contains(checksum_verification_copy("v0.1.0"), "Amentia-v0.1.0-macos-x86_64.dmg.sha256")
  require_contains(checksum_verification_copy("v0.1.0"), "Amentia-v0.1.0-release-manifest.json")
  require_contains(checksum_verification_copy("v0.1.0"), "app package metadata")
  require_contains(checksum_verification_copy("v0.1.0"), "smoke package metadata")
  require_contains(checksum_verification_copy("v0.1.0"), "first-run contract")
  require_contains(checksum_verification_copy("v0.1.0"), PACKAGED_FIRST_RUN_RECEIPT_PHRASE)

  developer_notes = release_notes(
    "v0.1.0",
    "developer-id",
    allow_untrusted_ad_hoc=False,
    draft=False,
  )
  require_contains(developer_notes, "Developer ID signed and notarized.")
  require_contains(developer_notes, platform_label())
  require_contains(developer_notes, DEFAULT_MODEL_ID)
  require_contains(developer_notes, "Installer assets:")
  require_contains(developer_notes, "Amentia-v0.1.0-macos-x86_64.dmg")
  require_contains(developer_notes, "Amentia-v0.1.0-release-manifest.json")
  require_contains(developer_notes, "SHA-256 checksum sidecar")
  require_contains(developer_notes, "README-FIRST.txt")
  require_contains(developer_notes, "release manifest")
  require_contains(developer_notes, "same download folder")
  require_contains(developer_notes, "Read README-FIRST.txt")
  require_contains(developer_notes, "Gatekeeper")
  require_contains(developer_notes, "verification steps")
  require_not_contains(developer_notes, first_run_path_copy())
  require_not_contains(developer_notes, first_run_receipt_copy())
  require_not_contains(developer_notes, "Native sandbox")
  require_not_contains(developer_notes, "process-only fallback")
  require_not_contains(developer_notes, "daily-driver next action")
  require_not_contains(developer_notes, "runtime readiness")
  require_not_contains(developer_notes, "app header and inspector")
  require_not_contains(developer_notes, release_size_budget_copy())
  require_not_contains(developer_notes, "Open Anyway")
  validate_release_notes(
    developer_notes,
    tag="v0.1.0",
    signing_mode="developer-id",
    allow_untrusted_ad_hoc=False,
    draft=False,
  )

  ad_hoc_notes = release_notes(
    "v0.1.0",
    "ad-hoc",
    allow_untrusted_ad_hoc=True,
    draft=False,
  )
  require_contains(ad_hoc_notes, "Untrusted ad-hoc prerelease.")
  require_contains(ad_hoc_notes, "Open Anyway")
  require_contains(ad_hoc_notes, "Control-clicks Amentia.app")
  validate_release_notes(
    ad_hoc_notes,
    tag="v0.1.0",
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=True,
    draft=False,
  )

  draft_notes = release_notes(
    "v0.1.0",
    "ad-hoc",
    allow_untrusted_ad_hoc=False,
    draft=True,
  )
  require_contains(draft_notes, "Draft ad-hoc build.")
  require_not_contains(draft_notes, "Untrusted ad-hoc prerelease.")
  validate_release_notes(
    draft_notes,
    tag="v0.1.0",
    signing_mode="ad-hoc",
    allow_untrusted_ad_hoc=False,
    draft=True,
  )

  guide = install_guide("v0.1.0", "ad-hoc")
  require_contains(guide, "Gatekeeper")
  require_contains(guide, "Control-click Amentia.app and choose Open.")
  require_contains(guide, "Installer assets:")
  require_contains(guide, "No Amentia login is required")
  require_contains(guide, "action safety mode")
  require_contains(guide, "Amentia-v0.1.0-macos-x86_64.dmg")
  require_contains(guide, "Amentia-v0.1.0-release-manifest.json")
  require_contains(guide, "download one verified local model")
  require_contains(guide, "First-run path:")
  require_contains(guide, DEFAULT_MODEL_ID)
  require_contains(guide, "Open a project folder.")
  require_contains(guide, "Check that Web Search and project safety are ready.")
  require_contains(guide, "Approve a safe local change only after reviewing it")
  require_contains(guide, "inspect the timeline receipt")
  require_contains(guide, FIRST_APP_OPEN_INSTALL_STEP)
  require_contains(guide, "Follow the next action")
  require_contains(guide, "Amentia status")
  require_contains(guide, "SHA-256 `.sha256` file")
  require_contains(guide, "same download folder")
  require_contains(guide, "shasum -a 256 -c Amentia-v0.1.0-macos-x86_64.dmg.sha256")
  require_contains(guide, "verify the downloaded installer")
  require_contains(guide, "sidecar hashes")
  require_contains(guide, first_run_receipt_copy())
  require_contains(guide, "source commit")
  require_contains(guide, "model delivery mode")
  require_contains(guide, "app package metadata")
  require_contains(guide, "smoke package metadata")
  require_contains(guide, "first-run contract")
  require_contains(guide, PACKAGED_SMOKE_PROOF_SCOPE)
  require_contains(guide, release_size_budget_copy())
  require_contains(guide, "sandbox status")
  require_contains(guide, "process-only fallback")
  validate_install_guide(guide, tag="v0.1.0", signing_mode="ad-hoc")

  developer_guide = install_guide("v0.1.0", "developer-id")
  require_contains(developer_guide, "Developer ID signed and notarized for normal Gatekeeper launch.")
  require_contains(developer_guide, "Gatekeeper")
  require_not_contains(developer_guide, "Open Anyway")
  validate_install_guide(developer_guide, tag="v0.1.0", signing_mode="developer-id")

  print("release text tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
