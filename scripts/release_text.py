#!/usr/bin/env python3
"""Generate release notes and DMG install guidance for Pith builds."""

from __future__ import annotations

import argparse
from pathlib import Path


RELEASE_NOTES_REQUIRED_PHRASES = (
  "macOS 12+ x86_64 DMG installer.",
  "Model weights are not bundled",
  "SHA-256 checksum sidecar",
  "README-FIRST.txt",
  "release manifest",
  "sidecar hashes",
  "Native sandbox",
  "process-only fallback",
  "daily-driver next action",
  "package size budget",
)
INSTALL_GUIDE_REQUIRED_PHRASES = (
  "Drag Pith.app to Applications.",
  "download one verified local model",
  "Open a workspace folder.",
  "Start a cowork session",
  "Follow the next action",
  "runtime readiness",
  "sandbox status",
  "process-only fallback",
  "SHA-256 `.sha256` file",
  "sidecar hashes",
  "source commit",
  "model delivery mode",
  "package size budget",
)


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
      "Privacy & Security or Control-clicks Pith.app and chooses Open."
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
- The daily-driver next action comes from runtime readiness and appears in the app header and inspector.
- Native sandbox is used when available; process-only fallback is disclosed in app status.
- The package size budget is enforced so model weights and heavyweight payloads stay out of the app.
- SHA-256 checksum sidecar is published next to the DMG.
- README-FIRST.txt and the release manifest are published as separate assets for pre-install review, including sidecar hashes.
- {trust_note}
"""


def validate_release_notes(
  text: str,
  *,
  tag: str,
  signing_mode: str,
  allow_untrusted_ad_hoc: bool,
  draft: bool,
) -> None:
  require_phrases(text, (f"Pith {tag}", *RELEASE_NOTES_REQUIRED_PHRASES), "release notes")
  trust_note = release_trust_note(signing_mode, allow_untrusted_ad_hoc, draft)
  require_phrases(text, (trust_note,), "release notes")
  if signing_mode == "developer-id" and "Open Anyway" in text:
    raise RuntimeError("Developer ID release notes must not mention manual Gatekeeper override")


def install_guide(tag: str, signing_mode: str) -> str:
  trust_note, open_note = install_trust_section(signing_mode)
  return f"""Pith {tag}

Install
1. Open this DMG.
2. Drag Pith.app to Applications.
3. Launch Pith and download one verified local model when prompted.
4. Open a workspace folder.
5. Start a cowork session with Map Workspace, Plan Next Step, or your own first request.
6. Follow the next action shown by Pith; it comes from runtime readiness, not a static setup checklist.

Trust
{trust_note}
{open_note}

Notes
- Pith runs local model work on this Mac.
- Model weights are not bundled in the app package.
- The SHA-256 `.sha256` file next to the DMG lets users verify the downloaded installer.
- The release manifest lists the DMG checksum, sidecar hashes, platform target, source commit, signing mode, and model delivery mode.
- The release manifest records the package size budget that CI enforces before upload.
- Pith reports sandbox status in app; native sandbox is used when available, otherwise process-only fallback keeps bounded execution visible.
- Only one local model runs at a time.
- Short, specific first requests work best with the default small local model.
"""


def validate_install_guide(text: str, *, tag: str, signing_mode: str) -> None:
  trust_note, open_note = install_trust_section(signing_mode)
  require_phrases(
    text,
    (f"Pith {tag}", *INSTALL_GUIDE_REQUIRED_PHRASES, trust_note, open_note),
    "install guide",
  )


def require_phrases(text: str, phrases: tuple[str, ...], label: str) -> None:
  missing = [
    phrase
    for phrase in phrases
    if phrase not in text
  ]
  if missing:
    raise RuntimeError(f"{label} is missing required copy: {', '.join(missing)}")


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
  guide_text = install_guide(args.tag, args.signing_mode)
  notes_text = release_notes(args.tag, args.signing_mode, allow_untrusted_ad_hoc, draft)
  validate_install_guide(guide_text, tag=args.tag, signing_mode=args.signing_mode)
  validate_release_notes(
    notes_text,
    tag=args.tag,
    signing_mode=args.signing_mode,
    allow_untrusted_ad_hoc=allow_untrusted_ad_hoc,
    draft=draft,
  )
  Path(args.install_guide_output).write_text(guide_text, encoding="utf-8")
  Path(args.notes_output).write_text(notes_text, encoding="utf-8")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
