# Development Environment

This document describes the CI-first development baseline for `Pith`.

## Required Local Tools

- Git

Local Rust, Swift, and Python toolchains are optional. The repository does not require contributors
to install local validation toolchains before pushing. GitHub Actions is the source of truth for
formatting, linting, tests, smoke coverage, and the native macOS app package.

## Remote Checks

Every push to `main` or `codex/**` runs the canonical CI suite:

- Rust formatting with `cargo fmt --all -- --check`
- Rust linting with `cargo clippy --workspace --all-targets -- -D warnings`
- Rust tests with `cargo test --workspace`
- x86_64 Swift app build
- x86_64 Swift app logic tests
- model pack manifest validation
- runtime smoke coverage through `scripts/runtime_smoke_test.py`
- signed-ready x86_64 macOS app bundle packaging on an Intel macOS runner
- packaged app launch smoke coverage through `scripts/smoke_launch_macos_app.py`

CI jobs use read-only repository permissions, explicit timeouts, stable artifact
names, and parallel build lanes. The final macOS package job depends on the
Swift app build, Swift tests, runtime build, and llama.cpp backend lane before it
assembles a distributable app artifact.

Do not treat a missing or broken local toolchain as a blocker. Push the branch and inspect the
remote CI logs instead.

## Runtime Bridge Development

The macOS shell looks for the runtime binary in this order:

1. `PITH_RUNTIME_PATH`
2. a bundled `pith-runtime-bin` executable next to the app executable

For manual runtime experiments, the bridge can still be pointed at a locally built runtime:

```bash
export PITH_RUNTIME_PATH=/absolute/path/to/pith-runtime-bin
```

One optional manual flow after building the Rust runtime:

```bash
cargo build -p pith-runtime-bin
export PITH_RUNTIME_PATH="$(pwd)/target/debug/pith-runtime-bin"
cd apps/pith-macos
swift run
```

## macOS App Packaging

The canonical package command is:

```bash
python3 scripts/package_macos_app.py
```

CI runs this on `macos-15-intel`. The Swift app executable, Swift logic tests,
`pith-runtime-bin`, and pinned llama.cpp backend build or run in parallel cached
jobs, then a packaging job downloads the executable artifacts, assembles
`Pith.app`, places executables under `Contents/MacOS`, bundles model metadata
and bundled plugin manifests under `Contents/Resources`, validates the app
bundle, and emits `artifacts/macos/Pith-macos-x86_64.zip`.

Package validation checks the product `Info.plist`, `PkgInfo`,
`PithPackage.json`, x86_64-only binaries, first-use model download metadata,
bundled plugin resource contracts, absence of model weights, symlink-free
packaged resources and optional backend inputs, llama.cpp dependency
portability, and zip contents. The zip must include the default model manifest
and every bundled plugin manifest, must not contain symlinks or model weight
files, and must not require external package manager paths at runtime. CI also
ad-hoc signs the app when `codesign` is available. Distribution signing and
notarization should be added only after identity and entitlements are finalized.

Public distribution builds must pass:

```bash
python3 scripts/validate_macos_distribution.py artifacts/macos/Pith.app
```

This gate requires Developer ID signing and Gatekeeper assessment. Ad-hoc signed
CI artifacts are for internal validation only.

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

1. `PITH_MODEL_PATH`
2. `PITH_LFM_MODEL_PATH` as a legacy alias
3. a sibling of the resolved manifest using the manifest `file_name`
4. `PITH_DATA_DIR/models/LFM2.5-350M-Q4_K_M.gguf`
5. repo-local `models/LFM2.5-350M-Q4_K_M.gguf`
6. repo-local `model-packs/LFM2.5-350M-Q4_K_M.gguf`

The repository should track the manifest, licensing notes, and small metadata only. The actual `LFM2.5-350M-Q4_K_M.gguf` file should not be committed to git history. The macOS app exposes the manifest `download_url` as a one-click local model download into the suggested app data path, while advanced local installs can still point `PITH_MODEL_PATH` at another local GGUF file.

The macOS Local Model panel also includes a small local model manager. It keeps a curated list of lightweight GGUF models, downloads each file into `PITH_DATA_DIR/models`, monitors which recommended models are present on disk, and can activate a downloaded model by writing a local `model-pack.json` beside the GGUF file. Activating a model stores the selected manifest and model paths in app preferences, injects `PITH_MODEL_PACK_MANIFEST` and `PITH_MODEL_PATH` for the runtime, and relaunches the local runtime so health checks report the selected model.

If either path is missing, Pith reports the local model as unavailable and blocks agent work until a real local runtime is configured. One local setup example is:

```bash
export PITH_LLAMACPP_PATH=/absolute/path/to/llama-cli
export PITH_MODEL_PACK_MANIFEST=/absolute/path/to/model-pack.json
export PITH_MODEL_PATH=/absolute/path/to/LFM2.5-350M-Q4_K_M.gguf
```

## Plugin Discovery

The runtime resolves bundled plugins in this order:

1. `PITH_PLUGIN_DIR`
2. an executable-relative `plugins/`
3. repo-local `plugins/`

Plugin development should keep discovery separate from execution. Discovery owns
manifest validation, registries, connector metadata, and enablement state. Execution owns bounded
runners, auth policy, credential storage, cancellation, sandbox policy, output envelopes, and logs.

The minimal runner surface starts with plugin-bundle-scoped `stdio` entrypoints bound to the native
sandbox policy. Runner success and failure paths should expose sandbox, exit, stdout, and stderr
diagnostics before new connector surfaces are added.

## GitHub Actions Notes

The workflow uses:

- `ubuntu-latest` for Rust checks
- `macos-15-intel` for the native x86_64 app bundle package

This is intentional because the product target is Intel macOS and GitHub retired the `macos-13` runner image in late 2025.
