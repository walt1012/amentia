#!/usr/bin/env python3
"""Unit checks for release artifact sidecar helpers."""

from __future__ import annotations

import tempfile
import json
from pathlib import Path

from release_artifacts import (
  checksum_text,
  release_manifest,
  validate_checksum_file,
  validate_install_guide,
  validate_release_manifest,
  write_checksum_file,
  write_release_manifest,
)
from release_text import install_guide as release_install_guide


SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


def main() -> int:
  with tempfile.TemporaryDirectory(prefix="pith-release-artifacts-") as root:
    root_path = Path(root)
    artifact = root_path / "Pith-v0.1.0-macos-x86_64.dmg"
    install_guide = root_path / "README-FIRST.txt"
    artifact.write_bytes(b"pith release artifact\n")
    install_guide.write_text(
      release_install_guide("v0.1.0", "ad-hoc"),
      encoding="utf-8",
    )

    text = checksum_text(artifact)
    if not text.endswith(f"  {artifact.name}\n"):
      raise AssertionError("checksum should reference the artifact basename")
    if str(root_path) in text:
      raise AssertionError("checksum should not expose runner-local paths")

    checksum_path = write_checksum_file(artifact)
    validate_checksum_file(artifact, checksum_path)
    validate_install_guide(install_guide)

    manifest = release_manifest(
      tag="v0.1.0",
      source_commit=SOURCE_COMMIT,
      signing_mode="ad-hoc",
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
    )
    if manifest["platform"]["architecture"] != "x86_64":
      raise AssertionError("release manifest should lock the macOS architecture")
    if manifest["sourceCommit"] != SOURCE_COMMIT:
      raise AssertionError("release manifest should record the source commit")
    if manifest["modelDelivery"]["modelWeightsBundled"] is not False:
      raise AssertionError("release manifest should not claim bundled model weights")

    manifest_path = write_release_manifest(
      tag="v0.1.0",
      source_commit=SOURCE_COMMIT,
      signing_mode="ad-hoc",
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
      output_path=root_path / "release-manifest.json",
    )
    validate_release_manifest(
      manifest_path,
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
    )

    manifest_data = manifest_path.read_text(encoding="utf-8")
    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["platform"]["architecture"] = "arm64"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong release platform should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    tampered_manifest = json.loads(manifest_data)
    tampered_manifest["sourceCommit"] = "short"
    manifest_path.write_text(json.dumps(tampered_manifest), encoding="utf-8")
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "wrong source commit should fail release manifest validation",
    )
    manifest_path.write_text(manifest_data, encoding="utf-8")

    artifact.write_bytes(b"tampered release artifact\n")
    assert_raises(
      lambda: validate_checksum_file(artifact, checksum_path),
      "tampered release artifact should fail checksum validation",
    )
    assert_raises(
      lambda: validate_release_manifest(
        manifest_path,
        artifact_path=artifact,
        checksum_path=checksum_path,
        install_guide_path=install_guide,
      ),
      "tampered release artifact should fail release manifest validation",
    )

  with tempfile.TemporaryDirectory(prefix="pith-release-artifacts-") as root:
    root_path = Path(root)
    weak_guide = root_path / "README-FIRST.txt"
    weak_guide.write_text("Install Pith from this DMG.\n", encoding="utf-8")
    assert_raises(
      lambda: validate_install_guide(weak_guide),
      "weak install guide should fail release guidance validation",
    )

  print("release artifact tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
