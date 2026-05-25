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
  bounded runners, MCP stdio sessions, permission gates, approval gates, output
  envelopes, repair hints, retry flows, and runner memory capture.
- Package proof: x86_64 app bundle, internal DMG workflow, packaged smoke
  coverage, release-state safety, native sandbox fallback, and unsigned
  distribution path with optional Developer ID upgrade later.
- Cowork loop: bounded Plan/Act/Observe execution, Web Search retrieval,
  connector-backed plugin commands, approval pause/resume, safe review-summary
  writes, and structured handoff metadata.

## Current Milestone: M7 Practical Cowork

Goal: turn the proven loop into everyday cowork flows that help users draft,
review, save, hand off, and continue real work without memorizing commands.

Current state:

- Done: natural saved-artifact requests such as handoffs, notes, summaries,
  plans, and briefs enter the same safe diff and approval path as explicit
  writes.
- Done: approved writes now end with structured continuation guidance tailored
  to saved files, notes, summaries, reviews, and handoffs.
- Done: natural connector requests can carry saved artifact references such as
  `docs/handoff.md` into a local Notion-style draft flow.
- Done: saved artifact connector inputs include bounded, workspace-safe previews
  before local drafts claim they are based on those files.
- Done: initial tool selection now has a compact planning contract before
  execution preparation.
- Done: connector planning now records candidate count, scores, selected command,
  and selection state on the timeline.
- Done: ambiguous connector candidate sets can ask the local planner for a
  bounded command-id decision instead of adding more brittle keyword routes.
- Done: Notion-style connector writes now have a separate inspection command
  that is approval-gated before any remote-write-capable runner can proceed.
- Done: natural publish/send/write connector requests route to inspection,
  while prepare/update requests stay on local drafts.
- Done: connector inspection handoffs preserve remote-write stage, target tool,
  and source artifact metadata for UI review and follow-up planning.
- Done: plugin runner output now enforces a compact remote-write contract before
  any connector timeline item can claim a completed third-party write.
- Active gap: connector write execution is still plugin-owned; Pith now provides
  the inspection and approval boundary, but not a hosted Notion writer.

M7 work order:

1. Make natural editing and saved-artifact requests feel safe, reviewable, and
   obvious.
2. Continue from approved writes into concise next-step handoffs.
3. Feed saved artifacts into connector drafts as bounded content, not only as
   path labels.
4. Harden connector updates around real cowork tasks, not demo commands.
5. Keep UI polish focused on clarity around setup, approvals, sources, and
   saved work.

M7 exit criteria:

- Users can save or update notes, handoffs, summaries, and docs without command
  syntax.
- Approved writes end with a useful continuation handoff.
- Saved artifacts used by connectors are read through the same workspace safety
  boundary as file tools.
- Ambiguous connector choices are resolved by a bounded local planner decision
  and recorded as planning evidence.
- Connector actions are practical enough for Notion-like third-party services,
  including inspect-before-remote-write.
- Connector runners cannot claim completed remote writes without target service,
  target tool, and completed-stage evidence.
- The packaged macOS app path remains green in CI.

## Next Milestone: M8 Release Candidate

- Tighten install, first-run, unsigned distribution, crash recovery, and real
  user smoke coverage.
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
