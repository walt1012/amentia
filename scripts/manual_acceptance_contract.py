#!/usr/bin/env python3
"""Validate structured manual release acceptance evidence for Pith."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from package_contract import DEFAULT_MODEL_ID
from release_artifacts import release_installer_asset_names
from release_identity import validate_public_release_tag


REQUIRED_TRUE_CHECKS = (
  "checksumVerified",
  "manifestReviewed",
  "gatekeeperHandled",
  "modelDownloadedAndActivated",
  "workspaceOpened",
  "coworkTurnCompleted",
  "webSearchProofInspected",
  "approvalDiffReceiptInspected",
  "restartRecoveryVerified",
  "noPithLoginRequired",
  "acceptedForVisiblePrerelease",
)
REQUIRED_TEXT_FIELDS = (
  "tag",
  "sourceCommit",
  "releaseWorkflowRunUrl",
  "dmgAssetName",
  "checksum",
  "gatekeeperPath",
  "selectedModelId",
  "workspaceDescription",
  "coworkRequest",
  "webSearchProof",
  "approvalReceipt",
  "restartRecoveryProof",
  "acceptedBy",
  "acceptedAt",
)


def validate_manual_acceptance_evidence(data: dict[str, object], *, tag: str) -> None:
  validate_public_release_tag(tag)
  require_equal(data, "tag", tag)
  require_string(data, "sourceCommit", length=40)
  require_git_sha(str(data["sourceCommit"]), "manual acceptance sourceCommit")
  require_string(data, "releaseWorkflowRunUrl", prefix="https://github.com/walt1012/pith/actions/runs/")
  require_equal(data, "dmgAssetName", release_installer_asset_names(tag)[0])
  require_string(data, "checksum", length=64)
  require_sha256(data, "checksum")
  require_equal(data, "selectedModelId", DEFAULT_MODEL_ID)
  for field in REQUIRED_TEXT_FIELDS:
    require_string(data, field)
  for check in REQUIRED_TRUE_CHECKS:
    require_true(data, check)


def manual_acceptance_template(
  *,
  tag: str,
  source_commit: str,
  release_workflow_run_url: str,
  checksum: str = "",
) -> dict[str, object]:
  validate_public_release_tag(tag)
  return {
    "tag": tag,
    "sourceCommit": source_commit,
    "releaseWorkflowRunUrl": release_workflow_run_url,
    "dmgAssetName": release_installer_asset_names(tag)[0],
    "checksum": checksum,
    "checksumVerified": False,
    "manifestReviewed": False,
    "gatekeeperPath": "",
    "gatekeeperHandled": False,
    "selectedModelId": DEFAULT_MODEL_ID,
    "modelDownloadedAndActivated": False,
    "workspaceDescription": "",
    "workspaceOpened": False,
    "coworkRequest": "",
    "coworkTurnCompleted": False,
    "webSearchProof": "",
    "webSearchProofInspected": False,
    "approvalReceipt": "",
    "approvalDiffReceiptInspected": False,
    "restartRecoveryProof": "",
    "restartRecoveryVerified": False,
    "noPithLoginRequired": False,
    "acceptedForVisiblePrerelease": False,
    "acceptedBy": "",
    "acceptedAt": "",
  }


def manual_acceptance_template_from_asset_dir(
  *,
  tag: str,
  asset_dir: Path,
) -> dict[str, object]:
  validate_public_release_tag(tag)
  dmg_name, checksum_name, _guide_name, manifest_name = release_installer_asset_names(tag)
  manifest = load_release_manifest(asset_dir / manifest_name)
  require_manifest_equal(manifest, "tag", tag)
  source_commit = require_manifest_string(manifest, "sourceCommit", length=40)
  require_git_sha(source_commit, "release manifest sourceCommit")
  verification = require_manifest_object(manifest, "verification")
  workflow_run_url = require_manifest_string(
    verification,
    "workflowRunUrl",
    prefix="https://github.com/walt1012/pith/actions/runs/",
  )
  checksum = require_dmg_artifact_checksum(manifest, dmg_name)
  require_sha256_hex(checksum, "release manifest DMG artifact sha256")
  require_checksum_sidecar(
    asset_dir / checksum_name,
    expected_checksum=checksum,
    expected_name=dmg_name,
  )
  require_dmg_asset_hash(asset_dir / dmg_name, expected_checksum=checksum)
  return manual_acceptance_template(
    tag=tag,
    source_commit=source_commit,
    release_workflow_run_url=workflow_run_url,
    checksum=checksum,
  )


def load_release_manifest(path: Path) -> dict[str, object]:
  if not path.is_file():
    raise FileNotFoundError(f"release manifest is missing: {path}")
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError("release manifest must be a JSON object")
  return data


def require_dmg_artifact_checksum(manifest: dict[str, object], dmg_name: str) -> str:
  artifacts = manifest.get("artifacts")
  if not isinstance(artifacts, list):
    raise RuntimeError("release manifest artifacts must be a list")
  for artifact in artifacts:
    if (
      isinstance(artifact, dict)
      and artifact.get("kind") == "dmg"
      and artifact.get("name") == dmg_name
    ):
      return require_manifest_string(artifact, "sha256", length=64)
  raise RuntimeError(f"release manifest must include DMG artifact {dmg_name}")


def require_checksum_sidecar(
  path: Path,
  *,
  expected_checksum: str,
  expected_name: str,
) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"checksum sidecar is missing: {path}")
  text = path.read_text(encoding="utf-8").strip()
  parts = text.split()
  if len(parts) != 2:
    raise RuntimeError("checksum sidecar must contain a digest and asset name")
  actual_checksum, actual_name = parts
  require_sha256_hex(actual_checksum, "checksum sidecar digest")
  if actual_checksum.lower() != expected_checksum.lower():
    raise RuntimeError("checksum sidecar digest must match release manifest")
  if actual_name != expected_name:
    raise RuntimeError("checksum sidecar asset name must match the DMG asset")


def require_dmg_asset_hash(path: Path, *, expected_checksum: str) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"DMG asset is missing: {path}")
  actual_checksum = sha256_hex(path)
  if actual_checksum != expected_checksum.lower():
    raise RuntimeError("DMG asset digest must match release manifest and checksum sidecar")


def sha256_hex(path: Path) -> str:
  hasher = hashlib.sha256()
  with path.open("rb") as file:
    for chunk in iter(lambda: file.read(1024 * 1024), b""):
      hasher.update(chunk)
  return hasher.hexdigest()


def require_manifest_equal(data: dict[str, object], key: str, expected: str) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(f"release manifest {key} must be {expected!r}, got {actual!r}")


def require_manifest_object(data: dict[str, object], key: str) -> dict[str, object]:
  actual = data.get(key)
  if not isinstance(actual, dict):
    raise RuntimeError(f"release manifest {key} must be an object")
  return actual


def require_manifest_string(
  data: dict[str, object],
  key: str,
  *,
  length: int | None = None,
  prefix: str | None = None,
) -> str:
  actual = data.get(key)
  if not isinstance(actual, str) or not actual.strip():
    raise RuntimeError(f"release manifest {key} must be a non-empty string")
  value = actual.strip()
  if length is not None and len(value) != length:
    raise RuntimeError(f"release manifest {key} must be {length} characters")
  if prefix is not None and not value.startswith(prefix):
    raise RuntimeError(f"release manifest {key} must start with {prefix}")
  return value


def require_equal(data: dict[str, object], key: str, expected: str) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(f"manual acceptance {key} must be {expected!r}, got {actual!r}")


def require_string(
  data: dict[str, object],
  key: str,
  *,
  length: int | None = None,
  prefix: str | None = None,
) -> None:
  actual = data.get(key)
  if not isinstance(actual, str) or not actual.strip():
    raise RuntimeError(f"manual acceptance {key} must be a non-empty string")
  value = actual.strip()
  if length is not None and len(value) != length:
    raise RuntimeError(f"manual acceptance {key} must be {length} characters")
  if prefix is not None and not value.startswith(prefix):
    raise RuntimeError(f"manual acceptance {key} must start with {prefix}")


def require_true(data: dict[str, object], key: str) -> None:
  if data.get(key) is not True:
    raise RuntimeError(f"manual acceptance {key} must be true")


def require_sha256(data: dict[str, object], key: str) -> None:
  value = str(data[key]).strip().lower()
  require_sha256_hex(value, f"manual acceptance {key}")


def require_sha256_hex(value: str, label: str) -> None:
  normalized = value.strip().lower()
  if len(normalized) != 64 or any(
    character not in "0123456789abcdef" for character in normalized
  ):
    raise RuntimeError(f"{label} must be a SHA-256 hex digest")


def require_git_sha(value: str, label: str) -> None:
  normalized = value.strip().lower()
  if len(normalized) != 40 or any(
    character not in "0123456789abcdef" for character in normalized
  ):
    raise RuntimeError(f"{label} must be a 40-character Git SHA hex digest")


def load_json(path: Path) -> dict[str, object]:
  data = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(data, dict):
    raise RuntimeError("manual acceptance receipt must be a JSON object")
  return data


def write_json(path: Path, data: dict[str, object]) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--tag", required=True)
  parser.add_argument("--evidence", type=Path)
  parser.add_argument("--template-output", type=Path)
  parser.add_argument("--asset-dir", type=Path)
  parser.add_argument("--source-commit", default="")
  parser.add_argument("--release-workflow-run-url", default="")
  args = parser.parse_args()

  try:
    if args.template_output is not None:
      if args.asset_dir is not None:
        template = manual_acceptance_template_from_asset_dir(
          tag=args.tag,
          asset_dir=args.asset_dir,
        )
      else:
        if not args.source_commit.strip():
          raise RuntimeError("manual acceptance template requires --source-commit or --asset-dir")
        if not args.release_workflow_run_url.strip():
          raise RuntimeError(
            "manual acceptance template requires --release-workflow-run-url or --asset-dir"
          )
        template = manual_acceptance_template(
          tag=args.tag,
          source_commit=args.source_commit,
          release_workflow_run_url=args.release_workflow_run_url,
        )
      write_json(args.template_output, template)
      print("Manual acceptance receipt template written")
      return 0
    if args.evidence is None:
      raise RuntimeError("manual acceptance validation requires --evidence")
    validate_manual_acceptance_evidence(load_json(args.evidence), tag=args.tag)
  except Exception as error:
    print(f"manual acceptance contract failed: {error}", file=sys.stderr)
    return 1

  print("Manual acceptance contract passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
