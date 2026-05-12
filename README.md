# Pith

`Pith` is a local-first macOS agent application for Intel Macs running macOS 12 and above.

The product goal is to combine:

- a premium native desktop experience
- a local model runtime
- structured task execution
- explicit approvals and diffs
- a plugin-capable architecture with built-in memory

The repository is intentionally English-only.

## Product Principles

- Build a native `macOS` application named `Pith` for `macOS 12+` on `x86_64` only.
- Keep the app lightweight and intentionally small while preserving a strong local agent loop.
- Favor a calm, premium, minimal UI inspired by high-quality agent tools such as Codex and Claude Code.
- Keep the default intelligence path fully local with no required external model API.
- Treat `LFM2.5-350M` as the default first-use model while keeping small GGUF alternatives available.
- Design plugins as first-class product modules so `Pith` can expand beyond code assistance into a broader local agent platform.
- Prefer free and open source dependencies, tooling, and model delivery paths.

## Repository Layout

```text
/
|-- apps/
|   `-- pith-macos/
|-- crates/
|   |-- pith-core/
|   |-- pith-memory/
|   |-- pith-model-runtime/
|   |-- pith-plugin-host/
|   |-- pith-protocol/
|   |-- pith-runtime-bin/
|   |-- pith-sandbox/
|   |-- pith-storage/
|   `-- pith-tools/
|-- plugins/
|   `-- bundled/
|-- docs/
|-- scripts/
|-- third_party/
`-- .github/
```

## Current Status

Milestones 1, 2, and 3 are complete on the active development branch.

Milestone 4 is underway. The current focus is turning plugins into real bounded local capabilities
instead of prompt templates while keeping the app compact, local-first, and workspace-safe.

The active branch includes guided local model choice, download, and activation, pause/continue/cancel
with persisted recovery state, workspace-bound thread filtering, bounded shell/model/web-search
execution, native sandbox diagnostics, progressive inspector disclosure, and typed plugin command
contracts with a minimal bounded `stdio` runner path.

Detailed milestone scope and implementation history live in [docs/development-plan.md](docs/development-plan.md).

Model packaging note:

- the repository tracks model pack manifests and small metadata
- the actual `LFM2.5-350M-Q4_K_M.gguf` weight file is downloaded by the app into local data storage, not committed to git history
- the in-app catalog stays intentionally small: default LFM plus modern tiny Granite, with new candidates added only after product-fit validation

## Planned Runtime Shape

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
shape. `pith-core` is the Rust orchestration layer, and its source tree is grouped by runtime, request,
turn, plugin, context, workspace, thread, and support domains. The macOS target follows the same rule
with app, runtime, local model, plugin, memory, timeline, workspace, and setup domains.

`pith-memory` owns memory semantics such as notes, events, summaries, and note ranking. `pith-storage`
owns durable runtime persistence for threads, workspace state, approvals, memory notes, and plugin
state.
`pith-plugin-host` owns plugin manifests, discovery, capability registries, connector metadata, and
plugin bundle lifecycle boundaries.

See [docs/development-environment.md](docs/development-environment.md) for local setup and CI notes.
