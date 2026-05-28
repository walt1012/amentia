# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: `Pith`, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the boundary.
- Intelligence: local model by default, no required hosted model API, one
  active local model at a time.
- First use: in-app verified GGUF download, defaulting to `LFM2.5-350M`.
- Retrieval: Web Search is the active retrieval layer; no generic local
  document RAG until the cowork loop is excellent.
- Plugins: real local capabilities and connectors, not prompt templates.
- Delivery: users install a downloadable macOS app package; CI proves the
  packaged app path.

## Product Shape

Learn from Codex and Claude Code at the durable boundaries: workspace context,
bounded file and shell tools, Web Search retrieval, reviewable diffs,
approvals, sandbox status, session continuity, and MCP-style local connectors.

Pith should differ intentionally: local-first inference, cowork-first tasks,
small-model constraints, no required hosted model API, and no marketplace shell
before one connector workflow is excellent.

Do not copy heavyweight surfaces just because Codex or Claude Code have them.
Pith should adapt the useful loop: understand context, choose a bounded tool,
show evidence, ask before writes, preserve memory, and recover cleanly.

## Architecture Boundaries

- `apps/pith-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, routing, notifications,
  request supervision, and lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, permissions, context,
  memory use, and plugin execution.
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

Connector evidence should stay generic at the protocol and timeline layers.
Service-specific details belong in connector output attributes and narrow
presenter adapters, not in broad app or runtime control flow.

## Foundation State

Milestones 1-8 are closed. Keep implementation detail in git history, not in
this roadmap.

Durable foundation:

- First-use model setup, resumable downloads, verified activation, curated
  model catalog, runtime recovery, and bounded local inference.
- Workspace-safe tools, sandbox diagnostics, bounded shell/model work, compact
  context packing, Web Search retrieval, and progressive UI surfaces.
- Plugin registry, inspect-before-install, enable/disable, connector auth,
  bounded runners, one-shot MCP stdio commands, permission gates, approvals,
  output envelopes, retries, and runner memory capture.
- Release proof: x86_64 app bundle, internal DMG workflow, mounted-DMG smoke,
  release-state safety, native sandbox fallback disclosure, and unsigned
  distribution path with optional Developer ID upgrade later.

Keep these gates healthy:

- Packaged smoke proves first-use model metadata, workspace opening, Web Search
  evidence, approval-gated writes, connector smoke, runtime recovery, and DMG
  install path together.
- Release builds run remote model catalog audit before publishing.
- Release assets stay limited to DMG, checksum, `README-FIRST.txt`, and release
  manifest.

## Recently Closed: M9 Cowork Connectors

Goal: make Pith useful for real non-code cowork tasks without turning the app
into a marketplace shell or a generic RAG product.

Exit state:

- Notion create-page is the reference connector workflow: draft, inspect,
  approval, publish, proof, retry, memory capture, command coverage, and
  packaged smoke.
- Notion publish accepts page IDs and copied page URLs, validates malformed
  targets locally, accepts forgiving parent aliases, preserves lightweight page
  structure from forgiving text input, reports title and content truncation,
  requires trusted remote proof, and exposes proof attributes in the timeline.
- Notion API-key authorization requires a local secret and explains token,
  scope, internal integration setup, parent-page sharing, local token storage,
  and non-OAuth setup before publish.
- Published Notion memory captures proof URL, parent, title/body truncation, and
  block count so later cowork turns can continue from the real remote result.
- Connector workflow contracts are reusable across manifest workflow,
  command bindings, output envelopes, proof attributes, bounded step budget,
  and CI checks.
- Connector proof evidence now has a generic timeline path for proof ID, URL,
  title, action title, workflow state, remote write state, retry, and service
  adapters.
- Web Search remains the retrieval layer. Saved artifacts and memory are
  context aids, not a local document RAG product.

Review status:

- Codex/Claude alignment is strongest at the durable boundaries: workspace
  scope, approval-gated writes, bounded tools, Web Search, sandbox visibility,
  session memory, and MCP-style connector execution.
- Pith's deliberate difference remains correct: cowork-first local work with a
  small model, not a hosted coding agent clone.
- The main daily-driver risk is presentation coupling. Timeline proof and evidence
  rendering must be kept generic before adding another connector.

M9 exit criteria:

- Notion setup explains local integration tokens, required scopes, shared
  parent pages, and the current non-OAuth posture.
- Notion create-page is boringly reliable through prepare, inspect, approval,
  publish, proof, retry, and memory capture.
- The app keeps connector UI progressive: show readiness, repair, workflow,
  and proof only when needed.
- Connector proof presentation handles generic workflow and remote-write
  evidence without baking new service logic into broad timeline presenters.
- Packaged smoke proves connector path, Web Search evidence, workspace
  approval, runtime recovery, and unsigned DMG install path together.

## Current Milestone: M10 Cowork Daily Driver

Goal: make Pith feel like a real cowork app before adding another integration.

Focus:

- Keep the first-run local model path boringly reliable.
- Keep one clear daily-driver stage shared by runtime readiness, app presenters,
  smoke tests, package metadata, release notes, and release checks.
- Keep Web Search as the active retrieval layer.
- Treat missing Web Search permission as a daily-driver setup gap, not a hidden
  degraded state.
- Keep workspace tools bounded, cancellable, and sandbox-visible.
- Keep connector execution generic, approval-gated, and proof-oriented.
- Keep the macOS app progressive and small; avoid admin panels growing into the
  main cowork loop.
- Keep release packaging and CI fast enough to trust for daily iteration.

Exit criteria:

- Packaged app smoke proves first-use setup, local model readiness, Web Search,
  workspace approval, sandbox status, connector proof, runtime recovery, and
  unsigned DMG install guidance together.
- The app can complete a practical cowork loop without developer-only setup.
- Development plan stays concise and tracks direction, not commit history.

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
