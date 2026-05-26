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

- Installer path is proven from mounted DMG, not only from raw app bundles.
- Release assets include DMG, basename-only SHA-256 checksum,
  `README-FIRST.txt`, and a machine-readable release manifest.
- Release sidecars validate first-use user guidance, model delivery mode,
  platform target, signing mode, source commit, checksum sidecar integrity, and
  install-guide integrity.
- The app reads packaged distribution metadata and presents the same
  Developer ID or ad-hoc Gatekeeper trust path and source commit that the DMG
  install guide and release manifest use.
- Packaged smoke covers first-use model metadata, app-owned model activation,
  workspace opening, Web Search source snapshots, approval-gated writes,
  runtime restart, and recovery of model, workspace, thread, and readiness
  state.
- Web Search now preserves a bounded search-result snapshot with a stable hash
  while clearly reporting that page contents were not fetched.
- The model manager now surfaces explicit first-run recovery guidance for
  paused downloads, runtime relaunch, downloaded-but-inactive models, and
  partial-file cleanup.
- Package validation checks that the compiled app executable still contains the
  first-run recovery and Gatekeeper trust copy required by the release path.
- Package gates reject bundled GGUF weights, unsafe zip entries, path
  traversal, symlink leakage, and non-`x86_64` executable outputs.
- CI structure is change-aware and guarded by workflow policy checks, but CI
  details stay in `docs/development-environment.md` rather than this roadmap.

M8 work order:

- Close M8 only when the release workflow, package metadata, source commit
  traceability, first-run model path, workspace path, Web Search evidence,
  approval-gated write path, connector smoke, and runtime recovery remain green
  together in CI.
- Keep packaged smoke focused on real user journeys; add UI automation for
  visible recovery copy only after the app has a stable UI automation harness.
- Defer page fetch and page-content snapshots until cowork tasks require
  evidence beyond search-result snapshots.
- Keep MCP one-shot until a real connector workflow proves persistent local
  sessions are necessary.

## Next Milestone: M9 Cowork Connectors

Goal: make Pith useful for real non-code cowork tasks without turning the app
into a marketplace shell or a generic RAG product.

Work order:

- Prove one or two real connector workflows end to end, from user intent to
  inspected draft, approval, execution proof, retry, and recovery.
- Narrow connector permissions around explicit user-visible actions rather than
  broad plugin trust.
- Keep Web Search as the active retrieval layer and add stronger page evidence
  only when connector tasks need it.
- Keep saved artifacts and memory as lightweight context aids, not a local
  document RAG system.
- Preserve the small native UI: progressive disclosure, no always-open admin
  panels, and no feature surface without a daily cowork use case.
- Refactor only at ownership boundaries exposed by connector work; do not split
  modules simply to reduce line counts.

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
