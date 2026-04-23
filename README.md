# Cavell

`Cavell` is a local-first macOS agent application for Intel Macs running macOS 12 and above.

The product goal is to combine:

- a premium native desktop experience
- a local model runtime
- structured task execution
- explicit approvals and diffs
- a plugin-first architecture

The repository is intentionally English-only.

## Repository Layout

```text
/
|-- apps/
|   `-- cavell-macos/
|-- crates/
|   |-- cavell-core/
|   |-- cavell-model-runtime/
|   |-- cavell-plugin-host/
|   |-- cavell-protocol/
|   |-- cavell-runtime-bin/
|   |-- cavell-storage/
|   `-- cavell-tools/
|-- plugins/
|   `-- official/
|-- docs/
|-- scripts/
|-- third_party/
`-- .github/
```

## Current Status

Milestone 0 is in progress.

Implemented foundation:

- monorepo scaffolding
- Rust workspace skeleton
- runtime protocol types
- local runtime binary scaffold
- macOS app source layout
- official `mem` plugin scaffold
- CI skeleton

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
