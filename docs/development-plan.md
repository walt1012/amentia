# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work. It
should feel native, focused, recoverable, and capable without becoming a
terminal skin, coding-only assistant, hosted-model frontend, generic chatbot,
or feature zoo.

## Non-Negotiables

- Product: `Pith`, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one useful workflow, not the product
  boundary.
- Intelligence: local model by default; no required external model API.
- First use: in-app model download, defaulting to `LFM2.5-350M`.
- Runtime: one active local model at a time.
- Plugins: real local capabilities, not prompt templates.
- Retrieval: default-enabled Web Search is the active retrieval layer; no
  generic local document RAG yet.
- Repository: English-only source, docs, commits, branches, and PR text.
- Foundation: free, open source, native, and lightweight.

## Product Standard

- A normal user can install the app, launch it, download a model, open a
  workspace, send a request, review results, and recover from common failures
  without using a terminal.
- Every core action has clear in-app state: ready, running, blocked, failed,
  cancelled, or recovered.
- Runtime, model, plugin, web search, sandbox, and packaging work counts as
  done only when it holds together in the packaged macOS app.
- CI proves the app path, but the app experience is the product.

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

## Codex and Claude Alignment

Matched foundation:

- Native local execution shape: workspace tools, shell execution, approvals,
  cancellation, sandbox diagnostics, and recovery.
- Local-first model path: first-use download, verified activation, bounded
  inference, and no required hosted model API.
- Retrieval baseline: Web Search is available as a first-class tool with
  bounded execution and explicit network permission.
- Extensibility baseline: plugin manifests, local install/remove,
  enable/disable, permission gates, connector credentials, bounded runners,
  and MCP stdio session handling.
- Product delivery path: x86_64 app bundle, DMG workflow, release-state safety,
  and packaged smoke coverage.

Not yet aligned:

- Agent loop: Pith still routes most turns to one prepared action instead of a
  model-guided Plan/Act/Observe loop.
- Tool contract: timeline items now share one local tool schema, but execution
  still runs through separate reducers instead of one loop-owned dispatcher.
- Real connectors: the Notion connector is still a safe dry-run proof, not a
  real external-service workflow.
- Source-grounded answers: Web Search results are visible in the timeline, but
  final answers need stronger source attribution and citation-ready wording.
- Workspace change workflow: Pith has review-diff support, but it does not yet
  have a general cowork flow for reviewing, applying, syncing, and explaining
  changes across local files and connectors.

Do not copy blindly:

- No hosted model dependency.
- No multi-agent orchestration before the single agent loop is excellent.
- No generic local vector database before Web Search and workspace context are
  reliable.
- No marketplace or remote MCP transport until local connector execution is
  safe and useful.
- No cosmetic refactor that only moves code around.

## Closed Foundation

Milestones 1-5 are closed. Keep their details in git history, not in this plan.

Closed capabilities:

- Local model setup, resumable downloads, verified single-model activation,
  runtime recovery, bounded shell/model work, workspace-safe tools, Web Search,
  sandbox diagnostics, compact context packing, and progressive inspector
  surfaces.
- Plugin registry, inspect-before-install, enable/disable, connector auth,
  bounded `stdio` runners, MCP stdio sessions, permission gates, approval
  gates, output envelopes, repair hints, retry flows, and runner memory
  capture.
- Timeline trust boundaries for approvals, plugin runs, connector blockers,
  source reveal, refresh recovery, runtime status, and credential-safe
  metadata.
- Daily-driver package proof: CI validates the app bundle, bundled runtime
  protocol, first-use model metadata, app support directories, workspace
  bootstrap/search, deterministic first request, Web Search, bundled MCP plugin
  command execution, connector authorization, approval recovery, launch smoke
  coverage, internal DMG shape, release-state safety, native sandbox fallback,
  and Developer ID upgrade path.

## Current Milestone: M6 Cowork Agent Loop and Real Connectors

M6 should make Pith feel like a real local cowork partner instead of a polished
single-action assistant. The implementation target is one compact, auditable
Plan/Act/Observe loop that can call tools, observe results, pause for approval,
resume, cancel, and produce a source-grounded final answer across files, web
results, plugins, and connectors.

Current review snapshot:

- Runtime supervision, model validation, sandbox boundaries, package delivery,
  workspace path safety, Web Search, and memory ranking are structurally sound;
  no broad rewrite is justified there.
- The main M6 blocker is architectural, not cosmetic: normal turns still use
  `compatibilitySingleAction`, and the Notion connector still proves metadata
  rather than a real external-service path.
- The macOS app is now a composition root with focused feature files. Keep
  splitting only when ownership or failure boundaries improve; do not split just
  to shrink line counts.
- The large plugin execution test file should be split by behavior after the
  loop and connector proof, so tests remain readable without interrupting the
  product-critical path.
- Do not broaden into generic RAG, multi-agent orchestration, marketplace work,
  or cosmetic module splitting during M6.

Implementation sequence:

1. Replace the compatibility coordinator with a request-scoped loop dispatcher
   that can execute up to three bounded steps, feed observations back into the
   next decision, stop on cancellation, and resume approvals in the same step.
2. Move tools into the dispatcher in this order: workspace read/search, Web
   Search, shell/write approval, plugin command, connector command, and
   workspace-change review/apply.
3. Prove cowork value with source-attributed Web Search answers, one real
   credential-safe Notion MCP connector path, and a minimal review/apply/handoff
   workflow for local workspace changes.

Active status:

- Done: single-action turns carry stable loop, step, local-tool, tool-call
  status, and Web Search source metadata.
- Done: approval resume preserves the same agent step metadata, so approved or
  denied tool work does not become detached timeline output.
- Done: workspace, shell-after-approval, Web Search, plugin, and connector
  timeline items share local tool schema attributes while keeping legacy UI
  fields.
- Done: turn execution now delegates each prepared action through a step
  dispatcher with explicit continue/stop control, so the next loop increment can
  iterate tool steps without growing the top-level turn reducer.
- Not done: the normal turn path still executes one prepared action, not a real
  Plan/Act/Observe loop.
- Not done: the bundled Notion connector is still a dry-run MCP server.

M6 exit gate:

- One user request can run at least three bounded agent steps across two tool
  types and then produce a final answer with visible observations.
- Cancelling the turn stops pending model/tool work and leaves a coherent
  timeline state.
- Approval-paused tools resume the same agent step after approval without
  losing workspace, memory, or connector context.
- Web Search final answers include source attribution.
- One real connector command works through the same loop and remains sandboxed,
  bounded, and credential-safe.

## Next Milestone: M7 Practical Cowork Workflows

M7 starts only after M6 exits. It should make Pith useful for day-to-day
cowork tasks rather than adding broad platform features.

- Workspace-aware editing loop with safe diffs and reviewable writes for notes,
  docs, config, and code.
- Practical handoff flows: summarize work, draft next actions, prepare connector
  updates, and optionally package local changes for Git-backed workspaces.
- Better context compaction for long sessions and small local models.
- Connector hardening based on the M6 real connector proof.
- Packaged app UX polish only where it directly helps daily cowork work.

## Engineering Discipline

- CI is hygiene, not a milestone.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and macOS app packaging.
- Prefer parallel jobs, pinned external inputs, and narrow caches over weaker
  checks.
- Keep commits scoped and fix CI from logs, not guesses.
- Split modules only when ownership or failure boundaries become clearer.
