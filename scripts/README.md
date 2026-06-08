# Scripts

This directory is reserved for repository automation scripts.

Planned uses:

- release packaging helpers
- schema generation helpers
- CI validation scripts

Current scripts:

- `check_english_policy.py`: rejects source and documentation text that violates the repository English-only policy.
- `ci_changes.py`: classifies changed files into CI execution lanes so heavy macOS and Rust checks run only when relevant.
- `create_macos_dmg.py`: creates and validates the user-facing macOS DMG installer from a packaged `Pith.app`, including bounded transient `hdiutil create` retry, the root install guide, checksum verification copy, mounted-DMG smoke launch, and optional first-run receipt output.
- `installer_artifact_contract.py`: validates the exact installer asset set before CI artifact upload, GitHub Release publish, or downloaded-release rehearsal so users only receive a matching DMG, checksum, install guide, and manifest; downloaded asset directories reject extra entries unless an internal CI build opts in.
- `macos_llama_backend.py`: stages and validates a self-contained llama.cpp backend for the packaged macOS app.
- `package_contract.py`: centralizes the stable macOS package contract shared by packaging, release manifests, distribution validation, packaged smoke checks, signing-mode policy, locked size-budget validation, and direct `PithPackage.json` validation in CI/release workflows.
- `package_macos_app.py`: builds and validates the x86_64 macOS app bundle and optional zip; CI can pass prebuilt app and runtime executables for faster installer packaging without creating extra user-facing archives, and records schema-versioned package metadata including daily-driver readiness provenance and package size budgets in `PithPackage.json`.
- `release_artifacts.py`: creates and validates user-facing release sidecars such as basename-only SHA-256 checksum files and source-commit release manifests with DMG, checksum, install-guide hashes, signing trust and Gatekeeper guidance, schema-versioned app package metadata, daily-driver readiness provenance, packaged first-run smoke receipt proof, release workflow proof, and tag-locked names.
- `release_copy_contract.py`: centralizes release notes and installer guide copy requirements plus shared copy validators used by release text generation, release sidecar validation, and DMG staging validation.
- `release_evidence_contract.py`: validates internal dry-run and publish-rehearsal evidence artifacts, including structured readiness, release-plan, release-rehearsal JSON, rehearsal Markdown, manual acceptance checklist terms, cross-file release consistency, and asset-contract consistency, before workflow upload without adding extra public release assets.
- `release_identity.py`: centralizes strict `vX.Y.Z` public release tag and three-part product version rules.
- `release_publish_contract.py`: validates existing GitHub Release assets before upload and the final GitHub Release state after publish by checking tag, title, remote tag source commit, draft/prerelease flags, signing-mode-specific release copy, exact user-facing installer assets, non-empty uploads, and tag-scoped download URLs.
- `release_readiness.py`: prepares maintainer-facing Markdown and machine-readable JSON readiness reports before dispatching the release workflow, including tag preparation, lightweight or annotated remote tag verification, exact source-commit CI lookup, the safe dry-run-first command, source-commit and successful-run matched dry-run artifact lookup and verification, guarded post-acceptance publish command, release-candidate checklist, and expected dry-run evidence.
- `release_rehearsal_contract.py`: validates a downloaded release asset directory against the installer contract, daily-driver readiness contract, first-run manifest contract, app package metadata, and packaged smoke proof, then can write compact Markdown and JSON rehearsal evidence with trust, Gatekeeper, smoke journey, release-decision gate, manual prerelease acceptance, metadata-match, and first app-open checks.
- `test_first_app_open_contract.py`: checks that Swift first-open copy stays aligned with the shared release first app-open copy contract.
- `release_state.py`: plans GitHub Release draft/prerelease safety for Developer ID, ad-hoc, and dry-run DMG builds, enforces tag/title identity, rejects placeholder manual acceptance evidence, revalidates release notes against the final publish state, and writes Markdown plus JSON release-plan evidence used in Actions.
- `release_text.py`: generates and validates GitHub Release notes and the DMG root install guide from the release signing mode, including exact installer asset names and the daily-driver next-action path users should follow after install.
- `runtime_smoke_test.py`: verifies the runtime handshake, model health, memory, web search, plugin, command, hook, and connector protocol surfaces in CI.
- `sign_macos_app_for_distribution.py`: signs `Pith.app` with Developer ID and Hardened Runtime before notarized release packaging.
- `smoke_launch_macos_app.py`: launches the packaged `Pith.app` on macOS CI with isolated app support, probes the packaged runtime protocol, and verifies app/runtime startup, first-use model metadata without bundled weights, app-owned model pack activation, workspace bootstrap, workspace search, deterministic first cowork request, packaged Web Search execution with bounded source snapshots, workspace write denial and approval, bundled MCP plugin command execution, connector authorization and approval, sandbox readiness, thread creation, runner memory capture, runtime recovery, runtime database initialization, and an optional structured first-run receipt.
- `test_create_macos_dmg.py`: checks DMG staging behavior that does not require macOS.
- `test_ci_changes.py`: checks CI change-lane classification rules.
- `test_installer_artifact_contract.py`: checks exact CI and release installer asset sets.
- `test_package_macos_app.py`: checks packaging helper behavior that does not require macOS.
- `test_package_contract.py`: checks the shared package contract constants, size budgets, and bundled-model guards.
- `test_notion_connector_contract.py`: checks bundled Notion connector MCP handoff, retry, and follow-up metadata.
- `test_release_artifacts.py`: checks checksum and release manifest sidecar behavior.
- `test_release_evidence_contract.py`: checks internal release evidence artifact sets, structured readiness, release-plan, release-rehearsal JSON consistency, rehearsal and manual acceptance Markdown content, cross-file agreement, and missing or extra file rejection.
- `test_release_identity.py`: checks shared product version and public release tag rules.
- `test_release_publish_contract.py`: checks final published GitHub Release source commit, state, copy, and asset validation.
- `test_release_readiness.py`: checks release readiness blocking, JSON evidence, and dry-run command generation.
- `test_release_rehearsal_contract.py`: checks downloaded release rehearsal validation plus Markdown and JSON summary generation.
- `test_release_state.py`: checks release state planning and release-plan evidence behavior that does not require GitHub Actions.
- `test_release_text.py`: checks release notes and DMG install guide copy generation.
- `test_smoke_launch_macos_app.py`: checks packaged app smoke validators that do not require macOS.
- `validate_macos_distribution.py`: checks Developer ID signing, Gatekeeper assessment, x86_64 metadata, in-app model delivery, no bundled model weights, package size budget, and optional notarized DMG validation for public macOS distribution builds.
- `validate_model_pack.py`: validates local model pack metadata, curated model catalog shape, and first-use resource packaging; use `--remote` during release audits.
- `validate_workflows.py`: validates GitHub Actions structure so checkout credentials, internal artifact retention, CI lane splits, package dependencies, shared release inputs, evidence-step ordering, user-facing installer uploads, and release assets do not regress.
- `test_validate_workflows.py`: checks workflow structure policy behavior without invoking GitHub Actions.
- `test_validate_macos_distribution.py`: checks public distribution metadata policy without invoking signing tools.

These scripts are safe to run locally when a matching toolchain exists, but CI is the canonical
execution environment.

CI keeps fast policy checks, Rust checks, Swift builds, runtime builds, the pinned llama.cpp backend,
and macOS packaging as separate gates. The llama.cpp backend is cached by pinned source revision, but
the packaged app smoke test and internal downloaded-release rehearsal still validate the staged
installer before release artifacts are uploaded.
Public GitHub Releases should upload the notarized
`Pith-<tag>-macos-x86_64.dmg`, checksum, root install guide, and release
manifest. If Developer ID credentials are unavailable, the release workflow
defaults to a draft ad-hoc DMG. A maintainer may explicitly publish that DMG as
an untrusted prerelease for users who accept the macOS Gatekeeper manual approval
path, but only after the generated manual acceptance checklist passes and
`manual_acceptance_confirmed=true` and `manual_acceptance_evidence` are set. It
must not be promoted as a normal trusted installer. The release workflow can
also run as a dry-run: it builds, validates, and rehearses the same DMG,
checksum, root install guide, manifest, release plan, rehearsal summary, and
manual acceptance checklist without creating or updating a GitHub Release.
Rehearsal summaries record that manual acceptance is still required before any
visible ad-hoc prerelease. The release state helper rejects
non-version tags, mismatched release titles, and accidental ad-hoc updates to an
already-public release unless that untrusted prerelease path was explicitly
requested. It also refuses to move an existing public GitHub Release back to
draft; release withdrawal should stay a deliberate maintainer action. Before
uploading over an existing release, the workflow rejects non-contract assets so
stale packages or model payloads cannot remain on the user download page. After
upload and release-state patching, the release workflow reads the GitHub Release
back and validates the final state, exact public asset set, non-empty uploads,
and download URLs. It then downloads the release assets back through GitHub
Releases and runs the same rehearsal contract a maintainer can use after manual
download. The rehearsal summary is uploaded as an internal workflow artifact,
not as an extra public Release asset. Tag-push and manual release dispatches
default to dry-run; publishing requires an explicit manual dispatch with
`dry_run=false`.
