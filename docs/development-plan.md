# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: `Pith`, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the product boundary.
- Intelligence: local model by default, no required hosted model API, one active
  local model at a time.
- First use: in-app verified GGUF download, defaulting to `LFM2.5-350M`.
- Retrieval: Web Search is the active retrieval layer; no generic local document
  RAG until the cowork loop is excellent.
- Plugins: real local capabilities and connectors, not prompt templates.
- Delivery: users install a downloadable macOS app package; CI proves the
  packaged app path.

## Product Shape

Learn from Codex and Claude Code at the durable boundaries: workspace context,
bounded tools, Web Search, approvals, sandbox status, session continuity,
reviewable evidence, and MCP-style local connectors.

Pith should differ intentionally: local-first inference, cowork-first tasks,
small-model constraints, no hosted model dependency, no marketplace shell before
one connector workflow is excellent, and no heavyweight distribution payloads.

The core loop is simple: understand context, choose a bounded tool, show
evidence, ask before writes, preserve useful memory, and recover cleanly.

## Architecture

- `apps/pith-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, routing, notifications, request
  supervision, and lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, permissions, context,
  memory use, and plugin execution.
- `crates/pith-tools`: bounded workspace tools, shell, Web Search, compaction,
  and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory meaning, ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records. Connector
evidence stays generic at protocol and timeline layers; service-specific detail
belongs in connector output attributes and narrow presenter adapters.

## Current State

Milestones 1-9 are closed. Keep implementation detail in git history, not in
this roadmap.

Working foundation:

- First-use model setup, resumable downloads, verified activation, curated model
  catalog, runtime recovery, and bounded local inference.
- Workspace-safe tools, sandbox diagnostics, bounded shell/model work, compact
  context packing, Web Search retrieval, and progressive UI surfaces.
- Plugin registry, inspect-before-install, enable/disable, connector auth,
  bounded runners, one-shot MCP stdio commands, permission gates, approvals,
  output envelopes, retries, and runner memory capture.
- Notion create-page as the reference cowork connector: draft, inspect,
  approval, publish, proof, retry, memory capture, and packaged smoke.
- Release proof for x86_64 app bundle, internal DMG, robust mounted-DMG smoke,
  unsigned install guidance, release manifest, remote catalog audit, public
  distribution metadata validation, shared package contract, locked package
  size budgets, packaged-smoke contract reuse, shared signing-mode policy, and
  a single user-facing CI installer artifact with an exact upload contract.

## M10: Cowork Daily Driver

Goal: make Pith feel like a real cowork app before adding another integration.

Build toward:

- One clear daily-driver stage shared by runtime readiness, app UI, smoke tests,
  package metadata, release notes, and release checks.
- Boring first-run setup: model download, activation, workspace open, Web
  Search readiness, sandbox status, and connector readiness are visible and
  recoverable.
- Practical cowork loop without developer-only setup: ask, retrieve, inspect,
  approve, execute, show proof, remember, and continue.
- Small package as a product boundary: ship app, local runtime, llama backend,
  metadata, and bundled connector definitions only; download model weights and
  optional connector data after install.
- Fast remote CI that proves policy, model metadata, runtime smoke, Swift app,
  shared package contract, direct package manifest validation, packaged app,
  DMG path, exact installer asset sets, and release metadata without exposing
  internal build artifacts as user-facing downloads.

Exit when:

- Packaged app smoke proves first-use setup, local model readiness, Web Search,
  workspace approval, sandbox status, connector proof, runtime recovery, and
  unsigned DMG install guidance together.
- Release manifest exposes the same daily-driver, sandbox, model-delivery, and
  package-size facts that CI validates.
- The main app surface stays progressive and calm; admin details do not crowd
  the cowork loop.

## M11: Connector Platform

Goal: make third-party local connectors safe and useful without building a
marketplace too early.

- Keep the Notion workflow as the reference contract.
- Add connector execution only when auth, permission, proof, retry, memory, and
  timeline evidence remain generic.
- Prefer one excellent connector path over many shallow examples.
- Avoid service-specific logic in broad runtime or app presenters.

## M12: Public Release

Goal: ship a usable macOS installer from GitHub Releases.

- Public assets stay limited to DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- Developer ID notarization is optional later; ad-hoc unsigned prereleases must
  clearly explain Gatekeeper manual approval.
- No bundled model weights, package-manager payloads, extra architectures, or
  unused runtimes.

## Guardrails

- No hosted model dependency.
- No generic local vector database before Web Search and workspace context are
  reliable.
- No multi-agent orchestration before the single cowork loop is excellent.
- No marketplace or remote MCP transport until local connector execution is safe
  and useful.
- No bundled Git runtime until bounded system Git proves insufficient for real
  packaged users.
- No cosmetic refactor that only moves code around.
- English-only source, docs, commits, branches, and PR text.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and macOS app packaging.
