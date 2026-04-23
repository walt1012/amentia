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
2. repo-local `third_party/llama.cpp/llama-cli`
3. repo-local `tools/llama.cpp/llama-cli`

The default model pack path resolves in this order:

1. `CAVELL_LFM_MODEL_PATH`
2. repo-local `models/LFM2.5-350M.gguf`
3. repo-local `model-packs/LFM2.5-350M.gguf`

If either path is missing, Cavell falls back to the built-in heuristic summarizer while still reporting model health in the inspector. One local setup example is:

```bash
export CAVELL_LLAMACPP_PATH=/absolute/path/to/llama-cli
export CAVELL_LFM_MODEL_PATH=/absolute/path/to/LFM2.5-350M.gguf
```

## GitHub Actions Notes

The workflow uses:

- `ubuntu-latest` for Rust checks
- `macos-15-intel` for the native Swift package build

This is intentional because the product target is Intel macOS and GitHub retired the `macos-13` runner image in late 2025.
