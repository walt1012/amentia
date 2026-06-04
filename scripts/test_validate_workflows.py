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

permissions:
  actions: read
  contents: read

concurrency:
  group: ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

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
      - name: Validate model pack manifest
        run: python3 scripts/validate_model_pack.py
      - name: Check English-only policy
        run: python3 scripts/check_english_policy.py
      - name: Test packaging helpers
        run: python3 scripts/test_package_macos_app.py
      - name: Test release identity helper
        run: python3 scripts/test_release_identity.py
      - name: Test distribution signing helper
        run: python3 scripts/test_sign_macos_app_for_distribution.py
      - name: Test package contract helper
        run: python3 scripts/test_package_contract.py
      - name: Test CI change classifier
        run: python3 scripts/test_ci_changes.py
      - name: Validate workflow structure
        run: python3 scripts/validate_workflows.py
      - name: Test workflow structure policy
        run: python3 scripts/test_validate_workflows.py
      - name: Test DMG staging helper
        run: python3 scripts/test_create_macos_dmg.py
      - name: Test release state helper
        run: python3 scripts/test_release_state.py
      - name: Test published release contract
        run: python3 scripts/test_release_publish_contract.py
      - name: Test release rehearsal contract
        run: python3 scripts/test_release_rehearsal_contract.py
      - name: Test installer artifact contract
        run: python3 scripts/test_installer_artifact_contract.py
      - name: Test release artifact helper
        run: python3 scripts/test_release_artifacts.py
      - name: Test release text helper
        run: python3 scripts/test_release_text.py
      - name: Test first app-open contract
        run: python3 scripts/test_first_app_open_contract.py
      - name: Test packaged smoke helper
        run: python3 scripts/test_smoke_launch_macos_app.py
      - name: Test connector workflow contracts
        run: python3 scripts/test_connector_workflow_contracts.py
      - name: Test Notion connector contract
        run: python3 scripts/test_notion_connector_contract.py
      - name: Test distribution validator
        run: python3 scripts/test_validate_macos_distribution.py
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
      - name: Restore cached Swift app executable
        id: swift_app_binary_cache
        uses: actions/cache/restore@v5
        with:
          key: swift-app-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('apps/pith-macos/Package.swift', 'apps/pith-macos/Package.resolved', 'apps/pith-macos/Sources/**/*.swift') }}
      - name: Use cached Swift app executable
        run: cp "$SWIFT_APP_BINARY_CACHE_DIR/$SWIFT_APP_BINARY" "$PREBUILT_ARTIFACT_DIR/$SWIFT_APP_BINARY"
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
      - name: Restore cached runtime executable
        id: runtime_binary_cache
        uses: actions/cache/restore@v5
        with:
          key: runtime-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('Cargo.lock', 'Cargo.toml', 'crates/**/*.rs', 'crates/**/Cargo.toml') }}
      - name: Use cached runtime executable
        run: cp "$RUNTIME_BINARY_CACHE_DIR/$RUNTIME_BINARY" "$PREBUILT_ARTIFACT_DIR/$RUNTIME_BINARY"
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
    steps:
      - name: Cache Swift app executable
        id: package_swift_cache
        uses: actions/cache@v5
        with:
          key: swift-app-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('apps/pith-macos/Package.swift', 'apps/pith-macos/Package.resolved', 'apps/pith-macos/Sources/**/*.swift') }}
      - name: Cache runtime executable
        id: package_runtime_cache
        uses: actions/cache@v5
        with:
          key: runtime-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('Cargo.lock', 'Cargo.toml', 'crates/**/*.rs', 'crates/**/Cargo.toml') }}
      - name: Build missing package executables
        run: swift build --package-path "$SWIFT_PACKAGE_PATH" -c release --arch x86_64 && cargo build -p pith-runtime-bin --release
      - name: Cache pinned llama.cpp backend
        id: package_llama_cache
        uses: actions/cache@v5
        with:
          key: llama-backend-${{ runner.os }}-${{ runner.arch }}-${{ env.LLAMA_CPP_REF }}-v1
      - name: Build pinned llama.cpp backend
        run: python3 scripts/package_macos_app.py --stage-llama-backend llama-cli --stage-llama-output cache
      - name: Validate packaged llama.cpp backend
        run: test -x "$PREBUILT_ARTIFACT_DIR/$LLAMA_BINARY"
      - name: Build x86_64 macOS app bundle
        run: |
          python3 scripts/package_macos_app.py \
            --skip-build \
            --no-zip \
            --source-commit "$GITHUB_SHA"
      - name: Create internal macOS disk image
        run: |
          python3 scripts/create_macos_dmg.py \
            artifacts/macos/Pith.app \
            "artifacts/macos/$MACOS_DMG_NAME" \
            --readme-file artifacts/macos/README-FIRST.txt \
            --smoke-launch-script scripts/smoke_launch_macos_app.py \
            --smoke-receipt-output artifacts/macos/packaged-smoke-receipt.json
      - name: Create internal macOS checksum
        run: |
          python3 scripts/release_artifacts.py \
            --tag "ci-${GITHUB_SHA::12}" \
            --source-commit "$GITHUB_SHA" \
            --signing-mode ad-hoc \
            --install-guide artifacts/macos/README-FIRST.txt \
            --package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json \
            --smoke-receipt artifacts/macos/packaged-smoke-receipt.json \
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
      - name: Rehearse internal installer download
        run: |
          python3 scripts/release_rehearsal_contract.py \
            --tag "ci-${GITHUB_SHA::12}" \
            --asset-dir artifacts/macos \
            --allow-extra-assets \
            --summary-output artifacts/macos/internal-release-rehearsal.md
          cat artifacts/macos/internal-release-rehearsal.md >> "$GITHUB_STEP_SUMMARY"
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

permissions:
  actions: read
  contents: write

concurrency:
  group: release-${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name }}
  cancel-in-progress: false

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
      - name: Create release DMG
        run: |
          dmg_path="artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg"
          python3 scripts/create_macos_dmg.py \
            artifacts/macos/Pith.app \
            "$dmg_path" \
            --readme-file artifacts/macos/README-FIRST.txt \
            --smoke-launch-script scripts/smoke_launch_macos_app.py \
            --smoke-receipt-output artifacts/macos/packaged-smoke-receipt.json
      - name: Create release checksum
        run: |
          python3 scripts/release_artifacts.py \
            --tag "$RELEASE_TAG" \
            --source-commit "$PITH_RELEASE_SHA" \
            --signing-mode "$PITH_RELEASE_SIGNING_MODE" \
            --install-guide artifacts/macos/README-FIRST.txt \
            --package-manifest artifacts/macos/Pith.app/Contents/Resources/PithPackage.json \
            --smoke-receipt artifacts/macos/packaged-smoke-receipt.json \
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
      - name: Sign app for public distribution
        if: env.PITH_RELEASE_SIGNING_MODE == 'developer-id'
        run: |
          python3 scripts/sign_macos_app_for_distribution.py \
            artifacts/macos/Pith.app \
            --identity "$MACOS_DEVELOPER_ID_APPLICATION"
          python3 scripts/validate_macos_distribution.py artifacts/macos/Pith.app
      - name: Notarize and staple DMG
        if: env.PITH_RELEASE_SIGNING_MODE == 'developer-id'
        run: |
          dmg_path="artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg"
          xcrun notarytool submit "$dmg_path" \
            --apple-id "$APPLE_ID" \
            --team-id "$APPLE_TEAM_ID" \
            --password "$APPLE_APP_SPECIFIC_PASSWORD" \
            --wait
          xcrun stapler staple "$dmg_path"
          python3 scripts/validate_macos_distribution.py \
            artifacts/macos/Pith.app \
            --dmg-path "$dmg_path"
      - name: Plan GitHub Release state
        run: |
          release_title="Pith $RELEASE_TAG"
          python3 scripts/release_state.py
          --title "$release_title"
          --tag "$RELEASE_TAG"
          --summary-output release-plan.md
          cat release-state.env >> "$GITHUB_ENV"
          cat release-plan.md >> "$GITHUB_STEP_SUMMARY"
      - name: Upload GitHub Release draft assets
        run: |
          release_title="Pith $RELEASE_TAG"
          gh release upload "$RELEASE_TAG" \\
            "artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg" \\
            "artifacts/macos/Pith-$RELEASE_TAG-macos-x86_64.dmg.sha256" \\
            "artifacts/macos/README-FIRST.txt" \\
            "artifacts/macos/Pith-$RELEASE_TAG-release-manifest.json" \\
            --clobber
      - name: Rehearse downloaded GitHub Release assets
        run: |
          rm -rf release-download
          mkdir -p release-download
          gh release download "$RELEASE_TAG" \\
            --dir release-download \\
            --clobber
          python3 scripts/release_rehearsal_contract.py \\
            --tag "$RELEASE_TAG" \\
            --asset-dir release-download \\
            --summary-output release-rehearsal.md
          cat release-rehearsal.md >> "$GITHUB_STEP_SUMMARY"
      - name: Apply final GitHub Release visibility
        run: |
          gh api \\
            -X PATCH \\
            "repos/$GITHUB_REPOSITORY/releases/$release_id" \\
            --input release-state.json
      - name: Validate final GitHub Release
        run: |
          gh api "repos/$GITHUB_REPOSITORY/releases/tags/$RELEASE_TAG" > release-published.json
          python3 scripts/release_publish_contract.py \\
            --tag "$RELEASE_TAG" \\
            --release-json release-published.json \\
            --expected-draft "$PITH_RELEASE_STATE_DRAFT" \\
            --expected-prerelease "$PITH_RELEASE_STATE_PRERELEASE" \\
            --signing-mode "$PITH_RELEASE_SIGNING_MODE" \\
            --allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"
      - name: Upload release rehearsal summary
        uses: actions/upload-artifact@v7
        with:
          name: release-rehearsal-${{ env.RELEASE_TAG }}
          path: release-rehearsal.md
          if-no-files-found: error
          retention-days: 30
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
      ci=VALID_CI.replace("  contents: read", "  contents: write"),
    )
    assert_issue(issue_messages(root), "contents: read")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace("  cancel-in-progress: true", "  cancel-in-progress: false"),
    )
    assert_issue(issue_messages(root), "cancel-in-progress: true")

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
        "      - name: Test distribution signing helper\n"
        "        run: python3 scripts/test_sign_macos_app_for_distribution.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_sign_macos_app_for_distribution.py")

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
        "      - name: Test CI change classifier\n"
        "        run: python3 scripts/test_ci_changes.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_ci_changes.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test release text helper\n"
        "        run: python3 scripts/test_release_text.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_release_text.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test first app-open contract\n"
        "        run: python3 scripts/test_first_app_open_contract.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_first_app_open_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test published release contract\n"
        "        run: python3 scripts/test_release_publish_contract.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_release_publish_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test release rehearsal contract\n"
        "        run: python3 scripts/test_release_rehearsal_contract.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_release_rehearsal_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test release artifact helper\n"
        "        run: python3 scripts/test_release_artifacts.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_release_artifacts.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - name: Test DMG staging helper\n"
        "        run: python3 scripts/test_create_macos_dmg.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_create_macos_dmg.py")

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
        "      - changes",
        "      - changes\n      - swift-tests",
      ),
    )
    assert_issue(issue_messages(root), "must not wait for swift-tests")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - changes",
        "      - changes\n      - swift-app",
      ),
    )
    assert_issue(issue_messages(root), "Swift executable directly")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - changes",
        "      - changes\n      - macos-runtime",
      ),
    )
    assert_issue(issue_messages(root), "runtime executable directly")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - changes",
        "      - changes\n      - macos-llama-backend",
      ),
    )
    assert_issue(issue_messages(root), "cached llama.cpp directly")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "      - changes",
        "      - changes\n      - repository-policy",
      ),
    )
    assert_issue(issue_messages(root), "depend only on changes")

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
      release=VALID_RELEASE.replace("  contents: write", "  contents: read"),
    )
    assert_issue(issue_messages(root), "contents: write")

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
      release=VALID_RELEASE.replace('          release_title="Pith $RELEASE_TAG"', ""),
    )
    assert_issue(issue_messages(root), "Release title")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace('          --title "$release_title"\n', ""),
    )
    assert_issue(issue_messages(root), "release state helper must receive --title")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          cat release-state.env >> "$GITHUB_ENV"\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "stage boundary")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          cat release-plan.md >> "$GITHUB_STEP_SUMMARY"\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "stage boundary")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "scripts/release_publish_contract.py",
        "scripts/missing_release_publish_contract.py",
      ),
    )
    assert_issue(issue_messages(root), "published release contract helper")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace("-X PATCH", "-X POST"),
    )
    assert_issue(issue_messages(root), "final release state patch")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "          rm -rf release-download\n",
        "          gh api \\\n"
        "            -X PATCH \\\n"
        "            \"repos/$GITHUB_REPOSITORY/releases/$release_id\" \\\n"
        "            --input release-state.json\n"
        "          rm -rf release-download\n",
        1,
      ),
    )
    assert_issue(issue_messages(root), "rehearsal must pass")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--expected-draft "$PITH_RELEASE_STATE_DRAFT"',
        "--missing-expected-draft",
      ),
    )
    assert_issue(issue_messages(root), "expected-draft")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--signing-mode "$PITH_RELEASE_SIGNING_MODE"',
        "--missing-signing-mode",
      ),
    )
    assert_issue(issue_messages(root), "signing-mode")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"',
        "--missing-untrusted-ad-hoc",
      ),
    )
    assert_issue(issue_messages(root), "allow-untrusted-ad-hoc")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "scripts/release_rehearsal_contract.py",
        "scripts/missing_release_rehearsal_contract.py",
      ),
    )
    assert_issue(issue_messages(root), "release download rehearsal helper")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        'gh release download "$RELEASE_TAG"',
        'gh release view "$RELEASE_TAG"',
      ),
    )
    assert_issue(issue_messages(root), "release download rehearsal")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          cat release-rehearsal.md >> "$GITHUB_STEP_SUMMARY"\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "release download rehearsal")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace("          retention-days: 30\n", ""),
    )
    assert_issue(issue_messages(root), "retention-days")

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
      ci=VALID_CI.replace(
        "--smoke-receipt artifacts/macos/packaged-smoke-receipt.json",
        "",
      ),
    )
    assert_issue(issue_messages(root), "packaged-smoke-receipt.json")

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
      ci=VALID_CI.replace(
        "python3 scripts/release_rehearsal_contract.py",
        "python3 scripts/missing_release_rehearsal_contract.py",
      ),
    )
    assert_issue(issue_messages(root), "release_rehearsal_contract.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace("--allow-extra-assets", "--missing-extra-assets"),
    )
    assert_issue(issue_messages(root), "--allow-extra-assets")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "            --asset-dir release-download \\\n",
        "            --asset-dir release-download \\\n            --allow-extra-assets \\\n",
      ),
    )
    assert_issue(issue_messages(root), "must not allow extra assets")

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
        "python3 scripts/sign_macos_app_for_distribution.py",
        "python3 scripts/missing_sign_macos_app_for_distribution.py",
      ),
    )
    assert_issue(issue_messages(root), "sign_macos_app_for_distribution.py")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace('--identity "$MACOS_DEVELOPER_ID_APPLICATION"', ""),
    )
    assert_issue(issue_messages(root), "MACOS_DEVELOPER_ID_APPLICATION")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace("xcrun notarytool submit", "xcrun missing_notarytool submit"),
    )
    assert_issue(issue_messages(root), "notarytool")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace('xcrun stapler staple "$dmg_path"', "echo missing stapler"),
    )
    assert_issue(issue_messages(root), "stapler")

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
        "--smoke-receipt artifacts/macos/packaged-smoke-receipt.json",
        "",
      ),
    )
    assert_issue(issue_messages(root), "packaged-smoke-receipt.json")

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
