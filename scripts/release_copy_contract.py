#!/usr/bin/env python3
"""Shared release copy contract for generated installer guidance."""

from __future__ import annotations

from package_contract import DEFAULT_MODEL_ID


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
  "Follow the next action",
  "runtime readiness",
  "sandbox status",
  "process-only fallback",
  "SHA-256 `.sha256` file",
  "shasum -a 256 -c",
  "sidecar hashes",
  "source commit",
  "model delivery mode",
  "package size budget",
  "first-run contract",
)
