# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: native `Pith` app, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the product boundary.
- Intelligence: local model by default, no required hosted model API, one active
  local model at a time.
- Setup: first use downloads and verifies a local model in-app, defaulting to
  `LFM2.5-350M`.
- Retrieval: Web Search is the active retrieval layer. Generic local document
  RAG stays deferred until the daily cowork loop is excellent.
- Extensions: plugins and connectors must be real local capabilities, not prompt
  templates or marketplace theater.
- Delivery: users install a downloadable macOS package from GitHub Releases.

## Product Contract

Pith learns from Codex and Claude Code at durable boundaries: workspace context,
bounded tools, Web Search, approvals, sandbox visibility, session continuity,
reviewable evidence, and MCP-style local connectors.

Pith also learns from Hermes Agent, but only where it supports the local macOS
cowork goal:

- Keep the core narrow and move optional capability to tools, connectors, and
  skills.
- Make execution observable, interruptible, resumable, and receipt-backed.
- Keep memory and skills bounded, curated, and progressively loaded.
- Prefer edge expansion over core bloat.
- Do not copy server-first messaging sprawl, multi-agent orchestration, or
  provider complexity before the single local cowork loop is excellent.

Pith stays intentionally different where it matters: local-first inference, no
account requirement, small-model constraints, cowork-first tasks, and a
lightweight app that downloads model weights after install.

The daily loop is:

1. Understand the workspace and request.
2. Retrieve only useful context.
3. Choose a bounded tool or connector.
4. Explain the action with a compact receipt.
5. Ask before writes or external effects.
6. Execute, show proof, remember useful state, and continue.

## Architecture Boundaries

- `apps/pith-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, routing, notifications, request
  supervision, and lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, context selection, memory
  use, plugin execution, and local execution safety.
- `crates/pith-tools`: bounded workspace tools, shell, Web Search, compaction,
  and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory meaning, ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, workspace change ledger, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records. Connector
evidence stays generic at protocol and timeline layers; service-specific detail
belongs in connector attributes and narrow presenter adapters. Refactor only
when these boundaries become clearer; do not split or merge files cosmetically.

## Current State

Active milestone: **M13 Product Quality and Identity**.

Ready foundations:

- Local model setup supports in-app download, verification, activation, pause,
  resume, cancel, recovery, local data reset, and one active model.
- First-run setup uses product-level language and hides raw GGUF details, paths,
  manifests, and hashes from the normal path.
- Workspace cowork loop has workspace-scoped tools, Web Search retrieval,
  approvals, sandbox diagnostics, bounded subprocesses, receipts, and recovery
  evidence.
- Sessions can be deleted without touching workspace files; approved writes feed
  a durable change ledger and can be previewed or reverted per session when
  files still match Pith's recorded writes.
- Connector baseline has local plugin registry, execution gates, credentials,
  retries, generic timeline evidence, memory capture, and Notion as the
  reference connector.
- macOS packaging produces an unsigned x86_64 DMG with app bundle metadata,
  unsigned install guidance, package-size checks, release copy, manifest,
  checksum, and packaged smoke proof.
- `v0.1.14` is published as the first visible ad-hoc prerelease with a manual
  acceptance receipt.
- CI is split by change area and remains the source of truth for Rust, Swift,
  package, policy, model, and release checks.

Current constraints:

- Keep M13 focused on installed-app quality, first-run clarity, UI polish,
  session safety, local data ownership, cleanup, and product identity.
- Keep Web Search as retrieval; generic local document RAG remains deferred.
- Keep connector expansion narrow until the M13 quality baseline is complete.
- Do not bundle Git, model weights, package-manager payloads, extra
  architectures, or unused runtimes.
- Release assets stay limited to the DMG, checksum, `README-FIRST.txt`, and
  release manifest.
- Visible ad-hoc prereleases require an explicit manual acceptance receipt.

## M12: Public Release

Status: complete for the first public ad-hoc prerelease.

Goal: ship a usable macOS installer from GitHub Releases.

Completed:

- Published `v0.1.14` as the current accepted ad-hoc prerelease.
- Keep the downloaded-asset acceptance receipt at
  `docs/release/manual-acceptance-receipt-v0.1.14.json` as the release gate.
- The release exposes exactly the DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- The GitHub Release page stays concise; detailed install, Gatekeeper,
  verification, and package metadata stay in `README-FIRST.txt` and the release
  manifest.

Evidence:

- A fresh unsigned install completes the daily cowork loop without hosted model
  dependency or manual model import.
- The GitHub Release exposes exactly the four public assets and final publish
  validation passes against the live release.
- Release plan, release manifest, `README-FIRST.txt`, release notes, packaged
  smoke proof, manual acceptance receipt, and downloaded-asset rehearsal
  describe the same user path.
- CI stays fast, split, strict, and understandable enough to block release
  contract drift.

## M13: Product Quality and Identity

Goal: make the shipped app feel intentional, maintainable, and worthy of daily
use before expanding the platform surface.

Finish now:

- Remove dead code, unused flows, stale scripts, and release-era scaffolding
  that no longer protects the product.
- Refactor for architecture clarity: preserve cohesive modules, merge accidental
  splits, and keep domain boundaries obvious.
- Audit the macOS app for clean human UI: no confusing internal wording, no
  awkward layout, no unnecessary controls in the primary path, and no fixed dark
  appearance.
- Strengthen Hermes-style execution quality where it fits Pith: every long task
  should be observable, cancellable, resumable when appropriate, and tied to a
  user-visible receipt.
- Keep memory and skill-like guidance bounded and curated; avoid pseudo-RAG or
  unbounded prompt stuffing.
- Design a refined app logo that is simple, distinctive, premium, and tied to
  Pith's local cowork identity.
- Ship the logo as a native macOS icon set with clean small-size readability,
  not just a large marketing image.

## M14: Connector and Skill Platform

Goal: make third-party local connectors safe and useful without building a
marketplace too early.

- Keep Notion as the reference connector until the release loop proves stable.
- Generalize connector contracts only after credentials, approvals, retries,
  proof, memory capture, and timeline evidence stay service-agnostic.
- Prove connector import, local enablement, credential storage, revocation, and
  removal before adding broad service catalogs.
- Add a small skill-like instruction layer only if it is progressively loaded,
  bounded, user-reviewable, and stored locally.
- Keep connector evidence generic in the timeline; service-specific detail must
  stay in connector attributes and narrow presenters.
- Add import/distribution only after connector secrets can be installed, used,
  revoked, and forgotten safely.
- Treat hooks as verification and automation points first, not arbitrary
  always-on automation.

## M15: Cowork Continuity

Goal: make Pith better over time without turning it into a remote server agent.

- Add a local follow-up queue for user-approved next actions.
- Add scheduled local work only after approvals, sandbox policy, and receipts
  work headlessly and fail closed.
- Add cross-session recall through bounded memory and session search before any
  local document RAG.
- Keep background work explicit, pausable, and easy to inspect from the app.
- Do not add messaging gateways, remote control channels, or multi-agent
  orchestration until the local single-agent experience is excellent.

## Later

- Developer ID signing and notarization when an Apple Developer account exists.
- Broader connector ecosystem after the local connector contract is proven.
- Native sandbox hardening beyond the current workspace and subprocess
  boundaries.
- Optional bundled Git only if bounded system Git proves insufficient for real
  packaged users.
- Local document RAG only after Web Search, workspace context, memory ranking,
  and session search are reliable in daily use.

## Guardrails

- No hosted model dependency.
- No required Pith login, hosted user session, or subscription gate.
- No generic local vector database before Web Search and workspace context are
  reliable.
- No multi-agent orchestration before the single cowork loop is excellent.
- No marketplace or remote MCP transport until local connector execution is safe
  and useful.
- No cosmetic refactor that only moves code around.
- English-only source, docs, commits, branches, and PR text.
