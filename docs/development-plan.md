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

M4 turns plugins into bounded local capabilities. It should prove the third-party
plugin loop without growing into a marketplace, admin console, or hosted
integration layer.

Done:

- Registry and lifecycle: typed plugin, command, connector, hook, capability
  registries; local install/remove; inspect-before-install; compact state
  diagnostics; stable identifiers; precise manifest contract repair hints.
- Execution: bounded `stdio` and MCP stdio runners with sandbox diagnostics,
  permission gates, approval gating, cancellation-safe retries, deterministic
  input/output contracts, model-independent command planning, invalid-envelope
  repair hints, MCP content diagnostics, and runner memory notes plus MCP result
  source attribution.
- Connectors: honest `none` or `local` credential stores, connector auth and
  clear flows, removal cleanup, source-revealable Notion-style connector
  contract, and no hidden Keychain or remote MCP claims.
- Recovery loop: plugin inspect/install, enable/remove, route, permission,
  approval, connector, runner, completion-stage, and RPC failures carry compact
  repair metadata plus retry input context; the third-party connector smoke path
  covers inspect, install, enable, pre-auth blockers, authorize, approve, fail,
  repair, retry, and refresh-after-fix without restarting the app.
- Timeline trust: approval, run, blocked, failed, and resolved cards preserve
  command IDs, plugin IDs, connector IDs, install blockers, input context,
  source paths where available, runtime status, and recovery hints without
  leaking credential handles; inspector summaries surface blockers, and recovery
  actions stay limited to issue cards or blocked rows; plugin lifecycle,
  connector, source reveal, and refresh operations update visible status.

Active Focus:

- Finish M4 by reviewing the third-party connector smoke path end to end.
- Close only real install, auth, run, repair, retry, refresh, or UI recovery blockers.
- Keep plugin UI progressive; avoid marketplace or admin-console sprawl.

M4 Exit Gate:

- A third-party connector plugin can complete install, inspect, enable,
  authorize, run, repair, and retry without hidden terminal knowledge.
- Invalid manifests explain the exact unsupported contract and how to fix it.
- Fixing a local manifest or runner can be followed by in-app refresh.
- Plugin output stays deterministic enough for the small local model.

## Not Now

- No hosted model dependency.
- No multi-agent workflows.
- No generic document RAG or local vector database.
- No broad connector marketplace.
- No manifest-declared Keychain credentials until native Keychain storage exists.
- No remote MCP transport until bounded local execution supports it.
- No cosmetic refactor that only moves code around.
- No large UI expansion before plugin execution is real.

## Engineering Discipline

- CI is hygiene, not a milestone.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and Swift build.
- Keep commits scoped and fix CI from logs, not guesses.
- Split modules only when ownership or failure boundaries become clearer.
