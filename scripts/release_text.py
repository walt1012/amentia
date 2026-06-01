#!/usr/bin/env python3
"""Generate release notes and DMG install guidance for Pith builds."""

from __future__ import annotations

import argparse
from pathlib import Path

from package_contract import (
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  DEFAULT_MODEL_ID,
  LOCAL_EXECUTION_SAFETY_MODES,
  MINIMUM_SYSTEM_VERSION,
  RELEASE_SIGNING_MODES,
  SUPPORTED_ARCH,
  package_size_budget,
)
from release_artifacts import release_installer_asset_names
from release_copy_contract import INSTALL_GUIDE_REQUIRED_PHRASES

RELEASE_NOTES_REQUIRED_PHRASES = (
  "DMG installer.",
  "Installer assets:",
  DEFAULT_MODEL_ID,
  "Model weights are not bundled",
  "No Pith login is required",
  "local execution mode",
  "SHA-256 checksum sidecar",
  "README-FIRST.txt",
  "release manifest",
  "sidecar hashes",
  "Native sandbox",
  "process-only fallback",
  "daily-driver next action",
  "package size budget",
)


def platform_label() -> str:
  return f"macOS {MINIMUM_SYSTEM_VERSION}+ {SUPPORTED_ARCH}"


def release_size_budget_copy() -> str:
  budget = package_size_budget()
  app_budget = mebibytes(budget["maxAppBundleBytes"])
  installer_budget = mebibytes(budget["maxZipArtifactBytes"])
  return f"package size budget: app <= {app_budget}, installer artifact <= {installer_budget}"


def installer_assets_copy(tag: str) -> str:
  dmg_name, checksum_name, guide_name, manifest_name = release_installer_asset_names(tag)
  return (
    "Installer assets: "
    f"{dmg_name}, {checksum_name}, {guide_name}, and {manifest_name}."
  )


def local_execution_copy() -> str:
  modes = ", ".join(LOCAL_EXECUTION_SAFETY_MODES)
  return (
    "No Pith login is required; local execution mode defaults to "
    f"{DEFAULT_LOCAL_EXECUTION_SAFETY_MODE}; available modes are {modes}."
  )


def first_run_path_copy() -> str:
  return (
    "First-run path: download the default verified local model "
    f"({DEFAULT_MODEL_ID}), open a workspace folder, confirm Web Search readiness "
    "and sandbox status, start a cowork session, approve a safe local change only "
    "after reviewing it, then inspect the proof shown in the timeline."
  )


def checksum_verification_copy(tag: str) -> str:
  _dmg_name, checksum_name, _guide_name, manifest_name = release_installer_asset_names(tag)
  return (
    f"Verify the installer before first launch with `shasum -a 256 -c {checksum_name}`, "
    f"then open {manifest_name} to confirm platform, signing mode, model delivery mode, "
    "and first-run contract."
  )


def mebibytes(bytes_value: int) -> str:
  one_mib = 1024 * 1024
  if bytes_value % one_mib == 0:
    return f"{bytes_value // one_mib} MiB"
  return f"{bytes_value / one_mib:.1f} MiB"


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
  size_budget = release_size_budget_copy()
  installer_assets = installer_assets_copy(tag)
  local_execution = local_execution_copy()
  first_run_path = first_run_path_copy()
  verification = checksum_verification_copy(tag)
  return f"""Pith {tag}

- {platform_label()} DMG installer.
- {installer_assets}
- Local-first app bundle with runtime, plugin manifests, model metadata, and llama.cpp backend.
- Model weights are not bundled; first launch guides the user to download one verified local model, defaulting to {DEFAULT_MODEL_ID}.
- {local_execution}
- {first_run_path}
- The daily-driver next action comes from runtime readiness and appears in the app header and inspector.
- Native sandbox is used when available; process-only fallback is disclosed in app status.
- The {size_budget} is enforced so model weights and heavyweight payloads stay out of the app.
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
  size_budget = release_size_budget_copy()
  installer_assets = installer_assets_copy(tag)
  local_execution = local_execution_copy()
  first_run_path = first_run_path_copy()
  verification = checksum_verification_copy(tag)
  return f"""Pith {tag}

Install
1. Open this DMG.
2. Drag Pith.app to Applications.
3. Launch Pith and download one verified local model when prompted; {DEFAULT_MODEL_ID} is the default.
4. Open a workspace folder.
5. Confirm Web Search readiness and sandbox status in the setup surface.
6. Start a cowork session with Map Workspace, Plan Next Step, or your own first request.
7. Approve a safe local change only after reviewing it, then inspect the proof in the timeline.
8. Follow the next action shown by Pith; it comes from runtime readiness, not a static setup checklist.

Trust
{trust_note}
{open_note}

Verify
{verification}

Notes
- {installer_assets}
- {local_execution}
- {first_run_path}
- Pith runs local model work on this Mac.
- Model weights are not bundled in the app package.
- The SHA-256 `.sha256` file next to the DMG lets users verify the downloaded installer.
- The release manifest lists the DMG checksum, sidecar hashes, platform target, source commit, signing mode, model delivery mode, and first-run contract.
- The release manifest records the {size_budget} that CI enforces before upload.
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
  parser.add_argument("--signing-mode", required=True, choices=sorted(RELEASE_SIGNING_MODES))
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
