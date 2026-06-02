#!/usr/bin/env python3
"""Unit checks for downloaded release rehearsal validation."""

from __future__ import annotations

import json
from pathlib import Path
from tempfile import TemporaryDirectory

from package_contract import DEFAULT_MODEL_ID
from package_contract import DAILY_DRIVER_CONTRACT
from release_artifacts import release_installer_asset_names
from release_artifacts import write_checksum_file
from release_artifacts import write_release_manifest
from release_rehearsal_contract import summary_markdown
from release_rehearsal_contract import validate_release_rehearsal
from release_rehearsal_contract import write_summary
from release_text import install_guide as release_install_guide


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
WORKFLOW_RUN_ID = "123456789"
WORKFLOW_RUN_URL = "https://github.com/walt1012/pith/actions/runs/123456789"


def write_downloaded_assets(root: Path, tag: str = "v0.1.0") -> None:
  dmg_name, checksum_name, guide_name, manifest_name = release_installer_asset_names(tag)
  artifact = root / dmg_name
  artifact.write_bytes(b"pith downloaded release artifact\n")
  guide = root / guide_name
  guide.write_text(release_install_guide(tag, "ad-hoc"), encoding="utf-8")
  checksum = write_checksum_file(artifact, root / checksum_name)
  write_release_manifest(
    tag=tag,
    source_commit=SOURCE_COMMIT,
    signing_mode="ad-hoc",
    artifact_path=artifact,
    checksum_path=checksum,
    install_guide_path=guide,
    output_path=root / manifest_name,
    workflow_run_id=WORKFLOW_RUN_ID,
    workflow_run_url=WORKFLOW_RUN_URL,
  )


def expect_failure(action, expected: str) -> None:
  try:
    action()
  except Exception as error:
    if expected not in str(error):
      raise AssertionError(f"expected {expected!r}, got {error!r}") from error
    return
  raise AssertionError(f"expected release rehearsal validation to fail: {expected}")


def main() -> int:
  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    summary = validate_release_rehearsal("v0.1.0", root)
    if summary["result"] != "passed":
      raise AssertionError("release rehearsal summary should pass")
    if summary["defaultModelId"] != DEFAULT_MODEL_ID:
      raise AssertionError("release rehearsal summary should record the default model")
    if "download-default-verified-local-model" not in summary["firstRun"].values():
      raise AssertionError("release rehearsal summary should include the first-run model step")
    if summary["dailyDriver"] != DAILY_DRIVER_CONTRACT:
      raise AssertionError("release rehearsal summary should record daily-driver readiness")
    if not any("Choose Map Workspace" in check for check in summary["firstAppOpenChecks"]):
      raise AssertionError("release rehearsal summary should name the first cowork prompts")
    markdown = summary_markdown(summary)
    if "Pith v0.1.0 Release Rehearsal" not in markdown:
      raise AssertionError("release rehearsal markdown should name the tag")
    if "## First App Open" not in markdown:
      raise AssertionError("release rehearsal markdown should include first app open checks")
    output = root / "rehearsal.md"
    write_summary(output, summary)
    if "Result: `passed`" not in output.read_text(encoding="utf-8"):
      raise AssertionError("release rehearsal summary file should record the result")

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["modelDelivery"]["defaultModelId"] = "wrong-model"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "default model",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["dailyDriver"]["nextActionSource"] = "static-checklist"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "daily driver nextActionSource must be runtime/readiness",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    _dmg_name, _checksum_name, _guide_name, manifest_name = release_installer_asset_names("v0.1.0")
    manifest_path = root / manifest_name
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["verification"]["packagedSmokeReceipt"]["result"] = "failed"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "packagedSmokeReceipt result must be 'passed'",
    )

  with TemporaryDirectory(prefix="pith-release-rehearsal-") as directory:
    root = Path(directory)
    write_downloaded_assets(root)
    (root / "Pith-v0.1.0-macos-x86_64.dmg").write_bytes(b"tampered artifact\n")
    expect_failure(
      lambda: validate_release_rehearsal("v0.1.0", root),
      "Release checksum does not match artifact",
    )

  print("release rehearsal contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
