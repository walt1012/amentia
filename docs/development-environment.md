# Development Environment

This document describes the CI-first development baseline for `Pith`.

## Required Local Tools

- Git

Local Rust, Swift, and Python toolchains are optional. The repository does not require contributors
to install local validation toolchains before pushing. GitHub Actions is the source of truth for
formatting, linting, tests, smoke coverage, and the native macOS app package.

## Remote Checks

Every push to `main` or `codex/**` runs the repository policy suite. Code,
packaging, model, plugin, workflow, scheduled, and manual runs fan out into the
canonical heavy gates they affect:

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
names, change-aware execution lanes, and parallel build lanes. A lightweight
change detection job keeps docs-only changes on fast policy checks while
workflow, Rust, Swift, packaging, model, plugin, and release script changes
still trigger the relevant heavy gates.

The repository policy suite also runs `scripts/validate_workflows.py`. That
check guards the workflow structure itself: checkout credentials stay disabled,
artifact uploads keep bounded retention, Rust lanes stay split, package
assembly does not wait behind Swift logic tests or the standalone llama backend
artifact job, and release jobs keep the CI gate plus the required DMG,
checksum, install guide, and manifest assets.

Rust formatting, clippy, tests, and runtime smoke run as separate jobs so
failures surface earlier instead of waiting behind a single serial Rust lane.
The final macOS package job depends only on executable-producing lanes: Swift
app build and runtime build. It restores the pinned llama.cpp backend directly
from the shared cache, building it inside the package lane only on cache miss,
so ordinary app/runtime changes avoid waiting for a separate backend artifact
handoff. Swift logic tests remain an independent required gate, but they no
longer block artifact assembly when the Swift executable is already available.

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
and `pith-runtime-bin` build or run in parallel cached jobs, then a packaging
job downloads the executable artifacts, restores or builds the pinned llama.cpp
backend from cache, assembles
`Pith.app`, places executables under `Contents/MacOS`, bundles model metadata
and bundled plugin manifests under `Contents/Resources`, validates the app
bundle, creates the DMG installer, and uploads one user-facing installer
artifact with bounded retention. Internal executable artifacts stay separate,
short-lived, and clearly named as internal.

Package validation checks the product `Info.plist`, `PkgInfo`,
`PithPackage.json`, source commit metadata, x86_64-only binaries, first-use
model download metadata,
bundled plugin resource contracts, absence of model weights, symlink-free
packaged resources and optional backend inputs, llama.cpp dependency
portability, sandbox fallback metadata, and package contents. The package must
include the default model manifest and every bundled plugin manifest, must not
contain symlinks or model weight files, and must not require external package
manager paths at runtime. CI also ad-hoc signs the app when `codesign` is
available.
Internal CI artifacts prove the package shape, but they are not public release
installers.

The packaged launch smoke is the release-candidate daily-driver proof. It
launches the app with isolated support state, probes the bundled runtime,
verifies first-use model metadata without model weights, opens a workspace,
creates a thread, sends a deterministic cowork request through the packaged
llama.cpp path, runs packaged Web Search from a fixture, executes a bundled MCP
stdio plugin command through connector authorization and approval, verifies
runner memory capture, and then checks app/runtime stability.

Trusted Developer ID distribution builds must pass:

```bash
python3 scripts/validate_macos_distribution.py \
  artifacts/macos/Pith.app \
  --dmg-path artifacts/macos/Pith-v0.1.0-macos-x86_64.dmg
```

This gate requires Developer ID signing, Gatekeeper assessment, notarization
stapling, x86_64 package metadata, in-app model delivery, no bundled model
weights, sandbox and daily-driver metadata, and package size budget compliance.
Ad-hoc signed CI artifacts and explicit untrusted prereleases prove the package
shape and user guidance, but they are not trusted macOS installers.

## GitHub Release Distribution

Users should download `Pith-<tag>-macos-x86_64.dmg` from the GitHub Release
page, open it, and drag `Pith.app` to Applications. The release workflow
publishes only strict `vX.Y.Z` product versions and supports two distribution
modes:

- Developer ID mode signs the app, creates a DMG, signs and notarizes the DMG,
  staples the notarization ticket, validates the app and DMG, then publishes the
  DMG, SHA-256 checksum, install guide, and release manifest to a normal GitHub
  Release.
- Ad-hoc mode builds the same x86_64 DMG shape when Developer ID secrets are
  missing. Tag-triggered builds and ordinary manual runs stay draft-only, but a
  manual run can publish a visible untrusted prerelease when
  `publish_untrusted_ad_hoc=true` and `draft=false` are both set. That release
  must remain marked as a prerelease and must explain that macOS Gatekeeper will
  require manual user approval before first launch.

Each release DMG includes `README-FIRST.txt` at the volume root. That file
summarizes the install steps, first-use model download, workspace opening,
first cowork request, and the trust path for either Developer ID notarized
builds or untrusted ad-hoc prereleases. The release workflow validates the
same copy again before publishing GitHub Release notes.

The release page also publishes `README-FIRST.txt` and a release manifest as
separate assets, so users and automation can inspect the platform target,
signing mode, source commit, checksum, sidecar hashes, exact asset set, asset
names, schema-versioned app package metadata, model delivery mode, and sandbox
fallback contract before opening the DMG. The release manifest records the
same expected installer asset names that CI and release upload validation
enforce. It also records the GitHub Actions run that enforced the source-commit
CI gate and mounted-DMG packaged smoke before upload.

Release publishing requires these repository secrets:

- `MACOS_CERTIFICATE_P12_BASE64`
- `MACOS_CERTIFICATE_PASSWORD`
- `MACOS_DEVELOPER_ID_APPLICATION`
- `APPLE_ID`
- `APPLE_TEAM_ID`
- `APPLE_APP_SPECIFIC_PASSWORD`

The release workflow must never publish an ad-hoc or non-notarized installer as
a normal trusted release. Without Developer ID secrets, it defaults to a draft
release and refuses to update an already-public release unless
`publish_untrusted_ad_hoc=true` is set explicitly. With that maintainer intent,
it may publish an untrusted ad-hoc prerelease for users who accept the
Gatekeeper warning path. With Developer ID secrets, it publishes the signed,
notarized, stapled DMG.

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
