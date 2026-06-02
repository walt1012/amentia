#!/usr/bin/env python3
"""Shared release copy contract for generated installer guidance."""

from __future__ import annotations

from package_contract import (
  DEFAULT_MODEL_ID,
  FIRST_APP_OPEN_CONTRACT_ID,
  PACKAGED_SMOKE_PROOF_SCOPE,
)


PACKAGED_FIRST_RUN_PROOF_PHRASE = "packaged first-run smoke receipt"
PACKAGED_FIRST_RUN_PROOF_SCOPE = PACKAGED_SMOKE_PROOF_SCOPE
FIRST_APP_OPEN_ACTION_COPY = (
  "Choose Map Workspace, Plan Next Step, or type a short cowork request."
)
FIRST_APP_OPEN_INSTALL_STEP = (
  "Start a cowork session with Map Workspace, Plan Next Step, "
  "or your own first request."
)

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
  PACKAGED_FIRST_RUN_PROOF_PHRASE,
  "Native sandbox",
  "process-only fallback",
  "daily-driver next action",
  "package size budget",
)


INSTALL_GUIDE_REQUIRED_PHRASES = (
  "Drag Pith.app to Applications.",
  "Installer assets:",
  "download one verified local model",
  "First-run path:",
  "No Pith login is required",
  "local execution mode",
  DEFAULT_MODEL_ID,
  "Open a workspace folder.",
  "Web Search readiness",
  "approve a safe local change",
  "inspect the proof",
  "Start a cowork session",
  "Map Workspace",
  "Plan Next Step",
  "Follow the next action",
  "runtime readiness",
  "sandbox status",
  "process-only fallback",
  "SHA-256 `.sha256` file",
  "shasum -a 256 -c",
  "sidecar hashes",
  PACKAGED_FIRST_RUN_PROOF_PHRASE,
  "source commit",
  "model delivery mode",
  "package size budget",
  "first-run contract",
)


def missing_required_phrases(text: str, phrases: tuple[str, ...]) -> list[str]:
  return [
    phrase
    for phrase in phrases
    if phrase not in text
  ]


def require_release_copy(text: str, phrases: tuple[str, ...], label: str) -> None:
  missing = missing_required_phrases(text, phrases)
  if missing:
    raise RuntimeError(f"{label} is missing required copy: {', '.join(missing)}")


def require_release_notes_copy(text: str, label: str = "release notes") -> None:
  require_release_copy(text, RELEASE_NOTES_REQUIRED_PHRASES, label)


def require_install_guide_copy(text: str, label: str = "install guide") -> None:
  require_release_copy(text, INSTALL_GUIDE_REQUIRED_PHRASES, label)
