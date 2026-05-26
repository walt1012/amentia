#!/usr/bin/env python3
"""Unit checks for GitHub Actions workflow policy validation."""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory

from validate_workflows import validate_workflows


VALID_CI = """name: CI

on:
  push:
  pull_request:

defaults:
  run:
    shell: bash

jobs:
  changes:
    timeout-minutes: 5
    steps:
      - name: Checkout
        uses: actions/checkout@v6
        with:
          persist-credentials: false
  repository-policy:
    timeout-minutes: 10
    steps:
      - name: Checkout
        uses: actions/checkout@v6
        with:
          persist-credentials: false
  rust-format:
    timeout-minutes: 10
  rust-clippy:
    timeout-minutes: 25
  rust-test:
    timeout-minutes: 25
  runtime-smoke:
    timeout-minutes: 25
  model-catalog-remote:
    timeout-minutes: 15
  swift-app:
    timeout-minutes: 25
  swift-tests:
    timeout-minutes: 25
  macos-runtime:
    timeout-minutes: 30
  macos-llama-backend:
    timeout-minutes: 45
  macos-package:
    timeout-minutes: 30
    needs:
      - changes
      - swift-app
      - macos-runtime
      - macos-llama-backend
    steps:
      - name: Create internal macOS checksum
        run: |
          python3 scripts/release_artifacts.py \
            --tag "ci-${GITHUB_SHA::12}" \
            --source-commit "$GITHUB_SHA" \
            --signing-mode ad-hoc \
            --install-guide artifacts/macos/README-FIRST.txt \
            --manifest-output artifacts/macos/internal-release-manifest.json
      - name: Upload macOS app artifact
        uses: actions/upload-artifact@v7
        with:
          path: |
            artifacts/macos/Pith-macos-x86_64.zip
            artifacts/macos/${{ env.MACOS_DMG_NAME }}
            artifacts/macos/${{ env.MACOS_DMG_NAME }}.sha256
            artifacts/macos/README-FIRST.txt
            artifacts/macos/internal-release-notes.md
            artifacts/macos/internal-release-manifest.json
          retention-days: 21
"""

VALID_RELEASE = """name: Release

on:
  push:
  workflow_dispatch:

defaults:
  run:
    shell: bash

jobs:
  release-dmg:
    timeout-minutes: 90
    steps:
      - name: Checkout release tag
        uses: actions/checkout@v6
        with:
          persist-credentials: false
      - name: Validate release tag and CI gate
        run: |
          gh run list --workflow CI --status success
      - name: Create release checksum
        run: |
          python3 scripts/release_artifacts.py \
            --tag "$RELEASE_TAG" \
            --source-commit "$PITH_RELEASE_SHA" \
            --signing-mode "$PITH_RELEASE_SIGNING_MODE" \
            --install-guide artifacts/macos/README-FIRST.txt \
            --manifest-output "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json"
      - name: Publish GitHub Release
        run: |
          python3 scripts/release_state.py
          gh release upload "$RELEASE_TAG" \\
            "artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg" \\
            "artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg.sha256" \\
            "artifacts/macos/README-FIRST.txt" \\
            "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json" \\
            --clobber
"""


def assert_issue(messages: list[str], expected: str) -> None:
  if not any(expected in message for message in messages):
    raise AssertionError(f"expected issue containing {expected!r}, got {messages!r}")


def write_workflows(
  root: Path,
  ci: str = VALID_CI,
  release: str = VALID_RELEASE,
) -> None:
  workflow_dir = root / ".github" / "workflows"
  workflow_dir.mkdir(parents=True)
  (workflow_dir / "ci.yml").write_text(ci, encoding="utf-8")
  (workflow_dir / "release.yml").write_text(release, encoding="utf-8")


def issue_messages(root: Path) -> list[str]:
  return [issue.message for issue in validate_workflows(root)]


def main() -> int:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(root)
    messages = issue_messages(root)
    if messages:
      raise AssertionError(f"expected no workflow policy issues, got {messages!r}")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace("persist-credentials: false", "fetch-depth: 1", 1),
    )
    assert_issue(issue_messages(root), "persist-credentials: false")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(root, ci=VALID_CI.replace("          retention-days: 21\n", ""))
    assert_issue(issue_messages(root), "retention-days")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - macos-llama-backend",
        "      - swift-tests\n      - macos-llama-backend",
      ),
    )
    assert_issue(issue_messages(root), "must not wait for swift-tests")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace("--workflow CI", "--workflow Release"),
    )
    assert_issue(issue_messages(root), "release CI gate")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "python3 scripts/release_artifacts.py",
        "shasum -a 256",
      ),
    )
    assert_issue(issue_messages(root), "release checksum")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace('--source-commit "$GITHUB_SHA"', ""),
    )
    assert_issue(issue_messages(root), "macos-package release manifest")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "            --manifest-output artifacts/macos/internal-release-manifest.json\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "internal-release-manifest.json")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace('--source-commit "$PITH_RELEASE_SHA"', ""),
    )
    assert_issue(issue_messages(root), "release manifest must include")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '            --manifest-output "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json"\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "Pith-$RELEASE_TAG-release-manifest.json")

  print("Workflow policy validation tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
