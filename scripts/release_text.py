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
from release_copy_contract import (
  FIRST_APP_OPEN_INSTALL_STEP,
  INSTALL_GUIDE_REQUIRED_PHRASES,
  PACKAGED_FIRST_RUN_PROOF_PHRASE,
  PACKAGED_FIRST_RUN_PROOF_SCOPE,
  RELEASE_NOTES_REQUIRED_PHRASES,
  require_release_copy,
  require_release_notes_copy,
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


def first_app_open_action_sentence() -> str:
  return FIRST_APP_OPEN_INSTALL_STEP


def first_app_open_action_clause() -> str:
  sentence = first_app_open_action_sentence().rstrip(".")
  return sentence[0].lower() + sentence[1:]


def first_run_path_copy() -> str:
  return (
    "First-run path: download the default verified local model "
    f"({DEFAULT_MODEL_ID}), open a workspace folder, check Web Search and "
    f"workspace safety, {first_app_open_action_clause()}, approve a safe local "
    "change only after reviewing it, then inspect the proof shown in the timeline."
  )


def checksum_verification_copy(tag: str) -> str:
  _dmg_name, checksum_name, _guide_name, manifest_name = release_installer_asset_names(tag)
  return (
    "Keep the DMG, checksum, install guide, and manifest in the same download folder, "
    f"then verify the installer before first launch with `shasum -a 256 -c {checksum_name}` "
    f"from that folder. Open {manifest_name} to confirm platform, signing mode, model delivery mode, "
    "app package metadata, smoke package metadata, first-run contract, and "
    f"{PACKAGED_FIRST_RUN_PROOF_PHRASE}."
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
      "This build is Developer ID signed and notarized for normal Gatekeeper launch.",
      "Open the DMG, drag Pith.app to Applications, then launch Pith.",
    )
  return (
    "This build is ad-hoc signed and not notarized.",
    "After dragging Pith.app to Applications, macOS Gatekeeper may block first launch. "
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
  installer_assets = installer_assets_copy(tag)
  return f"""Pith {tag}

- {platform_label()} DMG installer.
- {installer_assets}
- Model weights are not bundled; first launch downloads one verified local model, defaulting to {DEFAULT_MODEL_ID}.
- SHA-256 checksum sidecar is published next to the DMG.
- Keep the DMG, checksum, README-FIRST.txt, and release manifest in the same download folder.
- Read README-FIRST.txt before first launch for install, Gatekeeper, and verification steps.
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
  require_release_copy(text, (f"Pith {tag}",), "release notes")
  require_release_notes_copy(text)
  trust_note = release_trust_note(signing_mode, allow_untrusted_ad_hoc, draft)
  require_release_copy(text, (trust_note,), "release notes")
  if signing_mode == "developer-id" and "Open Anyway" in text:
    raise RuntimeError("Developer ID release notes must not mention manual Gatekeeper override")


def install_guide(tag: str, signing_mode: str) -> str:
  trust_note, open_note = install_trust_section(signing_mode)
  size_budget = release_size_budget_copy()
  installer_assets = installer_assets_copy(tag)
  local_execution = local_execution_copy()
  first_run_path = first_run_path_copy()
  verification = checksum_verification_copy(tag)
  first_run_proof = first_run_proof_copy()
  return f"""Pith {tag}

Install
1. Open this DMG.
2. Drag Pith.app to Applications.
3. Launch Pith and download one verified local model when prompted; {DEFAULT_MODEL_ID} is the default.
4. Open a workspace folder.
5. Check that Web Search and workspace safety are ready.
6. {FIRST_APP_OPEN_INSTALL_STEP}
7. Approve a safe local change only after reviewing it, then inspect the proof in the timeline.
8. Follow the next action shown by Pith; it comes from local service status, not a static setup checklist.

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
- Downloaded models and Pith sessions stay in local app data. Use Settings > Storage to reveal or delete Pith local data without deleting workspace folders.
- The SHA-256 `.sha256` file next to the DMG lets users verify the downloaded installer.
- The release manifest lists the DMG checksum, sidecar hashes, platform target, source commit, signing mode, model delivery mode, app package metadata, smoke package metadata, and first-run contract.
- {first_run_proof}
- The release manifest records the {size_budget} that CI enforces before upload.
- Pith reports sandbox status in app; native sandbox is used when available, otherwise process-only fallback keeps bounded execution visible.
- Only one local model runs at a time.
- Short, specific first prompts work best with the default small local model.
"""


def first_run_proof_copy() -> str:
  return (
    f"The release manifest includes a {PACKAGED_FIRST_RUN_PROOF_PHRASE} "
    f"proving the mounted-DMG path reached {PACKAGED_FIRST_RUN_PROOF_SCOPE}."
  )


def validate_install_guide(text: str, *, tag: str, signing_mode: str) -> None:
  trust_note, open_note = install_trust_section(signing_mode)
  require_release_copy(
    text,
    (f"Pith {tag}", *INSTALL_GUIDE_REQUIRED_PHRASES, trust_note, open_note),
    "install guide",
  )


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
