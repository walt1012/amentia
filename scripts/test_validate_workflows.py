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

env:
  SWIFT_APP_ARTIFACT: internal-PithApp-x86_64
  RUNTIME_ARTIFACT: internal-pith-runtime-bin-x86_64
  LLAMA_ARTIFACT: internal-llama-cli-x86_64
  MACOS_APP_ARTIFACT: Pith-installer-x86_64

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
      - name: Test release identity helper
        run: python3 scripts/test_release_identity.py
      - name: Test package contract helper
        run: python3 scripts/test_package_contract.py
      - name: Test installer artifact contract
        run: python3 scripts/test_installer_artifact_contract.py
      - name: Test connector workflow contracts
        run: python3 scripts/test_connector_workflow_contracts.py
      - name: Test Notion connector contract
        run: python3 scripts/test_notion_connector_contract.py
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
    steps:
      - name: Upload Swift app executable
        uses: actions/upload-artifact@v7
        with:
          name: ${{ env.SWIFT_APP_ARTIFACT }}
          retention-days: 1
  swift-tests:
    timeout-minutes: 25
  macos-runtime:
    timeout-minutes: 30
    steps:
      - name: Upload runtime executable
        uses: actions/upload-artifact@v7
        with:
          name: ${{ env.RUNTIME_ARTIFACT }}
          retention-days: 1
  macos-llama-backend:
    timeout-minutes: 45
    steps:
      - name: Upload llama.cpp backend
        uses: actions/upload-artifact@v7
        with:
          name: ${{ env.LLAMA_ARTIFACT }}
          retention-days: 1
  macos-package:
    timeout-minutes: 30
    needs:
      - changes
      - swift-app
      - macos-runtime
      - macos-llama-backend
    steps:
      - name: Build x86_64 macOS app bundle
        run: |
          python3 scripts/package_macos_app.py \
            --skip-build \
            --no-zip \
            --source-commit "$GITHUB_SHA"
      - name: Create internal macOS checksum
        run: |
          python3 scripts/release_artifacts.py \
            --tag "ci-${GITHUB_SHA::12}" \
            --source-commit "$GITHUB_SHA" \
            --signing-mode ad-hoc \
            --install-guide artifacts/macos/README-FIRST.txt \
            --package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json \
            --workflow-run-id "$GITHUB_RUN_ID" \
            --workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID" \
            --manifest-output artifacts/macos/internal-release-manifest.json
      - name: Validate installer artifact contract
        run: |
          python3 scripts/installer_artifact_contract.py \
            --tag "ci-${GITHUB_SHA::12}" \
            --asset "artifacts/macos/$MACOS_DMG_NAME" \
            --asset "artifacts/macos/$MACOS_DMG_NAME.sha256" \
            --asset artifacts/macos/README-FIRST.txt \
            --asset artifacts/macos/internal-release-manifest.json
      - name: Validate package contract
        run: |
          python3 scripts/package_contract.py \
            --manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json \
            --source-commit "$GITHUB_SHA" \
            --signing-mode ad-hoc
      - name: Upload macOS installer artifact
        uses: actions/upload-artifact@v7
        with:
          name: ${{ env.MACOS_APP_ARTIFACT }}
          path: |
            artifacts/macos/${{ env.MACOS_DMG_NAME }}
            artifacts/macos/${{ env.MACOS_DMG_NAME }}.sha256
            artifacts/macos/README-FIRST.txt
            artifacts/macos/internal-release-manifest.json
          retention-days: 7
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
          if ! [[ "$RELEASE_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            exit 1
          fi
          gh run list --workflow CI --status success
      - name: Audit remote model catalog metadata
        run: python3 scripts/validate_model_pack.py --remote
      - name: Create release checksum
        run: |
          python3 scripts/release_artifacts.py \
            --tag "$RELEASE_TAG" \
            --source-commit "$PITH_RELEASE_SHA" \
            --signing-mode "$PITH_RELEASE_SIGNING_MODE" \
            --install-guide artifacts/macos/README-FIRST.txt \
            --package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json \
            --workflow-run-id "$GITHUB_RUN_ID" \
            --workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID" \
            --manifest-output "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json"
      - name: Validate installer artifact contract
        run: |
          dmg_path="artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg"
          python3 scripts/installer_artifact_contract.py \
            --tag "$RELEASE_TAG" \
            --asset "$dmg_path" \
            --asset "$dmg_path.sha256" \
            --asset artifacts/macos/README-FIRST.txt \
            --asset "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json"
      - name: Validate package contract
        run: |
          python3 scripts/package_contract.py \
            --manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json \
            --source-commit "$PITH_RELEASE_SHA" \
            --signing-mode "$PITH_RELEASE_SIGNING_MODE" \
            --bundle-version "$PITH_RELEASE_VERSION"
      - name: Publish GitHub Release
        run: |
          python3 scripts/release_state.py
          --tag "$RELEASE_TAG"
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
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test release identity helper\n"
        "        run: python3 scripts/test_release_identity.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_release_identity.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test package contract helper\n"
        "        run: python3 scripts/test_package_contract.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_package_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test installer artifact contract\n"
        "        run: python3 scripts/test_installer_artifact_contract.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_installer_artifact_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test connector workflow contracts\n"
        "        run: python3 scripts/test_connector_workflow_contracts.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_connector_workflow_contracts.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test Notion connector contract\n"
        "        run: python3 scripts/test_notion_connector_contract.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_notion_connector_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(root, ci=VALID_CI.replace("          retention-days: 7\n", ""))
    assert_issue(issue_messages(root), "retention-days")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(root, ci=VALID_CI.replace("          retention-days: 1\n", "", 1))
    assert_issue(issue_messages(root), "internal artifact")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "SWIFT_APP_ARTIFACT: internal-PithApp-x86_64",
        "SWIFT_APP_ARTIFACT: PithApp-x86_64",
      ),
    )
    assert_issue(issue_messages(root), "internal-PithApp")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "            artifacts/macos/${{ env.MACOS_DMG_NAME }}",
        "            artifacts/macos/Pith-macos-x86_64.zip\n"
        "            artifacts/macos/${{ env.MACOS_DMG_NAME }}",
      ),
    )
    assert_issue(issue_messages(root), "internal zip")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "            artifacts/macos/internal-release-manifest.json",
        "            artifacts/macos/internal-release-notes.md\n"
        "            artifacts/macos/internal-release-manifest.json",
      ),
    )
    assert_issue(issue_messages(root), "internal release notes")

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
        "      - name: Audit remote model catalog metadata\n"
        "        run: python3 scripts/validate_model_pack.py --remote\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "validate_model_pack.py --remote")

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
      release=VALID_RELEASE.replace('          --tag "$RELEASE_TAG"\n', ""),
    )
    assert_issue(issue_messages(root), "release state helper")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          if ! [[ "$RELEASE_TAG" =~ ^v[0-9]+\\.[0-9]+\\.[0-9]+$ ]]; then\n'
        '            exit 1\n'
        '          fi\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "^v[0-9]+")

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
      ci=VALID_CI.replace("--no-zip", ""),
    )
    assert_issue(issue_messages(root), "--no-zip")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "python3 scripts/package_contract.py",
        "python3 scripts/missing_package_contract.py",
      ),
    )
    assert_issue(issue_messages(root), "package_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "--package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json",
        "",
      ),
    )
    assert_issue(issue_messages(root), "PithPackage.json")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace('--workflow-run-id "$GITHUB_RUN_ID"', ""),
    )
    assert_issue(issue_messages(root), "workflow-run-id")

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
      ci=VALID_CI.replace(
        "python3 scripts/installer_artifact_contract.py",
        "python3 scripts/missing_installer_artifact_contract.py",
      ),
    )
    assert_issue(issue_messages(root), "installer_artifact_contract.py")

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
        "python3 scripts/package_contract.py",
        "python3 scripts/missing_package_contract.py",
      ),
    )
    assert_issue(issue_messages(root), "package_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "--package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json",
        "",
      ),
    )
    assert_issue(issue_messages(root), "PithPackage.json")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"',
        "",
      ),
    )
    assert_issue(issue_messages(root), "workflow-run-url")

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

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "python3 scripts/installer_artifact_contract.py",
        "python3 scripts/missing_installer_artifact_contract.py",
      ),
    )
    assert_issue(issue_messages(root), "installer_artifact_contract.py")

  print("Workflow policy validation tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
