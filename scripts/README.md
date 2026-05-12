# Scripts

This directory is reserved for repository automation scripts.

Planned uses:

- release packaging helpers
- schema generation helpers
- CI validation scripts

Current scripts:

- `check_english_policy.py`: rejects CJK text in tracked source, documentation, and automation files
- `validate_model_pack.py`: checks the default model manifest and Swift local model catalog; add
  `--remote` during release audits to compare Hugging Face size, checksum, and license metadata
- `runtime_smoke_test.py`: verifies the runtime handshake, model health, memory, plugin, command,
  hook, and connector protocol surfaces in GitHub Actions

These scripts are safe to run locally when a matching toolchain exists, but CI is the canonical
execution environment.
