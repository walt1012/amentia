# Development Environment

This document describes the local development baseline for `Cavell`.

## Required Toolchains

- Xcode 15 or newer
- Swift 5.9 or newer
- Rust stable toolchain
- Python 3.11 or newer
- Git

## Repository Checks

Rust:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `python scripts/runtime_smoke_test.py`

macOS app:

- `cd apps/cavell-macos`
- `swift build`

## Runtime Bridge Development

The macOS shell looks for the runtime binary in this order:

1. `CAVELL_RUNTIME_PATH`
2. a bundled `cavell-runtime-bin` executable next to the app executable

For local development, the recommended path is:

```bash
export CAVELL_RUNTIME_PATH=/absolute/path/to/cavell-runtime-bin
```

One example flow after building the Rust runtime:

```bash
cargo build -p cavell-runtime-bin
export CAVELL_RUNTIME_PATH="$(pwd)/target/debug/cavell-runtime-bin"
cd apps/cavell-macos
swift run
```

## Local Model Runtime

The runtime now resolves the local model stack in this order:

1. `CAVELL_LLAMACPP_PATH`
2. an executable-relative `third_party/llama.cpp/llama-cli`
3. an executable-relative `tools/llama.cpp/llama-cli`
4. repo-local `third_party/llama.cpp/llama-cli`
5. repo-local `tools/llama.cpp/llama-cli`

The default model pack path resolves in this order:

1. `CAVELL_MODEL_PACK_MANIFEST`
2. `CAVELL_MODEL_PACK_ROOT`
3. `CAVELL_DATA_DIR/models/builtin/lfm2.5-350m/model-pack.json`
4. an executable-relative `models/builtin/lfm2.5-350m/model-pack.json`
5. an executable-relative `model-packs/lfm2.5-350m/model-pack.json`
6. repo-local `models/builtin/lfm2.5-350m/model-pack.json`
7. repo-local `model-packs/lfm2.5-350m/model-pack.json`

The resolved model file path then checks:

1. `CAVELL_LFM_MODEL_PATH`
2. a sibling of the resolved manifest using the manifest `file_name`
3. `CAVELL_DATA_DIR/models/LFM2.5-350M.gguf`
4. repo-local `models/LFM2.5-350M.gguf`
5. repo-local `model-packs/LFM2.5-350M.gguf`

The repository should track the manifest, licensing notes, and small metadata only. The actual `LFM2.5-350M.gguf` file should not be committed to git history. It should live in the local data directory, a release bundle, or another local install path.

If either path is missing, Cavell falls back to the built-in heuristic summarizer while still reporting model health in the inspector. One local setup example is:

```bash
export CAVELL_LLAMACPP_PATH=/absolute/path/to/llama-cli
export CAVELL_MODEL_PACK_MANIFEST=/absolute/path/to/model-pack.json
export CAVELL_LFM_MODEL_PATH=/absolute/path/to/LFM2.5-350M.gguf
```

## Plugin Discovery

The runtime resolves bundled plugins in this order:

1. `CAVELL_PLUGIN_DIR`
2. an executable-relative `plugins/`
3. repo-local `plugins/`

Milestone 1 uses a built-in memory module in the runtime, so the plugin root can stay empty until the plugin milestone.

## GitHub Actions Notes

The workflow uses:

- `ubuntu-latest` for Rust checks
- `macos-15-intel` for the native Swift package build

This is intentional because the product target is Intel macOS and GitHub retired the `macos-13` runner image in late 2025.
