# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: native `Pith` app, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the boundary.
- Intelligence: local model by default, no required hosted model API, one active
  local model at a time.
- Setup: first use downloads and verifies a GGUF model in-app, defaulting to
  `LFM2.5-350M`.
- Retrieval: Web Search is the active retrieval layer. Generic local document
  RAG waits until the daily cowork loop is excellent.
- Extensions: plugins and connectors must be real local capabilities, not prompt
  templates or marketplace theater.
- Delivery: users install a downloadable macOS package from GitHub Releases.

## Product Contract

Learn from Codex and Claude Code at durable boundaries: workspace context,
bounded tools, Web Search, approvals, sandbox visibility, session continuity,
reviewable evidence, and MCP-style local connectors.

Pith should stay intentionally different where it matters: local-first
inference, no account requirement, small-model constraints, cowork-first tasks,
and a lightweight package that downloads model weights after install.

The daily loop is:

1. Understand the workspace and request.
2. Retrieve only useful context.
3. Choose a bounded tool or connector.
4. Explain the action with a compact receipt.
5. Ask before writes or external effects.
6. Execute, show proof, remember useful state, and continue.

## Architecture

- `apps/pith-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, routing, notifications, request
  supervision, and lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, local execution safety,
  context, memory use, and plugin execution.
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
belongs in connector attributes and narrow presenter adapters.

## Current Stage

Milestones 1-9 are closed. Implementation history belongs in git history, not
this roadmap.

Active milestone: **M10 Cowork Daily Driver**.

Ready foundations:

- First-use model setup, verified activation, bounded local inference, and
  runtime recovery are in place.
- Workspace-safe tools, sandbox diagnostics, bounded subprocess execution, Web
  Search retrieval, and compact receipts are in place.
- Plugin registry, connector credentials, local execution gates, approvals,
  retries, runner memory capture, and Notion as the reference connector are in
  place.
- x86_64 app packaging, internal DMG creation, unsigned install guidance,
  package-size budgets, release manifests, and packaged smoke coverage are in
  place.
- Release assets are now contract-checked as a matching DMG, checksum,
  tag-specific install guide, and manifest set with stable tag/title identity.
- Release copy requirements and validators are centralized so release notes,
  install guides, sidecar validation, and DMG staging do not drift.
- CI change classification treats release, package, and signing contract edits as
  package-impacting changes.
- Workflow validation now requires the CI change-classifier test to stay in the
  repository policy lane.
- Workflow validation now pins the release copy, sidecar, DMG, package, and
  distribution policy tests in the repository policy lane.

Latest review decisions:

- Keep Web Search as retrieval for now; do not build generic local RAG yet.
- Keep connector expansion narrow until one local cowork loop is excellent.
- Do not bundle Git, model weights, package-manager payloads, extra
  architectures, or unused runtimes.
- Package resources must exclude generated caches, bytecode, and model weights.
- Keep development planning concise; move completed detail to history, tests, or
  release notes.

## M10: Cowork Daily Driver

Goal: make one local cowork loop feel complete in the packaged app.

Build now:

- First-run path: model download, activation, workspace open, Web Search
  readiness, sandbox status, and connector readiness are visible and
  recoverable.
- Release rehearsal: keep downloaded installer verification, first launch
  guidance, and manifest contracts aligned with the actual release assets.
- Receipts: every meaningful tool decision has a compact, actionable receipt.
- Package proof: CI and packaged smoke verify the user-facing DMG path, not only
  internal scripts.
- UI restraint: admin detail stays progressive and never crowds the cowork loop.

Exit when:

- A fresh unsigned DMG install can download the default model, open a workspace,
  run a cowork turn, use Web Search, request approval, execute safely, show
  proof, and recover from runtime failure.
- Runtime readiness, app copy, package metadata, release notes, and smoke tests
  all describe the same daily-driver contract.
- The main surface remains calm, with evidence available on demand.

## M11: Connector Platform

Goal: make third-party local connectors safe and useful without building a
marketplace too early.

- Keep Notion as the reference contract.
- Add another connector only after credentials, approvals, retries, proof,
  memory capture, and timeline evidence remain generic.
- Add import/distribution only after connector secrets can be installed, used,
  revoked, and forgotten safely.
- Treat hooks as verification and automation points first, not arbitrary
  always-on automation.

## M12: Public Release

Goal: ship a usable macOS installer from GitHub Releases.

- Public assets stay limited to DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- Run one full ad-hoc prerelease rehearsal: download from GitHub Release, verify
  checksum, open DMG, handle Gatekeeper, download the default model, open a
  workspace, run a cowork turn, and inspect proof.
- Developer ID notarization is optional later; unsigned prereleases must clearly
  explain Gatekeeper manual approval.
- No bundled model weights, package-manager payloads, extra architectures, or
  unused runtimes.

## Guardrails

- No hosted model dependency.
- No required Pith login, account, hosted user session, or subscription gate.
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
