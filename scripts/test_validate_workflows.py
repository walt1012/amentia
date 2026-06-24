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
      manual_acceptance_receipt_url:
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
  RELEASE_MANUAL_ACCEPTANCE_RECEIPT_URL: ${{ github.event_name == 'workflow_dispatch' && inputs.manual_acceptance_receipt_url || '' }}
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
            --manual-acceptance-receipt-url "$RELEASE_MANUAL_ACCEPTANCE_RECEIPT_URL"
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
          --manual-acceptance-receipt-url "$RELEASE_MANUAL_ACCEPTANCE_RECEIPT_URL"
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
            --manual-acceptance-receipt-url "$RELEASE_MANUAL_ACCEPTANCE_RECEIPT_URL"
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


def workflow_policy_messages(
  ci: str = VALID_CI,
  release: str = VALID_RELEASE,
) -> list[str]:
  with TemporaryDirectory() as directory:
    root = Path(directory)
    write_workflows(root, ci=ci, release=release)
    return issue_messages(root)


def assert_workflows_valid() -> None:
  messages = workflow_policy_messages()
  if messages:
    raise AssertionError(f"expected no workflow policy issues, got {messages!r}")


def assert_policy_issue(
  expected: str,
  *,
  ci: str = VALID_CI,
  release: str = VALID_RELEASE,
) -> None:
  assert_issue(workflow_policy_messages(ci=ci, release=release), expected)


def ci_policy_cases() -> list[tuple[str, str]]:
  return [
    (
      "persist-credentials: false",
      VALID_CI.replace("persist-credentials: false", "fetch-depth: 1", 1),
    ),
    ("contents: read", VALID_CI.replace("  contents: read", "  contents: write")),
    (
      "cancel-in-progress: true",
      VALID_CI.replace("  cancel-in-progress: true", "  cancel-in-progress: false"),
    ),
    (
      "test_release_identity.py",
      VALID_CI.replace(
        "      - name: Test release identity helper\n"
        "        run: python3 scripts/test_release_identity.py\n",
        "",
      ),
    ),
    (
      "test_sign_macos_app_for_distribution.py",
      VALID_CI.replace(
        "      - name: Test distribution signing helper\n"
        "        run: python3 scripts/test_sign_macos_app_for_distribution.py\n",
        "",
      ),
    ),
    (
      "test_package_contract.py",
      VALID_CI.replace(
        "      - name: Test package contract helper\n"
        "        run: python3 scripts/test_package_contract.py\n",
        "",
      ),
    ),
    (
      "test_ci_changes.py",
      VALID_CI.replace(
        "      - name: Test CI change classifier\n"
        "        run: python3 scripts/test_ci_changes.py\n",
        "",
      ),
    ),
    (
      "test_release_text.py",
      VALID_CI.replace(
        "      - name: Test release text helper\n"
        "        run: python3 scripts/test_release_text.py\n",
        "",
      ),
    ),
    (
      "test_first_app_open_contract.py",
      VALID_CI.replace(
        "      - name: Test first app-open contract\n"
        "        run: python3 scripts/test_first_app_open_contract.py\n",
        "",
      ),
    ),
    (
      "test_release_publish_contract.py",
      VALID_CI.replace(
        "      - name: Test published release contract\n"
        "        run: python3 scripts/test_release_publish_contract.py\n",
        "",
      ),
    ),
    (
      "test_release_rehearsal_contract.py",
      VALID_CI.replace(
        "      - name: Test release rehearsal contract\n"
        "        run: python3 scripts/test_release_rehearsal_contract.py\n",
        "",
      ),
    ),
    (
      "test_release_artifacts.py",
      VALID_CI.replace(
        "      - name: Test release artifact helper\n"
        "        run: python3 scripts/test_release_artifacts.py\n",
        "",
      ),
    ),
    (
      "test_create_macos_dmg.py",
      VALID_CI.replace(
        "      - name: Test DMG staging helper\n"
        "        run: python3 scripts/test_create_macos_dmg.py\n",
        "",
      ),
    ),
    (
      "test_installer_artifact_contract.py",
      VALID_CI.replace(
        "      - name: Test installer artifact contract\n"
        "        run: python3 scripts/test_installer_artifact_contract.py\n",
        "",
      ),
    ),
    (
      "test_connector_workflow_contracts.py",
      VALID_CI.replace(
        "      - name: Test connector workflow contracts\n"
        "        run: python3 scripts/test_connector_workflow_contracts.py\n",
        "",
      ),
    ),
    (
      "test_notion_connector_contract.py",
      VALID_CI.replace(
        "      - name: Test Notion connector contract\n"
        "        run: python3 scripts/test_notion_connector_contract.py\n",
        "",
      ),
    ),
    ("retention-days", VALID_CI.replace("          retention-days: 7\n", "")),
    ("internal artifact", VALID_CI.replace("          retention-days: 1\n", "", 1)),
    (
      "internal-AmentiaApp",
      VALID_CI.replace(
        "SWIFT_APP_ARTIFACT: internal-AmentiaApp-x86_64",
        "SWIFT_APP_ARTIFACT: AmentiaApp-x86_64",
      ),
    ),
    ("github.repository", VALID_CI.replace("${{ github.repository }}-", "")),
    (
      "internal zip",
      VALID_CI.replace(
        "            artifacts/macos/${{ env.MACOS_DMG_NAME }}",
        "            artifacts/macos/Amentia-macos-x86_64.zip\n"
        "            artifacts/macos/${{ env.MACOS_DMG_NAME }}",
      ),
    ),
    (
      "internal release notes",
      VALID_CI.replace(
        "            artifacts/macos/internal-release-manifest.json",
        "            artifacts/macos/internal-release-notes.md\n"
        "            artifacts/macos/internal-release-manifest.json",
      ),
    ),
    (
      "must not wait for swift-tests",
      VALID_CI.replace("      - changes", "      - changes\n      - swift-tests"),
    ),
    (
      "Swift executable directly",
      VALID_CI.replace("      - changes", "      - changes\n      - swift-app"),
    ),
    (
      "runtime executable directly",
      VALID_CI.replace("      - changes", "      - changes\n      - macos-runtime"),
    ),
    (
      "cached llama.cpp directly",
      VALID_CI.replace("      - changes", "      - changes\n      - macos-llama-backend"),
    ),
    (
      "depend only on changes",
      VALID_CI.replace("      - changes", "      - changes\n      - repository-policy"),
    ),
    (
      "macos-package release manifest",
      VALID_CI.replace('--source-commit "$GITHUB_SHA"', ""),
    ),
    ("--no-zip", VALID_CI.replace("--no-zip", "")),
    (
      "package_contract.py",
      VALID_CI.replace(
        "python3 scripts/package_contract.py",
        "python3 scripts/missing_package_contract.py",
      ),
    ),
    (
      "AmentiaPackage.json",
      VALID_CI.replace(
        "--package-manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json",
        "",
      ),
    ),
    (
      "packaged-smoke-receipt.json",
      VALID_CI.replace("--smoke-receipt artifacts/macos/packaged-smoke-receipt.json", ""),
    ),
    ("workflow-run-id", VALID_CI.replace('--workflow-run-id "$GITHUB_RUN_ID"', "")),
    (
      "internal-release-manifest.json",
      VALID_CI.replace(
        "            --manifest-output artifacts/macos/internal-release-manifest.json\n",
        "",
      ),
    ),
    (
      "installer_artifact_contract.py",
      VALID_CI.replace(
        "python3 scripts/installer_artifact_contract.py",
        "python3 scripts/missing_installer_artifact_contract.py",
      ),
    ),
    (
      "release_rehearsal_contract.py",
      VALID_CI.replace(
        "python3 scripts/release_rehearsal_contract.py",
        "python3 scripts/missing_release_rehearsal_contract.py",
      ),
    ),
    ("--allow-extra-assets", VALID_CI.replace("--allow-extra-assets", "--missing-extra-assets")),
  ]
def release_policy_cases() -> list[tuple[str, str]]:
  audit_step = (
    "      - name: Audit remote model catalog metadata\n"
    "        run: python3 scripts/validate_model_pack.py --remote\n"
  )
  final_validation_source = (
    '            --release-json release-published.json \\\n'
    '            --source-commit "$AMENTIA_RELEASE_SHA" \\\n'
  )
  return [
    ("release CI gate", VALID_RELEASE.replace("--workflow CI", "--workflow Release")),
    ("contents: write", VALID_RELEASE.replace("  contents: write", "  contents: read")),
    (
      "dry_run must include default: true",
      VALID_RELEASE.replace(
        "      dry_run:\n"
        "        default: true\n"
        "        type: boolean\n",
        "      dry_run:\n"
        "        default: false\n"
        "        type: boolean\n",
      ),
    ),
    (
      "prerelease must include default: true",
      VALID_RELEASE.replace(
        "      prerelease:\n"
        "        default: true\n"
        "        type: boolean\n",
        "      prerelease:\n"
        "        default: false\n"
        "        type: boolean\n",
      ),
    ),
    (
      "manual_acceptance_receipt_url must include HTTPS URL",
      VALID_RELEASE.replace(
        "HTTPS URL for the validated manual acceptance receipt",
        "Acceptance artifact, issue, or notes URL",
      ),
    ),
    (
      "RELEASE_DRY_RUN",
      VALID_RELEASE.replace(
        "RELEASE_DRY_RUN: ${{ github.event_name == 'workflow_dispatch' && inputs.dry_run || false }}",
        "RELEASE_DRY_RUN: ${{ github.event_name != 'workflow_dispatch' || inputs.dry_run }}",
      ),
    ),
    ("validate_model_pack.py --remote", VALID_RELEASE.replace(audit_step, "")),
    (
      "readiness must be written before remote model audit",
      VALID_RELEASE.replace(audit_step, "", 1).replace(
        "      - name: Write release readiness report\n",
        audit_step + "      - name: Write release readiness report\n",
        1,
      ),
    ),
    (
      "release checksum",
      VALID_RELEASE.replace("python3 scripts/release_artifacts.py", "shasum -a 256"),
    ),
    ("release state helper", VALID_RELEASE.replace('          --tag "$RELEASE_TAG"\n', "")),
    (
      "Release title",
      VALID_RELEASE.replace('          release_title="Amentia $RELEASE_TAG"', ""),
    ),
    (
      "release state helper must receive --title",
      VALID_RELEASE.replace('          --title "$release_title"\n', ""),
    ),
    (
      "release readiness helper must receive shared release input",
      VALID_RELEASE.replace('--requested-prerelease "$RELEASE_PRERELEASE"', ""),
    ),
    (
      "release state helper must receive shared release input",
      VALID_RELEASE.replace(
        '          --allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"\n',
        "",
      ),
    ),
    (
      "stage boundary",
      VALID_RELEASE.replace('          cat release-state.env >> "$GITHUB_ENV"\n', ""),
    ),
    (
      "stage boundary",
      VALID_RELEASE.replace('          cat release-plan.md >> "$GITHUB_STEP_SUMMARY"\n', ""),
    ),
    (
      "stage boundary",
      VALID_RELEASE.replace("--mode preupload-existing-assets", "--missing-preupload-mode"),
    ),
    (
      "final release state patch",
      VALID_RELEASE.replace(
        "      - name: Upload GitHub Release draft assets\n"
        "        if: env.RELEASE_DRY_RUN != 'true'\n",
        "      - name: Upload GitHub Release draft assets\n",
      ),
    ),
    (
      "stage boundary",
      VALID_RELEASE.replace("release-dry-run-${{ env.RELEASE_TAG }}", "release-dry-run-missing"),
    ),
    (
      "dry-run rehearsal",
      VALID_RELEASE.replace(
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
    ),
    (
      "published release contract helper",
      VALID_RELEASE.replace(
        "scripts/release_publish_contract.py",
        "scripts/missing_release_publish_contract.py",
      ),
    ),
    ("final release state patch", VALID_RELEASE.replace("-X PATCH", "-X POST")),
    (
      "release download rehearsal",
      VALID_RELEASE.replace(
        "          rm -rf release-download\n",
        "          gh api \\\n"
        "            -X PATCH \\\n"
        "            \"repos/$GITHUB_REPOSITORY/releases/$release_id\" \\\n"
        "            --input release-state.json\n"
        "          rm -rf release-download\n",
        1,
      ),
    ),
    (
      "release_tag_commit",
      VALID_RELEASE.replace(
        '          release_tag_commit="$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")"\n',
        "",
      ),
    ),
    (
      "git rev-parse",
      VALID_RELEASE.replace(
        '          release_tag_commit="$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")"\n',
        '          release_tag_commit="$(\n'
        '            git ls-remote --exit-code --tags origin \\\n'
        '              "refs/tags/$RELEASE_TAG" "refs/tags/$RELEASE_TAG^{}" | tail -n 1 | awk \'{print $1}\'\n'
        '          )"\n',
      ),
    ),
    (
      "generic error annotations",
      VALID_RELEASE.replace(
        '          test -n "${AMENTIA_RELEASE_ID:-}"\n',
        "          trap 'echo \"::error title=Release final validation failed::Validate final GitHub Release failed.\"' ERR\n"
        '          test -n "${AMENTIA_RELEASE_ID:-}"\n',
      ),
    ),
    (
      "gh release view",
      VALID_RELEASE.replace(
        '          release_id="$(gh release view "$RELEASE_TAG" --json databaseId --jq .databaseId)"\n',
        '          release_id="$(\n'
        '            gh api "repos/$GITHUB_REPOSITORY/releases?per_page=100" |\n'
        '              python3 -c \'import json, os, sys; tag = os.environ["RELEASE_TAG"]; releases = json.load(sys.stdin); match = next((release for release in releases if release.get("tag_name") == tag), None); print(match["id"] if match else "")\'\n'
        '          )"\n',
      ),
    ),
    (
      "AMENTIA_RELEASE_TAG_COMMIT",
      VALID_RELEASE.replace(
        "          python3 scripts/release_publish_contract.py \\\n",
        '          release_tag_commit="$(git rev-parse "$RELEASE_TAG^{commit}")"\n'
        "          python3 scripts/release_publish_contract.py \\\n",
      ),
    ),
    (
      "source-commit",
      VALID_RELEASE.replace('--source-commit "$AMENTIA_RELEASE_SHA"', "--missing-source-commit"),
    ),
    (
      "undefined SOURCE_COMMIT",
      VALID_RELEASE.replace(
        final_validation_source,
        final_validation_source.replace(
          '--source-commit "$AMENTIA_RELEASE_SHA"',
          '--source-commit "$SOURCE_COMMIT"',
        ),
        1,
      ),
    ),
    (
      "tag-commit",
      VALID_RELEASE.replace(
        '--tag-commit "$AMENTIA_RELEASE_TAG_COMMIT"',
        "--missing-tag-commit",
      ),
    ),
    (
      "expected-draft",
      VALID_RELEASE.replace(
        '--expected-draft "$AMENTIA_RELEASE_STATE_DRAFT"',
        "--missing-expected-draft",
      ),
    ),
    (
      "signing-mode",
      VALID_RELEASE.replace(
        '--signing-mode "$AMENTIA_RELEASE_SIGNING_MODE"',
        "--missing-signing-mode",
      ),
    ),
    (
      "allow-untrusted-ad-hoc",
      VALID_RELEASE.replace(
        '--allow-untrusted-ad-hoc "$RELEASE_ALLOW_UNTRUSTED_AD_HOC"',
        "--missing-untrusted-ad-hoc",
      ),
    ),
    (
      "manual-acceptance-receipt-url",
      VALID_RELEASE.replace(
        '--manual-acceptance-receipt-url "$RELEASE_MANUAL_ACCEPTANCE_RECEIPT_URL"',
        "--missing-manual-acceptance-receipt-url",
      ),
    ),
    (
      "release download rehearsal helper",
      VALID_RELEASE.replace(
        "scripts/release_rehearsal_contract.py",
        "scripts/missing_release_rehearsal_contract.py",
      ),
    ),
    (
      "release download rehearsal",
      VALID_RELEASE.replace('gh release download "$RELEASE_TAG"', 'gh release view "$RELEASE_TAG"'),
    ),
    (
      "release download rehearsal",
      VALID_RELEASE.replace('          cat release-rehearsal.md >> "$GITHUB_STEP_SUMMARY"\n', ""),
    ),
    ("retention-days", VALID_RELEASE.replace("          retention-days: 30\n", "")),
    (
      "^v[0-9]+",
      VALID_RELEASE.replace(
        '          if ! [[ "$RELEASE_TAG" =~ ^v[0-9]+\\.[0-9]+\\.[0-9]+$ ]]; then\n'
        '            exit 1\n'
        '          fi\n',
        "",
      ),
    ),
    (
      "must not allow extra assets",
      VALID_RELEASE.replace(
        "            --asset-dir release-download \\\n",
        "            --asset-dir release-download \\\n            --allow-extra-assets \\\n",
      ),
    ),
    ("source-commit", VALID_RELEASE.replace('--source-commit "$AMENTIA_RELEASE_SHA"', "")),
    (
      "package_contract.py",
      VALID_RELEASE.replace(
        "python3 scripts/package_contract.py",
        "python3 scripts/missing_package_contract.py",
      ),
    ),
    (
      "sign_macos_app_for_distribution.py",
      VALID_RELEASE.replace(
        "python3 scripts/sign_macos_app_for_distribution.py",
        "python3 scripts/missing_sign_macos_app_for_distribution.py",
      ),
    ),
    (
      "MACOS_DEVELOPER_ID_APPLICATION",
      VALID_RELEASE.replace('--identity "$MACOS_DEVELOPER_ID_APPLICATION"', ""),
    ),
    (
      "notarytool",
      VALID_RELEASE.replace("xcrun notarytool submit", "xcrun missing_notarytool submit"),
    ),
    (
      "stapler",
      VALID_RELEASE.replace('xcrun stapler staple "$dmg_path"', "echo missing stapler"),
    ),
    (
      "AmentiaPackage.json",
      VALID_RELEASE.replace(
        "--package-manifest artifacts/macos/Amentia.app/Contents/Resources/AmentiaPackage.json",
        "",
      ),
    ),
    (
      "packaged-smoke-receipt.json",
      VALID_RELEASE.replace("--smoke-receipt artifacts/macos/packaged-smoke-receipt.json", ""),
    ),
    (
      "workflow-run-url",
      VALID_RELEASE.replace(
        '--workflow-run-url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"',
        "",
      ),
    ),
    (
      "Amentia-$RELEASE_TAG-release-manifest.json",
      VALID_RELEASE.replace(
        '            --manifest-output "artifacts/macos/Amentia-$RELEASE_TAG-release-manifest.json"\n',
        "",
      ),
    ),
    (
      "installer_artifact_contract.py",
      VALID_RELEASE.replace(
        "python3 scripts/installer_artifact_contract.py",
        "python3 scripts/missing_installer_artifact_contract.py",
      ),
    ),
  ]


def main() -> int:
  assert_workflows_valid()

  for expected, ci in ci_policy_cases():
    assert_policy_issue(expected, ci=ci)
  for expected, release in release_policy_cases():
    assert_policy_issue(expected, release=release)

  print("Workflow policy validation tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
