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

## GitHub Actions Notes

The workflow uses:

- `ubuntu-latest` for Rust checks
- `macos-15-intel` for the native Swift package build

This is intentional because the product target is Intel macOS and GitHub retired the `macos-13` runner image in late 2025.
