# Scripts

This directory is reserved for repository automation scripts.

Planned uses:

- release packaging helpers
- schema generation helpers
- CI validation scripts

Current scripts:

- `check_english_policy.py`: rejects source and documentation text that violates the repository English-only policy.
- `ci_changes.py`: classifies changed files into CI execution lanes so heavy macOS and Rust checks run only when relevant.
- `create_macos_dmg.py`: creates and validates the user-facing macOS DMG installer from a packaged `Pith.app`, including the root install guide when provided.
- `macos_llama_backend.py`: stages and validates a self-contained llama.cpp backend for the packaged macOS app.
- `package_macos_app.py`: builds and validates the x86_64 macOS app bundle and release zip; CI can also pass prebuilt app and runtime executables for faster parallel packaging.
- `release_artifacts.py`: creates and validates user-facing release sidecars such as basename-only SHA-256 checksum files and release manifests.
- `release_state.py`: plans GitHub Release draft/prerelease safety for Developer ID and ad-hoc DMG builds.
- `release_text.py`: generates GitHub Release notes and the DMG root install guide from the release signing mode.
- `runtime_smoke_test.py`: verifies the runtime handshake, model health, memory, web search, plugin, command, hook, and connector protocol surfaces in CI.
- `sign_macos_app_for_distribution.py`: signs `Pith.app` with Developer ID and Hardened Runtime before notarized release packaging.
- `smoke_launch_macos_app.py`: launches the packaged `Pith.app` on macOS CI with isolated app support, probes the packaged runtime protocol, and verifies app/runtime startup, first-use model metadata without bundled weights, app-owned model pack activation, workspace bootstrap, workspace search, deterministic first cowork request, packaged web search execution, workspace write denial and approval, bundled MCP plugin command execution, connector authorization and approval, sandbox readiness, thread creation, runner memory capture, runtime recovery, and runtime database initialization.
- `test_create_macos_dmg.py`: checks DMG staging behavior that does not require macOS.
- `test_ci_changes.py`: checks CI change-lane classification rules.
- `test_package_macos_app.py`: checks packaging helper behavior that does not require macOS.
- `test_release_artifacts.py`: checks checksum and release manifest sidecar behavior.
- `test_release_state.py`: checks release state planning behavior that does not require GitHub Actions.
- `test_release_text.py`: checks release notes and DMG install guide copy generation.
- `validate_macos_distribution.py`: checks Developer ID signing, Gatekeeper assessment, and optional notarized DMG validation for public macOS distribution builds.
- `validate_model_pack.py`: validates local model pack metadata and first-use resource packaging; use `--remote` during release audits.
- `validate_workflows.py`: validates GitHub Actions structure so checkout credentials, artifact retention, CI lane splits, package dependencies, and release assets do not regress.
- `test_validate_workflows.py`: checks workflow structure policy behavior without invoking GitHub Actions.

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
state helper rejects accidental ad-hoc updates to an already-public release
unless that untrusted prerelease path was explicitly requested.
