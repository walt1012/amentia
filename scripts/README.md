# Scripts

This directory is reserved for repository automation scripts.

Planned uses:

- release packaging helpers
- schema generation helpers
- CI validation scripts

Current scripts:

- `check_english_policy.py`: rejects source and documentation text that violates the repository English-only policy.
- `ci_changes.py`: classifies changed files into CI execution lanes so heavy macOS and Rust checks run only when relevant.
- `create_macos_dmg.py`: creates and validates the user-facing macOS DMG installer from a packaged `Pith.app`, including the root install guide, checksum verification copy, and first-run contract when provided.
- `installer_artifact_contract.py`: validates the exact installer asset set before CI artifact upload or GitHub Release publish so users only receive a matching DMG, checksum, install guide, and manifest.
- `macos_llama_backend.py`: stages and validates a self-contained llama.cpp backend for the packaged macOS app.
- `package_contract.py`: centralizes the stable macOS package contract shared by packaging, release manifests, distribution validation, packaged smoke checks, signing-mode policy, locked size-budget validation, and direct `PithPackage.json` validation in CI/release workflows.
- `package_macos_app.py`: builds and validates the x86_64 macOS app bundle and optional zip; CI can pass prebuilt app and runtime executables for faster installer packaging without creating extra user-facing archives, and records schema-versioned package metadata including daily-driver readiness provenance and package size budgets in `PithPackage.json`.
- `release_artifacts.py`: creates and validates user-facing release sidecars such as basename-only SHA-256 checksum files and source-commit release manifests with DMG, checksum, install-guide hashes, schema-versioned app package metadata, daily-driver readiness provenance, release workflow proof, and tag-locked names.
- `release_copy_contract.py`: centralizes installer guide copy requirements shared by release text generation, release sidecar validation, and DMG staging validation.
- `release_identity.py`: centralizes strict `vX.Y.Z` public release tag and three-part product version rules.
- `release_state.py`: plans GitHub Release draft/prerelease safety for Developer ID and ad-hoc DMG builds, enforces tag/title identity, then revalidates release notes against the final publish state.
- `release_text.py`: generates and validates GitHub Release notes and the DMG root install guide from the release signing mode, including exact installer asset names and the daily-driver next-action path users should follow after install.
- `runtime_smoke_test.py`: verifies the runtime handshake, model health, memory, web search, plugin, command, hook, and connector protocol surfaces in CI.
- `sign_macos_app_for_distribution.py`: signs `Pith.app` with Developer ID and Hardened Runtime before notarized release packaging.
- `smoke_launch_macos_app.py`: launches the packaged `Pith.app` on macOS CI with isolated app support, probes the packaged runtime protocol, and verifies app/runtime startup, first-use model metadata without bundled weights, app-owned model pack activation, workspace bootstrap, workspace search, deterministic first cowork request, packaged Web Search execution with bounded source snapshots, workspace write denial and approval, bundled MCP plugin command execution, connector authorization and approval, sandbox readiness, thread creation, runner memory capture, runtime recovery, and runtime database initialization.
- `test_create_macos_dmg.py`: checks DMG staging behavior that does not require macOS.
- `test_ci_changes.py`: checks CI change-lane classification rules.
- `test_installer_artifact_contract.py`: checks exact CI and release installer asset sets.
- `test_package_macos_app.py`: checks packaging helper behavior that does not require macOS.
- `test_package_contract.py`: checks the shared package contract constants, size budgets, and bundled-model guards.
- `test_notion_connector_contract.py`: checks bundled Notion connector MCP handoff, retry, and follow-up metadata.
- `test_release_artifacts.py`: checks checksum and release manifest sidecar behavior.
- `test_release_identity.py`: checks shared product version and public release tag rules.
- `test_release_state.py`: checks release state planning behavior that does not require GitHub Actions.
- `test_release_text.py`: checks release notes and DMG install guide copy generation.
- `test_smoke_launch_macos_app.py`: checks packaged app smoke validators that do not require macOS.
- `validate_macos_distribution.py`: checks Developer ID signing, Gatekeeper assessment, x86_64 metadata, in-app model delivery, no bundled model weights, package size budget, and optional notarized DMG validation for public macOS distribution builds.
- `validate_model_pack.py`: validates local model pack metadata, curated model catalog shape, and first-use resource packaging; use `--remote` during release audits.
- `validate_workflows.py`: validates GitHub Actions structure so checkout credentials, internal artifact retention, CI lane splits, package dependencies, user-facing installer uploads, and release assets do not regress.
- `test_validate_workflows.py`: checks workflow structure policy behavior without invoking GitHub Actions.
- `test_validate_macos_distribution.py`: checks public distribution metadata policy without invoking signing tools.

These scripts are safe to run locally when a matching toolchain exists, but CI is the canonical
execution environment.

CI keeps fast policy checks, Rust checks, Swift builds, runtime builds, the pinned llama.cpp backend,
and macOS packaging as separate gates. The llama.cpp backend is cached by pinned source revision, but
the packaged app smoke test still validates the staged backend before release artifacts are uploaded.
Public GitHub Releases should upload the notarized
`Pith-<tag>-macos-x86_64.dmg`, checksum, root install guide, and release
manifest. If Developer ID credentials are unavailable, the release workflow
defaults to a draft ad-hoc DMG. A maintainer may explicitly publish that DMG as
an untrusted prerelease for users who accept the macOS Gatekeeper manual approval
path, but it must not be promoted as a normal trusted installer. The release
state helper rejects non-version tags, mismatched release titles, and accidental
ad-hoc updates to an already-public release unless that untrusted prerelease
path was explicitly requested.
