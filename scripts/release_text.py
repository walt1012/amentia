#!/usr/bin/env python3
"""Generate release notes and DMG install guidance for Pith builds."""

from __future__ import annotations

import argparse
from pathlib import Path


def parse_bool(value: str) -> bool:
  normalized = value.strip().lower()
  if normalized in {"1", "true", "yes", "on"}:
    return True
  if normalized in {"0", "false", "no", "off", ""}:
    return False
  raise ValueError(f"invalid boolean value: {value!r}")


def release_trust_note(
  signing_mode: str,
  allow_untrusted_ad_hoc: bool,
  draft: bool,
) -> str:
  if signing_mode == "developer-id":
    return "Developer ID signed and notarized."
  if allow_untrusted_ad_hoc and not draft:
    return (
      "Untrusted ad-hoc prerelease. This DMG is not notarized; macOS Gatekeeper "
      "will block first launch until the user manually chooses Open Anyway in "
      "Privacy & Security."
    )
  return (
    "Draft ad-hoc build. Public releases need Developer ID signing and "
    "notarization before users should install by default."
  )


def install_trust_section(signing_mode: str) -> tuple[str, str]:
  if signing_mode == "developer-id":
    return (
      "This build is Developer ID signed and notarized.",
      "Open the DMG, drag Pith.app to Applications, then launch Pith.",
    )
  return (
    "This build is ad-hoc signed and not notarized.",
    "After dragging Pith.app to Applications, macOS may block first launch. "
    "Open System Settings > Privacy & Security and choose Open Anyway, or "
    "Control-click Pith.app and choose Open.",
  )


def release_notes(
  tag: str,
  signing_mode: str,
  allow_untrusted_ad_hoc: bool,
  draft: bool,
) -> str:
  trust_note = release_trust_note(signing_mode, allow_untrusted_ad_hoc, draft)
  return f"""Pith {tag}

- macOS 12+ x86_64 DMG installer.
- Local-first app bundle with runtime, plugin manifests, model metadata, and llama.cpp backend.
- Model weights are not bundled; first launch guides the user to download one verified local model.
- {trust_note}
"""


def install_guide(tag: str, signing_mode: str) -> str:
  trust_note, open_note = install_trust_section(signing_mode)
  return f"""Pith {tag}

Install
1. Open this DMG.
2. Drag Pith.app to Applications.
3. Launch Pith and download one verified local model when prompted.
4. Open a workspace folder.
5. Start a cowork session with Map Workspace, Plan Next Step, or your own first request.

Trust
{trust_note}
{open_note}

Notes
- Pith runs local model work on this Mac.
- Model weights are not bundled in the app package.
- Only one local model runs at a time.
- Short, specific first requests work best with the default small local model.
"""


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--signing-mode", required=True, choices=["developer-id", "ad-hoc"])
  parser.add_argument("--allow-untrusted-ad-hoc", required=True)
  parser.add_argument("--draft", required=True)
  parser.add_argument("--install-guide-output", required=True)
  parser.add_argument("--notes-output", required=True)
  args = parser.parse_args()

  allow_untrusted_ad_hoc = parse_bool(args.allow_untrusted_ad_hoc)
  draft = parse_bool(args.draft)
  Path(args.install_guide_output).write_text(
    install_guide(args.tag, args.signing_mode),
    encoding="utf-8",
  )
  Path(args.notes_output).write_text(
    release_notes(args.tag, args.signing_mode, allow_untrusted_ad_hoc, draft),
    encoding="utf-8",
  )
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
