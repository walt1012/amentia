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
  SWIFT_APP_ARTIFACT: internal-AmentiaApp-x86_64
  RUNTIME_ARTIFACT: internal-amentia-runtime-bin-x86_64
  LLAMA_ARTIFACT: internal-llama-cli-x86_64
  MACOS_APP_ARTIFACT: Amentia-installer-x86_64

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
      - name: Test release readiness helper
        run: python3 scripts/test_release_readiness.py
      - name: Test published release contract
        run: python3 scripts/test_release_publish_contract.py
      - name: Test release rehearsal contract
        run: python3 scripts/test_release_rehearsal_contract.py
      - name: Test release evidence contract
        run: python3 scripts/test_release_evidence_contract.py
      - name: Test installer artifact contract
        run: python3 scripts/test_installer_artifact_contract.py
      - name: Test manual acceptance contract
        run: python3 scripts/test_manual_acceptance_contract.py
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
          key: swift-app-bin-${{ github.repository }}-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('apps/amentia-macos/Package.swift', 'apps/amentia-macos/Package.resolved', 'apps/amentia-macos/Sources/**/*.swift') }}
      - name: Use cached Swift app executable
        run: cp "$SWIFT_APP_BINARY_CACHE_DIR/$SWIFT_APP_BINARY" "$PREBUILT_ARTIFACT_DIR/$SWIFT_APP_BINARY"
      - name: Cache Swift build output
        uses: actions/cache@v5
        with:
          key: swift-build-${{ github.repository }}-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('apps/amentia-macos/Package.swift', 'apps/amentia-macos/Package.resolved', 'apps/amentia-macos/Sources/**/*.swift') }}
      - name: Upload Swift app executable
        uses: actions/upload-artifact@v7
        with:
          name: ${{ env.SWIFT_APP_ARTIFACT }}
          retention-days: 1
  swift-tests:
    timeout-minutes: 25
    steps:
      - name: Cache Swift test output
        uses: actions/cache@v5
        with:
          key: swift-test-${{ github.repository }}-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('apps/amentia-macos/Package.swift', 'apps/amentia-macos/Package.resolved', 'apps/amentia-macos/Sources/**/*.swift', 'apps/amentia-macos/Tests/**/*.swift') }}
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
          key: swift-app-bin-${{ github.repository }}-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('apps/amentia-macos/Package.swift', 'apps/amentia-macos/Package.resolved', 'apps/amentia-macos/Sources/**/*.swift') }}
      - name: Cache Swift package build output
        uses: actions/cache@v5
        with:
          key: package-swift-build-${{ github.repository }}-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('apps/amentia-macos/Package.swift', 'apps/amentia-macos/Package.resolved', 'apps/amentia-macos/Sources/**/*.swift') }}
      - name: Cache runtime executable
        id: package_runtime_cache
        uses: actions/cache@v5
        with:
          key: runtime-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('Cargo.lock', 'Cargo.toml', 'crates/**/*.rs', 'crates/**/Cargo.toml') }}
      - name: Build missing package executables
        run: swift build --package-path "$SWIFT_PACKAGE_PATH" -c release --arch x86_64 && cargo build -p amentia-runtime-bin --release
      - name: Cache pinned llama.cpp backend
        id: package_llama_cache
        uses: actions/cache@v5
        with:
          key: llama-backend-${{ runner.os }}-${{ runner.arch }}-${{ env.LLAMA_CPP_REF }}-v1
      - name: Build pinned llama.cpp backend
        run: bash scripts/build_macos_llama_backend.sh
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
            artifacts/macos/Amentia.app \
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
            --package-manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json \
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
            --manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json \
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
    inputs:
      tag:
        type: string
      draft:
        default: true
        type: boolean
      prerelease:
        default: true
        type: boolean
      publish_untrusted_ad_hoc:
        default: false
        type: boolean
      manual_acceptance_confirmed:
        description: "Confirm the validated manual acceptance receipt passed on a fresh Mac before visible ad-hoc publishing."
        default: false
        type: boolean
      manual_acceptance_evidence:
        description: "HTTPS URL for the validated manual acceptance receipt required before visible ad-hoc publishing."
        default: ""
        type: string
      dry_run:
        default: true
        type: boolean

defaults:
  run:
    shell: bash

permissions:
  actions: read
  contents: write

concurrency:
  group: release-${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name }}
  cancel-in-progress: false

env:
  RELEASE_DRAFT: ${{ github.event_name != 'workflow_dispatch' || inputs.draft }}
  RELEASE_PRERELEASE: ${{ github.event_name != 'workflow_dispatch' || inputs.prerelease }}
  RELEASE_ALLOW_UNTRUSTED_AD_HOC: ${{ github.event_name == 'workflow_dispatch' && inputs.publish_untrusted_ad_hoc || false }}
  RELEASE_MANUAL_ACCEPTANCE_CONFIRMED: ${{ github.event_name == 'workflow_dispatch' && inputs.manual_acceptance_confirmed || false }}
  RELEASE_MANUAL_ACCEPTANCE_EVIDENCE: ${{ github.event_name == 'workflow_dispatch' && inputs.manual_acceptance_evidence || '' }}
  RELEASE_DRY_RUN: ${{ github.event_name == 'workflow_dispatch' && inputs.dry_run || false }}

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
          git fetch --depth 1 origin "refs/tags/$RELEASE_TAG:refs/tags/$RELEASE_TAG"
          release_tag_commit="$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")"
          echo "AMENTIA_RELEASE_TAG_COMMIT=$release_tag_commit" >> "$GITHUB_ENV"
          gh run list --workflow CI --status success --json conclusion,headSha,url
          echo "AMENTIA_RELEASE_CI_RUN_URL=https://github.com/walt1012/amentia/actions/runs/100" >> "$GITHUB_ENV"
      - name: Write release readiness report
        run: |
          python3 scripts/release_readiness.py \
            --tag "$RELEASE_TAG" \
            --ci-run-url "$AMENTIA_RELEASE_CI_RUN_URL" \
            --output release-readiness.md \
            --json-output release-readiness.json \
            --dry-run "$RELEASE_DRY_RUN" \
            --signing-mode "$AMENTIA_RELEASE_SIGNING_MODE" \
            --requested-draft "$RELEASE_DRAFT" \
            --requested-prerelease "$RELEASE_PRERELEASE" \
            --allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC" \
            --manual-acceptance-confirmed "$RELEASE_MANUAL_ACCEPTANCE_CONFIRMED" \
            --manual-acceptance-evidence "$RELEASE_MANUAL_ACCEPTANCE_EVIDENCE"
          cat release-readiness.md >> "$GITHUB_STEP_SUMMARY"
      - name: Audit remote model catalog metadata
        run: python3 scripts/validate_model_pack.py --remote
      - name: Build pinned llama.cpp backend
        run: bash scripts/build_macos_llama_backend.sh
      - name: Create release DMG
        run: |
          dmg_path="artifacts/macos/Amentia-$RELEASE_TAG-macos-x86_64.dmg"
          python3 scripts/create_macos_dmg.py \
            artifacts/macos/Amentia.app \
            "$dmg_path" \
            --readme-file artifacts/macos/README-FIRST.txt \
            --smoke-launch-script scripts/smoke_launch_macos_app.py \
            --smoke-receipt-output artifacts/macos/packaged-smoke-receipt.json
      - name: Create release checksum
        run: |
          python3 scripts/release_artifacts.py \
            --tag "$RELEASE_TAG" \
            --source-commit "$AMENTIA_RELEASE_SHA" \
            --signing-mode "$AMENTIA_RELEASE_SIGNING_MODE" \
            --install-guide artifacts/macos/README-FIRST.txt \
            --package-manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json \
            --smoke-receipt artifacts/macos/packaged-smoke-receipt.json \
            --workflow-run-id "$GITHUB_RUN_ID" \
            --workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID" \
            --manifest-output "artifacts/macos/Amentia-$RELEASE_TAG-release-manifest.json"
      - name: Validate installer artifact contract
        run: |
          dmg_path="artifacts/macos/Amentia-$RELEASE_TAG-macos-x86_64.dmg"
          python3 scripts/installer_artifact_contract.py \
            --tag "$RELEASE_TAG" \
            --asset "$dmg_path" \
            --asset "$dmg_path.sha256" \
            --asset artifacts/macos/README-FIRST.txt \
            --asset "artifacts/macos/Amentia-$RELEASE_TAG-release-manifest.json"
      - name: Validate package contract
        run: |
          python3 scripts/package_contract.py \
            --manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json \
            --source-commit "$AMENTIA_RELEASE_SHA" \
            --signing-mode "$AMENTIA_RELEASE_SIGNING_MODE" \
            --bundle-version "$AMENTIA_RELEASE_VERSION"
      - name: Sign app for public distribution
        if: env.AMENTIA_RELEASE_SIGNING_MODE == 'developer-id'
        run: |
          python3 scripts/sign_macos_app_for_distribution.py \
            artifacts/macos/Amentia.app \
            --identity "$MACOS_DEVELOPER_ID_APPLICATION"
          python3 scripts/validate_macos_distribution.py artifacts/macos/Amentia.app
      - name: Notarize and staple DMG
        if: env.AMENTIA_RELEASE_SIGNING_MODE == 'developer-id'
        run: |
          dmg_path="artifacts/macos/Amentia-$RELEASE_TAG-macos-x86_64.dmg"
          xcrun notarytool submit "$dmg_path" \
            --apple-id "$APPLE_ID" \
            --team-id "$APPLE_TEAM_ID" \
            --password "$APPLE_APP_SPECIFIC_PASSWORD" \
            --wait
          xcrun stapler staple "$dmg_path"
          python3 scripts/validate_macos_distribution.py \
            artifacts/macos/Amentia.app \
            --dmg-path "$dmg_path"
      - name: Plan GitHub Release state
        run: |
          release_title="Amentia $RELEASE_TAG"
          python3 scripts/release_state.py
          --title "$release_title"
          --tag "$RELEASE_TAG"
          --signing-mode "$AMENTIA_RELEASE_SIGNING_MODE"
          --requested-draft "$RELEASE_DRAFT"
          --requested-prerelease "$RELEASE_PRERELEASE"
          --allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"
          --summary-output release-plan.md
          --plan-output release-plan.json
          --source-commit "$AMENTIA_RELEASE_SHA"
          --ci-run-url "$AMENTIA_RELEASE_CI_RUN_URL"
          --workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"
          --dry-run "$RELEASE_DRY_RUN"
          --manual-acceptance-confirmed "$RELEASE_MANUAL_ACCEPTANCE_CONFIRMED"
          --manual-acceptance-evidence "$RELEASE_MANUAL_ACCEPTANCE_EVIDENCE"
          cat release-state.env >> "$GITHUB_ENV"
          cat release-plan.md >> "$GITHUB_STEP_SUMMARY"
          gh release view "$RELEASE_TAG" --json databaseId,isDraft,tagName,name,assets > release-existing.json
          existing_draft="$(python3 -c 'import json; print(str(json.load(open("release-existing.json"))["isDraft"]).lower())')"
          python3 scripts/release_publish_contract.py \\
            --mode preupload-existing-assets \\
            --tag "$RELEASE_TAG" \\
            --release-json release-existing.json
      - name: Rehearse release dry-run assets
        if: env.RELEASE_DRY_RUN == 'true'
        run: |
          mkdir -p release-dry-run-assets
          python3 scripts/release_rehearsal_contract.py \\
            --tag "$RELEASE_TAG" \\
            --asset-dir release-dry-run-assets \\
            --summary-output release-dry-run-rehearsal.md \\
            --acceptance-output release-dry-run-manual-acceptance.md \\
            --json-output release-dry-run-rehearsal.json
          cat release-dry-run-rehearsal.md >> "$GITHUB_STEP_SUMMARY"
          cat release-dry-run-manual-acceptance.md >> "$GITHUB_STEP_SUMMARY"
      - name: Validate release dry-run evidence
        if: env.RELEASE_DRY_RUN == 'true'
        run: |
          python3 scripts/release_evidence_contract.py \\
            --mode dry-run \\
            --tag "$RELEASE_TAG" \\
            --evidence "artifacts/macos/Amentia-$RELEASE_TAG-macos-x86_64.dmg" \\
            --evidence "artifacts/macos/Amentia-$RELEASE_TAG-macos-x86_64.dmg.sha256" \\
            --evidence artifacts/macos/README-FIRST.txt \\
            --evidence "artifacts/macos/Amentia-$RELEASE_TAG-release-manifest.json" \\
            --evidence release-readiness.md \\
            --evidence release-readiness.json \\
            --evidence release-plan.md \\
            --evidence release-plan.json \\
            --evidence release-dry-run-rehearsal.md \\
            --evidence release-dry-run-rehearsal.json \\
            --evidence release-dry-run-manual-acceptance.md
      - name: Upload release dry-run assets
        if: env.RELEASE_DRY_RUN == 'true'
        uses: actions/upload-artifact@v7
        with:
          name: release-dry-run-${{ env.RELEASE_TAG }}
          path: |
            artifacts/macos/Amentia-${{ env.RELEASE_TAG }}-macos-x86_64.dmg
            artifacts/macos/Amentia-${{ env.RELEASE_TAG }}-macos-x86_64.dmg.sha256
            artifacts/macos/README-FIRST.txt
            artifacts/macos/Amentia-${{ env.RELEASE_TAG }}-release-manifest.json
            release-readiness.md
            release-readiness.json
            release-plan.md
            release-plan.json
            release-dry-run-rehearsal.md
            release-dry-run-rehearsal.json
            release-dry-run-manual-acceptance.md
          retention-days: 7
      - name: Upload GitHub Release draft assets
        if: env.RELEASE_DRY_RUN != 'true'
        run: |
          release_title="Amentia $RELEASE_TAG"
          gh release upload "$RELEASE_TAG" \\
            "artifacts/macos/Amentia-$RELEASE_TAG-macos-x86_64.dmg" \\
            "artifacts/macos/Amentia-$RELEASE_TAG-macos-x86_64.dmg.sha256" \\
            "artifacts/macos/README-FIRST.txt" \\
            "artifacts/macos/Amentia-$RELEASE_TAG-release-manifest.json" \\
            --clobber
      - name: Rehearse downloaded GitHub Release assets
        if: env.RELEASE_DRY_RUN != 'true'
        run: |
          rm -rf release-download
          mkdir -p release-download
          gh release download "$RELEASE_TAG" \\
            --dir release-download \\
            --clobber
          python3 scripts/release_rehearsal_contract.py \\
            --tag "$RELEASE_TAG" \\
            --asset-dir release-download \\
            --summary-output release-rehearsal.md \\
            --acceptance-output release-manual-acceptance.md \\
            --json-output release-rehearsal.json
          cat release-rehearsal.md >> "$GITHUB_STEP_SUMMARY"
          cat release-manual-acceptance.md >> "$GITHUB_STEP_SUMMARY"
      - name: Validate release rehearsal evidence
        if: env.RELEASE_DRY_RUN != 'true'
        run: |
          python3 scripts/release_evidence_contract.py \\
            --mode publish-rehearsal \\
            --tag "$RELEASE_TAG" \\
            --evidence release-readiness.md \\
            --evidence release-readiness.json \\
            --evidence release-plan.md \\
            --evidence release-plan.json \\
            --evidence release-rehearsal.md \\
            --evidence release-rehearsal.json \\
            --evidence release-manual-acceptance.md
      - name: Apply final GitHub Release visibility
        if: env.RELEASE_DRY_RUN != 'true'
        run: |
          release_id="$(gh release view "$RELEASE_TAG" --json databaseId --jq .databaseId)"
          test -n "$release_id"
          echo "AMENTIA_RELEASE_ID=$release_id" >> "$GITHUB_ENV"
          gh api \\
            -X PATCH \\
            "repos/$GITHUB_REPOSITORY/releases/$release_id" \\
            --input release-state.json
      - name: Validate final GitHub Release
        if: env.RELEASE_DRY_RUN != 'true'
        run: |
          test -n "${AMENTIA_RELEASE_ID:-}"
          gh api "repos/$GITHUB_REPOSITORY/releases/$AMENTIA_RELEASE_ID" \\
            > release-published.json
          python3 scripts/release_publish_contract.py \\
            --tag "$RELEASE_TAG" \\
            --release-json release-published.json \\
            --source-commit "$AMENTIA_RELEASE_SHA" \\
            --tag-commit "$AMENTIA_RELEASE_TAG_COMMIT" \\
            --expected-draft "$AMENTIA_RELEASE_STATE_DRAFT" \\
            --expected-prerelease "$AMENTIA_RELEASE_STATE_PRERELEASE" \\
            --signing-mode "$AMENTIA_RELEASE_SIGNING_MODE" \\
            --allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC" \\
            --manual-acceptance-evidence "$RELEASE_MANUAL_ACCEPTANCE_EVIDENCE"
      - name: Upload release rehearsal summary
        if: always() && env.RELEASE_DRY_RUN != 'true'
        uses: actions/upload-artifact@v7
        with:
          name: release-rehearsal-${{ env.RELEASE_TAG }}
          path: |
            release-readiness.md
            release-readiness.json
            release-plan.md
            release-plan.json
            release-rehearsal.md
            release-rehearsal.json
            release-manual-acceptance.md
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
        "      - name: Test release evidence contract\n"
        "        run: python3 scripts/test_release_evidence_contract.py\n",
        "",
      ),
    )
    assert_issue(issue_messages(root), "test_release_evidence_contract.py")

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
        "SWIFT_APP_ARTIFACT: internal-AmentiaApp-x86_64",
        "SWIFT_APP_ARTIFACT: AmentiaApp-x86_64",
      ),
    )
    assert_issue(issue_messages(root), "internal-AmentiaApp")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(root, ci=VALID_CI.replace("${{ github.repository }}-", ""))
    assert_issue(issue_messages(root), "github.repository")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      ci=VALID_CI.replace(
        "            artifacts/macos/${{ env.MACOS_DMG_NAME }}",
        "            artifacts/macos/Amentia-macos-x86_64.zip\n"
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
        "      dry_run:\n"
        "        default: true\n"
        "        type: boolean\n",
        "      dry_run:\n"
        "        default: false\n"
        "        type: boolean\n",
      ),
    )
    assert_issue(issue_messages(root), "dry_run must include default: true")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "      prerelease:\n"
        "        default: true\n"
        "        type: boolean\n",
        "      prerelease:\n"
        "        default: false\n"
        "        type: boolean\n",
      ),
    )
    assert_issue(issue_messages(root), "prerelease must include default: true")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "HTTPS URL for the validated manual acceptance receipt",
        "Acceptance artifact, issue, or notes URL",
      ),
    )
    assert_issue(issue_messages(root), "manual_acceptance_evidence must include HTTPS URL")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "RELEASE_DRY_RUN: ${{ github.event_name == 'workflow_dispatch' && inputs.dry_run || false }}",
        "RELEASE_DRY_RUN: ${{ github.event_name != 'workflow_dispatch' || inputs.dry_run }}",
      ),
    )
    assert_issue(issue_messages(root), "RELEASE_DRY_RUN")

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
    audit_step = (
      "      - name: Audit remote model catalog metadata\n"
      "        run: python3 scripts/validate_model_pack.py --remote\n"
    )
    release_with_early_audit = VALID_RELEASE.replace(audit_step, "", 1).replace(
      "      - name: Write release readiness report\n",
      audit_step + "      - name: Write release readiness report\n",
      1,
    )
    write_workflows(
      root,
      release=release_with_early_audit,
    )
    assert_issue(issue_messages(root), "readiness must be written before remote model audit")

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
      release=VALID_RELEASE.replace('          release_title="Amentia $RELEASE_TAG"', ""),
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
        '--requested-prerelease "$RELEASE_PRERELEASE"',
        "",
      ),
    )
    assert_issue(issue_messages(root), "release readiness helper must receive shared release input")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          --allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "release state helper must receive shared release input")

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
        "--mode preupload-existing-assets",
        "--missing-preupload-mode",
      ),
    )
    assert_issue(issue_messages(root), "stage boundary")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "      - name: Upload GitHub Release draft assets\n"
        "        if: env.RELEASE_DRY_RUN != 'true'\n",
        "      - name: Upload GitHub Release draft assets\n",
      ),
    )
    assert_issue(issue_messages(root), "final release state patch")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "release-dry-run-${{ env.RELEASE_TAG }}",
        "release-dry-run-missing",
      ),
    )
    assert_issue(issue_messages(root), "stage boundary")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "      - name: Upload release dry-run assets",
        "      - name: Upload release dry-run assets moved",
        1,
      ).replace(
        "      - name: Rehearse release dry-run assets",
        "      - name: Upload release dry-run assets",
        1,
      ).replace(
        "      - name: Upload release dry-run assets moved",
        "      - name: Rehearse release dry-run assets",
        1,
      ),
    )
    assert_issue(issue_messages(root), "dry-run rehearsal")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "      - name: Validate release dry-run evidence",
        "      - name: Missing release dry-run evidence",
        1,
      ),
    )
    assert_issue(issue_messages(root), "release workflow stage boundary")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "--evidence release-plan.md",
        "--missing-release-plan-md",
        1,
      ),
    )
    assert_issue(issue_messages(root), "dry-run evidence validation step")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "--evidence release-manual-acceptance.md",
        "--missing-release-manual-acceptance-md",
        1,
      ),
    )
    assert_issue(issue_messages(root), "publish rehearsal evidence validation step")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "      - name: Upload release dry-run assets",
        "      - name: Upload release dry-run assets moved",
        1,
      ).replace(
        "      - name: Validate release dry-run evidence",
        "      - name: Upload release dry-run assets",
        1,
      ).replace(
        "      - name: Upload release dry-run assets moved",
        "      - name: Validate release dry-run evidence",
        1,
      ),
    )
    assert_issue(issue_messages(root), "dry-run evidence validation")

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
    assert_issue(issue_messages(root), "publish rehearsal evidence validation")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          release_tag_commit="$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")"\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "release_tag_commit")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          release_tag_commit="$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")"\n',
        '          release_tag_commit="$(\n'
        '            git ls-remote --exit-code --tags origin \\\n'
        '              "refs/tags/$RELEASE_TAG" "refs/tags/$RELEASE_TAG^{}" | tail -n 1 | awk \'{print $1}\'\n'
        '          )"\n',
      ),
    )
    assert_issue(issue_messages(root), "git rev-parse")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          test -n "${AMENTIA_RELEASE_ID:-}"\n',
        "          trap 'echo \"::error title=Release final validation failed::Validate final GitHub Release failed.\"' ERR\n"
        '          test -n "${AMENTIA_RELEASE_ID:-}"\n',
      ),
    )
    assert_issue(issue_messages(root), "generic error annotations")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '          release_id="$(gh release view "$RELEASE_TAG" --json databaseId --jq .databaseId)"\n',
        '          release_id="$(\n'
        '            gh api "repos/$GITHUB_REPOSITORY/releases?per_page=100" |\n'
        '              python3 -c \'import json, os, sys; tag = os.environ["RELEASE_TAG"]; releases = json.load(sys.stdin); match = next((release for release in releases if release.get("tag_name") == tag), None); print(match["id"] if match else "")\'\n'
        '          )"\n',
      ),
    )
    assert_issue(issue_messages(root), "gh release view")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "          python3 scripts/release_publish_contract.py \\\n",
        '          release_tag_commit="$(git rev-parse "$RELEASE_TAG^{commit}")"\n'
        "          python3 scripts/release_publish_contract.py \\\n",
      ),
    )
    assert_issue(issue_messages(root), "AMENTIA_RELEASE_TAG_COMMIT")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--source-commit "$AMENTIA_RELEASE_SHA"',
        "--missing-source-commit",
      ),
    )
    assert_issue(issue_messages(root), "source-commit")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    final_validation_source = (
      '            --release-json release-published.json \\\n'
      '            --source-commit "$AMENTIA_RELEASE_SHA" \\\n'
    )
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        final_validation_source,
        final_validation_source.replace(
          '--source-commit "$AMENTIA_RELEASE_SHA"',
          '--source-commit "$SOURCE_COMMIT"',
        ),
        1,
      ),
    )
    assert_issue(issue_messages(root), "undefined SOURCE_COMMIT")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--tag-commit "$AMENTIA_RELEASE_TAG_COMMIT"',
        "--missing-tag-commit",
      ),
    )
    assert_issue(issue_messages(root), "tag-commit")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--expected-draft "$AMENTIA_RELEASE_STATE_DRAFT"',
        "--missing-expected-draft",
      ),
    )
    assert_issue(issue_messages(root), "expected-draft")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        '--signing-mode "$AMENTIA_RELEASE_SIGNING_MODE"',
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
        '--manual-acceptance-evidence "$RELEASE_MANUAL_ACCEPTANCE_EVIDENCE"',
        "--missing-manual-acceptance-evidence",
      ),
    )
    assert_issue(issue_messages(root), "manual-acceptance-evidence")

  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(
      root,
      release=VALID_RELEASE.replace(
        "      - name: Validate release rehearsal evidence",
        "      - name: Missing release rehearsal evidence",
        1,
      ),
    )
    assert_issue(issue_messages(root), "final release state patch")

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
        "--package-manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json",
        "",
      ),
    )
    assert_issue(issue_messages(root), "AmentiaPackage.json")

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
      release=VALID_RELEASE.replace('--source-commit "$AMENTIA_RELEASE_SHA"', ""),
    )
    assert_issue(issue_messages(root), "source-commit")

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
        "--package-manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json",
        "",
      ),
    )
    assert_issue(issue_messages(root), "AmentiaPackage.json")

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
        '            --manifest-output "artifacts/macos/Amentia-$RELEASE_TAG-release-manifest.json"\n',
        "",
      ),
    )
    assert_issue(issue_messages(root), "Amentia-$RELEASE_TAG-release-manifest.json")

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
