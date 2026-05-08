# Pith Development Plan

## Purpose

This document is the product and architecture map for `Pith`.

It is not a changelog, backlog, or implementation diary. Keep detailed work in code, commits, tests, review notes, and release notes.

## North Star

Pith is a small, strong, local-first macOS agent app.

It should feel like a native desktop workspace for controlled agent work, not a generic chatbot, web wrapper, terminal skin, or hosted-model frontend.

## Non-Negotiables

- Product name: `Pith`
- Platform: macOS 12+
- CPU target: `x86_64` only
- Core intelligence path: local model, no required external model API
- First-use model path: in-app download, defaulting to `LFM2.5-350M`
- Runtime rule: one active local model at a time
- Repository rule: English-only artifacts
- Development foundation: free and open source
- Product shape: small local daily-driver first, plugin-powered platform later

## Product Principles

- Small beats sprawling.
- Native beats web-shell.
- Structured events beat transcript parsing.
- Bounded execution beats best-effort execution.
- Progressive disclosure beats always-visible admin panels.
- Context engineering beats generic document RAG.
- Plugin contracts beat prompt-template shortcuts.

## Reference Lessons

Learn selectively:

- Codex: runtime boundary, typed timeline events, approvals, tools, threads, and turns.
- Claude Code: inspectable plugin bundles and visible capability surfaces.
- OpenHands: explicit execution boundary, sandbox lifecycle, health, cleanup, and recovery.
- Context Mode: compact tool-output context, artifact references, diagnostics, and session continuity.

Do not copy any reference blindly. The target remains a lightweight native macOS app.

## Architecture Direction

Use a two-process architecture:

- `Pith.app`: native SwiftUI/AppKit desktop shell.
- `pith-runtime`: local Rust runtime over typed JSON-RPC.

The app owns native interaction, timeline rendering, setup, approvals, and progressive surfaces.

The runtime owns orchestration, model readiness, tool execution, permissions, persistence, memory ranking, plugin metadata, sandbox diagnostics, and bounded subprocess control.

The protocol must stay typed, versioned, event-oriented, and explicit about provenance, permissions, approvals, artifacts, and recovery state.

## Model Direction

The first-use flow should let the user choose and download a verified small GGUF model inside the app.

Default:

- `LFM2.5-350M Q4_K_M`

Curated alternative:

- `Granite 4.0-H-350M Q4_K_M`

Catalog entries must verify URL, size, GGUF shape, and SHA-256 before activation. Avoid model-zoo growth unless a model clearly improves the small local loop.

Backend order:

1. `llama.cpp` plus GGUF
2. `OpenVINO` for Intel optimization after the core loop is stable

## Tools, Sandbox, And Context

Tools should be structured, inspectable, bounded, and permission-aware.

Sandbox direction:

- native macOS sandbox when available
- workspace-scoped file and shell behavior
- bounded shell execution with timeout, cancellation, cleanup, and compact output previews
- diagnostics for backend, active state, writable roots, temp root, and network policy

Context direction:

- compact prompt assembly for small local models
- ranked memory notes with attribution
- compact tool observations and artifact references
- web search as a separate permissioned network retrieval tool for current public information

Web search is not sandbox behavior. Current memory ranking is not generic RAG.

## Plugin Direction

Milestone 2 established plugin metadata, manager surfaces, manifest validation, capability visibility, and permission foundations.

Milestone 3 should only polish plugin visibility and keep capability boundaries clear.

Milestone 4 should add real plugin-owned execution contracts, third-party auth, MCP connectors, and connector workflows such as Notion.

Prompt-only plugin commands stay visible but blocked until they declare executable contracts.

## Roadmap

### Milestone 0: Foundation

Status: complete.

Outcome: monorepo, Swift app shell, Rust runtime workspace, JSON-RPC boundary, and CI baseline.

### Milestone 1: Local Agent MVP

Status: complete.

Outcome: workspace open, thread lifecycle, timeline, local model path, file and shell tools, approvals, diff review, and persistence.

### Milestone 2: Plugin MVP

Status: complete.

Outcome: plugin discovery, validation, manager surfaces, capability registry, permission visibility, and connector metadata baseline.

### Milestone 3: Premium Desktop Quality

Status: closeout and refactor.

Goal: make the local daily-driver loop stable, elegant, bounded, recoverable, and architecturally clean.

Required outcomes:

- first-use model download, pause, resume, cancel, verification, activation, and relaunch
- strict local model readiness with no degraded-generation fallback
- bounded model generation and shell execution
- native sandbox foundation and diagnostics
- compact timeline and progressive inspector surfaces
- stable workspace, thread, selection, runtime, and cancellation recovery
- compact context packing for small local models
- default-on permissioned web search for current public information
- refactor of large or mixed-responsibility areas that block future work

Exit criteria:

- fresh install can download, activate, and use a selected local model without hidden setup knowledge
- failed generation, timeout, or cancellation does not require app restart
- tools are bounded, inspectable, and workspace-scoped
- normal ready state feels quiet, native, and focused
- architecture is clean enough to enter real plugin execution work

### Milestone 4: Platform Expansion

Status: not started.

Candidate outcomes:

- real plugin-owned execution contracts
- third-party connector auth and execution
- Notion reference connector
- MCP client support
- context ledger and thread compaction
- optional local retrieval and reranking
- automation and multi-agent workflows

## Current Focus

Stay on Milestone 3 closeout:

- keep model setup and readiness solid
- keep execution bounded and cancellable
- keep sandbox native and diagnostic-driven
- keep context compact and explainable
- keep web search separate from sandbox and memory
- reduce architecture bottlenecks without cosmetic file splitting
- keep the UI calm and primary-task focused

Do not start broad connector execution, third-party auth, multi-agent workflows, or generic RAG until Milestone 4.

## Quality Gates

Every meaningful change should preserve:

- successful remote CI
- English-only repository artifacts
- no required external model API
- no silent model fallback
- no unbounded subprocess path
- no workspace escape through paths or symlinks
- no admin/debug surface becoming the primary UI

Remote CI verification is routine engineering hygiene, not a milestone.

## Immediate Next Actions

1. Finish Milestone 3 refactor around runtime routing, sandbox diagnostics, model setup, and Swift coordinator boundaries.
2. Keep this plan concise and outcome-based.
3. Run a broad architecture review before opening Milestone 4.
4. Enter Milestone 4 only when the local daily-driver loop is stable, bounded, and visually calm.
