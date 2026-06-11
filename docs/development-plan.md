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
  approvals, workspace change ledger, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records. Connector
evidence stays generic at protocol and timeline layers; service-specific detail
belongs in connector attributes and narrow presenter adapters.

## Current State

Active milestone: **M12 Public Release**.

Ready foundations:

- Local model setup works through in-app download, verification, activation,
  pause, resume, cancel, recovery, local data reset, and one active model.
- First-run model setup now hides GGUF quantization names from primary UI and
  uses consistent local service/local model language without raw paths or hashes
  in normal recovery copy.
- Workspace cowork loop has workspace-scoped tools, Web Search retrieval,
  approvals, sandbox diagnostics, bounded subprocesses, receipts, and recovery
  evidence.
- Sessions can be deleted without touching workspace files; approved writes feed
  a durable change ledger and can be previewed/reverted per session when files
  still match Pith's recorded writes.
- Connector baseline has local plugin registry, execution gates, credentials,
  retries, generic timeline evidence, memory capture, and Notion as the
  reference connector.
- macOS packaging produces an x86_64 DMG with app bundle metadata, unsigned
  install guidance, package-size checks, release copy, manifest, checksum, and
  packaged smoke proof.
- CI is split by change area and remains the source of truth for Rust, Swift,
  package, policy, model, and release checks.

Current constraints:

- Ship the first useful local cowork loop before adding new surfaces.
- Keep Web Search as retrieval; generic local document RAG remains deferred.
- Keep connector expansion narrow until the release install path is accepted.
- Do not bundle Git, model weights, package-manager payloads, extra
  architectures, or unused runtimes.
- Public release assets stay limited to the DMG, checksum, `README-FIRST.txt`,
  and release manifest.
- Visible ad-hoc prereleases require an explicit manual acceptance receipt.
- The current branch is release-candidate shaped and remote CI is green, but the
  live `v0.1.1` prerelease is not M12-complete because it predates the current
  four-asset release contract.

## M12: Public Release

Goal: ship a usable macOS installer from GitHub Releases.

Build now:

- Keep M12 frozen to release readiness, install confidence, first-run clarity,
  user-facing UI polish, session safety, and local data ownership.
- Cut the next M12 candidate from the latest green branch commit through the
  current release workflow, producing the DMG, checksum, `README-FIRST.txt`,
  and release manifest.
- Run the full first-launch acceptance path from downloaded release assets:
  verify checksum, open DMG, pass Gatekeeper, download the default model, open a
  workspace, run a cowork turn, use Web Search, approve one bounded action,
  inspect proof, and record the manual acceptance receipt.
- Keep normal UI language product-level: session, workspace, local service,
  local model, Web Search, connector, approval, proof, delete, and revert.
  Keep paths, manifests, package details, hashes, and diagnostics behind
  Advanced.
- Publish a draft prerelease from tag push, then make it visible only after the
  downloaded-asset rehearsal, live four-asset validation, and fresh-Mac manual
  acceptance receipt agree.
- Keep release readiness reports explicit that tag-push drafts do not imply a
  trusted or visible release.

Exit when:

- A fresh unsigned install completes the daily cowork loop without hidden hosted
  services or manual model import.
- The GitHub Release exposes exactly the four public assets and final publish
  validation passes against the live release.
- Release plan, release manifest, `README-FIRST.txt`, release notes, packaged
  smoke proof, manual acceptance receipt, and downloaded-asset rehearsal
  describe the same user path.
- CI stays fast, split, and strict enough to block release-contract drift.

## M13: Post-Release Quality and Identity

Goal: make the shipped app feel intentional, maintainable, and worthy of daily
use before expanding the platform surface.

- Remove dead code, unused flows, stale scripts, and release-era scaffolding
  that no longer protects the product.
- Refactor only where architecture becomes clearer: preserve cohesive modules,
  merge accidental splits, and keep domain boundaries obvious.
- Audit the macOS app for user-facing polish: no confusing internal wording,
  no awkward layout, no unnecessary controls in the primary path.
- Design a refined app logo that is simple, distinctive, premium, and tied to
  Pith's local cowork identity.
- Ship the logo as a native macOS icon set with clean small-size readability,
  not just a large marketing image.

## M14: Connector Platform

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
