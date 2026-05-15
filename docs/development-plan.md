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

- Typed plugin, command, connector, hook, and capability registries.
- Bounded `stdio` and MCP stdio runners with sandbox diagnostics, permissions,
  approvals, retries, visible repair hints, source reveal, route-failure
  metadata, output contracts, and runner memory notes.
- Local install/remove lifecycle, plugin state diagnostics, explicit `/plugin`
  routing, install readiness preflight, inspect-before-install, and a
  source-revealable connector-backed Notion command contract.
- Honest connector credentials: current stores are `none` or `local`; Keychain
  and remote MCP transports wait until implemented. Authenticated connectors
  must declare `credentialStore: local` explicitly, and connector auth failures
  return self-contained repair metadata with command and connector panel repair
  actions, re-authorization, input-run, retry, source reveal, and RPC recovery
  actions.
- Plugin install and enable flows now surface command/connector/hook counts and
  focus the plugin manager on the most likely next repair surface.
- Plugin command failures now keep retry input plus singular connector repair
  context on the failure card, so repair and retry actions do not depend on
  neighboring items.
- Runtime RPC recovery, runner attributes, inspector summaries, and repair
  actions preserve both singular and plural connector IDs without leaking
  credential handles.

Active:

- Tighten the third-party plugin debug loop: install, inspect, enable,
  authorize, run from the panel or an explicit `/plugin` turn, understand
  failure, repair, retry. Keep diagnostics compact enough to stay inside the
  minimal app surface.
- Keep output contracts narrow and deterministic for tiny local models:
  `content`, `message`, `items`, and `memoryNotes`, including MCP
  `structuredContent` or text envelopes when explicit.
- Preserve the small app shape: progressive plugin UI, no broad marketplace,
  no admin-console sprawl.

M4 exit criteria:

- A third-party connector plugin can complete install, inspect, enable,
  authorize, run, repair, and retry without hidden terminal knowledge.
- Invalid manifests explain the exact unsupported contract and how to fix it.
- Plugin output stays deterministic enough for the small local model.

## Next Order

1. Smoke the full third-party connector loop from install through retry.
2. Close only real blockers found in that loop, then review M4 exit readiness.

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
