# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS agent app for controlled local work.

It should feel native, focused, recoverable, and capable. It must not become a generic chatbot, web wrapper, terminal skin, hosted-model frontend, or feature zoo.

## Hard Rules

- Product name: `Pith`
- Platform: macOS 12+
- CPU target: `x86_64` only
- Core intelligence: local model, no required external model API
- First-use model flow: in-app download, defaulting to `LFM2.5-350M`
- Runtime model rule: one active local model at a time
- Repository artifacts: English only
- Foundation: free and open source

## Principles

- Small beats sprawling.
- Native beats web-shell.
- Structured events beat transcript parsing.
- Bounded execution beats best-effort execution.
- Progressive disclosure beats admin panels.
- Focused retrieval and context engineering beat generic retrieval systems.
- Plugin contracts beat prompt-template shortcuts.

## Architecture

- `Pith.app`: native SwiftUI/AppKit shell for interaction, setup, approvals, timeline, and progressive surfaces.
- `pith-runtime`: local Rust runtime for orchestration, model readiness, tools, permissions, persistence, memory ranking, plugins, sandbox diagnostics, and bounded subprocesses.
- Protocol: typed JSON-RPC with explicit provenance, permissions, approvals, artifacts, cancellation, and recovery state.

## Product Direction

Models:

- Default: `LFM2.5-350M Q4_K_M`
- Alternative: `Granite 4.0-H-350M Q4_K_M`
- Downloads must verify URL, size, GGUF shape, and SHA-256 before activation.
- Avoid model-zoo growth unless a model clearly improves the tiny local loop.

Tools and sandbox:

- Tools must be structured, inspectable, bounded, workspace-scoped, and permission-aware.
- Use native macOS sandbox when available.
- Shell, model generation, web retrieval, and helper subprocesses must support timeout, cancellation, cleanup, and compact output.

Context and retrieval:

- Compact prompt assembly for small local models.
- Ranked memory notes with attribution.
- Compact tool observations and artifact references.
- Web search is the current RAG retrieval layer for fresh public information.
- Do not add local document indexing, vector stores, or generic document RAG until real usage proves it is needed.

Plugins:

- Milestone 2 delivered metadata, discovery, validation, visibility, and permission foundations.
- Milestone 3 only polishes visibility and boundaries.
- Milestone 4 adds real plugin execution contracts, third-party auth, MCP connectors, and connector workflows such as Notion.

## Roadmap

- Milestone 0: complete. Monorepo, app shell, Rust runtime, JSON-RPC, CI.
- Milestone 1: complete. Local agent MVP with workspace, threads, timeline, model path, tools, approvals, diffs, and persistence.
- Milestone 2: complete. Plugin MVP metadata, registry, manager surfaces, permissions, and connector metadata.
- Milestone 3: current. Premium local daily-driver quality.
- Milestone 4: later. Real plugin execution, connectors, MCP, context ledger, thread compaction, and automation.

## Milestone 3 Exit

- Fresh install can download, verify, activate, and use one selected local model.
- Failed generation, timeout, cancellation, or runtime exit does not require app restart.
- Tools are bounded, inspectable, permission-aware, and workspace-scoped.
- Sandbox diagnostics expose backend, active state, writable roots, temp root, and network policy.
- Timeline, inspector, setup, and model management stay calm and progressive.
- Context remains compact and explainable for tiny local models.
- Web retrieval is default-on, permissioned, bounded, and separate from sandbox and memory ranking.
- Architecture is clean enough to start real plugin execution work.

## Not Yet

- No broad connector execution.
- No third-party auth flows.
- No multi-agent workflows.
- No local document indexing or vector store.
- No generic document RAG.
- No cosmetic splitting that makes architecture harder to understand.

## Next Moves

1. Close remaining Milestone 3 gaps in model setup, runtime recovery, sandbox diagnostics, context compactness, and Swift coordinator boundaries.
2. Keep UI quiet in the normal ready state.
3. Review architecture before opening Milestone 4.
