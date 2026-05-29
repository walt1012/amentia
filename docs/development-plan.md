# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: `Pith`, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the product boundary.
- Intelligence: local model by default, no required hosted model API, one active
  local model at a time.
- Identity: no Pith account, login, subscription, or hosted user session is
  required to use the app.
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
belongs in connector output attributes and narrow presenter adapters.

## Current State

Milestones 1-9 are closed. Keep implementation detail in git history, not in
this roadmap.

Working foundation:

- First-use model setup, resumable downloads, verified activation, curated model
  catalog, runtime recovery, and bounded local inference.
- Workspace-safe tools, sandbox diagnostics, bounded shell/model work, compact
  context packing, Web Search retrieval, and progressive UI surfaces.
- Plugin registry, inspect-before-install, enable/disable, connector
  credentials, bounded runners, one-shot MCP stdio commands, local execution
  gates, approvals, output envelopes, retries, and runner memory capture.
- Notion create-page as the reference cowork connector: draft, inspect,
  approval, publish, proof, retry, memory capture, and packaged smoke.
- Release proof for x86_64 app bundle, internal DMG, robust mounted-DMG smoke,
  unsigned install guidance, release manifest, remote catalog audit, public
  distribution metadata validation, shared package contract, locked package
  size budgets, packaged-smoke contract reuse, shared signing-mode policy, and
  a single user-facing CI installer artifact whose upload contract and release
  manifest share the same expected asset names, also reflected in generated
  release notes and DMG install guidance. User-facing package budget copy now
  describes installer artifacts, not internal zip implementation detail.
  Package metadata, release manifests, generated install copy, app trust copy,
  runtime readiness metrics, packaged smoke, and app status summaries now state
  the no-account product boundary and the local execution safety modes. The app
  now presents compact context receipts from stable timeline attributes:
  workspace snippets, Web Search source depth, local action mode, approval
  policy, account requirement, tool boundary, memory context ranking, source
  reason, and compaction decisions. The timeline also shows a short receipt
  summary on each relevant card, so long cowork sessions can be scanned without
  opening the inspector. The app now sends the selected local execution safety
  mode to `turn/start`; runtime behavior distinguishes read-only exploration,
  ask-before-change, and approved workspace execution.

## Alignment Review

Pith is aligned with Codex and Claude Code on the durable foundations: local
workspace context, bounded tools, approvals, sandbox visibility, Web Search,
MCP-style extension points, session continuity, hooks metadata, reviewable
evidence, and release verification.

The product should not copy their full shape. Pith remains a cowork app, so the
next work is to make one local cowork loop excellent rather than adding cloud
tasks, broad marketplace surfaces, generic local RAG, or multi-agent
orchestration too early.

Active gaps found in the review:

- Local execution safety modes now have release/package/runtime/app contracts,
  compact action/context receipts, and a Settings entry that changes real
  runtime write/shell behavior.
- Context management is present and visible. The remaining product gap is to
  make receipts directly actionable: jump to sources, open changed files, and
  retry blocked actions from the calm cowork surface.
- Connector execution is useful but still Notion-led. Keep service-specific
  logic narrow and make the protocol generic before adding another connector.
- Hooks and subagents should stay scoped to verification and automation after
  the single cowork loop is stable.
- Release packaging is strong, but the first public tag still needs a complete
  ad-hoc prerelease rehearsal from GitHub Release download to first launch.

## M10: Cowork Daily Driver

Goal: make Pith feel like a real cowork app before adding another integration.

Build toward:

- Explicit simple local execution modes: explore, ask-before-change, and
  approved workspace execution.
- Context receipts: workspace snippets, Web Search sources, memory notes, and
  compaction decisions shown in one calm review surface.
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
- The app can explain every tool decision with a compact context receipt and a
  clear local execution mode.
- Release manifest exposes the same daily-driver, sandbox, model-delivery, and
  package-size facts that CI validates.
- The main app surface stays progressive and calm; admin details do not crowd
  the cowork loop.

## M11: Connector Platform

Goal: make third-party local connectors safe and useful without building a
marketplace too early.

- Keep the Notion workflow as the reference contract.
- Add connector execution only when local credentials, execution gates, proof,
  retry, memory, and timeline evidence remain generic.
- Add third-party connector import only after the generic connector contract can
  run, approve, retry, prove, and forget secrets without service-specific UI.
- Prefer one excellent connector path over many shallow examples.
- Avoid service-specific logic in broad runtime or app presenters.
- Treat hooks as connector/runtime verification points first, not arbitrary
  always-on automation.

## M12: Public Release

Goal: ship a usable macOS installer from GitHub Releases.

- Public assets stay limited to DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- Run one full ad-hoc prerelease rehearsal before the first public tag:
  download from GitHub Release, verify checksum, open DMG, handle Gatekeeper,
  download the default model, open a workspace, run a cowork turn, and inspect
  proof.
- Developer ID notarization is optional later; ad-hoc unsigned prereleases must
  clearly explain Gatekeeper manual approval.
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
