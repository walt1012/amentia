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
- Treat `LFM2.5-350M` as the default built-in model pack baseline.
- Design plugins as first-class product modules so `Pith` can expand beyond code assistance into a broader local agent platform.
- Prefer free and open source dependencies, tooling, and model delivery paths.

## Repository Layout

```text
/
|-- apps/
|   `-- pith-macos/
|-- crates/
|   |-- pith-core/
|   |-- pith-model-runtime/
|   |-- pith-plugin-host/
|   |-- pith-protocol/
|   |-- pith-runtime-bin/
|   |-- pith-storage/
|   `-- pith-tools/
|-- plugins/
|   `-- official/
|-- docs/
|-- scripts/
|-- third_party/
`-- .github/
```

## Current Status

Milestone 1 is complete on the active development branch.

Milestone 2 is now in progress with plugin discovery, enable and disable flow, capability registry,
permission gating, per-plugin permissions and validation surfaces, command execution, and
shell-completed hook execution underway.

Delivered in Milestone 1:

- monorepo scaffolding
- Rust workspace skeleton and local runtime binary
- runtime protocol types and `stdio` JSON-RPC bridge
- macOS app shell with thread, timeline, and inspector views
- workspace-aware read, search, shell, diff preview, and approval-gated write tools
- SQLite-backed persistence for workspace, threads, approvals, and memory notes
- built-in memory retrieval, user workspace notes, and thread summary notes
- local model health inspection and local pack metadata bootstrap for the `LFM2.5-350M` runtime path
- CI checks

Milestone 1 exit criteria now covered:

- open a workspace
- create or resume a thread
- send a request through the local runtime
- approve file writes or shell commands
- inspect diff output
- receive a file change end to end

Model packaging note:

- the repository tracks model pack manifests and small metadata
- the actual `LFM2.5-350M.gguf` weight file should live in a local data directory or release bundle, not git history

## Planned Runtime Shape

- Native macOS shell in `SwiftUI`
- Local runtime in `Rust`
- JSON-RPC style communication over `stdio`
- Local model runtime with `LFM2.5-350M` as the default built-in model

## Development Notes

- macOS app target: `x86_64` on macOS 12+
- Core inference path must remain local-first
- Plugins are first-class product modules
- Repository artifacts should remain English-only

See [docs/development-plan.md](docs/development-plan.md) for the execution roadmap.
See [docs/development-environment.md](docs/development-environment.md) for local setup and CI notes.
