#!/usr/bin/env python3
"""Unit checks for release artifact sidecar helpers."""

from __future__ import annotations

import tempfile
from pathlib import Path

from release_artifacts import (
  checksum_text,
  release_manifest,
  validate_checksum_file,
  validate_release_manifest,
  write_checksum_file,
  write_release_manifest,
)


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
    install_guide.write_text("Install Pith from this DMG.\n", encoding="utf-8")

    text = checksum_text(artifact)
    if not text.endswith(f"  {artifact.name}\n"):
      raise AssertionError("checksum should reference the artifact basename")
    if str(root_path) in text:
      raise AssertionError("checksum should not expose runner-local paths")

    checksum_path = write_checksum_file(artifact)
    validate_checksum_file(artifact, checksum_path)

    manifest = release_manifest(
      tag="v0.1.0",
      signing_mode="ad-hoc",
      artifact_path=artifact,
      checksum_path=checksum_path,
      install_guide_path=install_guide,
    )
    if manifest["platform"]["architecture"] != "x86_64":
      raise AssertionError("release manifest should lock the macOS architecture")
    if manifest["modelDelivery"]["modelWeightsBundled"] is not False:
      raise AssertionError("release manifest should not claim bundled model weights")

    manifest_path = write_release_manifest(
      tag="v0.1.0",
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

  print("release artifact tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
