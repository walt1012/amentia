#!/usr/bin/env python3
"""Prepare and validate user-facing release artifact sidecar files."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


SUPPORTED_SIGNING_MODES = {"ad-hoc", "developer-id"}
SOURCE_COMMIT_HEX_LENGTH = 40
PUBLIC_TAG_PATTERN = re.compile(r"^v[0-9]+(\.[0-9]+){0,2}$")
INTERNAL_CI_TAG_PATTERN = re.compile(r"^ci-[0-9a-f]{12,40}$")
INSTALL_GUIDE_NAME = "README-FIRST.txt"
INTERNAL_CI_DMG_NAME = "Pith-macos-x86_64.dmg"
INSTALL_GUIDE_REQUIRED_PHRASES = (
  "Drag Pith.app to Applications",
  "download one verified local model",
  "Open a workspace folder",
  "Start a cowork session",
  "SHA-256",
  "release manifest",
)


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
  source_commit: str,
  signing_mode: str,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
) -> dict:
  validate_release_identity(tag, source_commit, signing_mode)
  validate_release_asset_names(
    tag=tag,
    artifact_path=artifact_path,
    checksum_path=checksum_path,
    install_guide_path=install_guide_path,
  )
  validate_checksum_file(artifact_path, checksum_path)
  validate_install_guide_for_tag(install_guide_path, tag)

  return {
    "schemaVersion": 1,
    "tag": tag,
    "releaseKind": release_kind(tag),
    "sourceCommit": source_commit,
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
        "name": checksum_path.name,
        "kind": "checksum",
        "sizeBytes": checksum_path.stat().st_size,
        "sha256": sha256_hex(checksum_path),
        "checks": artifact_path.name,
      },
      {
        "name": install_guide_path.name,
        "kind": "install-guide",
        "sizeBytes": install_guide_path.stat().st_size,
        "sha256": sha256_hex(install_guide_path),
      },
    ],
  }


def write_release_manifest(
  *,
  tag: str,
  source_commit: str,
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
        source_commit=source_commit,
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
  validate_manifest_identity(manifest)
  tag = manifest["tag"]
  validate_release_asset_names(
    tag=tag,
    artifact_path=artifact_path,
    checksum_path=checksum_path,
    install_guide_path=install_guide_path,
  )
  artifacts = manifest.get("artifacts")
  if not isinstance(artifacts, list):
    raise RuntimeError("Release manifest must include an artifacts list")
  validate_checksum_file(artifact_path, checksum_path)
  by_name = validate_manifest_artifacts(artifacts)
  artifact_entry = by_name.get(artifact_path.name)
  if artifact_entry is None:
    raise RuntimeError("Release manifest is missing the DMG artifact entry")
  if artifact_entry.get("kind") != "dmg":
    raise RuntimeError("Release manifest DMG entry kind is wrong")
  if artifact_entry.get("sizeBytes") != artifact_path.stat().st_size:
    raise RuntimeError("Release manifest DMG size does not match the artifact")
  if artifact_entry.get("sha256") != sha256_hex(artifact_path):
    raise RuntimeError("Release manifest DMG SHA-256 does not match the artifact")
  if artifact_entry.get("checksum") != checksum_path.name:
    raise RuntimeError("Release manifest DMG checksum file name is wrong")
  checksum_entry = by_name.get(checksum_path.name)
  if checksum_entry is None:
    raise RuntimeError("Release manifest is missing the checksum artifact entry")
  if checksum_entry.get("kind") != "checksum":
    raise RuntimeError("Release manifest checksum entry kind is wrong")
  if checksum_entry.get("checks") != artifact_path.name:
    raise RuntimeError("Release manifest checksum entry target is wrong")
  if checksum_entry.get("sizeBytes") != checksum_path.stat().st_size:
    raise RuntimeError("Release manifest checksum size does not match")
  if checksum_entry.get("sha256") != sha256_hex(checksum_path):
    raise RuntimeError("Release manifest checksum SHA-256 does not match")
  guide_entry = by_name.get(install_guide_path.name)
  if guide_entry is None:
    raise RuntimeError("Release manifest is missing the install guide entry")
  if guide_entry.get("kind") != "install-guide":
    raise RuntimeError("Release manifest install guide entry kind is wrong")
  if guide_entry.get("sizeBytes") != install_guide_path.stat().st_size:
    raise RuntimeError("Release manifest install guide size does not match")
  if guide_entry.get("sha256") != sha256_hex(install_guide_path):
    raise RuntimeError("Release manifest install guide SHA-256 does not match")
  validate_install_guide_for_tag(install_guide_path, tag)


def validate_manifest_artifacts(artifacts: list) -> dict[str, dict]:
  by_name: dict[str, dict] = {}
  for item in artifacts:
    if not isinstance(item, dict):
      raise RuntimeError("Release manifest artifact entries must be objects")
    name = item.get("name")
    if not isinstance(name, str) or not name.strip():
      raise RuntimeError("Release manifest artifact entries must include names")
    if name in {".", ".."} or "/" in name or "\\" in name:
      raise RuntimeError("Release manifest artifact names must be basenames")
    if name in by_name:
      raise RuntimeError("Release manifest artifact names must be unique")
    by_name[name] = item
  return by_name


def validate_release_identity(tag: str, source_commit: str, signing_mode: str) -> None:
  release_kind(tag)
  validate_source_commit(source_commit)
  if signing_mode not in SUPPORTED_SIGNING_MODES:
    raise RuntimeError(f"Unsupported release signing mode: {signing_mode}")


def release_kind(tag: str) -> str:
  if not isinstance(tag, str) or not tag.strip():
    raise RuntimeError("Release manifest tag is required")
  if PUBLIC_TAG_PATTERN.fullmatch(tag):
    return "public"
  if INTERNAL_CI_TAG_PATTERN.fullmatch(tag):
    return "internal-ci"
  raise RuntimeError("Release manifest tag must be a public v* tag or internal ci-* tag")


def validate_release_asset_names(
  *,
  tag: str,
  artifact_path: Path,
  checksum_path: Path,
  install_guide_path: Path,
) -> None:
  expected_dmg_name = release_dmg_name(tag)
  if artifact_path.name != expected_dmg_name:
    raise RuntimeError(
      f"Release DMG name must be {expected_dmg_name}: {artifact_path.name}"
    )
  expected_checksum_name = f"{artifact_path.name}.sha256"
  if checksum_path.name != expected_checksum_name:
    raise RuntimeError(
      f"Release checksum name must be {expected_checksum_name}: {checksum_path.name}"
    )
  if install_guide_path.name != INSTALL_GUIDE_NAME:
    raise RuntimeError(
      f"Release install guide must be named {INSTALL_GUIDE_NAME}: {install_guide_path.name}"
    )


def release_dmg_name(tag: str) -> str:
  if release_kind(tag) == "internal-ci":
    return INTERNAL_CI_DMG_NAME
  return f"Pith-{tag}-macos-x86_64.dmg"


def validate_source_commit(source_commit: str) -> None:
  if len(source_commit) != SOURCE_COMMIT_HEX_LENGTH:
    raise RuntimeError("Release manifest source commit must be a full SHA-1 hash")
  if any(character not in "0123456789abcdef" for character in source_commit):
    raise RuntimeError("Release manifest source commit must be lowercase hex")


def validate_manifest_identity(manifest: dict) -> None:
  if manifest.get("schemaVersion") != 1:
    raise RuntimeError("Release manifest schema version must be 1")
  tag = manifest.get("tag")
  kind = release_kind(tag)
  if manifest.get("releaseKind") != kind:
    raise RuntimeError("Release manifest kind does not match its tag")
  source_commit = manifest.get("sourceCommit")
  if not isinstance(source_commit, str):
    raise RuntimeError("Release manifest source commit is required")
  validate_source_commit(source_commit)
  if manifest.get("product") != "Pith":
    raise RuntimeError("Release manifest product must be Pith")
  platform = manifest.get("platform")
  if not isinstance(platform, dict):
    raise RuntimeError("Release manifest platform must be an object")
  expected_platform = {
    "os": "macOS",
    "minimumVersion": "12.0",
    "architecture": "x86_64",
  }
  for key, expected in expected_platform.items():
    if platform.get(key) != expected:
      raise RuntimeError(f"Release manifest platform {key} must be {expected}")

  signing_mode = manifest.get("signingMode")
  if signing_mode not in SUPPORTED_SIGNING_MODES:
    raise RuntimeError("Release manifest signing mode is unsupported")
  if manifest.get("trust") != release_trust(signing_mode):
    raise RuntimeError("Release manifest trust does not match signing mode")

  model_delivery = manifest.get("modelDelivery")
  if not isinstance(model_delivery, dict):
    raise RuntimeError("Release manifest model delivery must be an object")
  if model_delivery.get("mode") != "in-app-download":
    raise RuntimeError("Release manifest model delivery mode must be in-app-download")
  if model_delivery.get("defaultModelId") != "lfm2.5-350m":
    raise RuntimeError("Release manifest default model id must be lfm2.5-350m")
  if model_delivery.get("modelWeightsBundled") is not False:
    raise RuntimeError("Release manifest must state that model weights are not bundled")


def release_trust(signing_mode: str) -> str:
  if signing_mode == "developer-id":
    return "developer-id-signed-notarized"
  return "ad-hoc-not-notarized"


def validate_install_guide(install_guide_path: Path) -> None:
  if not install_guide_path.is_file():
    raise FileNotFoundError(f"Release install guide is missing: {install_guide_path}")
  text = install_guide_path.read_text(encoding="utf-8")
  missing = [
    phrase
    for phrase in INSTALL_GUIDE_REQUIRED_PHRASES
    if phrase not in text
  ]
  if missing:
    raise RuntimeError(
      "Release install guide is missing required user guidance: "
      + ", ".join(missing)
    )


def validate_install_guide_for_tag(install_guide_path: Path, tag: str) -> None:
  validate_install_guide(install_guide_path)
  text = install_guide_path.read_text(encoding="utf-8")
  if f"Pith {tag}" not in text:
    raise RuntimeError("Release install guide tag does not match the release manifest")


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
  parser.add_argument("--source-commit")
  parser.add_argument("--signing-mode", choices=sorted(SUPPORTED_SIGNING_MODES))
  args = parser.parse_args()

  try:
    checksum_path = write_checksum_file(
      args.artifact.resolve(),
      args.checksum_output.resolve() if args.checksum_output else None,
    )
    manifest_path = None
    if args.manifest_output:
      if (
        not args.tag
        or not args.source_commit
        or not args.signing_mode
        or not args.install_guide
      ):
        raise RuntimeError(
          "--manifest-output requires --tag, --source-commit, --signing-mode, "
          "and --install-guide"
        )
      manifest_path = write_release_manifest(
        tag=args.tag,
        source_commit=args.source_commit,
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
