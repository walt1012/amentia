# Scripts

This directory is reserved for repository automation scripts.

Planned uses:

- release packaging helpers
- schema generation helpers
- CI validation scripts

Current scripts:

- `runtime_smoke_test.py`: verifies the runtime handshake, model health, memory, plugin, command,
  hook, and connector protocol surfaces in GitHub Actions

These scripts are safe to run locally when a matching toolchain exists, but CI is the canonical
execution environment.
