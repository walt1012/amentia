# Scripts

This directory is reserved for repository automation scripts.

Planned uses:

- release packaging helpers
- schema generation helpers
- CI validation scripts

Current scripts:

- `check_english_policy.py`: rejects source and documentation text that violates the repository English-only policy.
- `package_macos_app.py`: builds and validates the x86_64 macOS app bundle and release zip; CI can also pass prebuilt app and runtime executables for faster parallel packaging.
- `runtime_smoke_test.py`: verifies the runtime handshake, model health, memory, web search, plugin, command, hook, and connector protocol surfaces in CI.
- `smoke_launch_macos_app.py`: launches the packaged `Pith.app` on macOS CI with isolated app support, probes the packaged runtime protocol, and verifies app/runtime startup, workspace bootstrap, workspace search, thread creation, and runtime database initialization.
- `test_package_macos_app.py`: checks packaging helper behavior that does not require macOS.
- `validate_model_pack.py`: validates local model pack metadata and first-use resource packaging; use `--remote` during release audits.

These scripts are safe to run locally when a matching toolchain exists, but CI is the canonical
execution environment.
