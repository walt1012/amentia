#!/usr/bin/env python3
"""Unit checks for release copy generation."""

from __future__ import annotations

from package_contract import (
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  DEFAULT_MODEL_ID,
  MINIMUM_SYSTEM_VERSION,
  SUPPORTED_ARCH,
)
from release_copy_contract import (
  INSTALL_GUIDE_REQUIRED_PHRASES,
  RELEASE_NOTES_REQUIRED_PHRASES,
)
from release_text import (
  first_run_path_copy,
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


def main() -> int:
  require_contains(platform_label(), MINIMUM_SYSTEM_VERSION)
  require_contains(platform_label(), SUPPORTED_ARCH)
  require_contains(release_size_budget_copy(), "app <= 250 MiB")
  require_contains(release_size_budget_copy(), "installer artifact <= 150 MiB")
  require_contains(local_execution_copy(), "No Pith login is required")
  require_contains(local_execution_copy(), DEFAULT_LOCAL_EXECUTION_SAFETY_MODE)
  require_contains(" ".join(RELEASE_NOTES_REQUIRED_PHRASES), "DMG installer.")
  require_contains(" ".join(RELEASE_NOTES_REQUIRED_PHRASES), "SHA-256 checksum sidecar")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), "shasum -a 256 -c")
  require_contains(" ".join(INSTALL_GUIDE_REQUIRED_PHRASES), "first-run contract")
  require_contains(first_run_path_copy(), DEFAULT_MODEL_ID)
  require_contains(first_run_path_copy(), "Web Search readiness")
  require_contains(first_run_path_copy(), "approve a safe local change")
  require_contains(first_run_path_copy(), "inspect the proof")
  require_contains(installer_assets_copy("v0.1.0"), "Pith-v0.1.0-macos-x86_64.dmg")
  require_contains(installer_assets_copy("v0.1.0"), "Pith-v0.1.0-release-manifest.json")
  require_contains(installer_assets_copy("ci-0123456789ab"), "Pith-macos-x86_64.dmg")
  require_contains(installer_assets_copy("ci-0123456789ab"), "internal-release-manifest.json")
  require_contains(checksum_verification_copy("v0.1.0"), "shasum -a 256 -c")
  require_contains(checksum_verification_copy("v0.1.0"), "Pith-v0.1.0-macos-x86_64.dmg.sha256")
  require_contains(checksum_verification_copy("v0.1.0"), "Pith-v0.1.0-release-manifest.json")
  require_contains(checksum_verification_copy("v0.1.0"), "first-run contract")

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
  require_contains(developer_notes, "No Pith login is required")
  require_contains(developer_notes, "local execution mode")
  require_contains(developer_notes, first_run_path_copy())
  require_contains(developer_notes, "Pith-v0.1.0-macos-x86_64.dmg")
  require_contains(developer_notes, "Pith-v0.1.0-release-manifest.json")
  require_contains(developer_notes, "SHA-256 checksum sidecar")
  require_contains(developer_notes, "release manifest")
  require_contains(developer_notes, "sidecar hashes")
  require_contains(developer_notes, "Native sandbox is used when available")
  require_contains(developer_notes, "process-only fallback")
  require_contains(developer_notes, "daily-driver next action")
  require_contains(developer_notes, "app header and inspector")
  require_contains(developer_notes, release_size_budget_copy())
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
  require_contains(ad_hoc_notes, "Control-clicks Pith.app")
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
  require_contains(guide, "Control-click Pith.app and choose Open.")
  require_contains(guide, "Installer assets:")
  require_contains(guide, "No Pith login is required")
  require_contains(guide, "local execution mode")
  require_contains(guide, "Pith-v0.1.0-macos-x86_64.dmg")
  require_contains(guide, "Pith-v0.1.0-release-manifest.json")
  require_contains(guide, "download one verified local model")
  require_contains(guide, "First-run path:")
  require_contains(guide, DEFAULT_MODEL_ID)
  require_contains(guide, "Open a workspace folder.")
  require_contains(guide, "Confirm Web Search readiness and sandbox status")
  require_contains(guide, "Approve a safe local change only after reviewing it")
  require_contains(guide, "inspect the proof")
  require_contains(guide, "Start a cowork session with Map Workspace, Plan Next Step")
  require_contains(guide, "Follow the next action")
  require_contains(guide, "runtime readiness")
  require_contains(guide, "SHA-256 `.sha256` file")
  require_contains(guide, "shasum -a 256 -c Pith-v0.1.0-macos-x86_64.dmg.sha256")
  require_contains(guide, "verify the downloaded installer")
  require_contains(guide, "sidecar hashes")
  require_contains(guide, "source commit")
  require_contains(guide, "model delivery mode")
  require_contains(guide, "first-run contract")
  require_contains(guide, release_size_budget_copy())
  require_contains(guide, "sandbox status")
  require_contains(guide, "process-only fallback")
  validate_install_guide(guide, tag="v0.1.0", signing_mode="ad-hoc")

  print("release text tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
