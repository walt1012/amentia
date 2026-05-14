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

M4 turns plugins into bounded local capabilities. The platform now has typed
registries, bounded `stdio` and MCP runners, permission gates, connector
authorization, sandbox diagnostics, runner-owned timeline output, runner memory
notes, local install/remove lifecycle safety, and structured repair hints for
registry, readiness, input-contract, approval-time, runner setup, MCP protocol,
process exit, output-contract failures, and explicit turn-to-plugin command
routing. The bundled Notion connector now carries a connector-backed command
contract so M4 can exercise real authorize/run/repair behavior without pretending
to include a hosted integration, and it declares the current `local` credential
store rather than promising native Keychain storage before that exists. Plugin
state refresh failures are surfaced in the plugin dashboard instead of being
silently treated as an empty catalog. Failed plugin command cards can retry the
same command input after the user repairs the runner, connector, or MCP issue.
MCP diagnostics call out stdout JSON-RPC framing mistakes and show whether
content or structured content was parsed as a Pith output envelope. Connector
secret environment bindings include a per-run index to avoid normalized
connector id collisions. User-facing timeline metadata shows connector service,
store, label, and binding status without exposing internal credential handles.

Current M4 focus:

- Tighten the third-party plugin debug loop: install, inspect, enable,
  authorize, run from the panel or an explicit `/plugin` turn, understand
  failure, repair, retry. Keep diagnostics compact enough to stay inside the
  minimal app surface.
- Keep output contracts narrow and deterministic for tiny local models:
  `content`, `message`, `items`, and `memoryNotes`, including MCP
  `structuredContent` or text envelopes when explicit.
- Preserve the small app shape: progressive plugin UI, no broad marketplace,
  no admin-console sprawl.

## Next Order

1. Exercise the full third-party plugin path with connector-backed examples and
   fix only the gaps that block install, authorize, run, repair, retry.
2. Review M4 exit readiness before starting broader connector/plugin discovery.

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
