#!/usr/bin/env python3
"""Unit checks for internal release evidence artifact validation."""

from __future__ import annotations

import json
from pathlib import Path
from tempfile import TemporaryDirectory

from release_artifacts import release_installer_asset_names
from release_evidence_contract import expected_evidence_names
from release_evidence_contract import validate_release_evidence_set


TAG = "v0.1.0"


def write_evidence_file(path: Path, mode: str = "dry-run") -> None:
  if path.suffix == ".json":
    payload = {"result": "passed"}
    if path.name == "release-readiness.json":
      payload = release_readiness_payload(mode)
    elif path.name == "release-plan.json":
      payload = release_plan_payload(mode)
    path.write_text(json.dumps(payload) + "\n", encoding="utf-8")
  elif path.suffix == ".md":
    path.write_text(f"# {path.stem}\n\nEvidence.\n", encoding="utf-8")
  else:
    path.write_bytes(b"release asset\n")


def release_readiness_payload(mode: str = "dry-run") -> dict[str, object]:
  workflow_mode = "dry-run" if mode == "dry-run" else "publish"
  return {
    "status": "ready",
    "tag": TAG,
    "sourceCommit": "0123456789abcdef0123456789abcdef01234567",
    "successfulCiRunUrl": "https://github.com/walt1012/pith/actions/runs/1",
    "workflowMode": workflow_mode,
    "signingMode": "ad-hoc",
    "requestedDraft": True,
    "requestedPrerelease": True,
    "allowUntrustedAdHoc": False,
    "plannedDraft": True,
    "plannedPrerelease": True,
    "workingTreeClean": True,
    "tagPointsAtSourceCommit": True,
    "releaseWorkflowInputsReady": True,
    "manualAcceptanceConfirmed": False,
    "manualAcceptanceEvidence": "",
    "preDispatchChecklist": ["Run the dry-run."],
    "expectedPublicAssets": list(release_installer_asset_names(TAG)),
    "expectedDryRunEvidence": list(expected_evidence_names("dry-run", TAG)),
    "tagCommands": ["git tag v0.1.0 HEAD", "git push origin v0.1.0"],
    "remoteTagVerificationCommand": "git ls-remote --tags origin refs/tags/v0.1.0",
    "successfulCiLookupCommand": "gh run list --workflow CI",
    "dryRunArtifactLookupCommand": "gh api repos/:owner/:repo/actions/artifacts",
    "dryRunArtifactDownloadCommand": "gh run download",
    "dryRunEvidenceValidationCommand": "python scripts/release_evidence_contract.py",
    "postAcceptancePublishCommand": "gh workflow run release.yml",
    "blockers": [],
    "nextCommand": "gh workflow run release.yml",
  }


def release_plan_payload(mode: str = "dry-run") -> dict[str, object]:
  workflow_mode = "dry-run" if mode == "dry-run" else "publish"
  github_mutation = "none" if workflow_mode == "dry-run" else "create-or-update"
  return {
    "tag": TAG,
    "title": f"Pith {TAG}",
    "sourceCommit": "0123456789abcdef0123456789abcdef01234567",
    "successfulCiRunUrl": "https://github.com/walt1012/pith/actions/runs/1",
    "releaseWorkflowRunUrl": "https://github.com/walt1012/pith/actions/runs/2",
    "workflowMode": workflow_mode,
    "githubMutation": github_mutation,
    "signingMode": "ad-hoc",
    "releaseExists": False,
    "existingReleaseState": "none",
    "requestedDraft": True,
    "requestedPrerelease": True,
    "allowVisibleAdHoc": False,
    "manualAcceptanceConfirmed": False,
    "manualAcceptanceEvidence": "",
    "plannedDraft": True,
    "plannedPrerelease": True,
    "trustPath": "Ad-hoc signed prerelease.",
    "nextMaintainerActions": ["Download the dry-run artifact."],
  }


def write_evidence_set(root: Path, mode: str) -> list[Path]:
  paths: list[Path] = []
  for name in expected_evidence_names(mode, TAG):
    path = root / name
    write_evidence_file(path, mode)
    paths.append(path)
  return paths


def expect_failure(action, expected: str) -> None:
  try:
    action()
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r}, got {error!r}") from error
    return
  raise AssertionError(f"expected release evidence validation to fail: {expected}")


def assert_dry_run_evidence_accepts_exact_set() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    validate_release_evidence_set(mode="dry-run", tag=TAG, evidence_paths=paths)
    expected = set(release_installer_asset_names(TAG))
    if not expected.issubset({path.name for path in paths}):
      raise AssertionError("dry-run evidence should include the public installer assets")


def assert_publish_rehearsal_accepts_exact_set() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    validate_release_evidence_set(
      mode="publish-rehearsal",
      tag=TAG,
      evidence_paths=paths,
    )
    if any(path.suffix == ".dmg" for path in paths):
      raise AssertionError("publish rehearsal evidence should not include public installer assets")


def assert_rejects_missing_extra_empty_and_invalid_json() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths[:-1],
      ),
      "missing",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    extra = root / "unexpected.json"
    write_evidence_file(extra, "dry-run")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths + [extra],
      ),
      "extra",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    paths[0].write_text("", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must not be empty",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    json_path = next(path for path in paths if path.suffix == ".json")
    json_path.write_text("[]\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "JSON must be an object",
    )


def assert_rejects_stale_structured_json() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    del stale["postAcceptancePublishCommand"]
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "missing required keys",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("publish-rehearsal")
    del stale["manualAcceptanceEvidence"]
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "missing required keys",
    )


def assert_rejects_inconsistent_structured_json() -> None:
  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["tag"] = "v9.9.9"
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must be 'v0.1.0'",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "publish-rehearsal")
    plan_path = root / "release-plan.json"
    stale = release_plan_payload("publish-rehearsal")
    stale["workflowMode"] = "dry-run"
    plan_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="publish-rehearsal",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must be 'publish'",
    )

  with TemporaryDirectory(prefix="pith-release-evidence-") as directory:
    root = Path(directory)
    paths = write_evidence_set(root, "dry-run")
    readiness_path = root / "release-readiness.json"
    stale = release_readiness_payload("dry-run")
    stale["expectedPublicAssets"] = []
    readiness_path.write_text(json.dumps(stale) + "\n", encoding="utf-8")
    expect_failure(
      lambda: validate_release_evidence_set(
        mode="dry-run",
        tag=TAG,
        evidence_paths=paths,
      ),
      "must match the release contract",
    )


def main() -> int:
  assert_dry_run_evidence_accepts_exact_set()
  assert_publish_rehearsal_accepts_exact_set()
  assert_rejects_missing_extra_empty_and_invalid_json()
  assert_rejects_stale_structured_json()
  assert_rejects_inconsistent_structured_json()
  print("release evidence contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
