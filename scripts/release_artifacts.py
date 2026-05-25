#!/usr/bin/env python3
"""Prepare and validate user-facing release artifact sidecar files."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


SUPPORTED_SIGNING_MODES = {"ad-hoc", "developer-id"}


def sha256_hex(path: Path) -> str:
  hasher = hashlib.sha256()
  with path.open("rb") as file:
    for chunk in iter(lambda: file.read(1024 * 1024), b""):
      hasher.update(chunk)
  return hasher.hexdigest()


def checksum_text(artifact_path: Path) -> str:
  if not artifact_path.is_file():
    raise FileNotFoundError(f"Release artifact is missing: {artifact_path}")
  return f"{sha256_hex(artifact_path)}  {artifact_path.name}\n"


def write_checksum_file(artifact_path: Path, checksum_path: Path | None = None) -> Path:
  output_path = checksum_path or artifact_path.with_name(f"{artifact_path.name}.sha256")
  output_path.parent.mkdir(parents=True, exist_ok=True)
  output_path.write_text(checksum_text(artifact_path), encoding="utf-8")
  validate_checksum_file(artifact_path, output_path)
  return output_path


def release_manifest(
  *,
  tag: str,
  signing_mode: str,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
) -> dict:
  validate_release_identity(tag, signing_mode)
  validate_checksum_file(artifact_path, checksum_path)
  if not install_guide_path.is_file():
    raise FileNotFoundError(f"Release install guide is missing: {install_guide_path}")

  return {
    "tag": tag,
    "product": "Pith",
    "platform": {
      "os": "macOS",
      "minimumVersion": "12.0",
      "architecture": "x86_64",
    },
    "signingMode": signing_mode,
    "trust": release_trust(signing_mode),
    "modelDelivery": {
      "mode": "in-app-download",
      "defaultModelId": "lfm2.5-350m",
      "modelWeightsBundled": False,
    },
    "artifacts": [
      {
        "name": artifact_path.name,
        "kind": "dmg",
        "sizeBytes": artifact_path.stat().st_size,
        "sha256": sha256_hex(artifact_path),
        "checksum": checksum_path.name,
      },
      {
        "name": install_guide_path.name,
        "kind": "install-guide",
        "sizeBytes": install_guide_path.stat().st_size,
      },
    ],
  }


def write_release_manifest(
  *,
  tag: str,
  signing_mode: str,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
  output_path: Path,
) -> Path:
  output_path.parent.mkdir(parents=True, exist_ok=True)
  output_path.write_text(
    json.dumps(
      release_manifest(
        tag=tag,
        signing_mode=signing_mode,
        artifact_path=artifact_path,
        checksum_path=checksum_path,
        install_guide_path=install_guide_path,
      ),
      indent=2,
      sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
  )
  validate_release_manifest(
    output_path,
    artifact_path=artifact_path,
    checksum_path=checksum_path,
    install_guide_path=install_guide_path,
  )
  return output_path


def validate_release_manifest(
  manifest_path: Path,
  *,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
) -> None:
  if not manifest_path.is_file():
    raise FileNotFoundError(f"Release manifest is missing: {manifest_path}")
  manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
  artifacts = manifest.get("artifacts")
  if not isinstance(artifacts, list):
    raise RuntimeError("Release manifest must include an artifacts list")
  by_name = {
    item.get("name"): item
    for item in artifacts
    if isinstance(item, dict)
  }
  artifact_entry = by_name.get(artifact_path.name)
  if artifact_entry is None:
    raise RuntimeError("Release manifest is missing the DMG artifact entry")
  if artifact_entry.get("sha256") != sha256_hex(artifact_path):
    raise RuntimeError("Release manifest DMG SHA-256 does not match the artifact")
  if artifact_entry.get("checksum") != checksum_path.name:
    raise RuntimeError("Release manifest DMG checksum file name is wrong")
  guide_entry = by_name.get(install_guide_path.name)
  if guide_entry is None:
    raise RuntimeError("Release manifest is missing the install guide entry")
  if manifest.get("modelDelivery", {}).get("modelWeightsBundled") is not False:
    raise RuntimeError("Release manifest must state that model weights are not bundled")


def validate_release_identity(tag: str, signing_mode: str) -> None:
  if not tag.strip():
    raise RuntimeError("Release manifest tag is required")
  if signing_mode not in SUPPORTED_SIGNING_MODES:
    raise RuntimeError(f"Unsupported release signing mode: {signing_mode}")


def release_trust(signing_mode: str) -> str:
  if signing_mode == "developer-id":
    return "developer-id-signed-notarized"
  return "ad-hoc-not-notarized"


def validate_checksum_file(artifact_path: Path, checksum_path: Path) -> None:
  if not checksum_path.is_file():
    raise FileNotFoundError(f"Release checksum is missing: {checksum_path}")

  fields = checksum_path.read_text(encoding="utf-8").strip().split()
  if len(fields) != 2:
    raise RuntimeError(f"Release checksum must contain hash and file name: {checksum_path}")

  expected_hash, expected_name = fields
  if expected_name != artifact_path.name:
    raise RuntimeError(
      "Release checksum must reference only the artifact file name: "
      f"{expected_name} != {artifact_path.name}"
    )
  if expected_hash != sha256_hex(artifact_path):
    raise RuntimeError(f"Release checksum does not match artifact: {checksum_path}")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--artifact", required=True, type=Path)
  parser.add_argument("--checksum-output", type=Path)
  parser.add_argument("--manifest-output", type=Path)
  parser.add_argument("--install-guide", type=Path)
  parser.add_argument("--tag")
  parser.add_argument("--signing-mode", choices=sorted(SUPPORTED_SIGNING_MODES))
  args = parser.parse_args()

  try:
    checksum_path = write_checksum_file(
      args.artifact.resolve(),
      args.checksum_output.resolve() if args.checksum_output else None,
    )
    manifest_path = None
    if args.manifest_output:
      if not args.tag or not args.signing_mode or not args.install_guide:
        raise RuntimeError(
          "--manifest-output requires --tag, --signing-mode, and --install-guide"
        )
      manifest_path = write_release_manifest(
        tag=args.tag,
        signing_mode=args.signing_mode,
        artifact_path=args.artifact.resolve(),
        checksum_path=checksum_path,
        install_guide_path=args.install_guide.resolve(),
        output_path=args.manifest_output.resolve(),
      )
  except Exception as error:
    print(f"release artifact preparation failed: {error}", file=sys.stderr)
    return 1

  print(f"Created release checksum: {checksum_path}")
  if manifest_path is not None:
    print(f"Created release manifest: {manifest_path}")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
