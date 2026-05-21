# Scripts

This directory is reserved for repository automation scripts.

Planned uses:

- release packaging helpers
- schema generation helpers
- CI validation scripts

Current scripts:

- `check_english_policy.py`: rejects source and documentation text that violates the repository English-only policy.
- `create_macos_dmg.py`: creates and validates the user-facing macOS DMG installer from a packaged `Pith.app`.
- `macos_llama_backend.py`: stages and validates a self-contained llama.cpp backend for the packaged macOS app.
- `package_macos_app.py`: builds and validates the x86_64 macOS app bundle and release zip; CI can also pass prebuilt app and runtime executables for faster parallel packaging.
- `runtime_smoke_test.py`: verifies the runtime handshake, model health, memory, web search, plugin, command, hook, and connector protocol surfaces in CI.
- `sign_macos_app_for_distribution.py`: signs `Pith.app` with Developer ID and Hardened Runtime before notarized release packaging.
- `smoke_launch_macos_app.py`: launches the packaged `Pith.app` on macOS CI with isolated app support, probes the packaged runtime protocol, and verifies app/runtime startup, workspace bootstrap, workspace search, packaged web search execution, sandbox readiness, thread creation, and runtime database initialization.
- `test_package_macos_app.py`: checks packaging helper behavior that does not require macOS.
- `validate_macos_distribution.py`: checks Developer ID signing, Gatekeeper assessment, and optional notarized DMG validation for public macOS distribution builds.
- `validate_model_pack.py`: validates local model pack metadata and first-use resource packaging; use `--remote` during release audits.

These scripts are safe to run locally when a matching toolchain exists, but CI is the canonical
execution environment.

CI keeps fast policy checks, Rust checks, Swift builds, runtime builds, the pinned llama.cpp backend,
and macOS packaging as separate gates. The llama.cpp backend is cached by pinned source revision, but
the packaged app smoke test still validates the staged backend before release artifacts are uploaded.
Public GitHub Releases should upload the notarized `Pith-<tag>-macos-x86_64.dmg`
and checksum. If Developer ID credentials are unavailable, the release workflow
may upload an ad-hoc DMG to a draft release only; do not promote that artifact
as a normal public installer.
