# Amentia

`Amentia` is a local-first macOS agent application for Intel Macs running macOS 12 and above.
It is intended to be a real daily-use desktop app, not a prototype or a thin
terminal wrapper.

The product goal is to combine:

- a premium native desktop experience
- a local model runtime
- structured task execution
- explicit approvals and diffs
- a plugin-capable architecture with built-in memory

The repository is intentionally English-only.

## Product Principles

- Build a native `macOS` application named `Amentia` for `macOS 12+` on `x86_64` only.
- Keep the app lightweight and intentionally small while preserving a strong local agent loop.
- Make the first-run path usable end to end: download a model, open a workspace, start work,
  recover from failures, and keep going without manual setup.
- Judge features by packaged-app usability, not by whether an internal API or script exists.
- Favor a calm, premium, minimal UI inspired by high-quality agent tools such as Codex and Claude Code.
- Keep the default intelligence path fully local with no required external model API.
- Treat `LFM2.5-350M` as the default first-use model while keeping small GGUF alternatives available.
- Design plugins as first-class product modules so `Amentia` can expand beyond code assistance into a broader local agent platform.
- Prefer free and open source dependencies, tooling, and model delivery paths.

## Repository Layout

```text
/
|-- apps/
|   `-- amentia-macos/
|-- crates/
|   |-- amentia-core/
|   |-- amentia-memory/
|   |-- amentia-model-runtime/
|   |-- amentia-plugin-host/
|   |-- amentia-protocol/
|   |-- amentia-runtime-bin/
|   |-- amentia-sandbox/
|   |-- amentia-storage/
|   `-- amentia-tools/
|-- plugins/
|   `-- bundled/
|-- docs/
|-- scripts/
|-- third_party/
`-- .github/
```

## Current Status

The next public macOS release should use Amentia app, DMG, checksum, and
release-manifest asset names.
The app has the daily-driver cowork foundation: first-use local model setup,
workspace flow, Web Search, sandbox visibility, approvals, compact context
receipts, actionable source/file evidence, session delete and revert safety,
plugin recovery, Notion connector proof, memory capture, bounded local
execution, and the x86_64 macOS DMG package path.

The active work is Milestone 13: product quality and identity. Keep the app
small, human, and usable from the installed package before expanding the
connector platform.

Detailed milestone scope lives in [docs/development-plan.md](docs/development-plan.md).

Model packaging note:

- the repository tracks model pack manifests and small metadata
- the actual `LFM2.5-350M-Q4_K_M.gguf` weight file is downloaded by the app into local data storage, not committed to git history
- the in-app catalog stays intentionally small: default LFM plus modern tiny Granite, with new candidates added only after product-fit validation

## Runtime Shape

- Native macOS shell in `SwiftUI`
- Local runtime in `Rust`
- JSON-RPC style communication over `stdio`
- Local model runtime with `LFM2.5-350M` as the default downloadable first-use model option

## Development Notes

- macOS app target: `x86_64` on macOS 12+
- Core inference path must remain local-first
- Plugins are first-class product modules
- Repository artifacts should remain English-only

See [docs/development-plan.md](docs/development-plan.md) for the execution roadmap.

## Source Organization

The repository should stay organized by product and runtime ownership rather than by incidental helper
shape. `amentia-core` is the Rust orchestration layer, and its source tree is grouped by runtime, request,
turn, plugin, context, workspace, thread, and support domains. The macOS target follows the same rule
with app, runtime, local model, plugin, memory, timeline, workspace, and setup domains.

`amentia-memory` owns memory semantics such as notes, events, summaries, and note ranking. `amentia-storage`
owns durable runtime persistence for threads, workspace state, approvals, memory notes, and plugin
state.
`amentia-plugin-host` owns plugin manifests, discovery, capability registries, connector metadata, and
plugin bundle lifecycle boundaries.

See [docs/development-environment.md](docs/development-environment.md) for local setup and CI notes.
