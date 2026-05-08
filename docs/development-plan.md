# Pith Development Plan

## 1. Document Purpose

This document defines the product, architecture, engineering rules, and phased execution plan for `Pith`.

`Pith` is intended to become a local-first macOS agent application with a premium desktop experience, a strong core agent loop, an extensible plugin architecture, and no dependency on external model APIs for its core intelligence path.

This plan is designed to be implementation-ready. It is not just a vision statement. Each section exists to reduce future rework when code implementation begins.

This document is not an implementation changelog. Completed micro-steps belong in commits, pull requests, and release notes. The plan should stay focused on product direction, architectural boundaries, milestone outcomes, and the next highest-leverage work.

## 2. Non-Negotiable Constraints

These constraints are mandatory and should be treated as project requirements, not preferences.

- Product name: `Pith`
- Target platform: macOS 12 and above
- CPU target: `x86_64` only
- Distribution target: signed and notarized `.app` bundle outside the Mac App Store
- Core inference path: fully local, no required external model API
- Default first-use model: `LFM2.5-350M`
- Development model: free and open source only
- Repository language policy: English only for source code, comments, docs, commit messages, branch names, PR titles, CI messages, and generated project templates where practical
- Product positioning: start as a small but strong local agent assistant, then grow into a plugin-powered local coworker platform
- Built-in memory requirement: local workspace memory is a core runtime module

## 3. Product Thesis

`Pith` should feel like a serious native agent workspace rather than a generic chat app.

The core thesis is:

- native desktop shell
- structured task execution timeline
- explicit action visibility
- local model ownership
- plugin-powered capability expansion
- workspace-aware agent execution

The first shipping goal is not to match the full intelligence quality of hosted frontier systems. The first shipping goal is to deliver a stable, elegant, local agent desktop application with a tight end-to-end task loop on Intel Macs, then expand that core into a broader platform.

Pith should be engineered as a controlled local agent runtime, not a model wrapper. The runtime owns bounded model execution, workspace-scoped tools, explicit approvals, context packing, plugin capability contracts, and machine-readable readiness diagnostics.

## 4. Source Study Summary

This plan is informed by direct inspection of the following reference repositories:

- `openai/codex`
- `anthropics/claude-code`
- `OpenHands/OpenHands`

### 4.1 What To Learn From Codex

From `openai/codex`, especially the `codex-rs/app-server` and `codex-rs/app-server-protocol` packages:

- The UI should remain a thin client over a structured runtime boundary.
- `Thread`, `Turn`, and `Item` are better primitives than raw chat messages.
- Bidirectional JSON-RPC works well for rich agent interfaces.
- The runtime should emit typed events instead of forcing the UI to infer state from plain text.
- Approvals, file operations, shell execution, and tool output should all flow through one unified event model.
- The runtime should be independently testable from the UI.
- Protocol schema generation is valuable and should be designed in from the start.

Relevant reference areas:

- `codex-rs/app-server/README.md`
- `codex-rs/app-server-protocol/`
- `codex-rs/thread-store/`
- `codex-rs/plugin/`
- `codex-rs/tools/`

### 4.2 What To Learn From Claude Code

From `anthropics/claude-code`, especially the plugin directory layout:

- Plugins should be visible product primitives, not hidden implementation details.
- A directory-based plugin format is simple, inspectable, and shareable.
- Commands, agents, skills, hooks, and MCP integrations should be separable capabilities within one plugin bundle.
- A plugin system becomes much more useful when bundled example plugins demonstrate best practices early without implying Pith-owned integrations are the long-term ceiling.
- The plugin manager should be part of the core user experience, not a later add-on.

Relevant reference areas:

- `plugins/README.md`
- `plugins/*/.claude-plugin/plugin.json`
- `plugins/*/commands`
- `plugins/*/agents`
- `plugins/*/hooks`
- `plugins/*/skills`

### 4.3 What To Learn From OpenHands

From `OpenHands/OpenHands`, especially the sandbox service layer:

- Agent actions should cross an explicit execution-environment boundary.
- The runtime should track sandbox lifecycle and readiness separately from chat state.
- Workspace scope, process lifecycle, health checks, and cleanup should be first-class runtime concerns.
- Pith should learn the boundary model, not the Docker dependency. The product direction remains a native macOS sandbox with no required third-party container runtime.

Relevant reference areas:

- `openhands/app_server/sandbox/README.md`
- `openhands/app_server/sandbox/sandbox_service.py`
- `openhands/app_server/sandbox/docker_sandbox_service.py`
- `openhands/app_server/sandbox/process_sandbox_service.py`

### 4.4 Source-Informed Direction For Pith

Based on those references, `Pith` should adopt:

- a native macOS shell
- a Rust runtime process
- a transport-neutral protocol with `stdio` as the first transport
- a timeline/event-centric data model
- a filesystem-based plugin format
- explicit approval gates
- typed plugin permissions
- a local model runtime abstraction that allows future stronger local models
- a native sandbox boundary for shell and plugin execution on macOS
- sandbox output as a context boundary: commands may produce large local artifacts, but prompts should receive
  only compact, budgeted, provenance-rich previews

## 5. Product Scope

### 5.1 In Scope For Phase 1

- native macOS application shell
- workspace selection
- thread list and thread resume
- prompt composer
- streaming assistant output
- local inference with `LFM2.5-350M`
- tool calls for filesystem and shell
- explicit approval UI for risky actions
- diff and patch review
- persistent thread history
- basic plugin discovery
- built-in memory module shell

### 5.2 Explicitly Out Of Scope For Phase 1

- App Store distribution
- cloud-hosted inference as a required dependency
- collaborative multi-user sessions
- remote execution farms
- iOS or Windows clients
- broad plugin marketplace publishing workflow
- deep multi-agent orchestration

### 5.3 Product Non-Goals

`Pith` should not become:

- a generic chatbot with a file picker
- a terminal emulator with minimal orchestration
- a plugin browser with weak first-party workflows
- a hosted inference frontend disguised as a local app

## 6. Experience Principles

### 6.1 UX Principles

- calm and minimal
- premium native macOS feel
- visible state transitions
- strong information hierarchy
- keyboard-first interaction
- low visual noise
- no decorative clutter

### 6.2 Functional Principles

- all important agent actions must be inspectable
- risky actions must be gated
- threads must be resumable
- plugins must have provenance
- the runtime must remain replaceable without rewriting the UI
- the core product must remain useful even with only bundled components

## 7. High-Level Architecture

## 7.1 Top-Level Split

Use a two-process architecture:

- `Pith.app`: native macOS UI process
- `pith-runtime`: local Rust runtime process

This split keeps the desktop shell responsive, makes runtime behavior testable, and mirrors the strongest architectural lesson from Codex.

### 7.2 Recommended Stack

Desktop UI:

- `SwiftUI` for primary UI construction
- targeted `AppKit` bridges for advanced macOS behaviors

Runtime:

- `Rust`

Storage:

- `SQLite` for structured metadata
- filesystem for artifacts, plugins, logs, and model packs

Inference:

- `llama.cpp` backend first
- `OpenVINO` backend second for Intel optimization

Transport:

- newline-delimited JSON-RPC over `stdio` first

### 7.3 Why SwiftUI Plus AppKit

This combination best matches the target product:

- polished native macOS feel
- strong keyboard shortcut support
- split views and inspector patterns
- window management
- security-scoped file access handling
- native settings and menu integration

Pure web-wrapper approaches should be avoided because they increase memory overhead, weaken the platform feel, and complicate long-term desktop quality.

### 7.4 Why Rust For The Runtime

Rust is the right choice for:

- process control
- event streaming
- plugin loading
- file and shell orchestration
- permission enforcement
- portable logic reuse
- deterministic serialization and protocol handling

## 8. Core Application Modules

The macOS app should be split into the following feature areas.

### 8.1 Shell Layer

Responsibilities:

- app lifecycle
- window lifecycle
- workspace open flow
- menu commands
- settings navigation
- runtime process launch and crash recovery UI

### 8.2 Navigation Layer

Responsibilities:

- workspace switcher
- thread list
- recent tasks
- pinned workspaces
- plugin manager entry

### 8.3 Timeline Layer

Responsibilities:

- render thread history
- stream new items
- show action provenance
- show tool steps
- show approvals
- show patch and diff artifacts
- show final summaries

### 8.4 Composer Layer

Responsibilities:

- prompt input
- file attachments
- slash command initiation
- mode selection
- send and cancel controls

### 8.5 Inspector Layer

Responsibilities:

- selected item detail
- diff detail
- plan detail
- plugin detail
- file preview
- runtime metadata

### 8.6 Settings Layer

Responsibilities:

- model packs
- plugin enablement
- permissions
- appearance
- keybindings
- runtime diagnostics

## 9. Runtime Modules

The Rust side should use a multi-crate workspace.

### 9.1 Proposed Crates

- `crates/pith-protocol`
- `crates/pith-core`
- `crates/pith-storage`
- `crates/pith-memory`
- `crates/pith-model-runtime`
- `crates/pith-tools`
- `crates/pith-plugin-host`
- `crates/pith-runtime-bin`

### 9.2 Responsibilities Per Crate

`pith-protocol`

- JSON-RPC envelopes
- request and notification types
- shared data models
- schema export

`pith-core`

- task orchestration
- turn execution
- event bus
- approval state machine
- thread lifecycle

`pith-storage`

- SQLite-backed runtime store
- thread, workspace, and approval persistence
- memory note persistence
- plugin state persistence
- storage migrations and legacy state import

`pith-model-runtime`

- model pack registry
- backend abstraction
- prompt assembly
- token streaming
- cancellation
- metrics

`pith-tools`

- filesystem tools
- shell tools
- diff generation
- future git tools

`pith-memory`

- workspace memory capture
- thread memory summaries
- local memory note ranking
- memory compaction hooks

`pith-plugin-host`

- plugin discovery
- manifest validation
- capability registration
- permission evaluation
- hook execution

`pith-runtime-bin`

- runtime bootstrap
- transport wiring
- config loading
- logging

## 10. Protocol Design

### 10.1 Transport

Phase 1 transport should be:

- `stdio`
- newline-delimited JSON messages
- bidirectional JSON-RPC style request and notification flow

Phase 2 can optionally add:

- localhost websocket transport for debugging

### 10.2 Core Protocol Primitives

The protocol should model:

- `Workspace`
- `Thread`
- `Turn`
- `TimelineItem`
- `Artifact`
- `ApprovalRequest`
- `PluginSummary`
- `TaskState`

### 10.3 Minimum Timeline Item Types

- user message
- assistant message
- assistant delta
- plan snapshot
- tool start
- tool completion
- shell command request
- shell command output artifact
- file patch proposal
- diff artifact
- approval requested
- approval resolved
- plugin event
- warning
- error
- final summary

### 10.4 Required Protocol Methods For Milestone 0

- `initialize`
- `workspace/open`
- `thread/start`
- `thread/list`
- `thread/read`
- `turn/start`
- `turn/cancel`
- `approval/respond`
- `plugin/list`
- `health/ping`

### 10.5 Protocol Quality Rules

- all types versioned from day one
- all payloads serializable with stable field naming
- all notifications typed and documented
- generated schemas checked into the repository
- integration tests for request and notification fixtures

## 11. Data Model

### 11.1 Core Entities

- `Workspace`
- `Thread`
- `Turn`
- `TimelineItem`
- `Artifact`
- `Plugin`
- `PluginInstall`
- `ApprovalRequest`
- `ModelPack`
- `Task`

### 11.2 Storage Strategy

Use SQLite for:

- threads
- turns
- timeline item metadata
- settings
- plugin registrations
- approval records
- workspace metadata
- memory notes

Use the filesystem for:

- attachments
- rendered diffs
- logs
- traces
- plugin bundles
- model files
- cache

### 11.3 Recommended Paths

- app root: `~/Library/Application Support/Pith/`
- database: `storage/pith.db`
- artifacts: `artifacts/`
- models: `models/`
- plugins: `plugins/`
- logs: `logs/`
- traces: `traces/`

## 12. Model Strategy

### 12.1 Reality Check On `LFM2.5-350M`

The default model is a hard requirement, but it should be treated realistically.

`LFM2.5-350M` is suitable for:

- lightweight orchestration
- structured output
- extraction
- short planning
- command formatting
- small context-assisted tasks

It is not sufficient as the final long-term answer for strong coding quality across complex repositories.

### 12.2 Product Strategy Implication

Phase 1 should use `LFM2.5-350M` as:

- the default first-use download
- the orchestration baseline
- the offline baseline after first setup

The architecture must also support optional future local packs for stronger coding performance without violating the requirement that the first-use path stays fully local after the selected model is downloaded.

Model files are delivered through the in-app first-use download flow, not bundled inside the `.app`
and not primarily imported from the filesystem. The app may ship catalog metadata and runtime
support, but the selected GGUF is downloaded, verified, activated, and then reused locally.

### 12.3 Curated Small Model Catalog

The app should not feel like a model zoo. Keep the first-use catalog compact, current, and easy to understand:

- `LFM2.5-350M Q4_K_M` remains the default fastest first-use download
- `Granite 4.0-H-350M Q4_K_M` is the preferred Apache-2.0 tiny model for tool, code, and context-assisted workflows

Retire older, redundant, or awkward-fit entries when they do not clearly beat the active set. Every built-in catalog entry must include verified file size and SHA-256 metadata before activation is allowed.

### 12.4 Backend Order

Recommended backend sequence:

1. `llama.cpp` plus GGUF
2. `OpenVINO` optimization path for Intel Macs

Reasoning:

- `llama.cpp` lowers delivery risk
- GGUF packaging is straightforward
- `OpenVINO` can become the performance path after the end-to-end product loop is stable

### 12.5 Model Runtime Requirements

- streaming token delivery
- cancellation
- bounded generation timeouts
- subprocess cleanup after timeout or cancellation
- runtime request paths that unblock after model failure
- configurable prompt templates
- structured output mode
- role-based prompt assembly
- context window management
- token accounting
- backend health reporting

### 12.6 Model Role Abstraction

Even if all roles use the same selected local model initially, the runtime should support separate logical roles:

- `default`
- `planner`
- `coder`
- `summarizer`

This avoids dead-end architecture.

## 13. Tooling Strategy

Phase 1 built-in tools should be:

- `read_file`
- `write_file`
- `list_directory`
- `search_files`
- `run_shell`
- `generate_diff`

Phase 2 tools should add:

- `git_status`
- `git_diff`
- `git_commit`
- `open_url`
- `download_asset`

Tool invocation rules:

- tools must produce structured results
- tool calls must emit timeline events
- all risky tools must support approval gating
- plugin-owned tools must declare provenance

## 14. Plugin System

### 14.1 Plugin Design Goals

Plugins must be able to extend:

- commands
- agents
- prompts
- hooks
- tools
- MCP connectors
- settings panes

### 14.2 Bundle Layout

Use a Claude-inspired directory-based layout with Pith naming.

```text
plugin-name/
|-- pith-plugin.json
|-- commands/
|-- agents/
|-- prompts/
|-- hooks/
|-- skills/
|-- mcp/
|-- assets/
`-- README.md
```

### 14.3 Manifest Shape

`pith-plugin.json` should include:

- `name`
- `version`
- `displayName`
- `description`
- `author`
- `homepage`
- `license`
- `capabilities`
- `permissions`
- `skills`
- `mcpServers`
- `appConnectors`
- `authPolicy`
- `entrypoints`
- `compatibility`
- `defaultEnabled`

### 14.4 Plugin Capability Types

- `command`
- `agent`
- `prompt_pack`
- `hook`
- `tool`
- `mcp_server`
- `skill`
- `connector`
- `settings`

### 14.5 Plugin Permissions

At minimum:

- `file.read`
- `file.write`
- `shell.exec`
- `network.outbound`
- `workspace.background`
- `model.invoke`
- `mcp.connect`

### 14.6 Plugin Lifecycle

- discover plugin bundle
- validate manifest
- register declared capabilities
- evaluate permissions
- show plugin in manager UI
- enable plugin per user or workspace
- activate plugin on demand
- run command capabilities only through explicit plugin-owned execution contracts; prompt-only command manifests can be listed for compatibility but are not runnable until they declare an executable contract
- prepare connector capabilities for third-party services such as Notion through manifest-declared auth, permission, and MCP or app surfaces

### 14.7 Bundled Example Plugins

Phase 1 bundled examples:

- `filesystem`
- `shell`
- `git`

Phase 2 connector examples:

- `workflow-coder`
- `workflow-research`
- `github`
- `notion`

### 14.8 Built-In Memory Scope

Initial built-in memory responsibilities:

- workspace facts store
- thread summaries
- user notes attached to workspaces
- lightweight memory note ranking into prompts, with ranking scores exposed for context attribution

Phase 2 memory responsibilities:

- cross-thread memory references
- plugin-provided memory ranking policies
- background memory compaction
- workspace context selection only after the context ledger and thread compaction paths are stable
- optional local embedding and rerank path only after lexical context selection proves useful

### 14.9 Context Engineering Direction

Pith should not treat the current memory ranking path as Codex-style RAG. The current implementation
is attribution-backed memory context: it ranks explicit user and workspace notes, fits them into a small
model budget, and exposes attribution so the UI and runtime can explain why context was included.

The Codex-inspired direction is context engineering, not a generic document RAG layer:

- context ledger for file reads, diffs, approvals, shell output, plugin output, and model observations
- thread compaction that preserves decisions, touched paths, unresolved tasks, and evidence references
- budget-aware prompt assembly that can explain what was kept, compressed, or dropped
- sandbox-backed output previews that keep raw shell output local while passing only compact evidence into prompts
- workspace context selection after ledger and compaction are reliable, starting with lexical scoring before embeddings
- optional local embeddings or reranking only when they improve the local daily loop without adding weight

Milestone 3 should close with ranked memory note packing, compact prompts, attribution, and bounded execution.
Milestone 4 may expand into workspace-level retrieval, but only as a continuation of the ledger and compaction
architecture rather than a separate search product.

## 15. Security And Approval Model

### 15.1 Approval Required For

- file writes
- shell execution
- destructive file removal
- plugin installation
- network-enabled plugin actions
- destructive git actions

### 15.2 Approval UX Requirements

- clear action summary
- plugin provenance
- affected paths
- command preview
- allow once or deny once
- future support for scoped allow rules

### 15.3 Sandboxing Strategy

Phase 1:

- local trust boundary
- explicit approvals
- bounded subprocess execution with timeout and cleanup
- native macOS workspace sandbox for shell actions when the platform backend is available
- workspace-local temporary files for sandboxed commands instead of broad host temp writes
- workspace-local shell output artifacts with compact preview attributes for model context
- no required Docker, VM, or third-party container runtime

Phase 2:

- stricter capability-based execution
- per-plugin environment controls
- improved subprocess isolation
- sandbox readiness surfaced as machine-readable runtime diagnostics

### 15.4 Provenance Requirements

Every user-visible action should identify:

- runtime source
- plugin source if any
- whether approval was required
- approval outcome
- timestamp

## 16. Repository Layout

Recommended monorepo layout:

```text
/
|-- apps/
|   `-- pith-macos/
|-- crates/
|   |-- pith-core/
|   |-- pith-model-runtime/
|   |-- pith-plugin-host/
|   |-- pith-sandbox/
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

## 17. Engineering Standards

### 17.1 Language Policy

All repository artifacts must remain English only.

This includes:

- source code
- comments
- documentation
- commit messages
- branch names
- PR titles
- automation text in repository-managed scripts

### 17.2 Code Quality Tooling

macOS app:

- `SwiftFormat`
- `SwiftLint`
- `XCTest`

Rust:

- `rustfmt`
- `clippy`
- `cargo test`

Repository:

- `.editorconfig`
- lint and format CI
- schema fixture validation
- release packaging checks

### 17.3 Branch And Commit Policy

- default branch prefix: `codex/`
- English-only commit messages
- conventional, readable commit titles
- small PRs with clear scope

### 17.4 Open Source Compliance

- prefer Apache-2.0 or MIT-compatible dependencies
- maintain `third_party/` notices where needed
- document model weights licensing clearly
- ensure bundled runtime dependencies permit redistribution

## 18. Delivery Roadmap

### Milestone 0: Foundation

Goal:

- establish a buildable monorepo and a working UI-to-runtime boundary

Deliverables:

- SwiftUI app shell
- Rust workspace scaffold
- JSON-RPC protocol crate
- runtime child-process bootstrap
- basic thread list shell
- CI skeleton
- formatting and lint configuration

Exit criteria:

- the app launches
- the runtime launches
- the UI sends one real request over `stdio`
- the runtime responds successfully

### Milestone 1: Local Agent MVP

Goal:

- deliver a usable local agent flow

Deliverables:

- workspace open flow
- thread creation and resume
- timeline rendering
- streaming output
- `LFM2.5-350M` inference integration
- cancel generation
- filesystem and shell tools
- approval flow
- diff proposal and review
- SQLite persistence

Exit criteria:

- a user can open a workspace, request a task, approve actions, and receive a file change end to end

### Milestone 2: Plugin MVP

Goal:

- establish plugin discovery, provenance, metadata, and manager foundations without expanding beyond the local daily-driver loop

Deliverables:

- plugin manager UI
- plugin enable and disable flow
- plugin installation and removal workflow
- plugin manifest validation and repair surfaces
- plugin capability registry
- memory-aware plugin integration points
- per-plugin permissions UI
- prompt-only plugin commands and hooks listed as metadata, but blocked until they declare executable contracts
- Codex-inspired plugin package metadata for skills, MCP servers, app connectors, and third-party auth policies
- connector registry metadata for disabled, needs-auth, and ready third-party app integrations
- Notion connector design spike limited to auth, permission scopes, and local execution boundaries

Exit criteria:

- bundled and locally installed plugin manifests load, validate, and surface provenance in-app
- connector metadata can be inspected without requiring third-party auth or execution
- prompt-only plugin commands are visible but blocked until they provide an execution contract
- real plugin-owned execution, third-party auth, and connector workflows are explicitly deferred to Milestone 4

### Milestone 3: Premium Desktop Quality

Goal:

- raise the application from working to daily-driver quality

Deliverables:

- guided local model delivery with `LFM2.5-350M` as the default, small local alternatives, persisted choice, and one-click download, pause, resume, cancel, activation, and relaunch
- strict local model readiness with verified model integrity, no silent degraded-generation path, one active model at a time, and clear selection, reset, and recovery states
- bounded local inference with generation timeouts, cancellation, subprocess cleanup, and runtime unblocking after model failure
- native local sandbox foundation for shell execution, with workspace-scoped policy, workspace-local temp routing, compact output previews, workspace-local output artifacts, and readiness diagnostics
- fresh-install flow from runtime launch to model, workspace, thread, and first request without hidden setup knowledge
- compact daily-loop surface built around the timeline header, setup progress, readiness chips, composer gating, and keyboard-first actions
- timeline quality improvements for stable selection, concise operation history, diff readability, streaming state, and contextual recovery
- inspector progressive disclosure for local model, memory, workspace search, plugin manager, thread, and diagnostics so secondary controls do not become primary chrome
- workspace and thread integrity through workspace-bound threads, restoration, stale restore handling, runtime crash recovery, and pending request cleanup
- local context management for small models through compact prompts, ranked memory note packing, budget-aware context headers, ranking score attribution, sandbox-backed tool observation previews, and a clear path toward context ledger design
- native desktop polish on Intel Macs, including better loading, blocking, empty, and error states without adding heavyweight surfaces
- plugin work limited to manager polish and capability visibility; broad connectors, third-party auth, real plugin execution contracts, and multi-agent workflows stay in Milestone 4 unless they unblock the local daily loop

Exit criteria:

- a fresh install can choose, download, activate, and run a selected small local model without hidden degraded-generation behavior
- model generation timeout, cancellation, or backend failure does not require restarting the app to recover the runtime loop
- shell execution is bounded by approvals, timeouts, cleanup, workspace-local temp routing, compact output artifacts, and native sandbox diagnostics without requiring third-party containers
- a user can bind a workspace, create or resume a thread, send the first local request, and recover from common setup failures without reading external docs
- the normal ready state feels quiet, stable, intentional, and distinctly native on Intel Mac hardware

### Milestone 4: Platform Expansion

Goal:

- extend beyond single-thread assistant behavior

Deliverables:

- real plugin-owned execution contracts
- multi-agent workflows
- automation
- background tasks
- context engineering expansion through context ledger, thread compaction, workspace retrieval, and optional local embedding/rerank components
- plugin-defined agents
- third-party connector execution and auth flows, with Notion as the first reference connector
- MCP client support

Exit criteria:

- Pith operates as a local coworker platform rather than only a coding assistant

## 19. Build Order

Recommended implementation order:

1. create the monorepo layout
2. scaffold the macOS application shell
3. scaffold the Rust workspace
4. define protocol types and fixtures
5. establish `stdio` runtime communication
6. implement runtime persistence
7. render the thread list and timeline shell
8. integrate `LFM2.5-350M`
9. implement streaming output and cancellation
10. implement filesystem and shell tools with approvals
11. implement diff and patch review
12. expand plugin management after the built-in memory module is stable

## 20. Completed Milestone Archive

Completed milestone details should live in commit history, pull requests, and release notes rather than the active plan.

### 20.1 Milestone 0 Outcome

Milestone 0 established the monorepo, SwiftUI app shell, Rust runtime workspace, JSON-RPC process boundary, basic thread shell, and CI baseline.

### 20.2 Milestone 1 Outcome

Milestone 1 established the local agent MVP: persistence, workspace-aware threads, local model integration, filesystem and shell tools, approvals, diff review, and timeline rendering.

## 21. Testing Strategy

### 21.1 Unit Tests

- protocol serialization
- manifest validation
- permission evaluation
- model runtime adapters
- approval reducer logic
- storage migrations

### 21.2 Integration Tests

- app launches runtime
- initialize handshake succeeds
- thread creation succeeds
- streaming response events render
- approval request and response loop works
- local model bootstrap and health checks succeed
- bounded model generation timeout and recovery succeed

### 21.3 UI Tests

- open workspace
- create thread
- send prompt
- cancel generation
- approve action
- inspect diff
- enable or disable plugin
- recover after model timeout or runtime crash

### 21.4 Performance Validation

Measure on actual Intel Mac hardware:

- cold app launch time
- runtime launch time
- first-token latency
- sustained token throughput
- idle memory usage
- loaded-model memory usage
- recovery after runtime crash

## 22. Release Strategy

### 22.1 Distribution

- direct download from GitHub Releases
- signed and notarized `.app`
- zipped release artifact

### 22.2 Release Requirements

- successful CI
- smoke test on Intel Mac
- first-use model download and activation verified
- model integrity verification confirmed through catalog size, GGUF magic, and SHA-256 metadata
- plugin bundle presence verified
- migration test for existing local data

## 23. Main Risks And Mitigations

### Risk 1: `LFM2.5-350M` is too weak for high-quality coding tasks

Mitigation:

- treat it as a required baseline model, not the only long-term model strategy
- optimize for structured orchestration
- keep the curated in-app catalog available with Granite while validating future small-model candidates before adding them

### Risk 2: Intel performance is not good enough

Mitigation:

- optimize the UI for low overhead
- start with `llama.cpp`
- validate on actual Intel hardware early
- add `OpenVINO` after the first working loop is stable

### Risk 3: Native UI and Rust runtime integration slows iteration

Mitigation:

- keep protocol narrow
- keep business logic out of the UI
- create protocol fixtures early
- add integration tests around the process boundary

### Risk 4: Plugin flexibility creates trust issues

Mitigation:

- typed permissions
- approval gates
- plugin provenance
- bundled example plugins as reference implementations

### Risk 5: x86-only macOS support complicates CI and release verification

Mitigation:

- keep release packaging explicit for `x86_64`
- add Intel hardware verification before release
- avoid accidental universal or arm-only release assumptions

## 24. Immediate Next Actions

Milestone 3 is in closeout. The current codebase has the core daily-driver path in place: guided
local model setup, strict model readiness, bounded model and shell subprocesses, runtime request
unblocking for heavy work, workspace-bound threads, first-request readiness diagnostics, native
sandbox diagnostics, compact context packing, ranked memory attribution, and progressive disclosure
for secondary controls.

The next product step is a broad review and refactor gate, not another feature expansion. Review the
Swift app, Rust runtime, protocol boundary, model catalog, plugin metadata, sandbox layer, and docs
as one system. Keep only the structures that directly support a small, strong local desktop agent.

Refactor priorities:

1. shrink large files and move cohesive responsibilities into small modules
2. keep the main timeline and local model flow as the primary product surface
3. preserve bounded execution, cancellation, integrity checks, and workspace scoping as non-negotiable runtime contracts
4. preserve the machine-readable setup contract from runtime launch through model, workspace, thread, and first request
5. keep memory ranking scoped as attribution-backed context packing until the context ledger and compaction design is ready
6. defer broad connector execution, third-party auth, multi-agent workflows, generic RAG, and platform expansion to Milestone 4
7. keep this plan outcome-based so implementation detail lives in code, commits, tests, and review notes

Remote CI verification remains routine for every pushed change, not a product milestone.
