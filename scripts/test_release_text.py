#!/usr/bin/env python3
"""Unit checks for release copy generation."""

from __future__ import annotations

from release_text import install_guide, release_notes


def require_contains(text: str, expected: str) -> None:
  if expected not in text:
    raise AssertionError(f"expected text to contain {expected!r}")


def require_not_contains(text: str, unexpected: str) -> None:
  if unexpected in text:
    raise AssertionError(f"expected text not to contain {unexpected!r}")


def main() -> int:
  developer_notes = release_notes(
    "v0.1.0",
    "developer-id",
    allow_untrusted_ad_hoc=False,
    draft=False,
  )
  require_contains(developer_notes, "Developer ID signed and notarized.")
  require_contains(developer_notes, "SHA-256 checksum sidecar")
  require_contains(developer_notes, "release manifest")
  require_not_contains(developer_notes, "Open Anyway")

  ad_hoc_notes = release_notes(
    "v0.1.0",
    "ad-hoc",
    allow_untrusted_ad_hoc=True,
    draft=False,
  )
  require_contains(ad_hoc_notes, "Untrusted ad-hoc prerelease.")
  require_contains(ad_hoc_notes, "Open Anyway")

  draft_notes = release_notes(
    "v0.1.0",
    "ad-hoc",
    allow_untrusted_ad_hoc=False,
    draft=True,
  )
  require_contains(draft_notes, "Draft ad-hoc build.")
  require_not_contains(draft_notes, "Untrusted ad-hoc prerelease.")

  guide = install_guide("v0.1.0", "ad-hoc")
  require_contains(guide, "Control-click Pith.app and choose Open.")
  require_contains(guide, "download one verified local model")
  require_contains(guide, "Open a workspace folder.")
  require_contains(guide, "Start a cowork session with Map Workspace, Plan Next Step")
  require_contains(guide, "SHA-256 `.sha256` file")
  require_contains(guide, "verify the downloaded installer")
  require_contains(guide, "model delivery mode")

  print("release text tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
