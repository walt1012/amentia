#!/usr/bin/env python3
"""Check that app first-open copy stays aligned with release copy."""

from __future__ import annotations

from pathlib import Path

from release_copy_contract import (
  FIRST_APP_OPEN_ACTION_COPY,
  FIRST_APP_OPEN_CONTRACT_ID,
  FIRST_APP_OPEN_INSTALL_STEP,
)


ROOT = Path(__file__).resolve().parents[1]
FIRST_REQUEST_PRESENTER = (
  ROOT
  / "apps"
  / "pith-macos"
  / "Sources"
  / "PithApp"
  / "Timeline"
  / "FirstRequestPromptPresenter.swift"
)
SETUP_PROGRESS_PRESENTER = (
  ROOT
  / "apps"
  / "pith-macos"
  / "Sources"
  / "PithApp"
  / "Setup"
  / "SetupProgressPresenter.swift"
)
DISTRIBUTION_TRUST_PRESENTER = (
  ROOT
  / "apps"
  / "pith-macos"
  / "Sources"
  / "PithApp"
  / "App"
  / "DistributionTrustPresenter.swift"
)

FORBIDDEN_FIRST_OPEN_COPY = (
  "Choose First Prompt",
  "Use Map Prompt",
  "Use Next Step Prompt",
)


def require_contains(text: str, expected: str, label: str) -> None:
  if expected not in text:
    raise AssertionError(f"{label} must contain {expected!r}")


def require_not_contains(text: str, unexpected: str, label: str) -> None:
  if unexpected in text:
    raise AssertionError(f"{label} must not contain stale copy {unexpected!r}")


def main() -> int:
  first_request_text = FIRST_REQUEST_PRESENTER.read_text(encoding="utf-8")
  setup_progress_text = SETUP_PROGRESS_PRESENTER.read_text(encoding="utf-8")
  distribution_trust_text = DISTRIBUTION_TRUST_PRESENTER.read_text(encoding="utf-8")
  app_text = f"{first_request_text}\n{setup_progress_text}\n{distribution_trust_text}"

  require_contains(
    first_request_text,
    FIRST_APP_OPEN_ACTION_COPY,
    "Swift first request presenter",
  )
  require_contains(
    first_request_text,
    f'firstAppOpenActionContractID = "{FIRST_APP_OPEN_CONTRACT_ID}"',
    "Swift first request presenter",
  )
  require_contains(
    distribution_trust_text,
    "FirstRequestPromptPresenter.firstAppOpenActionContractID",
    "Swift distribution trust presenter",
  )
  require_contains(
    distribution_trust_text,
    "FirstRequestPromptPresenter.firstAppOpenActionTrustSummary()",
    "Swift distribution trust presenter",
  )
  for phrase in ("Map Workspace", "Plan Next Step"):
    require_contains(first_request_text, phrase, "Swift first request presenter")
    require_contains(FIRST_APP_OPEN_INSTALL_STEP, phrase, "release install step")
  require_contains(
    first_request_text,
    "short cowork request",
    "Swift first request presenter",
  )
  require_contains(
    FIRST_APP_OPEN_INSTALL_STEP,
    "your own first request",
    "release install step",
  )
  require_contains(
    FIRST_APP_OPEN_CONTRACT_ID,
    "cowork-request",
    "first app-open contract id",
  )

  require_contains(
    setup_progress_text,
    "Choose Starter",
    "Swift setup progress presenter",
  )
  for phrase in FORBIDDEN_FIRST_OPEN_COPY:
    require_not_contains(app_text, phrase, "Swift first-open copy")

  print("first app-open contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
