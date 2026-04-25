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
|   `-- bundled/
|-- docs/
|-- scripts/
|-- third_party/
`-- .github/
```

## Current Status

Milestone 1 is complete on the active development branch.

Milestone 2 is now complete on the active development branch. It includes plugin discovery, enable
and disable flow, installation and removal workflow, capability registry, permission gating,
per-plugin permissions and validation surfaces, repair hints for invalid manifests, reviewed install
and removal prompts, memory-aware plugin command execution, memory-aware shell-completed hook
execution, executable bundled plugin examples, and a Notion connector manifest template plus
connector registry for the third-party plugin surface.

Milestone 3 is underway with one-click default local model delivery work: the macOS app streams
model downloads to disk, shows lightweight progress and speed status, lets users pause, continue,
or cancel long downloads, activates the default LFM model after download, and restarts the runtime
when needed so a fresh install reaches a real local model path with fewer manual steps. It also
restores the last workspace and key inspector disclosure state across launches, and surfaces a
clear relaunch path when the local runtime exits unexpectedly. A lightweight workspace search now
lets users find matching lines from the inspector without opening a separate file browser. Native
menu shortcuts cover runtime launch, workspace opening, thread creation, message send, and turn
cancellation without adding more visible controls. Plugin installation stays inside Plugin Manager
so the main toolbar remains focused on the local daily loop. The composer now explains
blocking states inline so users can recover the local runtime, model, workspace, or thread setup
without guessing why send is disabled. The timeline header now carries the same compact status
language for runtime recovery, first-use model download, workspace binding, and active streaming, plus a
single contextual next-action button when the daily loop is blocked. A compact readiness strip
keeps runtime, model, workspace, and thread state visible without opening inspector sections.
Thread summaries now carry their bound workspace, and the macOS app only offers workspace-matching
threads in the daily flow so a selected thread cannot silently run against a different project.
The same header now shows local setup progress and one setup callout for the current blocker:
first-use model download includes size, license, progress, pause, continue, and cancel controls,
while workspace and thread blockers explain their next action without opening inspector sections.
The welcome timeline starts with the actual fresh-install path instead of internal milestone
language, and the ready composer offers three compact first-message suggestions so a new user can
begin useful local work without opening another onboarding surface. Workspace search includes
empty-state guidance and now lives behind progressive disclosure, while a compact inspector session
card summarizes the current model, workspace, and thread state. The Local
Model panel now shows one contextual primary action and only reveals cancel while a download can be
cancelled, keeping pause, continue, activation, and readiness repair focused while deeper model
diagnostics stay tucked away; if the default model is already downloaded, the primary action can
select it directly. Model download start,
continue, pause, cancel, failure, and success events are also recorded in the timeline so recovery
does not depend on transient status text, and first-use runtime launch records a clear model-required
event when no ready local model exists. The composer stays gated until runtime, model, workspace,
and thread setup are ready. Timeline refreshes preserve the selected inspector item whenever it
still exists, so streaming updates do not pull focus away from review work. Diff timeline cards now stay compact, while selected diff
inspection uses a line-level view with change counts and highlighted additions, deletions, hunks,
and metadata. Timeline cards include lightweight kind pills, and secondary inspector sections stay
behind disclosure controls.

Delivered in Milestone 1:

- monorepo scaffolding
- Rust workspace skeleton and local runtime binary
- runtime protocol types and `stdio` JSON-RPC bridge
- macOS app shell with thread, timeline, and inspector views
- workspace-aware read, search, shell, diff preview, and approval-gated write tools
- SQLite-backed persistence for workspace, threads, approvals, and memory notes
- built-in memory retrieval, user workspace notes, and thread summary notes
- local model health inspection and local pack metadata bootstrap for the `LFM2.5-350M` runtime path

Milestone 1 exit criteria now covered:

- open a workspace
- create or resume a thread
- send a request through the local runtime
- approve file writes or shell commands
- inspect diff output
- receive a file change end to end

Model packaging note:

- the repository tracks model pack manifests and small metadata
- the actual `LFM2.5-350M-Q4_K_M.gguf` weight file should live in a local data directory or release bundle, not git history

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
