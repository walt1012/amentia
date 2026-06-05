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


def write_evidence_file(path: Path) -> None:
  if path.suffix == ".json":
    path.write_text(json.dumps({"result": "passed"}) + "\n", encoding="utf-8")
  elif path.suffix == ".md":
    path.write_text(f"# {path.stem}\n\nEvidence.\n", encoding="utf-8")
  else:
    path.write_bytes(b"release asset\n")


def write_evidence_set(root: Path, mode: str) -> list[Path]:
  paths: list[Path] = []
  for name in expected_evidence_names(mode, TAG):
    path = root / name
    write_evidence_file(path)
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
    write_evidence_file(extra)
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


def main() -> int:
  assert_dry_run_evidence_accepts_exact_set()
  assert_publish_rehearsal_accepts_exact_set()
  assert_rejects_missing_extra_empty_and_invalid_json()
  print("release evidence contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
