# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS agent app for controlled local work.
It should feel native, focused, recoverable, and capable without becoming a
generic chatbot, terminal skin, hosted-model frontend, or feature zoo.

## Product Rules

- Product: `Pith`, macOS 12+, `x86_64` only.
- Intelligence: local model by default; no required external model API.
- First use: in-app model download, defaulting to `LFM2.5-350M`.
- Runtime: one active local model at a time.
- Plugins: real local capabilities, not prompt templates.
- Retrieval: web search is the active retrieval layer; no generic local RAG yet.
- Repository: English-only source, docs, commits, branches, and PR text.
- Foundation: free and open source.

## Architecture Boundaries

- `apps/pith-macos`: native UI, setup, approvals, timeline, model manager,
  inspector, and app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, request routing, request
  supervision, notifications, and runtime lock boundaries.
- `crates/pith-core`: orchestration, request reducers, turn lifecycle,
  readiness, permissions, context packing, memory usage, and plugin execution.
- `crates/pith-tools`: bounded workspace tools, shell, web search, output
  compaction, and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory semantics, note ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory and storage do not conflict: memory owns meaning and ranking; storage
owns durable records.

## Closed Foundation

Milestones 1-3 are closed. Pith now has guided local model setup, resumable
downloads, verified single-model activation, runtime recovery, bounded shell
and model work, workspace-safe file tools, web search, native sandbox
diagnostics, compact context packing, progressive inspector surfaces, and
typed plugin registries.

## Current Milestone: M4 Real Plugin Platform

M4 turns plugins into bounded local capabilities:

- Done: command registries expose typed execution contracts.
- Done: plugin registry assembly is split by capability, connector, command,
  hook, metadata, and execution-contract ownership.
- Done: minimal `stdio` plugin runner is bounded, cancellable, plugin-root
  scoped, native-sandbox-bound, and supports runner-owned timeline items.
- Done: runner success and failure paths expose sandbox, exit, stdout, and
  stderr diagnostics in timeline metadata.
- Done: Notion-like connectors expose persisted local auth status, authorize,
  clear credential, and failure surfaces without a broad marketplace.
- Done: connector-backed plugin commands are blocked until required connector
  auth is present, and command registries expose the run blocker.
- Done: authorized connector commands pass non-secret credential references to
  bounded runner envelopes and timeline metadata.
- Done: connector credential references now use a local provider handle instead
  of scattering credential-shaped fields through runner input.
- Done: first MCP stdio adapter runs declared MCP server commands through the
  bounded plugin process path and parses `tools/call` responses.
- Done: MCP command execution now requires explicit plugin-declared
  `mcp.connect`, and connector-backed MCP commands also require
  `network.outbound` before any runner process starts.
- Done: connector credentials can resolve into per-run environment bindings,
  keeping secrets out of plugin registries, timeline metadata, and MCP stdin.
- Done: connector-backed MCP plugin commands now request user approval before
  runner launch and continue through the existing approval response path.
- Done: MCP stdio output now records protocol diagnostics for initialize
  responses, tool-call responses, malformed stdout, and tool errors.
- Done: connector registries expose non-secret local credential provider,
  handle, secret-presence, and update metadata for inspection and clearing.
- Done: selected timeline inspection now separates plugin diagnostics from
  sandbox diagnostics and only shows plugin context when a plugin item is
  selected.
- Done: plugin commands now surface required connector status and inline
  authorization actions at the blocked command row.
- Done: blocked command rows expose a focused manifest reveal action for local
  plugin debugging without adding a broad debug panel.
- Done: plugin command timeline items share a stable run id across command,
  approval, runner result, failure, and runner-owned items.
- Done: connector-backed plugin approvals show non-secret connector, provider,
  handle, label, and secret-binding metadata before launch.
- Active: keep plugin UI progressive: discover, inspect, enable, authorize,
  run, debug.

## Next Order

1. Keep plugin UI progressive: discover, inspect, enable, authorize, run, debug.
2. Let model-driven tool choice mature slowly; keep deterministic routing where
   tiny local models are not reliable yet.

## Not Now

- No hosted model dependency.
- No multi-agent workflows.
- No generic document RAG or local vector database.
- No broad connector marketplace.
- No cosmetic refactor that only moves code around.
- No large UI expansion before plugin execution is real.

## Engineering Discipline

- CI is hygiene, not a milestone.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and Swift build.
- Keep commits scoped and fix CI from logs, not guesses.
- Split modules only when ownership or failure boundaries become clearer.
