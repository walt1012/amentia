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
  RAG is deferred until the daily cowork loop is excellent.
- Extensions: plugins and connectors must be real local capabilities, not prompt
  templates or marketplace theater.
- Delivery: users install a downloadable macOS package from GitHub Releases.

## Product Contract

Learn from Codex and Claude Code at durable boundaries: workspace context,
bounded tools, Web Search, approvals, sandbox visibility, session continuity,
reviewable evidence, and MCP-style local connectors.

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

## Current State

Active milestone: **M12 Public Release**.

Closed foundations:

- Local model setup: first-use guidance, verified downloads, activation,
  bounded inference, restart recovery, and one active local model.
- Installed local model files can be re-verified and selected again after app
  updates or state loss; the UI must not present verification recovery as a
  fresh download requirement.
- Workspace loop: workspace-safe tools, sandbox diagnostics, bounded
  subprocesses, Web Search, approvals, compact receipts, and recovery evidence.
- Connector baseline: plugin registry, local execution gates, credentials,
  retries, memory capture, and Notion as the reference connector.
- macOS delivery: x86_64 DMG packaging, unsigned install guidance, size budgets,
  manifests, release copy, package metadata, transient DMG retry, and packaged
  smoke proof.
- Release automation: split CI lanes, dry-run DMG packaging, downloaded-asset
  rehearsal, release-plan guards, evidence-before-visibility ordering, final
  publish validation, and manual-acceptance gates.

Current decisions:

- Keep Web Search as retrieval for now; do not build generic local RAG yet.
- Keep connector expansion narrow until the local cowork loop ships cleanly.
- Do not start new product surfaces before the first release candidate is
  accepted on the real install path.
- Avoid broad refactors before M12 unless they remove a correctness, release,
  or safety risk.
- Do not bundle Git, model weights, package-manager payloads, extra
  architectures, or unused runtimes.
- Public release assets stay limited to DMG, checksum, `README-FIRST.txt`, and
  release manifest.
- Installer verification copy must keep those four assets in one download
  folder so checksum validation is obvious for normal users.
- Manual release dispatch defaults to a dry-run draft prerelease; stable
  visibility must be an explicit later decision.
- Visible ad-hoc prerelease publishing requires a repository-scoped validated
  manual acceptance receipt, not placeholders.
- Manual acceptance receipt templates must be derived from downloaded assets
  whose DMG bytes match the manifest and checksum sidecar.
- Remote CI is the source of truth for Rust formatting, tests, policy checks,
  model catalog validation, and macOS packaging.

## M12: Public Release

Goal: ship a usable macOS installer from GitHub Releases.

Build now:

- Treat `v0.1.2` as the current dry-run candidate; its remote CI dry-run passed
  and produced the expected four public-style assets.
- Keep installer guidance explicit for both Developer ID and ad-hoc Gatekeeper
  paths.
- Complete one manual first-launch acceptance on a fresh Mac: download from
  GitHub Release, verify checksum, open DMG, handle Gatekeeper, download the
  default model, open a workspace, run a cowork turn, use Web Search, approve a
  bounded action, inspect proof, generate the manual acceptance receipt from
  downloaded assets, fill it, and validate it.
- Keep normal-path UI language product-level: show session, local engine,
  connector, local model, workspace search, and first prompt concepts instead
  of internal runtime/thread/file details.
- Keep the first-run and daily-work UI calm, spacious, and humane: progressive
  disclosure before dense panels, plain-language actions before diagnostics,
  and soft native surfaces instead of noisy debug-style lists.
- Keep high-frequency surfaces aligned to that standard: cowork timeline,
  details pane, local engine, workspace search, connectors, and notes should
  read as product UI before advanced diagnostics appear.
- Record the accepted dry-run artifact and validated fresh-Mac receipt before
  changing release visibility.
- Publish the first ad-hoc prerelease only after manual acceptance passes.
- Require `manual_acceptance_confirmed=true` before any visible ad-hoc
  prerelease can be published, with `manual_acceptance_evidence` pointing to the
  repository-scoped validated receipt; final publish validation independently
  checks the receipt URL and release notes disclosure.
- Keep unsigned/ad-hoc copy explicit because Developer ID notarization is paid
  and optional later.

Exit when:

- The GitHub Release exposes exactly the four public assets and final publish
  validation passes against the live release.
- A fresh unsigned install can complete the daily cowork loop without hidden
  hosted services or manual file import.
- Release plan, release manifest, `README-FIRST.txt`, release notes, packaged
  smoke proof, and downloaded-asset rehearsal describe the same user path.
- CI remains fast, split, and strict enough to block release-contract drift.

## M13: Connector Platform

Goal: make third-party local connectors safe and useful without building a
marketplace too early.

- Keep Notion as the reference connector until the release loop proves stable.
- Generalize connector contracts only after credentials, approvals, retries,
  proof, memory capture, and timeline evidence stay service-agnostic.
- Prove connector import, local enablement, credential storage, revocation, and
  removal before adding broad service catalogs.
- Keep connector evidence generic in the timeline; service-specific detail must
  stay in connector attributes and narrow presenters.
- Add import/distribution only after connector secrets can be installed, used,
  revoked, and forgotten safely.
- Treat hooks as verification and automation points first, not arbitrary
  always-on automation.

## Later

- Developer ID signing and notarization when an Apple Developer account exists.
- Broader connector ecosystem after the local connector contract is proven.
- Cowork continuity, such as follow-up queues or scheduled work, after the first
  shipped local cowork loop.
- Split oversized plugin execution tests by behavior only if they start slowing
  review or hiding regressions.
- Local document RAG only after Web Search, workspace context, and memory
  ranking are reliable in daily use.
- Native sandbox hardening beyond the current workspace and subprocess
  boundaries.
- Optional bundled Git only if bounded system Git proves insufficient for real
  packaged users.

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
