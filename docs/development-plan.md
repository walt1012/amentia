# Pith Development Plan

## Product Direction

- Pith is a small, strong, local-first macOS cowork agent for real daily work.
- Target: `Pith`, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the product boundary.
- Intelligence: local model by default; no required hosted model API.
- First use: in-app verified GGUF download, defaulting to `LFM2.5-350M`.
- Runtime: one active local model at a time.
- Retrieval: Web Search is the active retrieval layer; no generic local
  document RAG until the cowork loop is excellent.
- Plugins: real local capabilities and connectors, not prompt templates.
- Git: use bounded system Git helpers for workspace review flows; do not embed
  a Git engine unless the packaged app cannot meet real user needs without it.
- Delivery: users install a downloadable macOS app package; CI proves the app
  path, but the app experience is the product.

## Architecture Map

- `apps/pith-macos`: native UI, setup, approvals, timeline, model manager,
  inspector, and app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, request routing, request
  supervision, notifications, and runtime lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, permissions, context,
  memory usage, and plugin execution.
- `crates/pith-tools`: bounded workspace tools, shell, Web Search, compaction,
  and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory semantics, note ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records.

## Closed Foundation

Milestones 1-6 are closed. Keep details in git history, not in this plan.

Closed capabilities:

- First-use model setup, resumable downloads, verified activation, runtime
  recovery, bounded shell/model work, workspace-safe tools, Web Search, sandbox
  diagnostics, compact context packing, and progressive inspector surfaces.
- Plugin registry, inspect-before-install, enable/disable, connector auth,
  bounded runners, one-shot MCP stdio commands, permission gates, approval
  gates, output envelopes, repair hints, retry flows, and runner memory capture.
- Package proof: x86_64 app bundle, internal DMG workflow, packaged smoke
  coverage, release-state safety, native sandbox fallback, and unsigned
  distribution path with optional Developer ID upgrade later.
- Cowork loop: bounded Plan/Act/Observe execution, Web Search retrieval,
  connector-backed plugin commands, approval pause/resume, safe review-summary
  writes, and structured handoff metadata.

## Closed Milestone: M7 Practical Cowork

Goal: turn the proven loop into everyday cowork flows that help users draft,
review, save, hand off, and continue real work without memorizing commands.

Closed state:

- Natural saved-artifact requests, approved writes, and continuation handoffs
  share the same safe diff and approval path.
- Connector drafts can consume saved artifacts through bounded workspace-safe
  previews, planning evidence, inspection gates, and remote-write proof.
- Web Search source grounding is honest about search-result attribution versus
  stronger page-fetch or snapshot verification.
- Timeline cards and inspector summaries expose source depth, connector write
  status, setup progress, and first-use actions without raw attribute hunting.
- Release and first-use copy now carries the non-developer path through model
  download, workspace opening, and the first cowork request.

Remaining gap:

- Connector write execution is still plugin-owned; Pith provides inspection,
  approval, and proof boundaries, but not a hosted Notion writer.

## Current Milestone: M8 Release Candidate

Goal: prove Pith works as a real downloadable macOS app for non-developer users.

Current state:

- Done: first-use UI copy now frames setup completion as starting a cowork
  session, not a coding-only prompt.
- Done: packaged smoke naming now follows the same first cowork request path.
- Done: packaged smoke now kills and restarts the runtime after the first
  cowork flow, then verifies model, workspace, thread, and readiness recovery.
- Done: packaged smoke now proves a local workspace write approval before
  plugin connector approval, then verifies that approved work survives runtime
  recovery.

M8 work order:

- Tighten install, first-run, unsigned distribution, crash recovery, and real
  user smoke coverage.
- Prove the non-developer path: install the DMG, launch Pith, download one
  verified model, open a workspace, use Web Search, approve work, and recover
  from a runtime failure.
- Add optional page fetch or source snapshots if M7 Web Search usage needs
  stronger citation behavior than search-result attribution.
- Promote MCP from one-shot command execution to persistent local sessions only
  if third-party connector workflows need dynamic tool discovery or shared
  session state.

## Guardrails

- No hosted model dependency.
- No generic local vector database before Web Search and workspace context are
  reliable.
- No multi-agent orchestration before the single cowork loop is excellent.
- No marketplace or remote MCP transport until local connector execution is
  safe and useful.
- No bundled Git runtime until bounded system Git proves insufficient for real
  packaged users.
- No cosmetic refactor that only moves code around.
- English-only source, docs, commits, branches, and PR text.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and macOS app packaging.
- Split modules only when ownership or failure boundaries become clearer.
