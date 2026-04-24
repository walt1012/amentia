# Development Environment

This document describes the local development baseline for `Pith`.

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

- `cd apps/pith-macos`
- `swift build`

## Runtime Bridge Development

The macOS shell looks for the runtime binary in this order:

1. `PITH_RUNTIME_PATH`
2. a bundled `pith-runtime-bin` executable next to the app executable

For local development, the recommended path is:

```bash
export PITH_RUNTIME_PATH=/absolute/path/to/pith-runtime-bin
```

One example flow after building the Rust runtime:

```bash
cargo build -p pith-runtime-bin
export PITH_RUNTIME_PATH="$(pwd)/target/debug/pith-runtime-bin"
cd apps/pith-macos
swift run
```

## Local Model Runtime

The runtime now resolves the local model stack in this order:

1. `PITH_LLAMACPP_PATH`
2. an executable-relative `third_party/llama.cpp/llama-cli`
3. an executable-relative `tools/llama.cpp/llama-cli`
4. repo-local `third_party/llama.cpp/llama-cli`
5. repo-local `tools/llama.cpp/llama-cli`

The default model pack path resolves in this order:

1. `PITH_MODEL_PACK_MANIFEST`
2. `PITH_MODEL_PACK_ROOT`
3. `PITH_DATA_DIR/models/builtin/lfm2.5-350m/model-pack.json`
4. an executable-relative `models/builtin/lfm2.5-350m/model-pack.json`
5. an executable-relative `model-packs/lfm2.5-350m/model-pack.json`
6. repo-local `models/builtin/lfm2.5-350m/model-pack.json`
7. repo-local `model-packs/lfm2.5-350m/model-pack.json`

The resolved model file path then checks:

1. `PITH_LFM_MODEL_PATH`
2. a sibling of the resolved manifest using the manifest `file_name`
3. `PITH_DATA_DIR/models/LFM2.5-350M-Q4_K_M.gguf`
4. repo-local `models/LFM2.5-350M-Q4_K_M.gguf`
5. repo-local `model-packs/LFM2.5-350M-Q4_K_M.gguf`

The repository should track the manifest, licensing notes, and small metadata only. The actual `LFM2.5-350M-Q4_K_M.gguf` file should not be committed to git history. It should live in the local data directory, a release bundle, or another local install path. The macOS app exposes the manifest `download_url` as a one-click local model download into the suggested app data path.

The macOS Local Model panel also includes a small local model manager. It keeps a curated list of lightweight GGUF models, downloads each file into `PITH_DATA_DIR/models`, monitors which recommended models are present on disk, and can activate a downloaded model by writing a local `model-pack.json` beside the GGUF file. Activating a model stores the selected manifest and model paths in app preferences, injects `PITH_MODEL_PACK_MANIFEST` and `PITH_LFM_MODEL_PATH` for the runtime, and relaunches the local runtime so health checks report the selected model.

If either path is missing, Pith reports the local model as unavailable and blocks agent work until a real local runtime is configured. One local setup example is:

```bash
export PITH_LLAMACPP_PATH=/absolute/path/to/llama-cli
export PITH_MODEL_PACK_MANIFEST=/absolute/path/to/model-pack.json
export PITH_LFM_MODEL_PATH=/absolute/path/to/LFM2.5-350M-Q4_K_M.gguf
```

## Plugin Discovery

The runtime resolves bundled plugins in this order:

1. `PITH_PLUGIN_DIR`
2. an executable-relative `plugins/`
3. repo-local `plugins/`

Milestone 1 uses a built-in memory module in the runtime, so the plugin root can stay empty until the plugin milestone.

## GitHub Actions Notes

The workflow uses:

- `ubuntu-latest` for Rust checks
- `macos-15-intel` for the native Swift package build

This is intentional because the product target is Intel macOS and GitHub retired the `macos-13` runner image in late 2025.
